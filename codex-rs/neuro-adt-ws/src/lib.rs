use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use http::header::{HeaderName, HeaderValue};
use neuro_types::{WsClientConfig, WsMessageEnvelope};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum NeuroWsClientError {
    #[error("websocket handshake failed: {0}")]
    Handshake(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("invalid websocket header `{name}`: {message}")]
    InvalidHeader { name: String, message: String },
    #[error("failed to serialize websocket message: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("websocket connection closed: {reason}")]
    ConnectionClosed { reason: String },
    #[error("response channel dropped before a response was received")]
    ResponseChannelDropped,
    #[error("request timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },
}

type PendingResponse = Result<WsMessageEnvelope<Value>, NeuroWsClientError>;
type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<PendingResponse>>>>;

pub struct NeuroWsClient {
    outbound: mpsc::Sender<Message>,
    pending: PendingMap,
    request_timeout: Duration,
    _reader_task: JoinHandle<()>,
    _writer_task: JoinHandle<()>,
}

impl NeuroWsClient {
    pub async fn connect(config: &WsClientConfig) -> Result<Self, NeuroWsClientError> {
        let mut request = config.url.as_str().into_client_request()?;

        for (name, value) in &config.connect_headers {
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
                NeuroWsClientError::InvalidHeader {
                    name: name.clone(),
                    message: error.to_string(),
                }
            })?;

            let header_value = HeaderValue::from_str(value).map_err(|error| {
                NeuroWsClientError::InvalidHeader {
                    name: name.clone(),
                    message: error.to_string(),
                }
            })?;

            request.headers_mut().insert(header_name, header_value);
        }

        let (websocket, _) = connect_async(request).await?;
        let (writer, reader) = websocket.split();

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (outbound, outbound_rx) = mpsc::channel(64);

        let writer_pending = Arc::clone(&pending);
        let writer_task = tokio::spawn(async move {
            writer_loop(writer, outbound_rx, writer_pending).await;
        });

        let reader_pending = Arc::clone(&pending);
        let reader_task = tokio::spawn(async move {
            reader_loop(reader, reader_pending).await;
        });

        Ok(Self {
            outbound,
            pending,
            request_timeout: Duration::from_secs(config.request_timeout_secs),
            _reader_task: reader_task,
            _writer_task: writer_task,
        })
    }

    pub fn is_connected(&self) -> bool {
        !self.outbound.is_closed()
    }

    pub async fn send_domain_request(
        &self,
        domain: &str,
        action: &str,
        payload: Value,
    ) -> Result<WsMessageEnvelope<Value>, NeuroWsClientError> {
        let request_id = Uuid::new_v4().to_string();
        let envelope = WsMessageEnvelope {
            id: request_id.clone(),
            domain: domain.to_owned(),
            action: action.to_owned(),
            payload,
            ok: None,
            error: None,
        };

        let serialized = serde_json::to_string(&envelope)?;
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(request_id.clone(), sender);

        if self
            .outbound
            .send(Message::Text(serialized.into()))
            .await
            .is_err()
        {
            self.pending.lock().await.remove(&request_id);
            return Err(NeuroWsClientError::ConnectionClosed {
                reason: "outbound channel is closed".to_owned(),
            });
        }

        let result = tokio::time::timeout(self.request_timeout, receiver).await;
        match result {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(NeuroWsClientError::ResponseChannelDropped),
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                Err(NeuroWsClientError::Timeout {
                    timeout_secs: self.request_timeout.as_secs(),
                })
            }
        }
    }
}

async fn writer_loop(
    mut writer: futures::stream::SplitSink<
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
    mut outbound_rx: mpsc::Receiver<Message>,
    pending: PendingMap,
) {
    while let Some(message) = outbound_rx.recv().await {
        if writer.send(message).await.is_err() {
            fail_pending(&pending, "writer failed to send frame").await;
            return;
        }
    }

    if writer.close().await.is_err() {
        fail_pending(&pending, "writer failed to close connection").await;
    }
}

async fn reader_loop(
    mut reader: futures::stream::SplitStream<
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    pending: PendingMap,
) {
    while let Some(frame) = reader.next().await {
        match frame {
            Ok(Message::Text(text)) => {
                dispatch_text(text.as_ref(), &pending).await;
            }
            Ok(Message::Binary(binary)) => {
                if let Ok(text) = String::from_utf8(binary.to_vec()) {
                    dispatch_text(text.as_str(), &pending).await;
                }
            }
            Ok(Message::Close(_)) => {
                fail_pending(&pending, "remote endpoint closed the websocket").await;
                return;
            }
            Ok(_) => {}
            Err(error) => {
                fail_pending(&pending, &format!("reader failure: {error}")).await;
                return;
            }
        }
    }

    fail_pending(&pending, "reader terminated without close frame").await;
}

async fn dispatch_text(raw: &str, pending: &PendingMap) {
    let parsed = serde_json::from_str::<WsMessageEnvelope<Value>>(raw);
    if let Ok(envelope) = parsed {
        let request_id = envelope.id.clone();
        if request_id.is_empty() {
            return;
        }

        if let Some(sender) = pending.lock().await.remove(&request_id) {
            let _ = sender.send(Ok(envelope));
        }
    }
}

async fn fail_pending(pending: &PendingMap, reason: &str) {
    let mut pending_guard = pending.lock().await;
    for (_, sender) in pending_guard.drain() {
        let _ = sender.send(Err(NeuroWsClientError::ConnectionClosed {
            reason: reason.to_owned(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dispatch_text_routes_response_by_id() {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = oneshot::channel();
        pending.lock().await.insert("req-1".to_string(), sender);

        dispatch_text(
            r#"{"id":"req-1","domain":"adt","action":"ping","payload":{"ok":true}}"#,
            &pending,
        )
        .await;

        let response = receiver
            .await
            .expect("response channel should receive value")
            .expect("response should be Ok");
        assert_eq!(response.id, "req-1");
        assert_eq!(response.domain, "adt");
        assert_eq!(response.action, "ping");
    }

    #[tokio::test]
    async fn fail_pending_notifies_all_waiters() {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = oneshot::channel();
        pending.lock().await.insert("req-2".to_string(), sender);

        fail_pending(&pending, "test-close").await;

        let error = receiver
            .await
            .expect("response channel should receive value")
            .expect_err("response should contain error");

        assert!(matches!(error, NeuroWsClientError::ConnectionClosed { .. }));
    }
}
