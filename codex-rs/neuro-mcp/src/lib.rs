use std::collections::BTreeMap;
use std::sync::Arc;

use neuro_engine::{NeuroEngine, NeuroEngineError};
use neuro_types::AdtUpdateSourceRequest;
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
    #[error("tool `{tool}` is mapped in parity catalog but not implemented in neuro-mcp yet")]
    UnsupportedTool { tool: String },
    #[error("invalid arguments for `{tool}`: {message}")]
    InvalidArguments { tool: String, message: String },
    #[error("engine error: {0}")]
    Engine(#[from] NeuroEngineError),
    #[error("failed to serialize tool response: {0}")]
    Serialize(#[from] serde_json::Error),
}

// Source of truth: vibing-steampunk/cmd/vsp/config_cmd.go::GetAllToolNames().
const VBS_TOOL_NAMES: &[&str] = &[
    "GetSource",
    "GetProgram",
    "GetClass",
    "GetInterface",
    "GetFunction",
    "GetFunctionGroup",
    "GetInclude",
    "GetTable",
    "GetTableContents",
    "GetStructure",
    "GetPackage",
    "GetMessages",
    "GetTransaction",
    "GetTypeInfo",
    "GetClassInfo",
    "GetClassComponents",
    "GetClassInclude",
    "GetCDSDependencies",
    "WriteSource",
    "WriteClass",
    "WriteProgram",
    "EditSource",
    "UpdateSource",
    "CreateObject",
    "DeleteObject",
    "CloneObject",
    "RenameObject",
    "MoveObject",
    "LockObject",
    "UnlockObject",
    "SearchObject",
    "GrepObjects",
    "GrepPackages",
    "GrepObject",
    "GrepPackage",
    "SyntaxCheck",
    "Activate",
    "ActivatePackage",
    "PrettyPrint",
    "GetPrettyPrinterSettings",
    "SetPrettyPrinterSettings",
    "RunUnitTests",
    "RunATCCheck",
    "GetATCCustomizing",
    "GetInactiveObjects",
    "CreatePackage",
    "CreateTable",
    "CompareSource",
    "CreateClassWithTests",
    "CreateTestInclude",
    "CreateAndActivateProgram",
    "UpdateClassInclude",
    "FindDefinition",
    "FindReferences",
    "CodeCompletion",
    "GetTypeHierarchy",
    "GetCallGraph",
    "GetCallersOf",
    "GetCalleesOf",
    "GetObjectStructure",
    "AnalyzeCallGraph",
    "CompareCallGraphs",
    "TraceExecution",
    "GetSystemInfo",
    "GetInstalledComponents",
    "GetConnectionInfo",
    "GetFeatures",
    "ListDumps",
    "GetDump",
    "ListTraces",
    "GetTrace",
    "GetSQLTraceState",
    "ListSQLTraces",
    "ImportFromFile",
    "ExportToFile",
    "DeployFromFile",
    "SaveToFile",
    "ListTransports",
    "GetTransport",
    "GetTransportInfo",
    "GetUserTransports",
    "CreateTransport",
    "ReleaseTransport",
    "DeleteTransport",
    "RunReport",
    "RunReportAsync",
    "GetAsyncResult",
    "GetVariants",
    "GetTextElements",
    "SetTextElements",
    "SetBreakpoint",
    "GetBreakpoints",
    "DeleteBreakpoint",
    "DebuggerListen",
    "DebuggerAttach",
    "DebuggerDetach",
    "DebuggerStep",
    "DebuggerGetStack",
    "DebuggerGetVariables",
    "AMDPDebuggerStart",
    "AMDPDebuggerResume",
    "AMDPDebuggerStop",
    "AMDPDebuggerStep",
    "AMDPGetVariables",
    "AMDPSetBreakpoint",
    "AMDPGetBreakpoints",
    "CallRFC",
    "ExecuteABAP",
    "GitTypes",
    "GitExport",
    "InstallZADTVSP",
    "InstallAbapGit",
    "ListDependencies",
    "InstallDummyTest",
    "UI5ListApps",
    "UI5GetApp",
    "UI5GetFileContent",
    "UI5CreateApp",
    "UI5DeleteApp",
    "UI5DeleteFile",
    "UI5UploadFile",
    "PublishServiceBinding",
    "UnpublishServiceBinding",
];

const NEURO_INTERNAL_TOOL_NAMES: &[&str] = &["diagnose", "search", "get_source", "update_source", "ws_request"];
const IMPLEMENTED_TOOL_NAMES: &[&str] = &[
    "diagnose",
    "search",
    "SearchObject",
    "get_source",
    "GetSource",
    "update_source",
    "UpdateSource",
    "WriteSource",
    "ws_request",
];

impl NeuroMcpFacade {
    pub fn new(engine: Arc<NeuroEngine>) -> Self {
        let mut registry = BTreeMap::new();

        for name in VBS_TOOL_NAMES {
            registry.insert(
                (*name).to_owned(),
                NeuroToolSpec {
                    name: (*name).to_owned(),
                    description: format!("VBS parity tool `{name}`"),
                },
            );
        }

        for name in NEURO_INTERNAL_TOOL_NAMES {
            registry.insert(
                (*name).to_owned(),
                NeuroToolSpec {
                    name: (*name).to_owned(),
                    description: format!("Neuro internal tool `{name}`"),
                },
            );
        }

        Self { engine, registry }
    }

    pub fn list_tools(&self) -> Vec<NeuroToolSpec> {
        self.registry.values().cloned().collect()
    }

    pub async fn invoke(&self, tool_name: &str, arguments: Value) -> Result<Value, NeuroMcpError> {
        match tool_name {
            "diagnose" => self.handle_diagnose().await,
            "search" | "SearchObject" => self.handle_search(arguments, tool_name).await,
            "get_source" | "GetSource" => self.handle_get_source(arguments, tool_name).await,
            "update_source" | "UpdateSource" | "WriteSource" => {
                self.handle_update_source(arguments, tool_name).await
            }
            "ws_request" => self.handle_ws_request(arguments, tool_name).await,
            _ => {
                if self.registry.contains_key(tool_name) && !is_implemented_tool(tool_name) {
                    Err(NeuroMcpError::UnsupportedTool {
                        tool: tool_name.to_owned(),
                    })
                } else {
                    Err(NeuroMcpError::UnknownTool(tool_name.to_owned()))
                }
            }
        }
    }

    async fn handle_diagnose(&self) -> Result<Value, NeuroMcpError> {
        let report = self.engine.diagnose().await;
        serde_json::to_value(report).map_err(Into::into)
    }

    async fn handle_search(&self, arguments: Value, tool_name: &str) -> Result<Value, NeuroMcpError> {
        let args: SearchArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let objects = self.engine.search(args.query.as_str(), args.max_results).await?;
        Ok(json!({ "objects": objects }))
    }

    async fn handle_get_source(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetSourceArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let response = self.engine.get_source(args.object_uri.as_str()).await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_update_source(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: UpdateSourceArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let response = self
            .engine
            .update_source(AdtUpdateSourceRequest {
                object_uri: args.object_uri,
                source: args.source,
                etag: args.etag,
            })
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_ws_request(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: WsRequestArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let response = self
            .engine
            .send_domain_request(args.domain.as_str(), args.action.as_str(), args.payload)
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }
}

fn is_implemented_tool(tool_name: &str) -> bool {
    IMPLEMENTED_TOOL_NAMES
        .iter()
        .any(|implemented| *implemented == tool_name)
}

#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default, alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetSourceArgs {
    #[serde(alias = "objectUri")]
    object_uri: String,
}

#[derive(Debug, Deserialize)]
struct UpdateSourceArgs {
    #[serde(alias = "objectUri")]
    object_uri: String,
    source: String,
    #[serde(default)]
    etag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WsRequestArgs {
    domain: String,
    action: String,
    #[serde(default)]
    payload: Value,
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
                    search_objects_path:
                        "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch"
                            .to_string(),
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
    async fn list_tools_contains_parity_entries() {
        let facade = build_facade().await;
        let tools = facade.list_tools();
        assert!(tools.iter().any(|tool| tool.name == "SearchObject"));
        assert!(tools.iter().any(|tool| tool.name == "GetSource"));
        assert!(tools.iter().any(|tool| tool.name == "diagnose"));
    }

    #[tokio::test]
    async fn invoke_known_but_unimplemented_tool_returns_explicit_error() {
        let facade = build_facade().await;
        let error = facade
            .invoke("Activate", json!({}))
            .await
            .expect_err("unimplemented parity tool should fail explicitly");
        assert!(matches!(error, NeuroMcpError::UnsupportedTool { .. }));
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
