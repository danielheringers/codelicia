use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine;
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
    "GetATCCustomizing",
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
    "GetProgram",
    "GetClass",
    "GetInterface",
    "GetFunction",
    "GetInclude",
    "GetFunctionGroup",
    "GetMessages",
    "GetPackage",
    "GetTable",
    "GetTableContents",
    "GetStructure",
    "GetTransaction",
    "GetTypeInfo",
    "GetCDSDependencies",
    "GetClassInclude",
    "UpdateClassInclude",
    "CreateTestInclude",
    "GetObjectStructure",
    "GetClassComponents",
    "GetCallGraph",
    "GetCallersOf",
    "GetCalleesOf",
    "AnalyzeCallGraph",
    "CompareCallGraphs",
    "GetInactiveObjects",
    "GetSystemInfo",
    "GetInstalledComponents",
    "GetATCCustomizing",
    "GetConnectionInfo",
    "GetFeatures",
    "PrettyPrint",
    "GetPrettyPrinterSettings",
    "SetPrettyPrinterSettings",
    "RunUnitTests",
    "RunATCCheck",
    "FindDefinition",
    "FindReferences",
    "CodeCompletion",
    "GetUserTransports",
    "GetTransportInfo",
    "ListTransports",
    "GetTransport",
    "CreateTransport",
    "ReleaseTransport",
    "DeleteTransport",
    "RunReport",
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
    "CreateObject",
    "CreatePackage",
    "DeleteObject",
    "PublishServiceBinding",
    "UnpublishServiceBinding",
    "LockObject",
    "UnlockObject",
    "Activate",
    "SyntaxCheck",
    "update_source",
    "UpdateSource",
    "WriteSource",
    "WriteProgram",
    "WriteClass",
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
            "GetProgram" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/programs/{name}/source/main",
                    &["programName", "program_name", "name"],
                )
                .await,
            "GetClass" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/classes/{name}/source/main",
                    &["className", "class_name", "name"],
                )
                .await,
            "GetInterface" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/interfaces/{name}/source/main",
                    &["interfaceName", "interface_name", "name"],
                )
                .await,
            "GetInclude" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/includes/{name}/source/main",
                    &["includeName", "include_name", "name"],
                )
                .await,
            "GetFunction" => self.handle_get_function(arguments, tool_name).await,
            "GetFunctionGroup" => self
                .handle_get_raw_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/functions/groups/{name}",
                    &["groupName", "group_name", "name"],
                    Some("application/xml"),
                )
                .await,
            "GetMessages" => self.handle_get_messages(arguments, tool_name).await,
            "GetPackage" => self.handle_get_package(arguments, tool_name).await,
            "GetTable" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/ddic/tables/{name}/source/main",
                    &["tableName", "table_name", "name"],
                )
                .await,
            "GetTableContents" => self.handle_get_table_contents(arguments, tool_name).await,
            "GetStructure" => self
                .handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/ddic/structures/{name}/source/main",
                    &["structureName", "structure_name", "name"],
                )
                .await,
            "GetTransaction" => self.handle_get_transaction(arguments, tool_name).await,
            "GetTypeInfo" => self.handle_get_type_info(arguments, tool_name).await,
            "GetCDSDependencies" => self.handle_get_cds_dependencies(arguments, tool_name).await,
            "GetClassInclude" => self.handle_get_class_include(arguments, tool_name).await,
            "UpdateClassInclude" => self.handle_update_class_include(arguments, tool_name).await,
            "CreateTestInclude" => self.handle_create_test_include(arguments, tool_name).await,
            "GetObjectStructure" => self.handle_get_object_structure(arguments, tool_name).await,
            "GetClassComponents" => self.handle_get_class_components(arguments, tool_name).await,
            "GetCallGraph" => self.handle_get_call_graph(arguments, tool_name).await,
            "GetCallersOf" => self.handle_get_callers_of(arguments, tool_name).await,
            "GetCalleesOf" => self.handle_get_callees_of(arguments, tool_name).await,
            "AnalyzeCallGraph" => self.handle_analyze_call_graph(arguments, tool_name).await,
            "CompareCallGraphs" => self.handle_compare_call_graphs(arguments, tool_name).await,
            "GetInactiveObjects" => self.handle_get_inactive_objects(arguments, tool_name).await,
            "GetInstalledComponents" => {
                self.handle_get_installed_components(arguments, tool_name).await
            }
            "GetATCCustomizing" => self.handle_get_atc_customizing(arguments, tool_name).await,
            "GetSystemInfo" => self.handle_get_system_info(arguments, tool_name).await,
            "GetConnectionInfo" => self.handle_get_connection_info(arguments, tool_name).await,
            "GetFeatures" => self.handle_get_features(arguments, tool_name).await,
            "PrettyPrint" => self.handle_pretty_print(arguments, tool_name).await,
            "GetPrettyPrinterSettings" => {
                self.handle_get_pretty_printer_settings(arguments, tool_name).await
            }
            "SetPrettyPrinterSettings" => {
                self.handle_set_pretty_printer_settings(arguments, tool_name).await
            }
            "FindDefinition" => self.handle_find_definition(arguments, tool_name).await,
            "FindReferences" => self.handle_find_references(arguments, tool_name).await,
            "CodeCompletion" => self.handle_code_completion(arguments, tool_name).await,
            "RunUnitTests" => self.handle_run_unit_tests(arguments, tool_name).await,
            "RunATCCheck" => self.handle_run_atc_check(arguments, tool_name).await,
            "GetUserTransports" => self.handle_get_user_transports(arguments, tool_name).await,
            "GetTransportInfo" => self.handle_get_transport_info(arguments, tool_name).await,
            "ListTransports" => self.handle_list_transports(arguments, tool_name).await,
            "GetTransport" => self.handle_get_transport(arguments, tool_name).await,
            "CreateTransport" => self.handle_create_transport(arguments, tool_name).await,
            "ReleaseTransport" => self.handle_release_transport(arguments, tool_name).await,
            "DeleteTransport" => self.handle_delete_transport(arguments, tool_name).await,
            "RunReport" => self.handle_run_report(arguments, tool_name).await,
            "GetVariants" => self.handle_get_variants(arguments, tool_name).await,
            "GetTextElements" => self.handle_get_text_elements(arguments, tool_name).await,
            "SetTextElements" => self.handle_set_text_elements(arguments, tool_name).await,
            "SetBreakpoint" => self.handle_set_breakpoint(arguments, tool_name).await,
            "GetBreakpoints" => self.handle_get_breakpoints(arguments, tool_name).await,
            "DeleteBreakpoint" => self.handle_delete_breakpoint(arguments, tool_name).await,
            "DebuggerListen" => self.handle_debugger_listen(arguments, tool_name).await,
            "DebuggerAttach" => self.handle_debugger_attach(arguments, tool_name).await,
            "DebuggerDetach" => self.handle_debugger_detach(arguments, tool_name).await,
            "DebuggerStep" => self.handle_debugger_step(arguments, tool_name).await,
            "DebuggerGetStack" => self.handle_debugger_get_stack(arguments, tool_name).await,
            "DebuggerGetVariables" => {
                self.handle_debugger_get_variables(arguments, tool_name).await
            }
            "AMDPDebuggerStart" => self.handle_amdp_start(arguments, tool_name).await,
            "AMDPDebuggerResume" => self.handle_amdp_resume(arguments, tool_name).await,
            "AMDPDebuggerStop" => self.handle_amdp_stop(arguments, tool_name).await,
            "AMDPDebuggerStep" => self.handle_amdp_step(arguments, tool_name).await,
            "AMDPGetVariables" => self.handle_amdp_get_variables(arguments, tool_name).await,
            "AMDPSetBreakpoint" => self.handle_amdp_set_breakpoint(arguments, tool_name).await,
            "AMDPGetBreakpoints" => self.handle_amdp_get_breakpoints(arguments, tool_name).await,
            "CreateObject" => self.handle_create_object(arguments, tool_name).await,
            "CreatePackage" => self.handle_create_package(arguments, tool_name).await,
            "DeleteObject" => self.handle_delete_object(arguments, tool_name).await,
            "PublishServiceBinding" => {
                self.handle_publish_service_binding(arguments, tool_name, true)
                    .await
            }
            "UnpublishServiceBinding" => {
                self.handle_publish_service_binding(arguments, tool_name, false)
                    .await
            }
            "LockObject" => self.handle_lock_object(arguments, tool_name).await,
            "UnlockObject" => self.handle_unlock_object(arguments, tool_name).await,
            "Activate" => self.handle_activate(arguments, tool_name).await,
            "SyntaxCheck" => self.handle_syntax_check(arguments, tool_name).await,
            "update_source" | "UpdateSource" | "WriteSource" => {
                self.handle_update_source(arguments, tool_name).await
            }
            "WriteProgram" => self
                .handle_update_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/programs/{name}/source/main",
                    &["programName", "program_name", "name"],
                )
                .await,
            "WriteClass" => self
                .handle_update_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/classes/{name}/source/main",
                    &["className", "class_name", "name"],
                )
                .await,
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

    async fn handle_get_source_by_pattern(
        &self,
        arguments: Value,
        tool_name: &str,
        path_pattern: &str,
        accepted_name_keys: &[&str],
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let name = args.extract_name(accepted_name_keys).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("expected one of {:?} in arguments", accepted_name_keys),
            }
        })?;
        let object_uri = build_source_uri(path_pattern, name.as_str());
        let response = self.engine.get_source(object_uri.as_str()).await?;
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

    async fn handle_update_source_by_pattern(
        &self,
        arguments: Value,
        tool_name: &str,
        path_pattern: &str,
        accepted_name_keys: &[&str],
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedSourceArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let name = args.extract_name(accepted_name_keys).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("expected one of {:?} in arguments", accepted_name_keys),
            }
        })?;
        let object_uri = build_source_uri(path_pattern, name.as_str());

        let response = self
            .engine
            .update_source(AdtUpdateSourceRequest {
                object_uri,
                source: args.source,
                etag: args.etag,
            })
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_get_function(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: FunctionSourceArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let function_name = args.function_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "function name is required".to_owned(),
        })?;
        let group_name = args.group_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "group name is required".to_owned(),
        })?;

        let object_uri = format!(
            "/sap/bc/adt/functions/groups/{}/fmodules/{}/source/main",
            encode_path_segment(group_name.as_str()),
            encode_path_segment(function_name.as_str())
        );

        let response = self.engine.get_source(object_uri.as_str()).await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_get_raw_by_pattern(
        &self,
        arguments: Value,
        tool_name: &str,
        path_pattern: &str,
        accepted_name_keys: &[&str],
        accept: Option<&str>,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;

        let name = args.extract_name(accepted_name_keys).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("expected one of {:?} in arguments", accepted_name_keys),
            }
        })?;

        let object_uri = build_source_uri(path_pattern, name.as_str());
        let raw = self.engine.get_raw_text(object_uri.as_str(), accept).await?;
        Ok(json!({
            "objectUri": object_uri,
            "raw": raw,
        }))
    }

    async fn handle_get_messages(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let name = args
            .extract_name(&["messageClass", "message_class", "name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "message class name is required".to_owned(),
            })?;

        let object_uri = format!(
            "/sap/bc/adt/messageclass/{}",
            encode_path_segment(name.to_lowercase().as_str())
        );
        let raw = self
            .engine
            .get_raw_text(object_uri.as_str(), Some("application/vnd.sap.adt.mc.messageclass+xml"))
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "raw": raw,
        }))
    }

    async fn handle_get_package(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let package_name = args
            .extract_name(&["packageName", "package_name", "name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package name is required".to_owned(),
            })?;

        let package_name = package_name.to_uppercase();
        let path = build_path_with_query(
            "/sap/bc/adt/repository/nodestructure",
            &[
                ("parent_type", "DEVC/K".to_owned()),
                ("parent_name", package_name.clone()),
                ("withShortDescriptions", "true".to_owned()),
            ],
        );
        let raw = self.engine.post_raw_text(path.as_str(), None, None, None).await?;
        Ok(json!({
            "package": package_name,
            "raw": raw,
        }))
    }

    async fn handle_get_transaction(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let tcode = args
            .extract_name(&["tcode", "transaction", "name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "transaction code is required".to_owned(),
            })?
            .to_uppercase();

        let path = format!(
            "/sap/bc/adt/vit/wb/object_type/TRAN/object_name/{}",
            encode_path_segment(tcode.as_str())
        );
        let raw = self
            .engine
            .get_raw_text(path.as_str(), Some("application/xml"))
            .await?;
        Ok(json!({
            "transaction": tcode,
            "raw": raw,
        }))
    }

    async fn handle_get_type_info(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let type_name = args
            .extract_name(&["typeName", "type_name", "name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "type name is required".to_owned(),
            })?
            .to_uppercase();

        let path = format!(
            "/sap/bc/adt/ddic/dataelements/{}",
            encode_path_segment(type_name.as_str())
        );
        let raw = self
            .engine
            .get_raw_text(path.as_str(), Some("application/xml"))
            .await?;
        Ok(json!({
            "typeName": type_name,
            "raw": raw,
        }))
    }

    async fn handle_get_cds_dependencies(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: NamedObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let ddls = args
            .extract_name(&["ddlsName", "ddls_name", "name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "ddls name is required".to_owned(),
            })?;

        let path = build_path_with_query(
            "/sap/bc/adt/testcodegen/dependencies/doubledata",
            &[("ddlsourceName", ddls)],
        );
        let raw = self
            .engine
            .get_raw_text(path.as_str(), Some("application/vnd.sap.adt.codegen.data.v1+xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_table_contents(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: TableContentsArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let table_name = args.table_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "tableName/table_name is required".to_owned(),
        })?;

        let table_name = table_name.to_ascii_uppercase();
        let max_rows = args.max_rows.unwrap_or(100).max(1);
        let path = build_path_with_query(
            "/sap/bc/adt/datapreview/ddic",
            &[
                ("rowNumber", max_rows.to_string()),
                ("ddicEntityName", table_name.clone()),
            ],
        );
        let sql_filter = args.sql_query.and_then(|query| {
            let trimmed = query.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        });
        let raw = self
            .engine
            .post_raw_text(
                path.as_str(),
                sql_filter.as_deref(),
                if sql_filter.is_some() {
                    Some("text/plain")
                } else {
                    None
                },
                Some("application/*"),
            )
            .await?;
        Ok(json!({
            "tableName": table_name,
            "maxRows": max_rows,
            "sqlFilter": sql_filter,
            "raw": raw,
        }))
    }

    async fn handle_create_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        self.create_object_request(&args, tool_name).await
    }

    async fn handle_create_package(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreatePackageArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let name = args.name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name is required".to_owned(),
        })?;
        let description = args.description.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "description is required".to_owned(),
        })?;
        let package_name = args.parent.unwrap_or_default();
        let transport = args.transport.unwrap_or_default();
        if !name.trim_start().starts_with('$') && transport.trim().is_empty() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message:
                    "transport is required for transportable packages (non-$ packages)".to_owned(),
            });
        }

        let create_args = CreateObjectArgs {
            object_type: Some("DEVC/K".to_owned()),
            name: Some(name),
            description: Some(description),
            package_name: Some(package_name),
            transport: if transport.trim().is_empty() {
                None
            } else {
                Some(transport)
            },
            parent_name: None,
            responsible: None,
            software_component: args.software_component,
            service_definition: None,
            binding_type: None,
            binding_version: None,
            binding_category: None,
        };
        self.create_object_request(&create_args, tool_name).await
    }

    async fn handle_delete_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: DeleteObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let lock_handle = args.lock_handle.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "lockHandle/lock_handle is required".to_owned(),
        })?;
        let mut query = vec![("lockHandle", lock_handle.clone())];
        if let Some(transport) = args.transport {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport));
            }
        }
        let path = build_path_with_query(object_url.as_str(), &query);
        let raw = self.engine.delete_raw_text(path.as_str(), None).await?;
        Ok(json!({
            "objectUrl": object_url,
            "lockHandle": lock_handle,
            "raw": raw,
        }))
    }

    async fn handle_publish_service_binding(
        &self,
        arguments: Value,
        tool_name: &str,
        publish: bool,
    ) -> Result<Value, NeuroMcpError> {
        let args: ServiceBindingArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let service_name = args.service_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "serviceName/service_name is required".to_owned(),
        })?;
        let service_version = args.service_version.unwrap_or_else(|| "0001".to_owned());
        let action = if publish { "publishjobs" } else { "unpublishjobs" };
        let endpoint = build_path_with_query(
            format!("/sap/bc/adt/businessservices/odatav2/{action}").as_str(),
            &[
                ("servicename", service_name.clone()),
                ("serviceversion", service_version.clone()),
            ],
        );
        let body = format!(
            "<adtcore:objectReferences xmlns:adtcore=\"http://www.sap.com/adt/core\">\n  <adtcore:objectReference adtcore:name=\"{}\"/>\n</adtcore:objectReferences>",
            escape_xml(service_name.as_str())
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body.as_str()),
                Some("application/*"),
                Some("application/*"),
            )
            .await?;
        Ok(json!({
            "serviceName": service_name,
            "serviceVersion": service_version,
            "action": if publish { "publish" } else { "unpublish" },
            "raw": raw,
        }))
    }

    async fn create_object_request(
        &self,
        args: &CreateObjectArgs,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let object_type = args.object_type.as_deref().ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectType/object_type is required".to_owned(),
        })?
        .trim()
        .to_ascii_uppercase();
        let name = args.name.as_deref().ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name is required".to_owned(),
        })?
        .trim()
        .to_ascii_uppercase();
        let description = args.description.as_deref().ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "description is required".to_owned(),
        })?
        .trim()
        .to_owned();
        let package_name = args
            .package_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        let parent_name = args
            .parent_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();

        if object_type != "DEVC/K" && package_name.is_empty() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "packageName/package_name is required".to_owned(),
            });
        }

        let type_info = resolve_create_object_type_info(
            object_type.as_str(),
            if parent_name.is_empty() {
                None
            } else {
                Some(parent_name.as_str())
            },
        )
        .ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: format!("unsupported object_type `{object_type}` or missing required parent_name"),
        })?;

        let responsible = args
            .responsible
            .clone()
            .or_else(|| std::env::var("NEURO_SAP_USER").ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "DDIC".to_owned())
            .to_ascii_uppercase();
        let body = build_create_object_body(
            object_type.as_str(),
            name.as_str(),
            description.as_str(),
            package_name.as_str(),
            parent_name.as_str(),
            responsible.as_str(),
            args,
            &type_info,
        );

        let mut query = Vec::new();
        if let Some(transport) = args.transport.as_deref() {
            let transport = transport.trim();
            if !transport.is_empty() {
                query.push(("corrNr", transport.to_owned()));
            }
        }
        let endpoint = build_path_with_query(type_info.creation_path.as_str(), &query);
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body.as_str()),
                Some(type_info.content_type),
                None,
            )
            .await?;

        Ok(json!({
            "status": "created",
            "objectType": object_type,
            "name": name,
            "objectUrl": build_object_url(
                object_type.as_str(),
                name.as_str(),
                if parent_name.is_empty() {
                    None
                } else {
                    Some(parent_name.as_str())
                }
            ),
            "raw": raw,
        }))
    }

    async fn handle_lock_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: LockObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let access_mode = args.access_mode.unwrap_or_else(|| "MODIFY".to_owned());
        let path = build_path_with_query(
            object_url.as_str(),
            &[
                ("_action", "LOCK".to_owned()),
                ("accessMode", access_mode.clone()),
            ],
        );

        let raw = self
            .engine
            .post_raw_text(
                path.as_str(),
                None,
                None,
                Some("application/vnd.sap.as+xml;charset=UTF-8;dataname=com.sap.adt.lock.result"),
            )
            .await?;
        Ok(json!({
            "objectUrl": object_url,
            "accessMode": access_mode,
            "raw": raw,
        }))
    }

    async fn handle_unlock_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: UnlockObjectArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let lock_handle = args.lock_handle.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "lockHandle/lock_handle is required".to_owned(),
        })?;

        let path = build_path_with_query(
            object_url.as_str(),
            &[
                ("_action", "UNLOCK".to_owned()),
                ("lockHandle", lock_handle.clone()),
            ],
        );

        let raw = self.engine.post_raw_text(path.as_str(), None, None, None).await?;
        Ok(json!({
            "objectUrl": object_url,
            "lockHandle": lock_handle,
            "raw": raw,
        }))
    }

    async fn handle_activate(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ActivateArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let object_name = args.object_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectName/object_name is required".to_owned(),
        })?;

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<adtcore:objectReferences xmlns:adtcore=\"http://www.sap.com/adt/core\">\n  <adtcore:objectReference adtcore:uri=\"{}\" adtcore:name=\"{}\"/>\n</adtcore:objectReferences>",
            object_url,
            object_name
        );
        let raw = self
            .engine
            .post_raw_text(
                "/sap/bc/adt/activation?method=activate&preauditRequested=true",
                Some(body.as_str()),
                Some("application/xml"),
                None,
            )
            .await?;
        Ok(json!({
            "objectUrl": object_url,
            "objectName": object_name,
            "raw": raw,
        }))
    }

    async fn handle_syntax_check(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: SyntaxCheckArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let source_content = args.content.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "content is required".to_owned(),
        })?;

        let source_url = if object_url.contains("/includes/") {
            object_url.clone()
        } else {
            format!("{object_url}/source/main")
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(source_content.as_bytes());
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<chkrun:checkObjectList xmlns:chkrun=\"http://www.sap.com/adt/checkrun\" xmlns:adtcore=\"http://www.sap.com/adt/core\">\n  <chkrun:checkObject adtcore:uri=\"{}\" chkrun:version=\"active\">\n    <chkrun:artifacts>\n      <chkrun:artifact chkrun:contentType=\"text/plain; charset=utf-8\" chkrun:uri=\"{}\">\n        <chkrun:content>{}</chkrun:content>\n      </chkrun:artifact>\n    </chkrun:artifacts>\n  </chkrun:checkObject>\n</chkrun:checkObjectList>",
            source_url,
            source_url,
            encoded
        );
        let raw = self
            .engine
            .post_raw_text(
                "/sap/bc/adt/checkruns?reporters=abapCheckRun",
                Some(body.as_str()),
                Some("application/*"),
                None,
            )
            .await?;
        Ok(json!({
            "objectUrl": object_url,
            "sourceUrl": source_url,
            "raw": raw,
        }))
    }

    async fn handle_get_class_include(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ClassIncludeArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let class_name = args.class_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "className/class_name is required".to_owned(),
        })?;
        let include_type = args.include_type.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "includeType/include_type is required".to_owned(),
        })?;
        let object_uri = build_class_include_uri(class_name.as_str(), include_type.as_str());
        let response = self.engine.get_source(object_uri.as_str()).await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_update_class_include(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: UpdateClassIncludeArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let class_name = args.class_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "className/class_name is required".to_owned(),
        })?;
        let include_type = args.include_type.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "includeType/include_type is required".to_owned(),
        })?;
        let lock_handle = args.lock_handle.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "lockHandle/lock_handle is required".to_owned(),
        })?;

        let base_uri = build_class_include_uri(class_name.as_str(), include_type.as_str());
        let mut query = vec![("lockHandle", lock_handle)];
        if let Some(transport) = args.transport {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport));
            }
        }
        let uri = build_path_with_query(base_uri.as_str(), &query);

        let raw = self
            .engine
            .put_raw_text(
                uri.as_str(),
                Some(args.source.as_str()),
                Some("text/plain; charset=utf-8"),
                None,
            )
            .await?;
        Ok(json!({
            "objectUri": base_uri,
            "raw": raw,
        }))
    }

    async fn handle_get_installed_components(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let raw = self
            .engine
            .get_raw_text("/sap/bc/adt/system/components", Some("application/xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_connection_info(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let diagnose = self.engine.diagnose().await;
        Ok(json!({
            "runtime": "neuro-mcp",
            "overallStatus": diagnose.overall_status,
            "components": diagnose.components,
            "metadata": diagnose.metadata,
        }))
    }

    async fn handle_create_test_include(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateTestIncludeArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let class_name = args.class_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "className/class_name is required".to_owned(),
        })?;
        let lock_handle = args.lock_handle.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "lockHandle/lock_handle is required".to_owned(),
        })?;

        let encoded_class_name = encode_path_segment(class_name.to_ascii_uppercase().as_str());
        let includes_path = format!("/sap/bc/adt/oo/classes/{encoded_class_name}/includes");
        let mut query = vec![("lockHandle", lock_handle)];
        if let Some(transport) = args.transport {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport));
            }
        }
        let endpoint = build_path_with_query(includes_path.as_str(), &query);
        let body = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<class:abapClassInclude xmlns:class=\"http://www.sap.com/adt/oo/classes\"\n  xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:name=\"dummy\" class:includeType=\"testclasses\"/>";
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body),
                Some("application/*"),
                None,
            )
            .await?;
        Ok(json!({
            "className": class_name,
            "includeType": "testclasses",
            "raw": raw,
        }))
    }

    async fn handle_get_object_structure(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetObjectStructureArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_name = args.object_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectName/object_name is required".to_owned(),
        })?;
        let max_results = args.max_results.unwrap_or(100).max(1);
        let endpoint = build_path_with_query(
            "/sap/bc/adt/cai/objectexplorer/objects",
            &[
                ("objectName", object_name.clone()),
                ("maxResults", max_results.to_string()),
            ],
        );
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/xml"))
            .await?;
        Ok(json!({
            "objectName": object_name,
            "maxResults": max_results,
            "raw": raw,
        }))
    }

    async fn handle_get_class_components(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetClassComponentsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let endpoint = build_path_with_query(
            format!("{}/objectstructure", args.class_url).as_str(),
            &[
                ("version", "active".to_owned()),
                ("withShortDescriptions", "true".to_owned()),
            ],
        );
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/*"))
            .await?;
        Ok(json!({
            "classUrl": args.class_url,
            "raw": raw,
        }))
    }

    async fn handle_get_call_graph(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetCallGraphArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args.object_uri.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUri/object_uri is required".to_owned(),
        })?;
        let direction = args.direction.unwrap_or_else(|| "callers".to_owned());
        let max_depth = args.max_depth.unwrap_or(3).max(1);
        let max_results = args.max_results.unwrap_or(100).max(1);

        let raw = self
            .request_call_graph(object_uri.as_str(), direction.as_str(), max_depth, max_results)
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "direction": direction,
            "maxDepth": max_depth,
            "maxResults": max_results,
            "raw": raw,
        }))
    }

    async fn handle_get_callers_of(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetCallTraversalArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args.object_uri.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUri/object_uri is required".to_owned(),
        })?;
        let max_depth = args.max_depth.unwrap_or(5).max(1);
        let raw = self
            .request_call_graph(object_uri.as_str(), "callers", max_depth, 500)
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "direction": "callers",
            "maxDepth": max_depth,
            "maxResults": 500,
            "raw": raw,
        }))
    }

    async fn handle_get_callees_of(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetCallTraversalArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args.object_uri.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUri/object_uri is required".to_owned(),
        })?;
        let max_depth = args.max_depth.unwrap_or(5).max(1);
        let raw = self
            .request_call_graph(object_uri.as_str(), "callees", max_depth, 500)
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "direction": "callees",
            "maxDepth": max_depth,
            "maxResults": 500,
            "raw": raw,
        }))
    }

    async fn handle_analyze_call_graph(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: AnalyzeCallGraphArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args.object_uri.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUri/object_uri is required".to_owned(),
        })?;
        let direction = args.direction.unwrap_or_else(|| "callees".to_owned());
        let max_depth = args.max_depth.unwrap_or(5).max(1);
        let max_results = 1000;

        let raw = self
            .request_call_graph(object_uri.as_str(), direction.as_str(), max_depth, max_results)
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "direction": direction,
            "maxDepth": max_depth,
            "maxResults": max_results,
            "raw": raw,
        }))
    }

    async fn handle_compare_call_graphs(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CompareCallGraphsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args.object_uri.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUri/object_uri is required".to_owned(),
        })?;
        let trace_data = args.trace_data.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "trace_data is required".to_owned(),
        })?;
        let actual_edges: Value =
            serde_json::from_str(trace_data.as_str()).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("trace_data must be valid JSON: {error}"),
            })?;
        let actual_edges_count = actual_edges
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0);

        let static_call_graph = self
            .request_call_graph(object_uri.as_str(), "callees", 10, 1000)
            .await?;
        Ok(json!({
            "objectUri": object_uri,
            "actualEdgesCount": actual_edges_count,
            "actualEdges": actual_edges,
            "staticCallGraph": static_call_graph,
            "note": "static vs actual edge comparison metrics are pending parser parity with VBS"
        }))
    }

    async fn request_call_graph(
        &self,
        object_uri: &str,
        direction: &str,
        max_depth: u32,
        max_results: u32,
    ) -> Result<String, NeuroMcpError> {
        let endpoint = build_path_with_query(
            "/sap/bc/adt/cai/callgraph",
            &[
                ("direction", direction.to_owned()),
                ("maxDepth", max_depth.to_string()),
                ("maxResults", max_results.to_string()),
            ],
        );
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cai:callGraphRequest xmlns:cai=\"http://www.sap.com/adt/cai\">\n  <cai:objectUri>{}</cai:objectUri>\n</cai:callGraphRequest>",
            object_uri
        );
        self.engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body.as_str()),
                Some("application/xml"),
                Some("application/xml"),
            )
            .await
            .map_err(Into::into)
    }

    async fn handle_get_inactive_objects(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let raw = self
            .engine
            .get_raw_text(
                "/sap/bc/adt/activation/inactiveobjects",
                Some("application/vnd.sap.adt.inactivectsobjects.v1+xml, application/xml;q=0.8"),
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_system_info(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let sap_client = std::env::var("NEURO_SAP_CLIENT").unwrap_or_default();
        let client_query = if sap_client.trim().is_empty() {
            "SELECT MANDT, MTEXT, LOGSYS FROM T000".to_owned()
        } else {
            format!(
                "SELECT MANDT, MTEXT, LOGSYS FROM T000 WHERE MANDT = '{}'",
                sap_client.trim()
            )
        };
        let client_info = self.run_freestyle_query(client_query.as_str(), 1).await?;
        let sap_basis = self
            .run_freestyle_query(
                "SELECT RELEASE, EXTRELEASE FROM CVERS WHERE COMPONENT = 'SAP_BASIS'",
                1,
            )
            .await?;
        let sap_aba = self
            .run_freestyle_query("SELECT RELEASE FROM CVERS WHERE COMPONENT = 'SAP_ABA'", 1)
            .await?;
        let hana = self
            .run_freestyle_query(
                "SELECT RELEASE FROM CVERS WHERE COMPONENT LIKE '%HDB%' OR COMPONENT LIKE '%HANA%'",
                1,
            )
            .await?;

        Ok(json!({
            "client": sap_client,
            "queries": {
                "t000": client_info,
                "sap_basis": sap_basis,
                "sap_aba": sap_aba,
                "hana": hana,
            }
        }))
    }

    async fn run_freestyle_query(&self, query: &str, max_rows: u32) -> Result<String, NeuroMcpError> {
        let endpoint = build_path_with_query(
            "/sap/bc/adt/datapreview/freestyle",
            &[("rowNumber", max_rows.max(1).to_string())],
        );
        self.engine
            .post_raw_text(
                endpoint.as_str(),
                Some(query),
                Some("text/plain"),
                Some("application/*"),
            )
            .await
            .map_err(Into::into)
    }

    async fn handle_get_atc_customizing(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let raw = self
            .engine
            .get_raw_text(
                "/sap/bc/adt/atc/customizing",
                Some("application/xml, application/vnd.sap.atc.customizing-v1+xml"),
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_features(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let diagnose = self.engine.diagnose().await;
        let adt = diagnose
            .components
            .iter()
            .find(|component| component.component == "adt_http")
            .map(|component| component.status);
        let ws = diagnose
            .components
            .iter()
            .find(|component| component.component == "neuro_ws")
            .map(|component| component.status);
        Ok(json!({
            "adtHttp": adt,
            "websocket": ws,
            "note": "feature probing parity with VBS is pending; this is runtime-based feature visibility"
        }))
    }

    async fn handle_pretty_print(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: PrettyPrintArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let raw = self
            .engine
            .post_raw_text(
                "/sap/bc/adt/abapsource/prettyprinter",
                Some(args.source.as_str()),
                Some("text/plain"),
                Some("text/plain"),
            )
            .await?;
        Ok(json!({ "source": raw }))
    }

    async fn handle_get_pretty_printer_settings(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let raw = self
            .engine
            .get_raw_text("/sap/bc/adt/abapsource/prettyprinter/settings", None)
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_set_pretty_printer_settings(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: SetPrettyPrinterSettingsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<prettyprintersettings:PrettyPrinterSettings\nxmlns:prettyprintersettings=\"http://www.sap.com/adt/prettyprintersettings\"\nprettyprintersettings:indentation=\"{}\" prettyprintersettings:style=\"{}\"/>",
            if args.indentation { "true" } else { "false" },
            args.style
        );
        let raw = self
            .engine
            .put_raw_text(
                "/sap/bc/adt/abapsource/prettyprinter/settings",
                Some(body.as_str()),
                Some("application/*"),
                None,
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_find_definition(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: FindDefinitionArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let line = args.line.max(1);
        let start_col = args.start_col.max(1);
        let end_col = args.end_col.max(start_col);
        let context_param = args
            .main_program
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|main_program| format!("?context={}", encode_query_value(main_program)))
            .unwrap_or_default();
        let uri = format!(
            "{}{}#start={},{};end={},{}",
            args.source_url, context_param, line, start_col, line, end_col
        );
        let filter = if args.implementation {
            "implementation"
        } else {
            "definition"
        };

        let endpoint = build_path_with_query(
            "/sap/bc/adt/navigation/target",
            &[("uri", uri), ("filter", filter.to_owned())],
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(args.source.as_str()),
                Some("text/plain"),
                Some("application/*"),
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_find_references(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: FindReferencesArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let mut uri = args.object_url;
        if let (Some(line), Some(column)) = (args.line, args.column) {
            if line > 0 && column > 0 {
                uri = format!("{uri}#start={line},{column}");
            }
        }
        let endpoint = build_path_with_query(
            "/sap/bc/adt/repository/informationsystem/usageReferences",
            &[("uri", uri)],
        );
        let body = "<?xml version=\"1.0\" encoding=\"ASCII\"?>\n<usagereferences:usageReferenceRequest xmlns:usagereferences=\"http://www.sap.com/adt/ris/usageReferences\">\n  <usagereferences:affectedObjects/>\n</usagereferences:usageReferenceRequest>";
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body),
                Some("application/*"),
                Some("application/*"),
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_code_completion(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CodeCompletionArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let line = args.line.max(1);
        let column = args.column.max(1);
        let uri = format!("{}#start={},{}", args.source_url, line, column);
        let endpoint = build_path_with_query(
            "/sap/bc/adt/abapsource/codecompletion/proposal",
            &[("uri", uri), ("signalCompleteness", "true".to_owned())],
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(args.source.as_str()),
                Some("application/*"),
                None,
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_run_unit_tests(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: RunUnitTestsArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let include_dangerous = args.include_dangerous.unwrap_or(false);
        let include_long = args.include_long.unwrap_or(false);

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<aunit:runConfiguration xmlns:aunit=\"http://www.sap.com/adt/aunit\">\n  <external>\n    <coverage active=\"false\"/>\n  </external>\n  <options>\n    <uriType value=\"semantic\"/>\n    <testDeterminationStrategy sameProgram=\"true\" assignedTests=\"false\"/>\n    <testRiskLevels harmless=\"true\" dangerous=\"{}\" critical=\"false\"/>\n    <testDurations short=\"true\" medium=\"true\" long=\"{}\"/>\n    <withNavigationUri enabled=\"true\"/>\n  </options>\n  <adtcore:objectSets xmlns:adtcore=\"http://www.sap.com/adt/core\">\n    <objectSet kind=\"inclusive\">\n      <adtcore:objectReferences>\n        <adtcore:objectReference adtcore:uri=\"{}\"/>\n      </adtcore:objectReferences>\n    </objectSet>\n  </adtcore:objectSets>\n</aunit:runConfiguration>",
            include_dangerous,
            include_long,
            escape_xml(object_url.as_str())
        );
        let raw = self
            .engine
            .post_raw_text(
                "/sap/bc/adt/abapunit/testruns",
                Some(body.as_str()),
                Some("application/*"),
                Some("application/*"),
            )
            .await?;
        Ok(json!({
            "objectUrl": object_url,
            "flags": {
                "includeDangerous": include_dangerous,
                "includeLong": include_long,
            },
            "raw": raw,
        }))
    }

    async fn handle_run_atc_check(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: RunAtcCheckArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let max_results = args.max_results.unwrap_or(100).max(1);
        let variant = if let Some(value) = args.variant {
            let trimmed = value.trim().to_owned();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        } else {
            None
        };

        let resolved_variant = if let Some(variant) = variant {
            variant
        } else {
            let customizing_raw = self
                .engine
                .get_raw_text(
                    "/sap/bc/adt/atc/customizing",
                    Some("application/xml, application/vnd.sap.atc.customizing-v1+xml"),
                )
                .await?;
            extract_system_check_variant(customizing_raw.as_str()).ok_or_else(|| {
                NeuroMcpError::InvalidArguments {
                    tool: tool_name.to_owned(),
                    message: "unable to resolve default ATC check variant from customizing"
                        .to_owned(),
                }
            })?
        };

        let worklist_request = build_path_with_query(
            "/sap/bc/adt/atc/worklists",
            &[("checkVariant", resolved_variant.clone())],
        );
        let worklist_id = self
            .engine
            .post_raw_text(worklist_request.as_str(), None, None, Some("text/plain"))
            .await?
            .trim()
            .to_owned();
        if worklist_id.is_empty() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "ATC worklist id was empty".to_owned(),
            });
        }

        let run_endpoint = build_path_with_query(
            "/sap/bc/adt/atc/runs",
            &[("worklistId", worklist_id.clone())],
        );
        let run_body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<atc:run maximumVerdicts=\"{}\" xmlns:atc=\"http://www.sap.com/adt/atc\">\n\t<objectSets xmlns:adtcore=\"http://www.sap.com/adt/core\">\n\t\t<objectSet kind=\"inclusive\">\n\t\t\t<adtcore:objectReferences>\n\t\t\t\t<adtcore:objectReference adtcore:uri=\"{}\"/>\n\t\t\t</adtcore:objectReferences>\n\t\t</objectSet>\n\t</objectSets>\n</atc:run>",
            max_results,
            escape_xml(object_url.as_str())
        );
        let run_raw = self
            .engine
            .post_raw_text(
                run_endpoint.as_str(),
                Some(run_body.as_str()),
                Some("application/xml"),
                Some("application/xml"),
            )
            .await?;
        let run_worklist_id = extract_xml_tag_value(run_raw.as_str(), "worklistId")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(worklist_id);

        let findings_endpoint = build_path_with_query(
            format!("/sap/bc/adt/atc/worklists/{run_worklist_id}").as_str(),
            &[("includeExemptedFindings", "false".to_owned())],
        );
        let findings_raw = self
            .engine
            .get_raw_text(findings_endpoint.as_str(), Some("application/atc.worklist.v1+xml"))
            .await?;

        Ok(json!({
            "objectUrl": object_url,
            "variant": resolved_variant,
            "worklistId": run_worklist_id,
            "run": run_raw,
            "worklist": findings_raw,
        }))
    }

    async fn handle_get_user_transports(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetUserTransportsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let user_name = args.user_name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "user_name is required".to_owned(),
        })?;
        let user_name = user_name.trim().to_ascii_uppercase();
        let endpoint = build_path_with_query(
            "/sap/bc/adt/cts/transportrequests",
            &[
                ("user", user_name.clone()),
                ("targets", "true".to_owned()),
            ],
        );
        let raw = self
            .engine
            .get_raw_text(
                endpoint.as_str(),
                Some(
                    "application/vnd.sap.adt.transportorganizertree.v1+xml, application/vnd.sap.adt.transportorganizer.v1+xml;q=0.9",
                ),
            )
            .await?;
        Ok(json!({
            "user": user_name,
            "raw": raw,
        }))
    }

    async fn handle_get_transport_info(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetTransportInfoArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args.object_url.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "objectUrl/object_url is required".to_owned(),
        })?;
        let dev_class = args.dev_class.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "devClass/dev_class is required".to_owned(),
        })?;

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<asx:abap xmlns:asx=\"http://www.sap.com/abapxml\" version=\"1.0\">\n  <asx:values>\n    <DATA>\n      <DEVCLASS>{}</DEVCLASS>\n      <OPERATION>I</OPERATION>\n      <URI>{}</URI>\n    </DATA>\n  </asx:values>\n</asx:abap>",
            escape_xml(dev_class.as_str()),
            escape_xml(object_url.as_str())
        );
        let raw = self
            .engine
            .post_raw_text(
                "/sap/bc/adt/cts/transportchecks",
                Some(body.as_str()),
                Some(
                    "application/vnd.sap.as+xml; charset=UTF-8; dataname=com.sap.adt.transport.service.checkData",
                ),
                Some(
                    "application/vnd.sap.as+xml;charset=UTF-8;dataname=com.sap.adt.transport.service.checkData",
                ),
            )
            .await?;
        Ok(json!({
            "objectUrl": object_url,
            "devClass": dev_class,
            "raw": raw,
        }))
    }

    async fn handle_list_transports(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ListTransportsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let user = args
            .user
            .or_else(|| std::env::var("NEURO_SAP_USER").ok())
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        let endpoint = if user.is_empty() {
            "/sap/bc/adt/cts/transportrequests".to_owned()
        } else {
            build_path_with_query(
                "/sap/bc/adt/cts/transportrequests",
                &[("user", user.clone())],
            )
        };
        let raw = self
            .engine
            .get_raw_text(
                endpoint.as_str(),
                Some("application/vnd.sap.adt.transportorganizertree.v1+xml"),
            )
            .await?;
        Ok(json!({
            "user": user,
            "raw": raw,
        }))
    }

    async fn handle_get_transport(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: TransportNumberArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let transport_number = args.transport.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "transport is required".to_owned(),
        })?;
        let endpoint = format!(
            "/sap/bc/adt/cts/transportrequests/{}",
            encode_path_segment(transport_number.trim().to_ascii_uppercase().as_str())
        );
        let raw = self
            .engine
            .get_raw_text(
                endpoint.as_str(),
                Some("application/vnd.sap.adt.transportorganizer.v1+xml"),
            )
            .await?;
        Ok(json!({
            "transport": transport_number,
            "raw": raw,
        }))
    }

    async fn handle_create_transport(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateTransportArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let description = args.description.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "description is required".to_owned(),
        })?;
        let package = args.package.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "package is required".to_owned(),
        })?;
        let req_type = if args
            .request_type
            .as_deref()
            .unwrap_or_default()
            .eq_ignore_ascii_case("customizing")
        {
            "W"
        } else {
            "K"
        };

        let mut query = Vec::new();
        if let Some(layer) = args.transport_layer {
            if !layer.trim().is_empty() {
                query.push(("transportLayer", layer));
            }
        }
        let endpoint = build_path_with_query("/sap/bc/adt/cts/transports", &query);
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tm:root xmlns:tm=\"http://www.sap.com/cts/adt/tm\">\n  <tm:request tm:desc=\"{}\" tm:type=\"{}\" tm:target=\"\" tm:cts_project=\"\">\n    <tm:abap_object tm:pgmid=\"R3TR\" tm:type=\"DEVC\" tm:name=\"{}\"/>\n  </tm:request>\n</tm:root>",
            escape_xml(description.as_str()),
            req_type,
            escape_xml(package.trim().to_ascii_uppercase().as_str())
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body.as_str()),
                Some("application/vnd.sap.as+xml"),
                Some("text/plain"),
            )
            .await?;
        Ok(json!({
            "transport": raw.trim(),
            "raw": raw,
        }))
    }

    async fn handle_release_transport(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ReleaseTransportArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let transport = args.transport.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "transport is required".to_owned(),
        })?;
        let action = if args.skip_atc.unwrap_or(false) {
            "relObjigchkatc"
        } else if args.ignore_locks.unwrap_or(false) {
            "relwithignlock"
        } else {
            "newreleasejobs"
        };
        let endpoint = format!(
            "/sap/bc/adt/cts/transportrequests/{}/{}",
            encode_path_segment(transport.trim().to_ascii_uppercase().as_str()),
            action
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                None,
                None,
                Some("application/vnd.sap.adt.transportorganizer.v1+xml"),
            )
            .await?;
        Ok(json!({
            "transport": transport,
            "action": action,
            "raw": raw,
        }))
    }

    async fn handle_delete_transport(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: TransportNumberArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let transport = args.transport.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "transport is required".to_owned(),
        })?;
        let endpoint = format!(
            "/sap/bc/adt/cts/transportrequests/{}",
            encode_path_segment(transport.trim().to_ascii_uppercase().as_str())
        );
        let raw = self
            .engine
            .delete_raw_text(
                endpoint.as_str(),
                Some("application/vnd.sap.adt.transportorganizer.v1+xml"),
            )
            .await?;
        Ok(json!({
            "transport": transport,
            "raw": raw,
        }))
    }

    async fn handle_run_report(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: RunReportArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let report = args.report.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "report is required".to_owned(),
        })?;
        let mut payload = serde_json::Map::new();
        payload.insert("report".to_owned(), json!(report));
        if let Some(variant) = args.variant {
            if !variant.trim().is_empty() {
                payload.insert("variant".to_owned(), json!(variant));
            }
        }
        if let Some(params) = args.params {
            payload.insert("params".to_owned(), params);
        }
        self.handle_report_ws_call("runReport", Value::Object(payload))
            .await
    }

    async fn handle_get_variants(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ReportNameArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let report = args.report.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "report is required".to_owned(),
        })?;
        self.handle_report_ws_call("getVariants", json!({ "report": report }))
            .await
    }

    async fn handle_get_text_elements(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetTextElementsArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let program = args.program.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "program is required".to_owned(),
        })?;
        let mut payload = serde_json::Map::new();
        payload.insert("program".to_owned(), json!(program));
        if let Some(language) = args.language {
            if !language.trim().is_empty() {
                payload.insert("language".to_owned(), json!(language));
            }
        }
        self.handle_report_ws_call("getTextElements", Value::Object(payload))
            .await
    }

    async fn handle_set_text_elements(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: SetTextElementsArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let program = args.program.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "program is required".to_owned(),
        })?;
        let mut payload = serde_json::Map::new();
        payload.insert("program".to_owned(), json!(program));
        if let Some(language) = args.language {
            if !language.trim().is_empty() {
                payload.insert("language".to_owned(), json!(language));
            }
        }
        if let Some(selection_texts) = args.selection_texts {
            payload.insert("selection_texts".to_owned(), selection_texts);
        }
        if let Some(text_symbols) = args.text_symbols {
            payload.insert("text_symbols".to_owned(), text_symbols);
        }
        if let Some(heading_texts) = args.heading_texts {
            payload.insert("heading_texts".to_owned(), heading_texts);
        }
        self.handle_report_ws_call("setTextElements", Value::Object(payload))
            .await
    }

    async fn handle_report_ws_call(
        &self,
        action: &str,
        payload: Value,
    ) -> Result<Value, NeuroMcpError> {
        let response = self.engine.send_domain_request("report", action, payload).await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_set_breakpoint(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        if !arguments.is_object() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "arguments must be a JSON object".to_owned(),
            });
        }
        self.handle_debug_ws_call("setBreakpoint", arguments).await
    }

    async fn handle_get_breakpoints(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_debug_ws_call("getBreakpoints", json!({})).await
    }

    async fn handle_delete_breakpoint(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: BreakpointIdArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let breakpoint_id = args.breakpoint_id.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "breakpoint_id is required".to_owned(),
        })?;
        self.handle_debug_ws_call("deleteBreakpoint", json!({ "breakpointId": breakpoint_id }))
            .await
    }

    async fn handle_debugger_listen(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: DebuggerListenArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let timeout = args.timeout.unwrap_or(60).clamp(1, 240);
        let user = std::env::var("NEURO_SAP_USER")
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        let payload = if user.is_empty() {
            json!({ "timeout": timeout })
        } else {
            json!({ "timeout": timeout, "user": user })
        };
        self.handle_debug_ws_call("listen", payload).await
    }

    async fn handle_debugger_attach(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: DebuggeeArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let debuggee_id = args.debuggee_id.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "debuggee_id is required".to_owned(),
        })?;
        self.handle_debug_ws_call("attach", json!({ "debuggeeId": debuggee_id }))
            .await
    }

    async fn handle_debugger_detach(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_debug_ws_call("detach", json!({})).await
    }

    async fn handle_debugger_step(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: StepArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let step_type = args.step_type.unwrap_or_else(|| "into".to_owned());
        self.handle_debug_ws_call("step", json!({ "type": step_type })).await
    }

    async fn handle_debugger_get_stack(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_debug_ws_call("getStack", json!({})).await
    }

    async fn handle_debugger_get_variables(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: VariablesScopeArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let scope = args.scope.unwrap_or_else(|| "system".to_owned());
        self.handle_debug_ws_call("getVariables", json!({ "scope": scope }))
            .await
    }

    async fn handle_amdp_start(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: AmdpStartArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let cascade_mode = args.cascade_mode.unwrap_or_else(|| "FULL".to_owned());
        let user = std::env::var("NEURO_SAP_USER")
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase();
        let payload = if user.is_empty() {
            json!({ "cascadeMode": cascade_mode })
        } else {
            json!({ "cascadeMode": cascade_mode, "user": user })
        };
        self.handle_amdp_ws_call("start", payload).await
    }

    async fn handle_amdp_resume(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_amdp_ws_call("resume", json!({})).await
    }

    async fn handle_amdp_stop(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_amdp_ws_call("stop", json!({})).await
    }

    async fn handle_amdp_step(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: StepArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let step_type = args.step_type.unwrap_or_else(|| "over".to_owned());
        self.handle_amdp_ws_call("step", json!({ "type": step_type }))
            .await
    }

    async fn handle_amdp_get_variables(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_amdp_ws_call("getVariables", json!({})).await
    }

    async fn handle_amdp_set_breakpoint(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: AmdpBreakpointArgs = serde_json::from_value(arguments).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            }
        })?;
        let program = args.program.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "program is required".to_owned(),
        })?;
        let line = args.line.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "line is required".to_owned(),
        })?;
        self.handle_amdp_ws_call("setBreakpoint", json!({ "program": program, "line": line }))
            .await
    }

    async fn handle_amdp_get_breakpoints(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        self.handle_amdp_ws_call("getBreakpoints", json!({})).await
    }

    async fn handle_debug_ws_call(
        &self,
        action: &str,
        payload: Value,
    ) -> Result<Value, NeuroMcpError> {
        let response = self.engine.send_domain_request("debug", action, payload).await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_amdp_ws_call(
        &self,
        action: &str,
        payload: Value,
    ) -> Result<Value, NeuroMcpError> {
        let response = self.engine.send_domain_request("amdp", action, payload).await?;
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
struct NamedObjectArgs {
    #[serde(flatten)]
    fields: BTreeMap<String, Value>,
}

impl NamedObjectArgs {
    fn extract_name(&self, keys: &[&str]) -> Option<String> {
        extract_non_empty_string(&self.fields, keys)
    }
}

#[derive(Debug, Deserialize)]
struct NamedSourceArgs {
    #[serde(flatten)]
    fields: BTreeMap<String, Value>,
    source: String,
    #[serde(default)]
    etag: Option<String>,
}

impl NamedSourceArgs {
    fn extract_name(&self, keys: &[&str]) -> Option<String> {
        extract_non_empty_string(&self.fields, keys)
    }
}

#[derive(Debug, Deserialize)]
struct GetSourceArgs {
    #[serde(alias = "objectUri")]
    object_uri: String,
}

#[derive(Debug, Deserialize)]
struct FunctionSourceArgs {
    #[serde(default, alias = "functionName", alias = "function_name", alias = "name")]
    function_name: Option<String>,
    #[serde(default, alias = "groupName", alias = "group_name", alias = "functionGroup")]
    group_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TableContentsArgs {
    #[serde(default, alias = "tableName", alias = "name")]
    table_name: Option<String>,
    #[serde(default, alias = "maxRows")]
    max_rows: Option<u32>,
    #[serde(default, alias = "sqlQuery")]
    sql_query: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateObjectArgs {
    #[serde(default, alias = "objectType")]
    object_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "packageName")]
    package_name: Option<String>,
    #[serde(default)]
    transport: Option<String>,
    #[serde(default, alias = "parentName")]
    parent_name: Option<String>,
    #[serde(default)]
    responsible: Option<String>,
    #[serde(default, alias = "softwareComponent")]
    software_component: Option<String>,
    #[serde(default, alias = "serviceDefinition")]
    service_definition: Option<String>,
    #[serde(default, alias = "bindingType")]
    binding_type: Option<String>,
    #[serde(default, alias = "bindingVersion")]
    binding_version: Option<String>,
    #[serde(default, alias = "bindingCategory")]
    binding_category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreatePackageArgs {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    transport: Option<String>,
    #[serde(default, alias = "softwareComponent", alias = "software_component")]
    software_component: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteObjectArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "lockHandle", alias = "lock_handle")]
    lock_handle: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServiceBindingArgs {
    #[serde(default, alias = "serviceName")]
    service_name: Option<String>,
    #[serde(default, alias = "serviceVersion")]
    service_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RunUnitTestsArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "includeDangerous", alias = "include_dangerous")]
    include_dangerous: Option<bool>,
    #[serde(default, alias = "includeLong", alias = "include_long")]
    include_long: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RunAtcCheckArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default, alias = "maxResults", alias = "max_results")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetUserTransportsArgs {
    #[serde(default, alias = "user", alias = "userName")]
    user_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetTransportInfoArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "devClass", alias = "dev_class")]
    dev_class: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListTransportsArgs {
    #[serde(default)]
    user: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransportNumberArgs {
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTransportArgs {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default, alias = "transportLayer", alias = "transport_layer")]
    transport_layer: Option<String>,
    #[serde(default, alias = "type")]
    request_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseTransportArgs {
    #[serde(default)]
    transport: Option<String>,
    #[serde(default, alias = "ignoreLocks", alias = "ignore_locks")]
    ignore_locks: Option<bool>,
    #[serde(default, alias = "skipATC", alias = "skip_atc")]
    skip_atc: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RunReportArgs {
    #[serde(default)]
    report: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ReportNameArgs {
    #[serde(default)]
    report: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetTextElementsArgs {
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetTextElementsArgs {
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    selection_texts: Option<Value>,
    #[serde(default)]
    text_symbols: Option<Value>,
    #[serde(default)]
    heading_texts: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct BreakpointIdArgs {
    #[serde(default, alias = "breakpointId", alias = "breakpoint_id")]
    breakpoint_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DebuggerListenArgs {
    #[serde(default)]
    timeout: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct DebuggeeArgs {
    #[serde(default, alias = "debuggeeId", alias = "debuggee_id")]
    debuggee_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StepArgs {
    #[serde(default, alias = "type", alias = "stepType", alias = "step_type")]
    step_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VariablesScopeArgs {
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AmdpStartArgs {
    #[serde(default, alias = "cascadeMode", alias = "cascade_mode")]
    cascade_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AmdpBreakpointArgs {
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    line: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LockObjectArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "accessMode", alias = "access_mode")]
    access_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnlockObjectArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "lockHandle", alias = "lock_handle")]
    lock_handle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActivateArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default, alias = "objectName", alias = "object_name", alias = "name")]
    object_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyntaxCheckArgs {
    #[serde(default, alias = "objectUrl", alias = "object_url")]
    object_url: Option<String>,
    #[serde(default)]
    content: Option<String>,
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
struct ClassIncludeArgs {
    #[serde(default, alias = "className", alias = "name")]
    class_name: Option<String>,
    #[serde(default, alias = "includeType")]
    include_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateClassIncludeArgs {
    #[serde(default, alias = "className", alias = "name")]
    class_name: Option<String>,
    #[serde(default, alias = "includeType")]
    include_type: Option<String>,
    source: String,
    #[serde(default, alias = "lockHandle")]
    lock_handle: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTestIncludeArgs {
    #[serde(default, alias = "className", alias = "name")]
    class_name: Option<String>,
    #[serde(default, alias = "lockHandle")]
    lock_handle: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrettyPrintArgs {
    source: String,
}

#[derive(Debug, Deserialize)]
struct GetObjectStructureArgs {
    #[serde(default, alias = "objectName", alias = "name")]
    object_name: Option<String>,
    #[serde(default, alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetClassComponentsArgs {
    #[serde(alias = "classUrl")]
    class_url: String,
}

#[derive(Debug, Deserialize)]
struct GetCallGraphArgs {
    #[serde(default, alias = "objectUri")]
    object_uri: Option<String>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default, alias = "maxDepth")]
    max_depth: Option<u32>,
    #[serde(default, alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetCallTraversalArgs {
    #[serde(default, alias = "objectUri")]
    object_uri: Option<String>,
    #[serde(default, alias = "maxDepth")]
    max_depth: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeCallGraphArgs {
    #[serde(default, alias = "objectUri")]
    object_uri: Option<String>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default, alias = "maxDepth")]
    max_depth: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CompareCallGraphsArgs {
    #[serde(default, alias = "objectUri")]
    object_uri: Option<String>,
    #[serde(default, alias = "traceData")]
    trace_data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetPrettyPrinterSettingsArgs {
    indentation: bool,
    style: String,
}

#[derive(Debug, Deserialize)]
struct FindDefinitionArgs {
    #[serde(alias = "sourceUrl")]
    source_url: String,
    source: String,
    line: u32,
    #[serde(alias = "start_column", alias = "startColumn", alias = "startCol")]
    start_col: u32,
    #[serde(alias = "end_column", alias = "endColumn", alias = "endCol")]
    end_col: u32,
    #[serde(default)]
    implementation: bool,
    #[serde(default, alias = "mainProgram")]
    main_program: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FindReferencesArgs {
    #[serde(alias = "objectUrl")]
    object_url: String,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CodeCompletionArgs {
    #[serde(alias = "sourceUrl")]
    source_url: String,
    source: String,
    line: u32,
    column: u32,
}

#[derive(Debug, Deserialize)]
struct WsRequestArgs {
    domain: String,
    action: String,
    #[serde(default)]
    payload: Value,
}

fn extract_non_empty_string(fields: &BTreeMap<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        fields.get(*key).and_then(|value| {
            value.as_str().and_then(|text| {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            })
        })
    })
}

struct CreateObjectTypeInfo {
    creation_path: String,
    root_name: &'static str,
    namespace: &'static str,
    content_type: &'static str,
}

fn resolve_create_object_type_info(
    object_type: &str,
    parent_name: Option<&str>,
) -> Option<CreateObjectTypeInfo> {
    let info = match object_type {
        "PROG/P" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/programs/programs".to_owned(),
            root_name: "program:abapProgram",
            namespace: r#"xmlns:program="http://www.sap.com/adt/programs/programs""#,
            content_type: "application/*",
        },
        "PROG/I" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/programs/includes".to_owned(),
            root_name: "include:abapInclude",
            namespace: r#"xmlns:include="http://www.sap.com/adt/programs/includes""#,
            content_type: "application/*",
        },
        "CLAS/OC" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/oo/classes".to_owned(),
            root_name: "class:abapClass",
            namespace: r#"xmlns:class="http://www.sap.com/adt/oo/classes""#,
            content_type: "application/*",
        },
        "INTF/OI" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/oo/interfaces".to_owned(),
            root_name: "intf:abapInterface",
            namespace: r#"xmlns:intf="http://www.sap.com/adt/oo/interfaces""#,
            content_type: "application/*",
        },
        "FUGR/F" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/functions/groups".to_owned(),
            root_name: "group:abapFunctionGroup",
            namespace: r#"xmlns:group="http://www.sap.com/adt/functions/groups""#,
            content_type: "application/*",
        },
        "FUGR/FF" => CreateObjectTypeInfo {
            creation_path: format!(
                "/sap/bc/adt/functions/groups/{}/fmodules",
                encode_path_segment(parent_name?.to_ascii_uppercase().as_str())
            ),
            root_name: "fmodule:abapFunctionModule",
            namespace: r#"xmlns:fmodule="http://www.sap.com/adt/functions/fmodules""#,
            content_type: "application/*",
        },
        "DEVC/K" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/packages".to_owned(),
            root_name: "pack:package",
            namespace: r#"xmlns:pack="http://www.sap.com/adt/packages""#,
            content_type: "application/*",
        },
        "DDLS/DF" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/ddic/ddl/sources".to_owned(),
            root_name: "ddl:ddlSource",
            namespace: r#"xmlns:ddl="http://www.sap.com/adt/ddic/ddlsources""#,
            content_type: "application/*",
        },
        "BDEF/BDO" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/bo/behaviordefinitions".to_owned(),
            root_name: "bdef:behaviorDefinition",
            namespace: r#"xmlns:bdef="http://www.sap.com/adt/bo/behaviordefinitions""#,
            content_type: "application/vnd.sap.adt.blues.v1+xml",
        },
        "SRVD/SRV" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/ddic/srvd/sources".to_owned(),
            root_name: "srvd:srvdSource",
            namespace: r#"xmlns:srvd="http://www.sap.com/adt/ddic/srvdsources""#,
            content_type: "application/*",
        },
        "SRVB/SVB" => CreateObjectTypeInfo {
            creation_path: "/sap/bc/adt/businessservices/bindings".to_owned(),
            root_name: "srvb:serviceBinding",
            namespace: r#"xmlns:srvb="http://www.sap.com/adt/ddic/ServiceBindings""#,
            content_type: "application/*",
        },
        _ => return None,
    };
    Some(info)
}

fn build_create_object_body(
    object_type: &str,
    name: &str,
    description: &str,
    package_name: &str,
    parent_name: &str,
    responsible: &str,
    args: &CreateObjectArgs,
    type_info: &CreateObjectTypeInfo,
) -> String {
    if object_type == "DEVC/K" {
        let software_component = if name.starts_with('$') {
            "LOCAL".to_owned()
        } else {
            args.software_component
                .clone()
                .unwrap_or_default()
                .to_ascii_uppercase()
        };
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{} {} xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\">\n  <pack:attributes pack:packageType=\"development\"/>\n  <pack:superPackage adtcore:name=\"{}\" adtcore:type=\"DEVC/K\"/>\n  <pack:applicationComponent/>\n  <pack:transport>\n    <pack:softwareComponent pack:name=\"{}\"/>\n    <pack:transportLayer pack:name=\"\"/>\n  </pack:transport>\n  <pack:translation/>\n  <pack:useAccesses/>\n  <pack:packageInterfaces/>\n  <pack:subPackages/>\n</{}>",
            type_info.root_name,
            type_info.namespace,
            escape_xml(description),
            escape_xml(name),
            object_type,
            escape_xml(responsible),
            escape_xml(package_name),
            escape_xml(software_component.as_str()),
            type_info.root_name
        );
    }

    if object_type == "FUGR/FF" {
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{} {} xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\">\n  <adtcore:containerRef adtcore:name=\"{}\" adtcore:type=\"FUGR/F\"\n    adtcore:uri=\"/sap/bc/adt/functions/groups/{}\"/>\n</{}>",
            type_info.root_name,
            type_info.namespace,
            escape_xml(description),
            escape_xml(name),
            object_type,
            escape_xml(responsible),
            escape_xml(parent_name),
            encode_path_segment(parent_name.to_ascii_lowercase().as_str()),
            type_info.root_name
        );
    }

    if object_type == "SRVD/SRV" {
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{} {} xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\"\n  srvd:srvdSourceType=\"S\">\n  <adtcore:packageRef adtcore:name=\"{}\"/>\n</{}>",
            type_info.root_name,
            type_info.namespace,
            escape_xml(description),
            escape_xml(name),
            object_type,
            escape_xml(responsible),
            escape_xml(package_name),
            type_info.root_name
        );
    }

    if object_type == "SRVB/SVB" {
        let binding_type = args
            .binding_type
            .clone()
            .unwrap_or_else(|| "ODATA".to_owned());
        let binding_version = args
            .binding_version
            .clone()
            .unwrap_or_else(|| "V2".to_owned());
        let binding_category = args
            .binding_category
            .clone()
            .unwrap_or_else(|| "0".to_owned());
        let service_definition = args
            .service_definition
            .clone()
            .unwrap_or_default()
            .to_ascii_uppercase();
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{} {} xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\">\n  <adtcore:packageRef adtcore:name=\"{}\"/>\n  <srvb:services srvb:name=\"{}\">\n    <srvb:content srvb:version=\"0001\">\n      <srvb:serviceDefinition adtcore:name=\"{}\"/>\n    </srvb:content>\n  </srvb:services>\n  <srvb:binding srvb:category=\"{}\" srvb:type=\"{}\" srvb:version=\"{}\">\n    <srvb:implementation adtcore:name=\"\"/>\n  </srvb:binding>\n</{}>",
            type_info.root_name,
            type_info.namespace,
            escape_xml(description),
            escape_xml(name),
            object_type,
            escape_xml(responsible),
            escape_xml(package_name),
            escape_xml(name),
            escape_xml(service_definition.as_str()),
            escape_xml(binding_category.as_str()),
            escape_xml(binding_type.as_str()),
            escape_xml(binding_version.as_str()),
            type_info.root_name
        );
    }

    if object_type == "BDEF/BDO" {
        return format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<blue:blueSource xmlns:blue=\"http://www.sap.com/wbobj/blue\" xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\">\n  <adtcore:packageRef adtcore:name=\"{}\"/>\n</blue:blueSource>",
            escape_xml(description),
            escape_xml(name),
            object_type,
            escape_xml(responsible),
            escape_xml(package_name)
        );
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{} {} xmlns:adtcore=\"http://www.sap.com/adt/core\"\n  adtcore:description=\"{}\"\n  adtcore:name=\"{}\"\n  adtcore:type=\"{}\"\n  adtcore:responsible=\"{}\">\n  <adtcore:packageRef adtcore:name=\"{}\"/>\n</{}>",
        type_info.root_name,
        type_info.namespace,
        escape_xml(description),
        escape_xml(name),
        object_type,
        escape_xml(responsible),
        escape_xml(package_name),
        type_info.root_name
    )
}

fn build_object_url(object_type: &str, name: &str, parent_name: Option<&str>) -> String {
    let encoded_name = encode_path_segment(name);
    match object_type {
        "PROG/P" => format!("/sap/bc/adt/programs/programs/{encoded_name}"),
        "PROG/I" => format!("/sap/bc/adt/programs/includes/{encoded_name}"),
        "CLAS/OC" => format!("/sap/bc/adt/oo/classes/{encoded_name}"),
        "INTF/OI" => format!("/sap/bc/adt/oo/interfaces/{encoded_name}"),
        "FUGR/F" => format!("/sap/bc/adt/functions/groups/{encoded_name}"),
        "FUGR/FF" => format!(
            "/sap/bc/adt/functions/groups/{}/fmodules/{encoded_name}",
            encode_path_segment(
                parent_name
                    .unwrap_or_default()
                    .to_ascii_uppercase()
                    .as_str()
            )
        ),
        "DEVC/K" => format!("/sap/bc/adt/packages/{encoded_name}"),
        "DDLS/DF" => format!(
            "/sap/bc/adt/ddic/ddl/sources/{}",
            encode_path_segment(name.to_ascii_lowercase().as_str())
        ),
        "BDEF/BDO" => format!(
            "/sap/bc/adt/bo/behaviordefinitions/{}",
            encode_path_segment(name.to_ascii_lowercase().as_str())
        ),
        "SRVD/SRV" => format!(
            "/sap/bc/adt/ddic/srvd/sources/{}",
            encode_path_segment(name.to_ascii_lowercase().as_str())
        ),
        "SRVB/SVB" => format!(
            "/sap/bc/adt/businessservices/bindings/{}",
            encode_path_segment(name.to_ascii_lowercase().as_str())
        ),
        _ => String::new(),
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn extract_system_check_variant(customizing_xml: &str) -> Option<String> {
    let marker = "name=\"systemCheckVariant\"";
    let marker_index = customizing_xml.find(marker)?;
    let fragment = &customizing_xml[marker_index + marker.len()..];
    extract_xml_attr_value(fragment, "value")
}

fn extract_xml_attr_value(fragment: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = fragment.find(pattern.as_str())? + pattern.len();
    let tail = &fragment[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_owned())
}

fn extract_xml_tag_value(xml: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{tag_name}>");
    let close_tag = format!("</{tag_name}>");
    let start = xml.find(open_tag.as_str())? + open_tag.len();
    let tail = &xml[start..];
    let end = tail.find(close_tag.as_str())?;
    Some(tail[..end].trim().to_owned())
}

fn encode_path_segment(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn encode_query_value(value: &str) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("value", value);
    serializer
        .finish()
        .strip_prefix("value=")
        .unwrap_or_default()
        .to_owned()
}

fn build_source_uri(path_pattern: &str, name: &str) -> String {
    path_pattern.replace("{name}", encode_path_segment(name).as_str())
}

fn build_class_include_uri(class_name: &str, include_type: &str) -> String {
    let encoded_class_name = encode_path_segment(class_name.to_ascii_uppercase().as_str());
    if include_type.eq_ignore_ascii_case("main") {
        format!("/sap/bc/adt/oo/classes/{encoded_class_name}/source/main")
    } else {
        format!(
            "/sap/bc/adt/oo/classes/{encoded_class_name}/includes/{}",
            include_type.to_ascii_lowercase()
        )
    }
}

fn build_path_with_query(path: &str, query: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in query {
        serializer.append_pair(key, value.as_str());
    }
    let encoded = serializer.finish();
    if encoded.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{encoded}")
    }
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
            .invoke("ActivatePackage", json!({}))
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
