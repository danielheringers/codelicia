use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use neuro_adt_core::{AdtClient, AdtClientError};
use neuro_adt_ws::{NeuroWsClient, NeuroWsClientError};
use neuro_types::{
    AdtObjectSummary, AdtSourceResponse, AdtUpdateSourceRequest, AdtUpdateSourceResponse,
    DiagnoseStatus, NeuroEngineConfig, RuntimeDiagnoseComponent, RuntimeDiagnoseResponse,
    WsMessageEnvelope,
};
use serde_json::{Value, json};
use thiserror::Error;

pub struct NeuroEngine {
    adt: AdtClient,
    ws: Option<Arc<NeuroWsClient>>,
    safety: neuro_types::SafetyPolicy,
}

#[derive(Debug, Error)]
pub enum NeuroEngineError {
    #[error("ADT client error: {0}")]
    Adt(#[from] AdtClientError),
    #[error("WebSocket client error: {0}")]
    Ws(#[from] NeuroWsClientError),
    #[error("WebSocket client is not configured")]
    WsUnavailable,
    #[error("safety policy violation: {0}")]
    SafetyViolation(String),
}

impl NeuroEngine {
    pub async fn new(config: NeuroEngineConfig) -> Result<Self, NeuroEngineError> {
        let NeuroEngineConfig { adt, ws, safety } = config;
        let adt_client = AdtClient::new(adt)?;

        let ws_client = match ws {
            Some(ws_config) => Some(Arc::new(NeuroWsClient::connect(&ws_config).await?)),
            None => None,
        };

        Ok(Self {
            adt: adt_client,
            ws: ws_client,
            safety,
        })
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> Result<Vec<AdtObjectSummary>, NeuroEngineError> {
        let response = self.adt.search_objects(query, max_results).await?;
        Ok(response.objects)
    }

    pub async fn get_source(
        &self,
        object_uri: &str,
    ) -> Result<AdtSourceResponse, NeuroEngineError> {
        self.adt.get_source(object_uri).await.map_err(Into::into)
    }

    pub async fn update_source(
        &self,
        request: AdtUpdateSourceRequest,
    ) -> Result<AdtUpdateSourceResponse, NeuroEngineError> {
        self.enforce_source_update_policy(&request)?;
        self.adt.update_source(&request).await.map_err(Into::into)
    }

    pub async fn send_domain_request(
        &self,
        domain: &str,
        action: &str,
        payload: Value,
    ) -> Result<WsMessageEnvelope<Value>, NeuroEngineError> {
        self.enforce_domain_policy(domain)?;
        let ws = self.ws.as_ref().ok_or(NeuroEngineError::WsUnavailable)?;
        ws.send_domain_request(domain, action, payload)
            .await
            .map_err(Into::into)
    }

    pub async fn diagnose(&self) -> RuntimeDiagnoseResponse {
        let adt_started = Instant::now();
        let adt_ping = self.adt.ping().await;
        let adt_latency_ms = u64::try_from(adt_started.elapsed().as_millis()).ok();

        let mut components = vec![match adt_ping {
            Ok(()) => guarded_runtime_diagnose_component(
                "adt_http",
                DiagnoseStatus::Healthy,
                "ADT HTTP endpoint reachable",
                adt_latency_ms,
            ),
            Err(error) => guarded_runtime_diagnose_component(
                "adt_http",
                DiagnoseStatus::Degraded,
                error.to_string(),
                adt_latency_ms,
            ),
        }];

        let ws_component = match &self.ws {
            Some(client) if client.is_connected() => guarded_runtime_diagnose_component(
                "neuro_ws",
                DiagnoseStatus::Healthy,
                "WebSocket channel is connected",
                None,
            ),
            Some(_) => guarded_runtime_diagnose_component(
                "neuro_ws",
                DiagnoseStatus::Degraded,
                "WebSocket channel exists but is disconnected",
                None,
            ),
            None => guarded_runtime_diagnose_component(
                "neuro_ws",
                DiagnoseStatus::Unavailable,
                "WebSocket is not configured",
                None,
            ),
        };
        components.push(ws_component);

        components.push(guarded_runtime_diagnose_component(
            "safety_policy",
            DiagnoseStatus::Healthy,
            format!(
                "read_only={}, blocked_patterns={}, domain_whitelist={}, require_etag_for_updates={}",
                self.safety.read_only,
                self.safety.blocked_source_patterns.len(),
                self.safety.allowed_ws_domains.len(),
                self.safety.require_etag_for_updates
            ),
            None,
        ));

        let overall_status =
            components
                .iter()
                .fold(DiagnoseStatus::Healthy, |current, component| {
                    if component.status > current {
                        component.status
                    } else {
                        current
                    }
                });

        let timestamp_epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        let mut metadata = BTreeMap::new();
        metadata.insert("policy_read_only".to_owned(), json!(self.safety.read_only));
        metadata.insert(
            "blocked_patterns".to_owned(),
            json!(self.safety.blocked_source_patterns),
        );
        metadata.insert(
            "allowed_ws_domains".to_owned(),
            json!(self.safety.allowed_ws_domains),
        );
        metadata.insert(
            "require_etag_for_updates".to_owned(),
            json!(self.safety.require_etag_for_updates),
        );

        let report = RuntimeDiagnoseResponse {
            timestamp_epoch_secs,
            overall_status,
            components,
            metadata,
        };
        debug_assert!(report.validate_component_names().is_ok());
        report
    }

    fn enforce_source_update_policy(
        &self,
        request: &AdtUpdateSourceRequest,
    ) -> Result<(), NeuroEngineError> {
        if self.safety.read_only {
            return Err(NeuroEngineError::SafetyViolation(
                "updates are disabled by read_only safety policy".to_owned(),
            ));
        }

        if self.safety.require_etag_for_updates && request.etag.is_none() {
            return Err(NeuroEngineError::SafetyViolation(
                "updates require an etag but request.etag was not provided".to_owned(),
            ));
        }

        if let Some(pattern) = self
            .safety
            .blocked_source_patterns
            .iter()
            .find(|pattern| !pattern.is_empty() && request.source.contains(pattern.as_str()))
        {
            return Err(NeuroEngineError::SafetyViolation(format!(
                "source update blocked due to forbidden pattern `{pattern}`"
            )));
        }

        Ok(())
    }

    fn enforce_domain_policy(&self, domain: &str) -> Result<(), NeuroEngineError> {
        if self.safety.allowed_ws_domains.is_empty() {
            return Ok(());
        }

        let is_allowed = self
            .safety
            .allowed_ws_domains
            .iter()
            .any(|allowed_domain| allowed_domain == domain);

        if is_allowed {
            Ok(())
        } else {
            Err(NeuroEngineError::SafetyViolation(format!(
                "domain `{domain}` is not in allowed_ws_domains"
            )))
        }
    }
}

fn guarded_runtime_diagnose_component(
    component: &str,
    status: DiagnoseStatus,
    detail: impl Into<String>,
    latency_ms: Option<u64>,
) -> RuntimeDiagnoseComponent {
    match RuntimeDiagnoseComponent::try_new(component, status, detail, latency_ms) {
        Ok(component) => component,
        Err(error) => RuntimeDiagnoseComponent {
            component: "diagnose_naming_guard".to_owned(),
            status: DiagnoseStatus::Unavailable,
            detail: error.to_string(),
            latency_ms: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuro_types::{AdtAuth, AdtHttpConfig, AdtHttpEndpoints};
    use std::env;

    fn build_engine_with_policy(safety: neuro_types::SafetyPolicy) -> NeuroEngine {
        let adt = AdtClient::new(AdtHttpConfig {
            base_url: "http://127.0.0.1:18080".to_string(),
            auth: AdtAuth::Anonymous,
            timeout_secs: 1,
            csrf_fetch_path: "/sap/bc/adt".to_string(),
            endpoints: AdtHttpEndpoints {
                search_objects_path:
                    "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch"
                        .to_string(),
            },
            insecure_tls: false,
            sap_client: None,
            sap_language: None,
        })
        .expect("adt client should build");

        NeuroEngine {
            adt,
            ws: None,
            safety,
        }
    }

    #[test]
    fn source_update_policy_blocks_when_read_only() {
        let engine = build_engine_with_policy(neuro_types::SafetyPolicy {
            read_only: true,
            blocked_source_patterns: Vec::new(),
            allowed_ws_domains: Vec::new(),
            require_etag_for_updates: false,
        });

        let result = engine.enforce_source_update_policy(&AdtUpdateSourceRequest {
            object_uri: "/sap/bc/adt/programs/programs/z_foo".to_string(),
            source: "REPORT z_foo.".to_string(),
            etag: Some("\"1\"".to_string()),
        });

        assert!(matches!(result, Err(NeuroEngineError::SafetyViolation(_))));
    }

    #[test]
    fn source_update_policy_requires_etag_when_enabled() {
        let engine = build_engine_with_policy(neuro_types::SafetyPolicy {
            read_only: false,
            blocked_source_patterns: Vec::new(),
            allowed_ws_domains: Vec::new(),
            require_etag_for_updates: true,
        });

        let result = engine.enforce_source_update_policy(&AdtUpdateSourceRequest {
            object_uri: "/sap/bc/adt/programs/programs/z_foo".to_string(),
            source: "REPORT z_foo.".to_string(),
            etag: None,
        });

        assert!(matches!(result, Err(NeuroEngineError::SafetyViolation(_))));
    }

    #[test]
    fn domain_policy_blocks_non_whitelisted_domain() {
        let engine = build_engine_with_policy(neuro_types::SafetyPolicy {
            read_only: false,
            blocked_source_patterns: Vec::new(),
            allowed_ws_domains: vec!["adt".to_string()],
            require_etag_for_updates: false,
        });

        let result = engine.enforce_domain_policy("not-allowed");
        assert!(matches!(result, Err(NeuroEngineError::SafetyViolation(_))));
    }

    #[tokio::test]
    async fn diagnose_component_names_do_not_use_legacy_vsp_prefix() {
        let engine = build_engine_with_policy(neuro_types::SafetyPolicy {
            read_only: false,
            blocked_source_patterns: Vec::new(),
            allowed_ws_domains: Vec::new(),
            require_etag_for_updates: false,
        });

        let report = engine.diagnose().await;
        assert_eq!(report.validate_component_names(), Ok(()));
        assert!(
            report
                .components
                .iter()
                .all(|component| !component.component.contains("vsp"))
        );
        assert!(
            report
                .components
                .iter()
                .any(|component| component.component == "neuro_ws")
        );
    }

    #[test]
    fn diagnose_component_guard_blocks_legacy_name() {
        let component = guarded_runtime_diagnose_component(
            "vsp_ws",
            DiagnoseStatus::Healthy,
            "legacy name",
            Some(4),
        );

        assert_eq!(component.component, "diagnose_naming_guard");
        assert_eq!(component.status, DiagnoseStatus::Unavailable);
        assert_eq!(
            component.detail,
            "diagnose component `vsp_ws` contains blocked legacy token `vsp`"
        );
        assert_eq!(component.latency_ms, None);
    }

    #[tokio::test]
    #[ignore = "requires real ADT provider env/secrets"]
    async fn real_provider_smoke_diagnose_reports_healthy_adt() {
        let base_url = match env::var("NEURO_SMOKE_ADT_BASE_URL") {
            Ok(value) if !value.is_empty() => value,
            _ => return,
        };

        let auth = match (
            env::var("NEURO_SMOKE_ADT_USERNAME").ok(),
            env::var("NEURO_SMOKE_ADT_PASSWORD").ok(),
            env::var("NEURO_SMOKE_ADT_COOKIE").ok(),
        ) {
            (Some(username), Some(password), _) => AdtAuth::Basic { username, password },
            (_, _, Some(cookie)) => AdtAuth::Cookie { cookie },
            _ => return,
        };

        let config = NeuroEngineConfig {
            adt: AdtHttpConfig {
                base_url,
                auth,
                timeout_secs: env::var("NEURO_SMOKE_ADT_TIMEOUT_SECS")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(30),
                csrf_fetch_path: env::var("NEURO_SMOKE_ADT_CSRF_FETCH_PATH")
                    .unwrap_or_else(|_| "/sap/bc/adt".to_owned()),
                endpoints: AdtHttpEndpoints {
                    search_objects_path: env::var("NEURO_SMOKE_ADT_SEARCH_PATH").unwrap_or_else(
                        |_| {
                            "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch"
                                .to_owned()
                        },
                    ),
                },
                insecure_tls: env::var("NEURO_SMOKE_ADT_INSECURE_TLS")
                    .ok()
                    .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true")),
                sap_client: env::var("NEURO_SMOKE_ADT_SAP_CLIENT").ok(),
                sap_language: env::var("NEURO_SMOKE_ADT_SAP_LANGUAGE").ok(),
            },
            ws: None,
            safety: neuro_types::SafetyPolicy::default(),
        };

        let engine = NeuroEngine::new(config)
            .await
            .expect("real-provider smoke engine should initialize");
        let report = engine.diagnose().await;

        assert_eq!(report.validate_component_names(), Ok(()));
        assert_eq!(
            report
                .components
                .iter()
                .find(|component| component.component == "adt_http")
                .map(|component| component.status),
            Some(DiagnoseStatus::Healthy)
        );
    }
}
