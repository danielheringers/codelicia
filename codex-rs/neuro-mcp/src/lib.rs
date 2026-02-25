use std::collections::BTreeMap;
use std::sync::Arc;

use neuro_engine::{NeuroEngine, NeuroEngineError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Clone, Debug, Serialize)]
pub struct NeuroToolSpec {
    pub name: String,
    pub description: String,
}

pub struct NeuroMcpFacade {
    engine: Arc<NeuroEngine>,
    registry: BTreeMap<String, NeuroToolSpec>,
}

#[derive(Debug, Error)]
pub enum NeuroMcpError {
    #[error("unknown tool `{0}`")]
    UnknownTool(String),
    #[error("invalid arguments for `{tool}`: {message}")]
    InvalidArguments { tool: String, message: String },
    #[error("engine error: {0}")]
    Engine(#[from] NeuroEngineError),
    #[error("failed to serialize tool response: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl NeuroMcpFacade {
    pub fn new(engine: Arc<NeuroEngine>) -> Self {
        let mut registry = BTreeMap::new();
        registry.insert(
            "diagnose".to_owned(),
            NeuroToolSpec {
                name: "diagnose".to_owned(),
                description: "Run a runtime health diagnosis for ADT, WS, and safety policy"
                    .to_owned(),
            },
        );
        registry.insert(
            "search".to_owned(),
            NeuroToolSpec {
                name: "search".to_owned(),
                description: "Search ADT objects by free-text query".to_owned(),
            },
        );

        Self { engine, registry }
    }

    pub fn list_tools(&self) -> Vec<NeuroToolSpec> {
        self.registry.values().cloned().collect()
    }

    pub async fn invoke(&self, tool_name: &str, arguments: Value) -> Result<Value, NeuroMcpError> {
        match tool_name {
            "diagnose" => {
                let report = self.engine.diagnose().await;
                serde_json::to_value(report).map_err(Into::into)
            }
            "search" => {
                let args: SearchArgs = serde_json::from_value(arguments).map_err(|error| {
                    NeuroMcpError::InvalidArguments {
                        tool: "search".to_owned(),
                        message: error.to_string(),
                    }
                })?;

                let objects = self
                    .engine
                    .search(args.query.as_str(), args.max_results)
                    .await?;

                Ok(json!({
                    "objects": objects,
                }))
            }
            _ => Err(NeuroMcpError::UnknownTool(tool_name.to_owned())),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    max_results: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuro_types::{AdtAuth, AdtHttpConfig, AdtHttpEndpoints, NeuroEngineConfig, SafetyPolicy};

    async fn build_facade() -> NeuroMcpFacade {
        let engine = NeuroEngine::new(NeuroEngineConfig {
            adt: AdtHttpConfig {
                base_url: "http://127.0.0.1:18080".to_string(),
                auth: AdtAuth::Anonymous,
                timeout_secs: 1,
                csrf_fetch_path: "/sap/bc/adt".to_string(),
                endpoints: AdtHttpEndpoints {
                    search_objects_path: "/sap/bc/adt/discovery/search".to_string(),
                },
                insecure_tls: false,
                sap_client: None,
                sap_language: None,
            },
            ws: None,
            safety: SafetyPolicy {
                read_only: false,
                blocked_source_patterns: Vec::new(),
                allowed_ws_domains: Vec::new(),
                require_etag_for_updates: false,
            },
        })
        .await
        .expect("engine should build");

        NeuroMcpFacade::new(Arc::new(engine))
    }

    #[tokio::test]
    async fn list_tools_contains_diagnose_and_search() {
        let facade = build_facade().await;
        let tools = facade.list_tools();
        assert!(tools.iter().any(|tool| tool.name == "diagnose"));
        assert!(tools.iter().any(|tool| tool.name == "search"));
    }

    #[tokio::test]
    async fn invoke_unknown_tool_returns_error() {
        let facade = build_facade().await;
        let error = facade
            .invoke("unknown", json!({}))
            .await
            .expect_err("unknown tool should fail");
        assert!(matches!(error, NeuroMcpError::UnknownTool(_)));
    }
}
