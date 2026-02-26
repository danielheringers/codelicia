use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::Duration;

use neuro_types::{
    AdtAuth, AdtHttpConfig, AdtObjectSummary, AdtSearchResponse, AdtSourceResponse,
    AdtUpdateSourceRequest, AdtUpdateSourceResponse,
};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use reqwest::{Method, Response, StatusCode, Url, header};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AdtClient {
    client: reqwest::Client,
    config: AdtHttpConfig,
    base_url: Url,
    csrf_token: Arc<RwLock<Option<String>>>,
}

#[derive(Debug, Error)]
pub enum AdtClientError {
    #[error("invalid base url `{base_url}`: {source}")]
    InvalidBaseUrl {
        base_url: String,
        #[source]
        source: url::ParseError,
    },
    #[error("invalid path `{path}`: {source}")]
    InvalidPath {
        path: String,
        #[source]
        source: url::ParseError,
    },
    #[error("failed to build HTTP client: {message}")]
    ClientBuild { message: String },
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json serialization/deserialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("response body is not valid UTF-8: {0}")]
    Utf8(#[from] FromUtf8Error),
    #[error("missing CSRF token in response headers")]
    MissingCsrfToken,
    #[error("{operation} failed with status {status}: {body}")]
    UnexpectedStatus {
        operation: &'static str,
        status: StatusCode,
        body: String,
    },
    #[error("protocol error: {0}")]
    Protocol(String),
}

