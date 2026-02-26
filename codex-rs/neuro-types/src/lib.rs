use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtHttpConfig {
    pub base_url: String,
    #[serde(default)]
    pub auth: AdtAuth,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_csrf_fetch_path")]
    pub csrf_fetch_path: String,
    #[serde(default)]
    pub endpoints: AdtHttpEndpoints,
    #[serde(default)]
    pub insecure_tls: bool,
    #[serde(default)]
    pub sap_client: Option<String>,
    #[serde(default)]
    pub sap_language: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdtHttpEndpoints {
    #[serde(default = "default_search_path")]
    pub search_objects_path: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdtAuth {
    Basic {
        username: String,
        password: String,
    },
    Cookie {
        cookie: String,
    },
    #[default]
    Anonymous,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtSearchRequest {
    pub query: String,
    pub max_results: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtSearchResponse {
    pub objects: Vec<AdtObjectSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtObjectSummary {
    pub uri: String,
    pub name: String,
    #[serde(default, alias = "type")]
    pub object_type: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtSourceResponse {
    pub object_uri: String,
    pub source: String,
    #[serde(default)]
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtUpdateSourceRequest {
    pub object_uri: String,
    pub source: String,
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdtUpdateSourceResponse {
    pub object_uri: String,
    pub status_code: u16,
    #[serde(default)]
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsClientConfig {
    pub url: String,
    #[serde(default = "default_ws_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default)]
    pub connect_headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsMessageEnvelope<T> {
    pub id: String,
    pub domain: String,
    pub action: String,
    pub payload: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuroEngineConfig {
    pub adt: AdtHttpConfig,
    #[serde(default)]
    pub ws: Option<WsClientConfig>,
    #[serde(default)]
    pub safety: SafetyPolicy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SafetyPolicy {
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub blocked_source_patterns: Vec<String>,
    #[serde(default)]
    pub allowed_ws_domains: Vec<String>,
    #[serde(default)]
    pub require_etag_for_updates: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnoseStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeDiagnoseComponent {
    pub component: String,
    pub status: DiagnoseStatus,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnoseComponentNameGuardError {
    pub component: String,
    pub blocked_token: &'static str,
}

impl fmt::Display for DiagnoseComponentNameGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "diagnose component `{}` contains blocked legacy token `{}`",
            self.component, self.blocked_token
        )
    }
}

impl std::error::Error for DiagnoseComponentNameGuardError {}

impl RuntimeDiagnoseComponent {
    pub fn try_new(
        component: impl Into<String>,
        status: DiagnoseStatus,
        detail: impl Into<String>,
        latency_ms: Option<u64>,
    ) -> Result<Self, DiagnoseComponentNameGuardError> {
        let component = component.into();
        validate_diagnose_component_name(component.as_str())?;

        Ok(Self {
            component,
            status,
            detail: detail.into(),
            latency_ms,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeDiagnoseResponse {
    pub timestamp_epoch_secs: u64,
    pub overall_status: DiagnoseStatus,
    pub components: Vec<RuntimeDiagnoseComponent>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

impl RuntimeDiagnoseResponse {
    pub fn validate_component_names(&self) -> Result<(), DiagnoseComponentNameGuardError> {
        self.components.iter().try_for_each(|component| {
            validate_diagnose_component_name(component.component.as_str())
        })
    }
}

const LEGACY_DIAGNOSE_COMPONENT_TOKEN: &str = "vsp";

pub fn is_legacy_diagnose_component_name(component: &str) -> bool {
    component
        .to_ascii_lowercase()
        .contains(LEGACY_DIAGNOSE_COMPONENT_TOKEN)
}

pub fn validate_diagnose_component_name(
    component: &str,
) -> Result<(), DiagnoseComponentNameGuardError> {
    if is_legacy_diagnose_component_name(component) {
        Err(DiagnoseComponentNameGuardError {
            component: component.to_owned(),
            blocked_token: LEGACY_DIAGNOSE_COMPONENT_TOKEN,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsDomainRequest {
    pub domain: String,
    pub action: String,
    pub payload: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeuroRuntimeErrorCode {
    AdtHttpError,
    AdtAuthError,
    AdtCsrfError,
    WsTimeout,
    WsUnavailable,
    SafetyViolation,
    RuntimeInitError,
    InvalidArgument,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeuroRuntimeError {
    pub code: NeuroRuntimeErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeuroCommandResponse<T> {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<NeuroRuntimeError>,
}

impl<T> NeuroCommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: NeuroRuntimeError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

const fn default_timeout_secs() -> u64 {
    30
}

fn default_csrf_fetch_path() -> String {
    "/sap/bc/adt".to_owned()
}

fn default_search_path() -> String {
    "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch".to_owned()
}

const fn default_ws_timeout_secs() -> u64 {
    15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_component_name_detection_is_case_insensitive() {
        assert!(is_legacy_diagnose_component_name("vsp_ws"));
        assert!(is_legacy_diagnose_component_name("VSP"));
        assert!(is_legacy_diagnose_component_name("NeuroVspHealth"));
        assert!(!is_legacy_diagnose_component_name("neuro_ws"));
    }

    #[test]
    fn runtime_diagnose_component_constructor_rejects_legacy_name() {
        let result = RuntimeDiagnoseComponent::try_new(
            "vsp_ws",
            DiagnoseStatus::Healthy,
            "legacy component",
            None,
        );

        let error = match result {
            Ok(component) => panic!(
                "expected guard error, got component {}",
                component.component
            ),
            Err(error) => error,
        };

        assert_eq!(
            error,
            DiagnoseComponentNameGuardError {
                component: "vsp_ws".to_owned(),
                blocked_token: LEGACY_DIAGNOSE_COMPONENT_TOKEN,
            }
        );
    }

    #[test]
    fn runtime_diagnose_response_validator_accepts_guarded_names() {
        let adt_component = match RuntimeDiagnoseComponent::try_new(
            "adt_http",
            DiagnoseStatus::Healthy,
            "ok",
            Some(7),
        ) {
            Ok(component) => component,
            Err(error) => panic!("unexpected guard error: {error}"),
        };
        let ws_component = match RuntimeDiagnoseComponent::try_new(
            "neuro_ws",
            DiagnoseStatus::Unavailable,
            "not configured",
            None,
        ) {
            Ok(component) => component,
            Err(error) => panic!("unexpected guard error: {error}"),
        };

        let response = RuntimeDiagnoseResponse {
            timestamp_epoch_secs: 0,
            overall_status: DiagnoseStatus::Healthy,
            components: vec![adt_component, ws_component],
            metadata: BTreeMap::new(),
        };

        assert_eq!(response.validate_component_names(), Ok(()));
    }
}