impl AdtClient {
    pub fn new(config: AdtHttpConfig) -> Result<Self, AdtClientError> {
        let mut normalized_base = config.base_url.clone();
        if !normalized_base.ends_with('/') {
            normalized_base.push('/');
        }

        let base_url =
            Url::parse(&normalized_base).map_err(|source| AdtClientError::InvalidBaseUrl {
                base_url: normalized_base,
                source,
            })?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(config.insecure_tls)
            .build()
            .map_err(|error| AdtClientError::ClientBuild {
                message: error.to_string(),
            })?;

        Ok(Self {
            client,
            config,
            base_url,
            csrf_token: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn ping(&self) -> Result<(), AdtClientError> {
        let url = self.build_url(&self.config.csrf_fetch_path)?;
        let response = self.send_authenticated(self.client.get(url)).await?;
        let _ = ensure_success("ping", response, &[StatusCode::OK, StatusCode::NO_CONTENT]).await?;
        Ok(())
    }

    pub async fn search_objects(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> Result<AdtSearchResponse, AdtClientError> {
        let mut url = self.build_url(&self.config.endpoints.search_objects_path)?;
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("query", query);
            if let Some(limit) = max_results {
                query_pairs.append_pair("maxResults", &limit.to_string());
            }
        }

        let response = self.send_authenticated(self.client.get(url)).await?;
        let response = ensure_success("search_objects", response, &[StatusCode::OK]).await?;
        let payload = response.bytes().await?;
        parse_search_response_payload(&payload)
    }

    pub async fn get_source(&self, object_uri: &str) -> Result<AdtSourceResponse, AdtClientError> {
        let url = self.build_url(object_uri)?;

        let response = self.send_authenticated(self.client.get(url)).await?;
        let response = ensure_success("get_source", response, &[StatusCode::OK]).await?;

        let etag = header_value_to_string(response.headers(), header::ETAG);
        let is_json = header_value_to_string(response.headers(), header::CONTENT_TYPE)
            .map(|content_type| content_type.contains("json"))
            .unwrap_or(false);

        let body = response.bytes().await?;
        let source = if is_json {
            parse_source_payload(&body)?
        } else {
            String::from_utf8(body.to_vec())?
        };

        Ok(AdtSourceResponse {
            object_uri: object_uri.to_owned(),
            source,
            etag,
        })
    }

    pub async fn get_text(
        &self,
        object_uri: &str,
        accept: Option<&str>,
    ) -> Result<String, AdtClientError> {
        let url = self.build_url(object_uri)?;
        let mut request = self.client.get(url);
        if let Some(accept) = accept {
            request = request.header(header::ACCEPT, accept);
        }

        let response = self.send_authenticated(request).await?;
        let response = ensure_success("get_text", response, &[StatusCode::OK]).await?;
        response_to_string(response).await
    }

    pub async fn post_text(
        &self,
        object_uri: &str,
        body: Option<&str>,
        content_type: Option<&str>,
        accept: Option<&str>,
    ) -> Result<String, AdtClientError> {
        let url = self.build_url(object_uri)?;
        let body = body.map(str::to_owned);
        let content_type = content_type.map(str::to_owned);
        let accept = accept.map(str::to_owned);

        let response = self
            .send_with_csrf_retry(Method::POST, url, move |builder| {
                let mut request = builder;

                if let Some(accept) = accept.as_deref() {
                    request = request.header(header::ACCEPT, accept);
                }
                if let Some(content_type) = content_type.as_deref() {
                    request = request.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = body.clone() {
                    request = request.body(body);
                }

                request
            })
            .await?;

        let response = ensure_success(
            "post_text",
            response,
            &[
                StatusCode::OK,
                StatusCode::CREATED,
                StatusCode::ACCEPTED,
                StatusCode::NO_CONTENT,
            ],
        )
        .await?;
        response_to_string(response).await
    }

    pub async fn put_text(
        &self,
        object_uri: &str,
        body: Option<&str>,
        content_type: Option<&str>,
        accept: Option<&str>,
    ) -> Result<String, AdtClientError> {
        let url = self.build_url(object_uri)?;
        let body = body.map(str::to_owned);
        let content_type = content_type.map(str::to_owned);
        let accept = accept.map(str::to_owned);

        let response = self
            .send_with_csrf_retry(Method::PUT, url, move |builder| {
                let mut request = builder;

                if let Some(accept) = accept.as_deref() {
                    request = request.header(header::ACCEPT, accept);
                }
                if let Some(content_type) = content_type.as_deref() {
                    request = request.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = body.clone() {
                    request = request.body(body);
                }

                request
            })
            .await?;

        let response = ensure_success(
            "put_text",
            response,
            &[
                StatusCode::OK,
                StatusCode::CREATED,
                StatusCode::ACCEPTED,
                StatusCode::NO_CONTENT,
            ],
        )
        .await?;
        response_to_string(response).await
    }

    pub async fn delete_text(
        &self,
        object_uri: &str,
        accept: Option<&str>,
    ) -> Result<String, AdtClientError> {
        let url = self.build_url(object_uri)?;
        let accept = accept.map(str::to_owned);

        let response = self
            .send_with_csrf_retry(Method::DELETE, url, move |builder| {
                let mut request = builder;

                if let Some(accept) = accept.as_deref() {
                    request = request.header(header::ACCEPT, accept);
                }

                request
            })
            .await?;

        let response = ensure_success(
            "delete_text",
            response,
            &[
                StatusCode::OK,
                StatusCode::ACCEPTED,
                StatusCode::NO_CONTENT,
            ],
        )
        .await?;
        response_to_string(response).await
    }

    pub async fn update_source(
        &self,
        request: &AdtUpdateSourceRequest,
    ) -> Result<AdtUpdateSourceResponse, AdtClientError> {
        let url = self.build_url(&request.object_uri)?;
        let source = request.source.clone();
        let etag = request.etag.clone();

        let response = self
            .send_with_csrf_retry(Method::PUT, url, move |builder| {
                let mut request_builder = builder
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(source.clone());

                if let Some(etag_value) = etag.as_deref() {
                    request_builder = request_builder.header(header::IF_MATCH, etag_value);
                }

                request_builder
            })
            .await?;

        let response = ensure_success(
            "update_source",
            response,
            &[StatusCode::OK, StatusCode::CREATED, StatusCode::NO_CONTENT],
        )
        .await?;

        let status_code = response.status().as_u16();
        let etag = header_value_to_string(response.headers(), header::ETAG);

        Ok(AdtUpdateSourceResponse {
            object_uri: request.object_uri.clone(),
            status_code,
            etag,
        })
    }

    fn build_url(&self, path: &str) -> Result<Url, AdtClientError> {
        let mut url = if path.starts_with("http://") || path.starts_with("https://") {
            Url::parse(path).map_err(|source| AdtClientError::InvalidPath {
                path: path.to_owned(),
                source,
            })?
        } else {
            let trimmed = path.trim_start_matches('/');
            self.base_url
                .join(trimmed)
                .map_err(|source| AdtClientError::InvalidPath {
                    path: path.to_owned(),
                    source,
                })?
        };

        self.apply_sap_query_params(&mut url);
        Ok(url)
    }

    fn apply_sap_query_params(&self, url: &mut Url) {
        let mut query_pairs = url.query_pairs_mut();
        if let Some(client) = self.config.sap_client.as_deref() {
            query_pairs.append_pair("sap-client", client);
        }
        if let Some(language) = self.config.sap_language.as_deref() {
            query_pairs.append_pair("sap-language", language);
        }
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth {
            AdtAuth::Basic { username, password } => {
                builder.basic_auth(username.as_str(), Some(password.as_str()))
            }
            AdtAuth::Cookie { cookie } => builder.header(header::COOKIE, cookie.as_str()),
            AdtAuth::Anonymous => builder,
        }
    }

    async fn send_authenticated(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<Response, AdtClientError> {
        let response = self.apply_auth(builder).send().await?;
        Ok(response)
    }

    async fn send_with_csrf_retry<F>(
        &self,
        method: Method,
        url: Url,
        mut build: F,
    ) -> Result<Response, AdtClientError>
    where
        F: FnMut(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let cached_token = { self.csrf_token.read().await.clone() };

        let initial_response = self
            .send_with_optional_csrf(
                method.clone(),
                url.clone(),
                cached_token.as_deref(),
                &mut build,
            )
            .await?;

        if !should_retry_with_csrf(&initial_response) {
            return Ok(initial_response);
        }

        let refreshed_token = self.fetch_csrf_token().await?;
        {
            let mut csrf_token = self.csrf_token.write().await;
            *csrf_token = Some(refreshed_token.clone());
        }

        self.send_with_optional_csrf(method, url, Some(refreshed_token.as_str()), &mut build)
            .await
    }

    async fn send_with_optional_csrf<F>(
        &self,
        method: Method,
        url: Url,
        csrf_token: Option<&str>,
        build: &mut F,
    ) -> Result<Response, AdtClientError>
    where
        F: FnMut(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let mut builder = self.client.request(method, url);
        builder = self.apply_auth(builder);

        if let Some(token) = csrf_token {
            builder = builder.header("x-csrf-token", token);
        }

        let builder = build(builder);
        let response = builder.send().await?;
        Ok(response)
    }

    async fn fetch_csrf_token(&self) -> Result<String, AdtClientError> {
        let url = self.build_url(&self.config.csrf_fetch_path)?;

        let response = self
            .send_authenticated(self.client.get(url).header("x-csrf-token", "fetch"))
            .await?;
        let response = ensure_success(
            "fetch_csrf_token",
            response,
            &[StatusCode::OK, StatusCode::NO_CONTENT],
        )
        .await?;

        let token = header_value_to_string(response.headers(), "x-csrf-token")
            .filter(|value| !value.is_empty())
            .ok_or(AdtClientError::MissingCsrfToken)?;

        Ok(token)
    }
}

fn parse_search_response(payload: Value) -> Result<AdtSearchResponse, AdtClientError> {
    if payload.is_array() {
        let objects: Vec<AdtObjectSummary> = serde_json::from_value(payload)?;
        return Ok(AdtSearchResponse { objects });
    }

    let response: SearchPayload = serde_json::from_value(payload)?;
    Ok(AdtSearchResponse {
        objects: response.objects,
    })
}

fn parse_search_response_payload(payload: &[u8]) -> Result<AdtSearchResponse, AdtClientError> {
    let json_result = match serde_json::from_slice::<Value>(payload) {
        Ok(value) => parse_search_response(value),
        Err(error) => Err(AdtClientError::Json(error)),
    };
    if let Ok(parsed) = json_result {
        return Ok(parsed);
    }
    let json_error = json_result.err().expect("json_result should contain error");

    let xml_result = parse_search_xml_response(payload);
    if let Ok(parsed) = xml_result {
        return Ok(parsed);
    }
    let xml_error = xml_result.err().expect("xml_result should contain error");

    Err(AdtClientError::Protocol(format!(
        "search payload is neither JSON nor ADT XML (json error: {json_error}; xml error: {xml_error})"
    )))
}

fn parse_search_xml_response(payload: &[u8]) -> Result<AdtSearchResponse, AdtClientError> {
    let mut reader = Reader::from_reader(payload);
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut objects = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                if element.local_name().as_ref() == b"objectReference" {
                    if let Some(object) = parse_xml_object_reference(&element)? {
                        objects.push(object);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(AdtClientError::Protocol(format!(
                    "failed to parse ADT XML search response: {error}"
                )));
            }
        }

        buffer.clear();
    }

    Ok(AdtSearchResponse { objects })
}

fn parse_xml_object_reference(
    element: &BytesStart<'_>,
) -> Result<Option<AdtObjectSummary>, AdtClientError> {
    let mut uri: Option<String> = None;
    let mut name: Option<String> = None;
    let mut object_type: Option<String> = None;
    let mut package: Option<String> = None;

    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| {
            AdtClientError::Protocol(format!("invalid ADT XML attribute in search response: {error}"))
        })?;

        let local_key = xml_local_name(attribute.key.as_ref());
        let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
        let normalized = if value.trim().is_empty() {
            None
        } else {
            Some(value)
        };

        match local_key {
            b"uri" => uri = normalized,
            b"name" => name = normalized,
            b"type" => object_type = normalized,
            b"packageName" => package = normalized,
            _ => {}
        }
    }

    let Some(uri) = uri else {
        return Ok(None);
    };
    let Some(name) = name else {
        return Ok(None);
    };

    Ok(Some(AdtObjectSummary {
        uri,
        name,
        object_type,
        package,
    }))
}

fn xml_local_name(value: &[u8]) -> &[u8] {
    value.rsplit(|byte| *byte == b':').next().unwrap_or(value)
}

fn parse_source_payload(payload: &[u8]) -> Result<String, AdtClientError> {
    let value: Value = serde_json::from_slice(payload)?;

    if let Some(source) = extract_source(&value) {
        return Ok(source);
    }

    Err(AdtClientError::Protocol(
        "source payload did not include a `source`/`content` field".to_owned(),
    ))
}

fn extract_source(value: &Value) -> Option<String> {
    if let Some(source) = value.get("source").and_then(Value::as_str) {
        return Some(source.to_owned());
    }

    if let Some(source) = value.get("content").and_then(Value::as_str) {
        return Some(source.to_owned());
    }

    value.get("data").and_then(extract_source)
}

fn header_value_to_string(
    headers: &reqwest::header::HeaderMap,
    name: impl reqwest::header::AsHeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn should_retry_with_csrf(response: &Response) -> bool {
    if response.status() != StatusCode::FORBIDDEN {
        return false;
    }

    header_value_to_string(response.headers(), "x-csrf-token")
        .map(|value| value.eq_ignore_ascii_case("required"))
        .unwrap_or(false)
}

async fn ensure_success(
    operation: &'static str,
    response: Response,
    accepted_statuses: &[StatusCode],
) -> Result<Response, AdtClientError> {
    if accepted_statuses
        .iter()
        .any(|status| *status == response.status())
    {
        return Ok(response);
    }

    let status = response.status();
    let body: String = response.text().await.unwrap_or_default();

    Err(AdtClientError::UnexpectedStatus {
        operation,
        status,
        body,
    })
}

async fn response_to_string(response: Response) -> Result<String, AdtClientError> {
    let body = response.bytes().await?;
    if body.is_empty() {
        return Ok(String::new());
    }
    String::from_utf8(body.to_vec()).map_err(Into::into)
}

#[derive(Debug, Deserialize)]
struct SearchPayload {
    #[serde(default, alias = "results", alias = "items")]
    objects: Vec<AdtObjectSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    struct MissingHeader(&'static str);

    impl Match for MissingHeader {
        fn matches(&self, request: &Request) -> bool {
            !request
                .headers
                .keys()
                .any(|name| name.as_str().eq_ignore_ascii_case(self.0))
        }
    }

    fn test_client(base_url: String) -> AdtClient {
        AdtClient::new(AdtHttpConfig {
            base_url,
            auth: AdtAuth::Anonymous,
            timeout_secs: 5,
            csrf_fetch_path: "/csrf".to_string(),
            endpoints: neuro_types::AdtHttpEndpoints {
                search_objects_path: "/search".to_string(),
            },
            insecure_tls: false,
            sap_client: None,
            sap_language: None,
        })
        .expect("client should build")
    }

    #[test]
    fn build_url_appends_sap_query_parameters() {
        let client = AdtClient::new(AdtHttpConfig {
            base_url: "https://example.sap".to_string(),
            auth: AdtAuth::Anonymous,
            timeout_secs: 5,
            csrf_fetch_path: "/sap/bc/adt".to_string(),
            endpoints: neuro_types::AdtHttpEndpoints {
                search_objects_path:
                    "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch"
                        .to_string(),
            },
            insecure_tls: false,
            sap_client: Some("100".to_string()),
            sap_language: Some("EN".to_string()),
        })
        .expect("client should build");

        let url = client
            .build_url("/sap/bc/adt/repository/informationsystem/search?operation=quickSearch")
            .expect("url should be built");
        let query = url.query().unwrap_or_default();

        assert!(query.contains("sap-client=100"));
        assert!(query.contains("sap-language=EN"));
    }

    #[test]
    fn parse_search_response_accepts_array_payloads() {
        let payload = serde_json::json!([
            {
                "uri": "/sap/bc/adt/packages/zpkg",
                "name": "ZPKG"
            }
        ]);
        let parsed = parse_search_response(payload).expect("search response should parse");
        assert_eq!(parsed.objects.len(), 1);
        assert_eq!(parsed.objects[0].name, "ZPKG");
    }

    #[test]
    fn parse_search_response_payload_accepts_adt_xml() {
        let payload = r#"
            <adtcore:objectReferences xmlns:adtcore="http://www.sap.com/adt/core">
                <adtcore:objectReference
                    adtcore:uri="/sap/bc/adt/programs/programs/ztest_program"
                    adtcore:type="PROG/P"
                    adtcore:name="ZTEST_PROGRAM"
                    adtcore:packageName="ZTEST" />
            </adtcore:objectReferences>
        "#;

        let parsed = parse_search_response_payload(payload.as_bytes())
            .expect("search XML response should parse");

        assert_eq!(parsed.objects.len(), 1);
        assert_eq!(parsed.objects[0].name, "ZTEST_PROGRAM");
        assert_eq!(parsed.objects[0].object_type.as_deref(), Some("PROG/P"));
        assert_eq!(parsed.objects[0].package.as_deref(), Some("ZTEST"));
    }

    #[tokio::test]
    async fn search_objects_integration_includes_query_and_parses_payload() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("query", "ZCL_NEURO"))
            .and(query_param("maxResults", "7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "objects": [
                    {
                        "uri": "/sap/bc/adt/classes/zcl_neuro",
                        "name": "ZCL_NEURO",
                        "type": "CLAS"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let result = client
            .search_objects("ZCL_NEURO", Some(7))
            .await
            .expect("search should succeed");

        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.objects[0].name, "ZCL_NEURO");
        assert_eq!(result.objects[0].object_type.as_deref(), Some("CLAS"));
    }

    #[tokio::test]
    async fn search_objects_integration_parses_xml_payload() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("query", "ZTEST*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/xml")
                    .set_body_string(
                        r#"<adtcore:objectReferences xmlns:adtcore="http://www.sap.com/adt/core">
                                <adtcore:objectReference
                                    adtcore:uri="/sap/bc/adt/programs/programs/ztest_program"
                                    adtcore:type="PROG/P"
                                    adtcore:name="ZTEST_PROGRAM"
                                    adtcore:packageName="ZTEST"/>
                            </adtcore:objectReferences>"#,
                    ),
            )
            .mount(&server)
            .await;

        let result = client
            .search_objects("ZTEST*", None)
            .await
            .expect("XML search should succeed");

        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.objects[0].name, "ZTEST_PROGRAM");
        assert_eq!(result.objects[0].object_type.as_deref(), Some("PROG/P"));
        assert_eq!(result.objects[0].package.as_deref(), Some("ZTEST"));
    }

    #[tokio::test]
    async fn update_source_retries_with_csrf_token() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("PUT"))
            .and(path("/object/source"))
            .and(MissingHeader("x-csrf-token"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-csrf-token", "required")
                    .set_body_string("csrf token missing"),
            )
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/csrf"))
            .and(header("x-csrf-token", "fetch"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-csrf-token", "token-123"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/object/source"))
            .and(header("x-csrf-token", "token-123"))
            .respond_with(ResponseTemplate::new(204).insert_header("etag", "\"v2\""))
            .expect(1)
            .mount(&server)
            .await;

        let result = client
            .update_source(&AdtUpdateSourceRequest {
                object_uri: "/object/source".to_string(),
                source: "REPORT z_neuro.".to_string(),
                etag: Some("\"v1\"".to_string()),
            })
            .await
            .expect("update_source should succeed after csrf retry");

        assert_eq!(result.status_code, 204);
        assert_eq!(result.etag.as_deref(), Some("\"v2\""));
    }

    #[tokio::test]
    async fn post_text_retries_with_csrf_token() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("POST"))
            .and(path("/object/action"))
            .and(MissingHeader("x-csrf-token"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-csrf-token", "required")
                    .set_body_string("csrf token missing"),
            )
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/csrf"))
            .and(header("x-csrf-token", "fetch"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-csrf-token", "token-123"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/object/action"))
            .and(header("x-csrf-token", "token-123"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;

        let result = client
            .post_text(
                "/object/action",
                Some("<x/>"),
                Some("application/xml"),
                Some("application/xml"),
            )
            .await
            .expect("post_text should succeed after csrf retry");
        assert_eq!(result, "ok");
    }

    #[tokio::test]
    async fn put_text_retries_with_csrf_token() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("PUT"))
            .and(path("/object/action"))
            .and(MissingHeader("x-csrf-token"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-csrf-token", "required")
                    .set_body_string("csrf token missing"),
            )
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/csrf"))
            .and(header("x-csrf-token", "fetch"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-csrf-token", "token-123"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/object/action"))
            .and(header("x-csrf-token", "token-123"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;

        let result = client
            .put_text(
                "/object/action",
                Some("<x/>"),
                Some("application/xml"),
                Some("application/xml"),
            )
            .await
            .expect("put_text should succeed after csrf retry");
        assert_eq!(result, "ok");
    }

    #[tokio::test]
    async fn delete_text_retries_with_csrf_token() {
        let server = MockServer::start().await;
        let client = test_client(server.uri());

        Mock::given(method("DELETE"))
            .and(path("/object/action"))
            .and(MissingHeader("x-csrf-token"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-csrf-token", "required")
                    .set_body_string("csrf token missing"),
            )
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/csrf"))
            .and(header("x-csrf-token", "fetch"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-csrf-token", "token-123"))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path("/object/action"))
            .and(header("x-csrf-token", "token-123"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;

        let result = client
            .delete_text("/object/action", Some("application/xml"))
            .await
            .expect("delete_text should succeed after csrf retry");
        assert_eq!(result, "ok");
    }
}
