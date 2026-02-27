use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use neuro_engine::{NeuroEngine, NeuroEngineError};
use neuro_types::AdtUpdateSourceRequest;
use regex::{Regex, RegexBuilder};
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

const NEURO_INTERNAL_TOOL_NAMES: &[&str] = &[
    "diagnose",
    "search",
    "get_source",
    "update_source",
    "ws_request",
];
const IMPLEMENTED_TOOL_NAMES: &[&str] = &[
    "diagnose",
    "search",
    "get_source",
    "update_source",
    "ws_request",
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
            "GetProgram" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/programs/{name}/source/main",
                    &["programName", "program_name", "name"],
                )
                .await
            }
            "GetClass" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/classes/{name}/source/main",
                    &["className", "class_name", "name"],
                )
                .await
            }
            "GetInterface" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/interfaces/{name}/source/main",
                    &["interfaceName", "interface_name", "name"],
                )
                .await
            }
            "GetInclude" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/includes/{name}/source/main",
                    &["includeName", "include_name", "name"],
                )
                .await
            }
            "GetFunction" => self.handle_get_function(arguments, tool_name).await,
            "GetFunctionGroup" => {
                self.handle_get_raw_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/functions/groups/{name}",
                    &["groupName", "group_name", "name"],
                    Some("application/xml"),
                )
                .await
            }
            "GetMessages" => self.handle_get_messages(arguments, tool_name).await,
            "GetPackage" => self.handle_get_package(arguments, tool_name).await,
            "GetTable" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/ddic/tables/{name}/source/main",
                    &["tableName", "table_name", "name"],
                )
                .await
            }
            "GetTableContents" => self.handle_get_table_contents(arguments, tool_name).await,
            "GetStructure" => {
                self.handle_get_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/ddic/structures/{name}/source/main",
                    &["structureName", "structure_name", "name"],
                )
                .await
            }
            "GetTransaction" => self.handle_get_transaction(arguments, tool_name).await,
            "GetTypeInfo" => self.handle_get_type_info(arguments, tool_name).await,
            "GetCDSDependencies" => self.handle_get_cds_dependencies(arguments, tool_name).await,
            "GetClassInfo" => self.handle_get_class_info(arguments, tool_name).await,
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
            "TraceExecution" => self.handle_trace_execution(arguments, tool_name).await,
            "GetInactiveObjects" => self.handle_get_inactive_objects(arguments, tool_name).await,
            "GetInstalledComponents" => {
                self.handle_get_installed_components(arguments, tool_name)
                    .await
            }
            "GetATCCustomizing" => self.handle_get_atc_customizing(arguments, tool_name).await,
            "GetSystemInfo" => self.handle_get_system_info(arguments, tool_name).await,
            "GetConnectionInfo" => self.handle_get_connection_info(arguments, tool_name).await,
            "GetFeatures" => self.handle_get_features(arguments, tool_name).await,
            "PrettyPrint" => self.handle_pretty_print(arguments, tool_name).await,
            "GetPrettyPrinterSettings" => {
                self.handle_get_pretty_printer_settings(arguments, tool_name)
                    .await
            }
            "SetPrettyPrinterSettings" => {
                self.handle_set_pretty_printer_settings(arguments, tool_name)
                    .await
            }
            "FindDefinition" => self.handle_find_definition(arguments, tool_name).await,
            "FindReferences" => self.handle_find_references(arguments, tool_name).await,
            "CodeCompletion" => self.handle_code_completion(arguments, tool_name).await,
            "GrepObject" => self.handle_grep_object(arguments, tool_name).await,
            "GrepObjects" => self.handle_grep_objects(arguments, tool_name).await,
            "GrepPackage" => self.handle_grep_package(arguments, tool_name).await,
            "GrepPackages" => self.handle_grep_packages(arguments, tool_name).await,
            "RunUnitTests" => self.handle_run_unit_tests(arguments, tool_name).await,
            "RunATCCheck" => self.handle_run_atc_check(arguments, tool_name).await,
            "ActivatePackage" => self.handle_activate_package(arguments, tool_name).await,
            "GetUserTransports" => self.handle_get_user_transports(arguments, tool_name).await,
            "GetTransportInfo" => self.handle_get_transport_info(arguments, tool_name).await,
            "ListTransports" => self.handle_list_transports(arguments, tool_name).await,
            "GetTransport" => self.handle_get_transport(arguments, tool_name).await,
            "CreateTransport" => self.handle_create_transport(arguments, tool_name).await,
            "ReleaseTransport" => self.handle_release_transport(arguments, tool_name).await,
            "DeleteTransport" => self.handle_delete_transport(arguments, tool_name).await,
            "ListDumps" => self.handle_list_dumps(arguments, tool_name).await,
            "GetDump" => self.handle_get_dump(arguments, tool_name).await,
            "ListTraces" => self.handle_list_traces(arguments, tool_name).await,
            "GetTrace" => self.handle_get_trace(arguments, tool_name).await,
            "GetSQLTraceState" => self.handle_get_sql_trace_state(arguments, tool_name).await,
            "ListSQLTraces" => self.handle_list_sql_traces(arguments, tool_name).await,
            "RunReport" => self.handle_run_report(arguments, tool_name).await,
            "RunReportAsync" => self.handle_run_report_async(arguments, tool_name).await,
            "GetAsyncResult" => self.handle_get_async_result(arguments, tool_name).await,
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
                self.handle_debugger_get_variables(arguments, tool_name)
                    .await
            }
            "AMDPDebuggerStart" => self.handle_amdp_start(arguments, tool_name).await,
            "AMDPDebuggerResume" => self.handle_amdp_resume(arguments, tool_name).await,
            "AMDPDebuggerStop" => self.handle_amdp_stop(arguments, tool_name).await,
            "AMDPDebuggerStep" => self.handle_amdp_step(arguments, tool_name).await,
            "AMDPGetVariables" => self.handle_amdp_get_variables(arguments, tool_name).await,
            "AMDPSetBreakpoint" => self.handle_amdp_set_breakpoint(arguments, tool_name).await,
            "AMDPGetBreakpoints" => self.handle_amdp_get_breakpoints(arguments, tool_name).await,
            "UI5ListApps" => self.handle_ui5_list_apps(arguments, tool_name).await,
            "UI5GetApp" => self.handle_ui5_get_app(arguments, tool_name).await,
            "UI5GetFileContent" => self.handle_ui5_get_file_content(arguments, tool_name).await,
            "UI5UploadFile" => self.handle_ui5_upload_file(arguments, tool_name).await,
            "UI5DeleteFile" => self.handle_ui5_delete_file(arguments, tool_name).await,
            "UI5CreateApp" => self.handle_ui5_create_app(arguments, tool_name).await,
            "UI5DeleteApp" => self.handle_ui5_delete_app(arguments, tool_name).await,
            "CallRFC" => self.handle_call_rfc(arguments, tool_name).await,
            "ExecuteABAP" => self.handle_execute_abap(arguments, tool_name).await,
            "MoveObject" => self.handle_move_object(arguments, tool_name).await,
            "GetTypeHierarchy" => self.handle_get_type_hierarchy(arguments, tool_name).await,
            "GitTypes" => self.handle_git_types(arguments, tool_name).await,
            "GitExport" => self.handle_git_export(arguments, tool_name).await,
            "InstallZADTVSP" => self.handle_install_zadtvsp(arguments, tool_name).await,
            "InstallAbapGit" => self.handle_install_abap_git(arguments, tool_name).await,
            "ListDependencies" => self.handle_list_dependencies(arguments, tool_name).await,
            "InstallDummyTest" => self.handle_install_dummy_test(arguments, tool_name).await,
            "CreateObject" => self.handle_create_object(arguments, tool_name).await,
            "CreatePackage" => self.handle_create_package(arguments, tool_name).await,
            "CreateTable" => self.handle_create_table(arguments, tool_name).await,
            "CreateAndActivateProgram" => {
                self.handle_create_and_activate_program(arguments, tool_name)
                    .await
            }
            "CreateClassWithTests" => {
                self.handle_create_class_with_tests(arguments, tool_name)
                    .await
            }
            "DeleteObject" => self.handle_delete_object(arguments, tool_name).await,
            "CloneObject" => self.handle_clone_object(arguments, tool_name).await,
            "RenameObject" => self.handle_rename_object(arguments, tool_name).await,
            "CompareSource" => self.handle_compare_source(arguments, tool_name).await,
            "EditSource" => self.handle_edit_source(arguments, tool_name).await,
            "SaveToFile" | "ExportToFile" => self.handle_save_to_file(arguments, tool_name).await,
            "DeployFromFile" | "ImportFromFile" => {
                self.handle_deploy_from_file(arguments, tool_name).await
            }
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
            "WriteProgram" => {
                self.handle_update_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/programs/programs/{name}/source/main",
                    &["programName", "program_name", "name"],
                )
                .await
            }
            "WriteClass" => {
                self.handle_update_source_by_pattern(
                    arguments,
                    tool_name,
                    "/sap/bc/adt/oo/classes/{name}/source/main",
                    &["className", "class_name", "name"],
                )
                .await
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

    async fn handle_search(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: SearchArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;

        let objects = self
            .engine
            .search(args.query.as_str(), args.max_results)
            .await?;
        Ok(json!({ "objects": objects }))
    }

    async fn handle_get_source(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: UpdateSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: NamedSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: FunctionSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;

        let function_name = args
            .function_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "function name is required".to_owned(),
            })?;
        let group_name = args
            .group_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;

        let name = args.extract_name(accepted_name_keys).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("expected one of {:?} in arguments", accepted_name_keys),
            }
        })?;

        let object_uri = build_source_uri(path_pattern, name.as_str());
        let raw = self
            .engine
            .get_raw_text(object_uri.as_str(), accept)
            .await?;
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
            .get_raw_text(
                object_uri.as_str(),
                Some("application/vnd.sap.adt.mc.messageclass+xml"),
            )
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let raw = self
            .engine
            .post_raw_text(path.as_str(), None, None, None)
            .await?;
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: NamedObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
            .get_raw_text(
                path.as_str(),
                Some("application/vnd.sap.adt.codegen.data.v1+xml"),
            )
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_table_contents(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: TableContentsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let table_name = args
            .table_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: CreateObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        self.create_object_request(&args, tool_name).await
    }

    async fn handle_create_package(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreatePackageArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let name = args.name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name is required".to_owned(),
        })?;
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package_name = args.parent.unwrap_or_default();
        let transport = args.transport.unwrap_or_default();
        if !name.trim_start().starts_with('$') && transport.trim().is_empty() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "transport is required for transportable packages (non-$ packages)"
                    .to_owned(),
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
        let args: DeleteObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUrl/object_url is required".to_owned(),
            })?;
        let lock_handle = args
            .lock_handle
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: ServiceBindingArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let service_name = args
            .service_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "serviceName/service_name is required".to_owned(),
            })?;
        let service_version = args.service_version.unwrap_or_else(|| "0001".to_owned());
        let action = if publish {
            "publishjobs"
        } else {
            "unpublishjobs"
        };
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
        let object_type = args
            .object_type
            .as_deref()
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectType/object_type is required".to_owned(),
            })?
            .trim()
            .to_ascii_uppercase();
        let name = args
            .name
            .as_deref()
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "name is required".to_owned(),
            })?
            .trim()
            .to_ascii_uppercase();
        let description = args
            .description
            .as_deref()
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
            message: format!(
                "unsupported object_type `{object_type}` or missing required parent_name"
            ),
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
        let args: LockObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: UnlockObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUrl/object_url is required".to_owned(),
            })?;
        let lock_handle = args
            .lock_handle
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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

        let raw = self
            .engine
            .post_raw_text(path.as_str(), None, None, None)
            .await?;
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
        let args: ActivateArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUrl/object_url is required".to_owned(),
            })?;
        let object_name = args
            .object_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectName/object_name is required".to_owned(),
            })?;

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<adtcore:objectReferences xmlns:adtcore=\"http://www.sap.com/adt/core\">\n  <adtcore:objectReference adtcore:uri=\"{}\" adtcore:name=\"{}\"/>\n</adtcore:objectReferences>",
            object_url, object_name
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
        let args: SyntaxCheckArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUrl/object_url is required".to_owned(),
            })?;
        let source_content = args
            .content
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
            source_url, source_url, encoded
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
        let args: ClassIncludeArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let class_name = args
            .class_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "className/class_name is required".to_owned(),
            })?;
        let include_type = args
            .include_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: UpdateClassIncludeArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let class_name = args
            .class_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "className/class_name is required".to_owned(),
            })?;
        let include_type = args
            .include_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "includeType/include_type is required".to_owned(),
            })?;
        let lock_handle = args
            .lock_handle
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let class_name = args
            .class_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "className/class_name is required".to_owned(),
            })?;
        let lock_handle = args
            .lock_handle
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
            .post_raw_text(endpoint.as_str(), Some(body), Some("application/*"), None)
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
        let object_name = args
            .object_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUri/object_uri is required".to_owned(),
            })?;
        let direction = args.direction.unwrap_or_else(|| "callers".to_owned());
        let max_depth = args.max_depth.unwrap_or(3).max(1);
        let max_results = args.max_results.unwrap_or(100).max(1);

        let raw = self
            .request_call_graph(
                object_uri.as_str(),
                direction.as_str(),
                max_depth,
                max_results,
            )
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
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUri/object_uri is required".to_owned(),
            })?;
        let direction = args.direction.unwrap_or_else(|| "callees".to_owned());
        let max_depth = args.max_depth.unwrap_or(5).max(1);
        let max_results = 1000;

        let raw = self
            .request_call_graph(
                object_uri.as_str(),
                direction.as_str(),
                max_depth,
                max_results,
            )
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
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUri/object_uri is required".to_owned(),
            })?;
        let trace_data = args
            .trace_data
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "trace_data is required".to_owned(),
            })?;
        let actual_edges: Value = serde_json::from_str(trace_data.as_str()).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("trace_data must be valid JSON: {error}"),
            }
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

    async fn run_freestyle_query(
        &self,
        query: &str,
        max_rows: u32,
    ) -> Result<String, NeuroMcpError> {
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
        let args: PrettyPrintArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: FindDefinitionArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: FindReferencesArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: CodeCompletionArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: RunUnitTestsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: RunAtcCheckArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
            .get_raw_text(
                findings_endpoint.as_str(),
                Some("application/atc.worklist.v1+xml"),
            )
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
        let user_name = args
            .user_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "user_name is required".to_owned(),
            })?;
        let user_name = user_name.trim().to_ascii_uppercase();
        let endpoint = build_path_with_query(
            "/sap/bc/adt/cts/transportrequests",
            &[("user", user_name.clone()), ("targets", "true".to_owned())],
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
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objectUrl/object_url is required".to_owned(),
            })?;
        let dev_class = args
            .dev_class
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let transport_number = args
            .transport
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package = args
            .package
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let transport = args
            .transport
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let transport = args
            .transport
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: RunReportArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: ReportNameArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: GetTextElementsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let program = args
            .program
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: SetTextElementsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let program = args
            .program
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let response = self
            .engine
            .send_domain_request("report", action, payload)
            .await?;
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
        let args: BreakpointIdArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let breakpoint_id = args
            .breakpoint_id
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: DebuggerListenArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: DebuggeeArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let debuggee_id = args
            .debuggee_id
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let args: StepArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let step_type = args.step_type.unwrap_or_else(|| "into".to_owned());
        self.handle_debug_ws_call("step", json!({ "type": step_type }))
            .await
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
        let args: VariablesScopeArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: AmdpStartArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: StepArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let args: AmdpBreakpointArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let program = args
            .program
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
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
        let response = self
            .engine
            .send_domain_request("debug", action, payload)
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_amdp_ws_call(
        &self,
        action: &str,
        payload: Value,
    ) -> Result<Value, NeuroMcpError> {
        let response = self
            .engine
            .send_domain_request("amdp", action, payload)
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_ui5_list_apps(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5ListAppsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let max_results = args.max_results.unwrap_or(100).max(1);
        let mut query = vec![("maxResults", max_results.to_string())];
        if let Some(query_name) = args.query {
            if !query_name.trim().is_empty() {
                query.push(("name", query_name));
            }
        }
        let endpoint = build_path_with_query("/sap/bc/adt/filestore/ui5-bsp/objects", &query);
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/atom+xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_ui5_get_app(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5AppArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let app_name = app_name.trim().to_ascii_uppercase();
        let endpoint = format!(
            "/sap/bc/adt/filestore/ui5-bsp/objects/{}/content",
            encode_path_segment(app_name.as_str())
        );
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/atom+xml"))
            .await?;
        Ok(json!({
            "appName": app_name,
            "raw": raw,
        }))
    }

    async fn handle_ui5_get_file_content(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5FileArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let file_path = args
            .file_path
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "file_path is required".to_owned(),
            })?;
        let endpoint = build_ui5_file_content_path(app_name.as_str(), file_path.as_str());
        let raw = self.engine.get_raw_text(endpoint.as_str(), None).await?;
        Ok(json!({
            "appName": app_name,
            "filePath": file_path,
            "raw": raw,
        }))
    }

    async fn handle_ui5_upload_file(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5UploadFileArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let file_path = args
            .file_path
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "file_path is required".to_owned(),
            })?;
        let content = args
            .content
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "content is required".to_owned(),
            })?;
        let endpoint = build_ui5_file_content_path(app_name.as_str(), file_path.as_str());
        let raw = self
            .engine
            .put_raw_text(
                endpoint.as_str(),
                Some(content.as_str()),
                Some(
                    args.content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream"),
                ),
                None,
            )
            .await?;
        Ok(json!({
            "appName": app_name,
            "filePath": file_path,
            "raw": raw,
        }))
    }

    async fn handle_ui5_delete_file(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5FileArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let file_path = args
            .file_path
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "file_path is required".to_owned(),
            })?;
        let app_upper = app_name.trim().to_ascii_uppercase();
        let file_rel = file_path.trim().trim_start_matches('/');
        let full_path = format!("{app_upper}/{file_rel}");
        let endpoint = format!(
            "/sap/bc/adt/filestore/ui5-bsp/objects/{}",
            encode_path_segment(full_path.as_str())
        );
        let raw = self.engine.delete_raw_text(endpoint.as_str(), None).await?;
        Ok(json!({
            "appName": app_name,
            "filePath": file_path,
            "raw": raw,
        }))
    }

    async fn handle_ui5_create_app(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5CreateAppArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package_name = args
            .package_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package_name is required".to_owned(),
            })?;

        let mut query = Vec::new();
        if let Some(transport) = args.transport {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport));
            }
        }
        let endpoint = build_path_with_query("/sap/bc/adt/filestore/ui5-bsp/objects", &query);
        let app_upper = app_name.trim().to_ascii_uppercase();
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<bsp:application xmlns:bsp=\"http://www.sap.com/adt/bsp\"\n    xmlns:adtcore=\"http://www.sap.com/adt/core\"\n    adtcore:name=\"{}\"\n    adtcore:description=\"{}\"\n    adtcore:packageName=\"{}\">\n</bsp:application>",
            escape_xml(app_upper.as_str()),
            escape_xml(description.as_str()),
            escape_xml(package_name.as_str())
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(body.as_str()),
                Some("application/xml"),
                None,
            )
            .await?;
        Ok(json!({
            "appName": app_upper,
            "raw": raw,
        }))
    }

    async fn handle_ui5_delete_app(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: Ui5DeleteAppArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let app_name = args
            .app_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "app_name is required".to_owned(),
            })?;
        let app_upper = app_name.trim().to_ascii_uppercase();
        let mut endpoint = format!(
            "/sap/bc/adt/filestore/ui5-bsp/objects/{}",
            encode_path_segment(app_upper.as_str())
        );
        if let Some(transport) = args.transport {
            if !transport.trim().is_empty() {
                endpoint = build_path_with_query(endpoint.as_str(), &[("corrNr", transport)]);
            }
        }
        let raw = self.engine.delete_raw_text(endpoint.as_str(), None).await?;
        Ok(json!({
            "appName": app_upper,
            "raw": raw,
        }))
    }

    async fn handle_call_rfc(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CallRfcArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let function = args
            .function
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "function is required".to_owned(),
            })?;

        let mut payload = serde_json::Map::new();
        payload.insert("function".to_owned(), json!(function));
        if let Some(params) = args.params {
            let params_value = if let Some(params_str) = params.as_str() {
                serde_json::from_str::<Value>(params_str).unwrap_or_else(|_| json!(params_str))
            } else {
                params
            };
            if let Some(object) = params_value.as_object() {
                for (key, value) in object {
                    payload.insert(key.clone(), value.clone());
                }
            } else {
                payload.insert("params".to_owned(), params_value);
            }
        }
        let response = self
            .engine
            .send_domain_request("rfc", "call", Value::Object(payload))
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_move_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: MoveObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_type = args
            .object_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_type is required".to_owned(),
            })?;
        let object_name = args
            .object_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_name is required".to_owned(),
            })?;
        let new_package = args
            .new_package
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "new_package is required".to_owned(),
            })?;
        let response = self
            .engine
            .send_domain_request(
                "rfc",
                "moveToPackage",
                json!({
                    "object": object_type,
                    "obj_name": object_name,
                    "new_package": new_package,
                }),
            )
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_get_type_hierarchy(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetTypeHierarchyArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let source_url = args
            .source_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "source_url is required".to_owned(),
            })?;
        let source = args.source.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "source is required".to_owned(),
        })?;
        let line = args
            .line
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "line is required".to_owned(),
            })?
            .max(1);
        let column = args
            .column
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "column is required".to_owned(),
            })?
            .max(1);
        let type_param = if args.super_types.unwrap_or(false) {
            "superTypes"
        } else {
            "subTypes"
        };
        let uri = format!("{source_url}#start={line},{column}");
        let endpoint = build_path_with_query(
            "/sap/bc/adt/abapsource/typehierarchy",
            &[("uri", uri), ("type", type_param.to_owned())],
        );
        let raw = self
            .engine
            .post_raw_text(
                endpoint.as_str(),
                Some(source.as_str()),
                Some("text/plain"),
                Some("application/*"),
            )
            .await?;
        Ok(json!({
            "sourceUrl": source_url,
            "line": line,
            "column": column,
            "type": type_param,
            "raw": raw,
        }))
    }

    async fn handle_git_types(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let response = self
            .engine
            .send_domain_request("git", "getTypes", json!({}))
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn handle_git_export(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GitExportArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let mut payload = serde_json::Map::new();
        if let Some(packages) = args.packages {
            let items = packages
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if !items.is_empty() {
                payload.insert("packages".to_owned(), json!(items));
            }
        }
        if let Some(objects) = args.objects {
            let parsed = serde_json::from_str::<Value>(objects.as_str()).map_err(|error| {
                NeuroMcpError::InvalidArguments {
                    tool: tool_name.to_owned(),
                    message: format!("invalid objects JSON: {error}"),
                }
            })?;
            payload.insert("objects".to_owned(), parsed);
        }
        payload.insert(
            "includeSubpackages".to_owned(),
            json!(args.include_subpackages.unwrap_or(true)),
        );

        if !payload.contains_key("packages") && !payload.contains_key("objects") {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "either packages or objects parameter is required".to_owned(),
            });
        }

        let response = self
            .engine
            .send_domain_request("git", "export", Value::Object(payload))
            .await?;
        serde_json::to_value(response).map_err(Into::into)
    }

    async fn update_source_with_lock(
        &self,
        source_uri: &str,
        source: &str,
        lock_handle: &str,
        transport: Option<&str>,
    ) -> Result<String, NeuroMcpError> {
        let mut query = vec![("lockHandle", lock_handle.to_owned())];
        if let Some(transport) = transport {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport.trim().to_owned()));
            }
        }
        let endpoint = build_path_with_query(source_uri, &query);
        self.engine
            .put_raw_text(
                endpoint.as_str(),
                Some(source),
                Some("text/plain"),
                Some("application/*"),
            )
            .await
            .map_err(Into::into)
    }

    async fn handle_get_class_info(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetClassInfoArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let class_name = args
            .class_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "class_name/className/name is required".to_owned(),
            })?;
        let class_name = class_name.trim().to_ascii_uppercase();

        let structure_endpoint = build_path_with_query(
            "/sap/bc/adt/cai/objectexplorer/objects",
            &[
                ("objectName", class_name.clone()),
                ("maxResults", "100".to_owned()),
            ],
        );
        let structure_raw = self
            .engine
            .get_raw_text(structure_endpoint.as_str(), Some("application/xml"))
            .await?;
        let source_uri = format!(
            "/sap/bc/adt/oo/classes/{}/source/main",
            encode_path_segment(class_name.as_str())
        );
        let source_raw = self
            .engine
            .get_raw_text(source_uri.as_str(), Some("text/plain"))
            .await
            .unwrap_or_default();

        let mut methods = BTreeSet::new();
        let mut attributes = BTreeSet::new();
        let mut interfaces = BTreeSet::new();
        let mut has_test_class = false;

        for reference in parse_xml_references(structure_raw.as_str()) {
            let typ = reference.object_type.to_ascii_uppercase();
            if typ.contains("METHOD") {
                methods.insert(reference.name);
            } else if typ.contains("ATTR") {
                attributes.insert(reference.name);
            } else if typ.contains("INTF") {
                interfaces.insert(reference.name);
            } else if typ.contains("TEST") {
                has_test_class = true;
            }
        }

        let source_upper = source_raw.to_ascii_uppercase();
        let is_abstract = source_upper.contains(" DEFINITION ABSTRACT");
        let is_final = source_upper.contains(" DEFINITION FINAL");
        let category = if is_abstract {
            "Abstract"
        } else if is_final {
            "Final"
        } else {
            "Regular"
        };

        Ok(json!({
            "name": class_name,
            "category": category,
            "isAbstract": is_abstract,
            "isFinal": is_final,
            "hasTestClass": has_test_class,
            "methods": methods.into_iter().collect::<Vec<_>>(),
            "attributes": attributes.into_iter().collect::<Vec<_>>(),
            "interfaces": interfaces.into_iter().collect::<Vec<_>>(),
            "raw": {
                "structure": structure_raw,
                "source": source_raw,
            }
        }))
    }

    async fn handle_trace_execution(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: TraceExecutionArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_uri = args
            .object_uri
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_uri/objectUri is required".to_owned(),
            })?;
        let max_depth = args.max_depth.unwrap_or(5).max(1);
        let static_graph = self
            .request_call_graph(object_uri.as_str(), "callees", max_depth, 1000)
            .await
            .ok();

        let mut executed_tests = Value::Null;
        if args.run_tests.unwrap_or(false) {
            let test_object_uri = args
                .test_object_uri
                .clone()
                .unwrap_or_else(|| object_uri.clone());
            executed_tests = self
                .handle_run_unit_tests(json!({ "objectUrl": test_object_uri }), "RunUnitTests")
                .await
                .unwrap_or_else(|error| json!({ "error": error.to_string() }));
        }

        let trace_user = args.trace_user.unwrap_or_default();
        let list_traces = self
            .handle_list_traces(
                json!({
                    "user": if trace_user.trim().is_empty() { Value::Null } else { json!(trace_user) },
                    "max_results": 5
                }),
                "ListTraces",
            )
            .await?;
        let traces_raw = list_traces
            .get("raw")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let latest_trace_id = extract_first_tag_value(traces_raw.as_str(), "id");

        let trace_analysis = if let Some(trace_id) = latest_trace_id.clone() {
            self.handle_get_trace(
                json!({ "trace_id": normalize_trace_id(trace_id.as_str()), "tool_type": "hitlist" }),
                "GetTrace",
            )
            .await
            .unwrap_or_else(|error| json!({ "error": error.to_string() }))
        } else {
            Value::Null
        };

        Ok(json!({
            "object_uri": object_uri,
            "max_depth": max_depth,
            "static_call_graph": static_graph,
            "list_traces": list_traces,
            "latest_trace_analysis": trace_analysis,
            "executed_tests": executed_tests
        }))
    }

    async fn handle_activate_package(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ActivatePackageArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let package_filter = args.package.unwrap_or_default().trim().to_ascii_uppercase();
        let max_objects = args.max_objects.unwrap_or(100).max(1) as usize;

        let raw = self
            .engine
            .get_raw_text(
                "/sap/bc/adt/activation/inactiveobjects",
                Some("application/vnd.sap.adt.inactivectsobjects.v1+xml, application/xml;q=0.8"),
            )
            .await?;

        let mut refs = parse_xml_references(raw.as_str())
            .into_iter()
            .filter(|reference| !reference.uri.is_empty())
            .collect::<Vec<_>>();
        if !package_filter.is_empty() {
            let expected = format!(
                "/sap/bc/adt/packages/{}",
                package_filter.to_ascii_lowercase()
            );
            refs.retain(|reference| {
                reference
                    .parent_uri
                    .as_deref()
                    .is_some_and(|parent| parent.eq_ignore_ascii_case(expected.as_str()))
            });
        }
        refs.sort_by_key(|reference| object_type_priority(reference.object_type.as_str()));
        refs.truncate(max_objects);

        let mut activated = Vec::new();
        let mut failed = Vec::new();
        for reference in refs {
            let body = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<adtcore:objectReferences xmlns:adtcore=\"http://www.sap.com/adt/core\">\n  <adtcore:objectReference adtcore:uri=\"{}\" adtcore:name=\"{}\"/>\n</adtcore:objectReferences>",
                escape_xml(reference.uri.as_str()),
                escape_xml(reference.name.as_str())
            );
            match self
                .engine
                .post_raw_text(
                    "/sap/bc/adt/activation?method=activate&preauditRequested=true",
                    Some(body.as_str()),
                    Some("application/xml"),
                    None,
                )
                .await
            {
                Ok(response_raw) => activated.push(json!({
                    "name": reference.name,
                    "type": reference.object_type,
                    "uri": reference.uri,
                    "raw": response_raw,
                })),
                Err(error) => failed.push(json!({
                    "name": reference.name,
                    "type": reference.object_type,
                    "uri": reference.uri,
                    "reason": error.to_string(),
                })),
            }
        }

        Ok(json!({
            "activated": activated,
            "failed": failed,
            "summary": format!(
                "Activated {} objects, {} failed",
                activated.len(),
                failed.len()
            )
        }))
    }

    async fn handle_list_dumps(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ListDumpsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let mut query = Vec::new();
        let mut filters = Vec::new();
        if let Some(user) = args.user.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("user eq '{}'", user.trim()));
        }
        if let Some(exception_type) = args.exception_type.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("exceptionType eq '{}'", exception_type.trim()));
        }
        if let Some(program) = args.program.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("program eq '{}'", program.trim()));
        }
        if let Some(package) = args.package.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("package eq '{}'", package.trim()));
        }
        if let Some(date_from) = args.date_from.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("datetime ge '{}000000'", date_from.trim()));
        }
        if let Some(date_to) = args.date_to.filter(|value| !value.trim().is_empty()) {
            filters.push(format!("datetime le '{}235959'", date_to.trim()));
        }
        if !filters.is_empty() {
            query.push(("$filter", filters.join(" and ")));
        }
        query.push(("$top", args.max_results.unwrap_or(100).max(1).to_string()));
        let endpoint = build_path_with_query("/sap/bc/adt/runtime/dumps", &query);
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/atom+xml;type=feed"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_dump(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetDumpArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let dump_id = args
            .dump_id
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "dump_id is required".to_owned(),
            })?;
        let normalized_id = normalize_dump_id(dump_id.as_str());
        let endpoint = format!(
            "/sap/bc/adt/runtime/dump/{}",
            encode_path_segment(normalized_id.as_str())
        );
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("text/html"))
            .await?;
        Ok(json!({
            "dump_id": normalized_id,
            "raw": raw
        }))
    }

    async fn handle_list_traces(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ListTracesArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let mut query = Vec::new();
        if let Some(user) = args.user.filter(|value| !value.trim().is_empty()) {
            query.push(("user", user.trim().to_owned()));
        }
        if let Some(process_type) = args.process_type.filter(|value| !value.trim().is_empty()) {
            query.push(("processType", process_type.trim().to_owned()));
        }
        if let Some(object_type) = args.object_type.filter(|value| !value.trim().is_empty()) {
            query.push(("objectType", object_type.trim().to_owned()));
        }
        query.push(("$top", args.max_results.unwrap_or(100).max(1).to_string()));
        let endpoint = build_path_with_query("/sap/bc/adt/runtime/traces/abaptraces", &query);
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/atom+xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_get_trace(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetTraceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let trace_id = args
            .trace_id
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "trace_id is required".to_owned(),
            })?;
        let tool_type = args.tool_type.unwrap_or_else(|| "hitlist".to_owned());
        let endpoint = format!(
            "/sap/bc/adt/runtime/traces/abaptraces/{}/{}",
            encode_path_segment(normalize_trace_id(trace_id.as_str()).as_str()),
            encode_path_segment(tool_type.as_str())
        );
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/xml"))
            .await?;
        Ok(json!({
            "trace_id": normalize_trace_id(trace_id.as_str()),
            "tool_type": tool_type,
            "raw": raw
        }))
    }

    async fn handle_get_sql_trace_state(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let raw = self
            .engine
            .get_raw_text("/sap/bc/adt/st05/trace/state", Some("application/xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_list_sql_traces(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ListSqlTracesArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let mut query = Vec::new();
        if let Some(user) = args.user.filter(|value| !value.trim().is_empty()) {
            query.push(("user", user.trim().to_owned()));
        }
        query.push(("$top", args.max_results.unwrap_or(100).max(1).to_string()));
        let endpoint = build_path_with_query("/sap/bc/adt/st05/trace/directory", &query);
        let raw = self
            .engine
            .get_raw_text(endpoint.as_str(), Some("application/atom+xml"))
            .await?;
        Ok(json!({ "raw": raw }))
    }

    async fn handle_run_report_async(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: RunReportArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
        let response = self
            .engine
            .send_domain_request("report", "runReport", Value::Object(payload))
            .await?;
        let payload = response.payload;
        let job_name = extract_json_string(&payload, &["jobname", "jobName", "job_name"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "runReport response does not contain job name".to_owned(),
            })?;
        let job_count = extract_json_string(&payload, &["jobcount", "jobCount", "job_count"])
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "runReport response does not contain job count".to_owned(),
            })?;
        let task_payload = json!({
            "job_name": job_name,
            "job_count": job_count,
            "report": extract_json_string(&payload, &["report"]).unwrap_or_default()
        });
        let task_id =
            base64::engine::general_purpose::STANDARD.encode(task_payload.to_string().as_bytes());
        Ok(json!({
            "task_id": task_id,
            "status": "started",
            "message": "Report execution started in background. Use GetAsyncResult to check status.",
            "job_name": task_payload["job_name"],
            "job_count": task_payload["job_count"]
        }))
    }

    async fn handle_get_async_result(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GetAsyncResultArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let task_id = args
            .task_id
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "task_id is required".to_owned(),
            })?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(task_id.as_bytes())
            .map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("invalid task_id: {error}"),
            })?;
        let decoded_json: Value = serde_json::from_slice(decoded.as_slice()).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("task_id payload is invalid JSON: {error}"),
            }
        })?;
        let job_name = extract_json_string(&decoded_json, &["job_name"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "task_id missing job_name".to_owned(),
            }
        })?;
        let job_count = extract_json_string(&decoded_json, &["job_count"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "task_id missing job_count".to_owned(),
            }
        })?;
        let status_response = self
            .engine
            .send_domain_request(
                "report",
                "getJobStatus",
                json!({
                    "jobname": job_name,
                    "jobcount": job_count
                }),
            )
            .await?;

        let mut spool_outputs = Vec::new();
        let spool_ids =
            extract_json_string_array(&status_response.payload, &["spool_ids", "spoolIds"]);
        for spool_id in spool_ids {
            let spool_response = self
                .engine
                .send_domain_request("report", "getSpoolOutput", json!({ "spool_id": spool_id }))
                .await?;
            spool_outputs.push(serde_json::to_value(spool_response)?);
        }

        Ok(json!({
            "task_id": task_id,
            "wait": args.wait.unwrap_or(false),
            "status": status_response,
            "spool_outputs": spool_outputs
        }))
    }

    async fn handle_create_and_activate_program(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateAndActivateProgramArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let program_name = args
            .program_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "program_name is required".to_owned(),
            })?;
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package_name = args
            .package_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package_name is required".to_owned(),
            })?;
        let source = args.source.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "source is required".to_owned(),
        })?;
        let transport = args.transport.unwrap_or_default();
        let program_name = program_name.trim().to_ascii_uppercase();
        let package_name = package_name.trim().to_ascii_uppercase();
        let create_args = CreateObjectArgs {
            object_type: Some("PROG/P".to_owned()),
            name: Some(program_name.clone()),
            description: Some(description),
            package_name: Some(package_name),
            transport: if transport.trim().is_empty() {
                None
            } else {
                Some(transport.clone())
            },
            parent_name: None,
            responsible: None,
            software_component: None,
            service_definition: None,
            binding_type: None,
            binding_version: None,
            binding_category: None,
        };
        self.create_object_request(&create_args, tool_name).await?;

        let object_url = build_object_url("PROG/P", program_name.as_str(), None);
        let lock = self
            .handle_lock_object(
                json!({
                    "objectUrl": object_url,
                    "accessMode": "MODIFY"
                }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;
        let source_url = format!("{object_url}/source/main");
        self.update_source_with_lock(
            source_url.as_str(),
            source.as_str(),
            lock_handle.as_str(),
            Some(transport.as_str()),
        )
        .await?;
        self.handle_unlock_object(
            json!({ "objectUrl": object_url, "lockHandle": lock_handle }),
            "UnlockObject",
        )
        .await?;
        let activation = self
            .handle_activate(
                json!({
                    "objectUrl": object_url,
                    "objectName": program_name
                }),
                "Activate",
            )
            .await?;
        Ok(json!({
            "success": true,
            "programName": program_name,
            "objectUrl": object_url,
            "activation": activation
        }))
    }

    async fn handle_create_class_with_tests(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateClassWithTestsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let class_name = args
            .class_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "class_name is required".to_owned(),
            })?;
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package_name = args
            .package_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package_name is required".to_owned(),
            })?;
        let class_source = args
            .class_source
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "class_source is required".to_owned(),
            })?;
        let test_source = args
            .test_source
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "test_source is required".to_owned(),
            })?;
        let transport = args.transport.unwrap_or_default();
        let class_name = class_name.trim().to_ascii_uppercase();
        let create_args = CreateObjectArgs {
            object_type: Some("CLAS/OC".to_owned()),
            name: Some(class_name.clone()),
            description: Some(description),
            package_name: Some(package_name.trim().to_ascii_uppercase()),
            transport: if transport.trim().is_empty() {
                None
            } else {
                Some(transport.clone())
            },
            parent_name: None,
            responsible: None,
            software_component: None,
            service_definition: None,
            binding_type: None,
            binding_version: None,
            binding_category: None,
        };
        self.create_object_request(&create_args, tool_name).await?;

        let object_url = build_object_url("CLAS/OC", class_name.as_str(), None);
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": object_url, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;

        self.update_source_with_lock(
            format!("{object_url}/source/main").as_str(),
            class_source.as_str(),
            lock_handle.as_str(),
            Some(transport.as_str()),
        )
        .await?;
        self.handle_create_test_include(
            json!({
                "className": class_name,
                "lockHandle": lock_handle,
                "transport": transport
            }),
            "CreateTestInclude",
        )
        .await?;
        self.handle_update_class_include(
            json!({
                "className": class_name,
                "includeType": "testclasses",
                "source": test_source,
                "lockHandle": lock_handle,
                "transport": transport
            }),
            "UpdateClassInclude",
        )
        .await?;
        self.handle_unlock_object(
            json!({ "objectUrl": object_url, "lockHandle": lock_handle }),
            "UnlockObject",
        )
        .await?;
        let activation = self
            .handle_activate(
                json!({ "objectUrl": object_url, "objectName": class_name }),
                "Activate",
            )
            .await?;
        let unit_tests = self
            .handle_run_unit_tests(json!({ "objectUrl": object_url }), "RunUnitTests")
            .await
            .unwrap_or_else(|error| json!({ "error": error.to_string() }));
        Ok(json!({
            "success": true,
            "className": class_name,
            "objectUrl": object_url,
            "activation": activation,
            "unitTests": unit_tests
        }))
    }

    async fn handle_create_table(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CreateTableArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let table_name = args.name.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name is required".to_owned(),
        })?;
        let description = args
            .description
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "description is required".to_owned(),
            })?;
        let package_name = args.package.unwrap_or_else(|| "$TMP".to_owned());
        let delivery_class = args.delivery_class.unwrap_or_else(|| "A".to_owned());
        let table_name = table_name.trim().to_ascii_uppercase();
        let fields = parse_table_fields(args.fields, tool_name)?;
        if fields.is_empty() {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "at least one field is required".to_owned(),
            });
        }
        let mut query = Vec::new();
        if let Some(transport) = args.transport.clone() {
            if !transport.trim().is_empty() {
                query.push(("corrNr", transport));
            }
        }
        let create_endpoint = build_path_with_query("/sap/bc/adt/ddic/tables", &query);
        let create_body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<blue:blueSource xmlns:blue=\"http://www.sap.com/wbobj/blue\"\n                 xmlns:adtcore=\"http://www.sap.com/adt/core\"\n                 adtcore:name=\"{}\"\n                 adtcore:type=\"TABL/DT\"\n                 adtcore:description=\"{}\">\n  <adtcore:packageRef adtcore:name=\"{}\"/>\n</blue:blueSource>",
            escape_xml(table_name.as_str()),
            escape_xml(description.as_str()),
            escape_xml(package_name.trim().to_ascii_uppercase().as_str())
        );
        self.engine
            .post_raw_text(
                create_endpoint.as_str(),
                Some(create_body.as_str()),
                Some("application/vnd.sap.adt.tables.v2+xml"),
                Some("application/vnd.sap.adt.tables.v2+xml"),
            )
            .await?;

        let table_url = format!(
            "/sap/bc/adt/ddic/tables/{}",
            encode_path_segment(table_name.to_ascii_lowercase().as_str())
        );
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": table_url, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;
        let ddl = build_table_ddl(
            table_name.as_str(),
            description.as_str(),
            delivery_class.as_str(),
            fields.as_slice(),
        );
        self.update_source_with_lock(
            format!("{table_url}/source/main").as_str(),
            ddl.as_str(),
            lock_handle.as_str(),
            args.transport.as_deref(),
        )
        .await?;
        self.handle_unlock_object(
            json!({ "objectUrl": table_url, "lockHandle": lock_handle }),
            "UnlockObject",
        )
        .await?;
        let activation = self
            .handle_activate(
                json!({ "objectUrl": table_url, "objectName": table_name }),
                "Activate",
            )
            .await?;
        Ok(json!({
            "status": "created",
            "table": table_name,
            "package": package_name,
            "field_count": fields.len(),
            "activation": activation
        }))
    }

    async fn handle_compare_source(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CompareSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let type1 = args.type1.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "type1 is required".to_owned(),
        })?;
        let name1 = args.name1.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name1 is required".to_owned(),
        })?;
        let type2 = args.type2.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "type2 is required".to_owned(),
        })?;
        let name2 = args.name2.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "name2 is required".to_owned(),
        })?;

        let source_uri_1 = resolve_source_uri(
            type1.as_str(),
            name1.as_str(),
            args.parent1.as_deref(),
            args.include1.as_deref(),
        )
        .ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: format!("unsupported type1 `{}`", type1),
        })?;
        let source_uri_2 = resolve_source_uri(
            type2.as_str(),
            name2.as_str(),
            args.parent2.as_deref(),
            args.include2.as_deref(),
        )
        .ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: format!("unsupported type2 `{}`", type2),
        })?;
        let source_1 = self
            .engine
            .get_raw_text(source_uri_1.as_str(), Some("text/plain"))
            .await?;
        let source_2 = self
            .engine
            .get_raw_text(source_uri_2.as_str(), Some("text/plain"))
            .await?;
        let diff = generate_line_diff(
            format!("{}:{}", type1, name1).as_str(),
            format!("{}:{}", type2, name2).as_str(),
            source_1.as_str(),
            source_2.as_str(),
        );
        Ok(json!(diff))
    }

    async fn handle_clone_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: CloneObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_type = args
            .object_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_type is required".to_owned(),
            })?;
        let source_name = args
            .source_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "source_name is required".to_owned(),
            })?;
        let target_name = args
            .target_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "target_name is required".to_owned(),
            })?;
        let package_name = args
            .package
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package is required".to_owned(),
            })?;

        let source_uri = resolve_source_uri(object_type.as_str(), source_name.as_str(), None, None)
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("unsupported object_type `{}`", object_type),
            })?;
        let source = self
            .engine
            .get_raw_text(source_uri.as_str(), Some("text/plain"))
            .await?;
        let target_upper = target_name.trim().to_ascii_uppercase();
        let source_upper = source_name.trim().to_ascii_uppercase();
        let replacement_pattern = match object_type.trim().to_ascii_uppercase().as_str() {
            "PROG" | "PROG/P" => {
                format!(r"(?i)(REPORT\s+){}", regex::escape(source_upper.as_str()))
            }
            "CLAS" | "CLAS/OC" => {
                format!(r"(?i)(CLASS\s+){}", regex::escape(source_upper.as_str()))
            }
            "INTF" | "INTF/OI" => {
                format!(
                    r"(?i)(INTERFACE\s+){}",
                    regex::escape(source_upper.as_str())
                )
            }
            _ => {
                return Err(NeuroMcpError::InvalidArguments {
                    tool: tool_name.to_owned(),
                    message: "CloneObject currently supports PROG, CLAS, INTF".to_owned(),
                });
            }
        };
        let re = Regex::new(replacement_pattern.as_str()).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("failed to build clone regex: {error}"),
            }
        })?;
        let cloned_source = re
            .replace_all(source.as_str(), format!("${{1}}{target_upper}"))
            .to_string();

        let create_type = normalize_creatable_type(object_type.as_str()).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("unsupported object_type `{}`", object_type),
            }
        })?;
        let create_args = CreateObjectArgs {
            object_type: Some(create_type.to_owned()),
            name: Some(target_upper.clone()),
            description: Some(format!("Copy of {}", source_upper)),
            package_name: Some(package_name.trim().to_ascii_uppercase()),
            transport: None,
            parent_name: None,
            responsible: None,
            software_component: None,
            service_definition: None,
            binding_type: None,
            binding_version: None,
            binding_category: None,
        };
        self.create_object_request(&create_args, tool_name).await?;

        let object_url = build_object_url(create_type, target_upper.as_str(), None);
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": object_url, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;
        self.update_source_with_lock(
            format!("{object_url}/source/main").as_str(),
            cloned_source.as_str(),
            lock_handle.as_str(),
            None,
        )
        .await?;
        self.handle_unlock_object(
            json!({ "objectUrl": object_url, "lockHandle": lock_handle }),
            "UnlockObject",
        )
        .await?;
        self.handle_activate(
            json!({ "objectUrl": object_url, "objectName": target_upper }),
            "Activate",
        )
        .await?;
        Ok(json!({
            "success": true,
            "sourceName": source_upper,
            "targetName": target_upper,
            "objectType": object_type,
            "package": package_name
        }))
    }

    async fn handle_rename_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: RenameObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_type = args
            .obj_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "objType is required".to_owned(),
            })?;
        let old_name = args
            .old_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "oldName is required".to_owned(),
            })?;
        let new_name = args
            .new_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "newName is required".to_owned(),
            })?;
        let package_name = args
            .package_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "packageName is required".to_owned(),
            })?;
        let transport = args.transport.unwrap_or_default();

        let clone = self
            .handle_clone_object(
                json!({
                    "object_type": object_type,
                    "source_name": old_name,
                    "target_name": new_name,
                    "package": package_name
                }),
                "CloneObject",
            )
            .await?;
        let create_type = normalize_creatable_type(object_type.as_str()).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("unsupported objType `{}`", object_type),
            }
        })?;
        let old_object_url =
            build_object_url(create_type, old_name.to_ascii_uppercase().as_str(), None);
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": old_object_url, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle for old object".to_owned(),
            }
        })?;
        let delete = self
            .handle_delete_object(
                json!({
                    "objectUrl": old_object_url,
                    "lockHandle": lock_handle,
                    "transport": transport
                }),
                "DeleteObject",
            )
            .await?;
        Ok(json!({
            "success": true,
            "clone": clone,
            "delete_old": delete
        }))
    }

    async fn handle_edit_source(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: EditSourceArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_url is required".to_owned(),
            })?;
        let old_string = args
            .old_string
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "old_string is required".to_owned(),
            })?;
        let new_string = args
            .new_string
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "new_string is required".to_owned(),
            })?;
        let replace_all = args.replace_all.unwrap_or(false);
        let case_insensitive = args.case_insensitive.unwrap_or(false);
        let syntax_check = args.syntax_check.unwrap_or(true);
        let source_url =
            if object_url.contains("/includes/") || object_url.ends_with("/source/main") {
                object_url.clone()
            } else {
                format!("{object_url}/source/main")
            };
        let source = self
            .engine
            .get_raw_text(source_url.as_str(), Some("text/plain"))
            .await?;
        let old_pattern = regex::escape(old_string.as_str());
        let regex = RegexBuilder::new(old_pattern.as_str())
            .case_insensitive(case_insensitive)
            .build()
            .map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("invalid old_string pattern: {error}"),
            })?;
        let match_count = regex.find_iter(source.as_str()).count();
        if match_count == 0 {
            return Ok(json!({
                "success": false,
                "matchCount": 0,
                "message": "old_string not found in source"
            }));
        }
        if !replace_all && match_count > 1 {
            return Ok(json!({
                "success": false,
                "matchCount": match_count,
                "message": "old_string matches multiple locations; set replace_all=true or provide more context"
            }));
        }
        let new_source = if replace_all {
            regex
                .replace_all(source.as_str(), new_string.as_str())
                .to_string()
        } else {
            regex
                .replace(source.as_str(), new_string.as_str())
                .to_string()
        };
        if syntax_check {
            let syntax = self
                .handle_syntax_check(
                    json!({
                        "objectUrl": object_url,
                        "content": new_source
                    }),
                    "SyntaxCheck",
                )
                .await?;
            let raw = syntax
                .get("raw")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if raw.contains("severity=\"E\"")
                || raw.contains("severity=\"A\"")
                || raw.contains("severity=\"X\"")
            {
                return Ok(json!({
                    "success": false,
                    "matchCount": match_count,
                    "message": "Edit would introduce syntax errors. Changes NOT saved.",
                    "syntax": syntax
                }));
            }
        }
        let lock_target = if object_url.contains("/includes/") {
            parent_object_uri_from_include(object_url.as_str())
                .unwrap_or_else(|| object_url.clone())
        } else {
            object_url.clone()
        };
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": lock_target, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;
        self.update_source_with_lock(
            source_url.as_str(),
            new_source.as_str(),
            lock_handle.as_str(),
            args.transport.as_deref(),
        )
        .await?;
        self.handle_unlock_object(
            json!({
                "objectUrl": lock_target,
                "lockHandle": lock_handle
            }),
            "UnlockObject",
        )
        .await?;
        let activate_target = if object_url.contains("/includes/") {
            parent_object_uri_from_include(object_url.as_str())
                .unwrap_or_else(|| object_url.clone())
        } else {
            object_url.clone()
        };
        let activate_name = activate_target
            .split('/')
            .next_back()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let activation = self
            .handle_activate(
                json!({
                    "objectUrl": activate_target,
                    "objectName": activate_name
                }),
                "Activate",
            )
            .await?;
        Ok(json!({
            "success": true,
            "matchCount": match_count,
            "activation": activation,
            "message": if replace_all {
                format!("Successfully replaced {} occurrences", match_count)
            } else {
                "Successfully edited source".to_owned()
            }
        }))
    }

    async fn handle_save_to_file(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: SaveToFileArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_type = args
            .object_type
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_type/objType is required".to_owned(),
            })?;
        let object_name = args
            .object_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_name/objectName is required".to_owned(),
            })?;
        let include = args.include.unwrap_or_else(|| "main".to_owned());
        let source_uri = resolve_source_uri(
            object_type.as_str(),
            object_name.as_str(),
            args.parent.as_deref(),
            Some(include.as_str()),
        )
        .ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: format!("unsupported object_type `{}`", object_type),
        })?;
        let source = self
            .engine
            .get_raw_text(source_uri.as_str(), Some("text/plain"))
            .await?;

        let output_path = resolve_output_path(
            args.output,
            object_type.as_str(),
            object_name.as_str(),
            include.as_str(),
        );
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("failed to create output directory: {error}"),
            })?;
        }
        fs::write(output_path.as_path(), source.as_bytes()).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("failed to write file: {error}"),
            }
        })?;
        Ok(json!({
            "objectType": object_type,
            "objectName": object_name,
            "filePath": output_path,
            "lineCount": source.lines().count(),
            "success": true
        }))
    }

    async fn handle_deploy_from_file(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: DeployFromFileArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let file_path = args
            .file_path
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "file_path is required".to_owned(),
            })?;
        let parsed = parse_abap_file_info(file_path.as_str(), tool_name)?;
        let source = fs::read_to_string(file_path.as_str()).map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("failed to read file `{}`: {error}", file_path),
            }
        })?;
        let transport = args.transport.unwrap_or_default();

        if let Some(include_type) = parsed.include_type.clone() {
            let parent_url = build_object_url("CLAS/OC", parsed.object_name.as_str(), None);
            let lock = self
                .handle_lock_object(
                    json!({ "objectUrl": parent_url, "accessMode": "MODIFY" }),
                    "LockObject",
                )
                .await?;
            let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
                NeuroMcpError::InvalidArguments {
                    tool: tool_name.to_owned(),
                    message: "LockObject did not return lockHandle".to_owned(),
                }
            })?;
            if include_type.eq_ignore_ascii_case("testclasses") {
                let _ = self
                    .handle_create_test_include(
                        json!({
                            "className": parsed.object_name,
                            "lockHandle": lock_handle,
                            "transport": transport
                        }),
                        "CreateTestInclude",
                    )
                    .await;
            }
            self.handle_update_class_include(
                json!({
                    "className": parsed.object_name,
                    "includeType": include_type,
                    "source": source,
                    "lockHandle": lock_handle,
                    "transport": transport
                }),
                "UpdateClassInclude",
            )
            .await?;
            self.handle_unlock_object(
                json!({ "objectUrl": parent_url, "lockHandle": lock_handle }),
                "UnlockObject",
            )
            .await?;
            self.handle_activate(
                json!({ "objectUrl": parent_url, "objectName": parsed.object_name }),
                "Activate",
            )
            .await?;
            return Ok(json!({
                "filePath": file_path,
                "objectName": parsed.object_name,
                "objectType": parsed.object_type,
                "includeType": include_type,
                "success": true,
                "created": false
            }));
        }

        let object_url = build_object_url(
            parsed.object_type.as_str(),
            parsed.object_name.as_str(),
            parsed.parent_name.as_deref(),
        );
        let mut created = false;
        let exists = self
            .engine
            .get_raw_text(object_url.as_str(), Some("application/xml"))
            .await
            .is_ok();
        if !exists {
            let package_name = args.package_name.unwrap_or_else(|| "$TMP".to_owned());
            let create_args = CreateObjectArgs {
                object_type: Some(parsed.object_type.clone()),
                name: Some(parsed.object_name.clone()),
                description: Some(format!("Created from file {}", file_path)),
                package_name: Some(package_name.to_ascii_uppercase()),
                transport: if transport.trim().is_empty() {
                    None
                } else {
                    Some(transport.clone())
                },
                parent_name: parsed.parent_name.clone(),
                responsible: None,
                software_component: None,
                service_definition: None,
                binding_type: None,
                binding_version: None,
                binding_category: None,
            };
            self.create_object_request(&create_args, tool_name).await?;
            created = true;
        }
        let lock = self
            .handle_lock_object(
                json!({ "objectUrl": object_url, "accessMode": "MODIFY" }),
                "LockObject",
            )
            .await?;
        let lock_handle = extract_json_string(&lock, &["lockHandle"]).ok_or_else(|| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "LockObject did not return lockHandle".to_owned(),
            }
        })?;
        self.update_source_with_lock(
            format!("{object_url}/source/main").as_str(),
            source.as_str(),
            lock_handle.as_str(),
            Some(transport.as_str()),
        )
        .await?;
        self.handle_unlock_object(
            json!({ "objectUrl": object_url, "lockHandle": lock_handle }),
            "UnlockObject",
        )
        .await?;
        self.handle_activate(
            json!({ "objectUrl": object_url, "objectName": parsed.object_name }),
            "Activate",
        )
        .await?;
        Ok(json!({
            "filePath": file_path,
            "objectUrl": object_url,
            "objectName": parsed.object_name,
            "objectType": parsed.object_type,
            "success": true,
            "created": created
        }))
    }

    async fn handle_grep_object(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GrepObjectArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_url = args
            .object_url
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_url is required".to_owned(),
            })?;
        let pattern = args
            .pattern
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "pattern is required".to_owned(),
            })?;
        self.grep_single_object(
            object_url.as_str(),
            pattern.as_str(),
            args.case_insensitive.unwrap_or(false),
            args.context_lines.unwrap_or(0),
        )
        .await
    }

    async fn handle_grep_objects(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GrepObjectsArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let object_urls = args
            .object_urls
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "object_urls is required".to_owned(),
            })?;
        let pattern = args
            .pattern
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "pattern is required".to_owned(),
            })?;
        let mut objects = Vec::new();
        let mut total_matches = 0usize;
        for object_url in object_urls {
            let result = self
                .grep_single_object(
                    object_url.as_str(),
                    pattern.as_str(),
                    args.case_insensitive.unwrap_or(false),
                    args.context_lines.unwrap_or(0),
                )
                .await?;
            let match_count = result
                .get("matchCount")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            if match_count > 0 {
                total_matches += match_count;
                objects.push(result);
            }
        }
        Ok(json!({
            "success": true,
            "objects": objects,
            "totalMatches": total_matches,
            "message": if total_matches == 0 {
                "No matches found".to_owned()
            } else {
                format!("Found {} match(es) across {} object(s)", total_matches, objects.len())
            }
        }))
    }

    async fn handle_grep_package(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GrepPackageArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let package_name = args
            .package_name
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package_name is required".to_owned(),
            })?;
        let pattern = args
            .pattern
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "pattern is required".to_owned(),
            })?;
        let package_raw = self
            .handle_get_package(json!({ "packageName": package_name }), "GetPackage")
            .await?
            .get("raw")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let type_filters = args
            .object_types
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let max_results = args.max_results.unwrap_or(100).max(1) as usize;
        let mut objects = Vec::new();
        let mut total_matches = 0usize;
        for reference in parse_xml_references(package_raw.as_str()) {
            if !is_source_object_type(reference.object_type.as_str()) {
                continue;
            }
            if !type_filters.is_empty()
                && !type_filters
                    .iter()
                    .any(|filter| filter.eq_ignore_ascii_case(reference.object_type.as_str()))
            {
                continue;
            }
            let grep_result = self
                .grep_single_object(
                    reference.uri.as_str(),
                    pattern.as_str(),
                    args.case_insensitive.unwrap_or(false),
                    0,
                )
                .await?;
            let match_count = grep_result
                .get("matchCount")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            if match_count > 0 {
                total_matches += match_count;
                objects.push(grep_result);
            }
            if objects.len() >= max_results {
                break;
            }
        }
        Ok(json!({
            "success": true,
            "packageName": package_name,
            "objects": objects,
            "totalMatches": total_matches,
            "message": if total_matches == 0 {
                "No matches found in package".to_owned()
            } else {
                format!(
                    "Found {} match(es) across {} object(s)",
                    total_matches,
                    objects.len()
                )
            }
        }))
    }

    async fn handle_grep_packages(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: GrepPackagesArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let packages = args
            .packages
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "packages is required".to_owned(),
            })?;
        let pattern = args
            .pattern
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "pattern is required".to_owned(),
            })?;
        let include_subpackages = args.include_subpackages.unwrap_or(false);
        let max_results = args.max_results.unwrap_or(0).max(0) as usize;
        let mut package_queue = packages.clone();
        if include_subpackages {
            let mut seen = BTreeSet::new();
            let mut expanded = Vec::new();
            for package in packages {
                collect_subpackages(self, package.as_str(), &mut seen, &mut expanded).await?;
            }
            package_queue = expanded;
        }

        let mut objects = Vec::new();
        let mut total_matches = 0usize;
        for package in package_queue.clone() {
            let pkg_result = self
                .handle_grep_package(
                    json!({
                        "package_name": package,
                        "pattern": pattern,
                        "case_insensitive": args.case_insensitive.unwrap_or(false),
                        "object_types": args.object_types.clone().unwrap_or_default().join(","),
                        "max_results": if max_results == 0 { 1000 } else { max_results },
                    }),
                    "GrepPackage",
                )
                .await?;
            if let Some(pkg_objects) = pkg_result.get("objects").and_then(Value::as_array) {
                for object in pkg_objects {
                    objects.push(object.clone());
                    total_matches += object
                        .get("matchCount")
                        .and_then(Value::as_u64)
                        .unwrap_or_default() as usize;
                    if max_results > 0 && objects.len() >= max_results {
                        break;
                    }
                }
            }
            if max_results > 0 && objects.len() >= max_results {
                break;
            }
        }
        Ok(json!({
            "success": true,
            "packages": package_queue,
            "objects": objects,
            "totalMatches": total_matches
        }))
    }

    async fn grep_single_object(
        &self,
        object_url: &str,
        pattern: &str,
        case_insensitive: bool,
        context_lines: u32,
    ) -> Result<Value, NeuroMcpError> {
        let source_url =
            if object_url.contains("/includes/") || object_url.ends_with("/source/main") {
                object_url.to_owned()
            } else {
                format!("{object_url}/source/main")
            };
        let source = self
            .engine
            .get_raw_text(source_url.as_str(), Some("text/plain"))
            .await?;
        let regex = RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .build()
            .map_err(|error| NeuroMcpError::InvalidArguments {
                tool: "GrepObject".to_owned(),
                message: format!("invalid regex pattern: {error}"),
            })?;
        let lines = source.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let mut matches = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if regex.is_match(line.as_str()) {
                let start = idx.saturating_sub(context_lines as usize);
                let end = usize::min(lines.len(), idx + context_lines as usize + 1);
                matches.push(json!({
                    "lineNumber": idx + 1,
                    "matchedLine": line,
                    "contextBefore": lines[start..idx].to_vec(),
                    "contextAfter": lines[idx + 1..end].to_vec()
                }));
            }
        }
        Ok(json!({
            "success": true,
            "objectUrl": object_url,
            "objectName": object_url.split('/').next_back().unwrap_or_default(),
            "matches": matches,
            "matchCount": matches.len(),
            "message": if matches.is_empty() { "No matches found" } else { "Matches found" }
        }))
    }

    async fn handle_execute_abap(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: ExecuteAbapArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let code = args.code.ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "code is required".to_owned(),
        })?;
        let prefix = args
            .program_prefix
            .unwrap_or_else(|| "ZTEMP_EXEC_".to_owned())
            .to_ascii_uppercase();
        let return_variable = args
            .return_variable
            .unwrap_or_else(|| "lv_result".to_owned());
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let suffix = format!("{:08}", millis % 100_000_000);
        let program_name = format!("{prefix}{suffix}");
        let risk_level = args.risk_level.unwrap_or_else(|| "harmless".to_owned());
        let risk_level_abap = match risk_level.to_ascii_lowercase().as_str() {
            "dangerous" => "RISK LEVEL DANGEROUS",
            "critical" => "RISK LEVEL CRITICAL",
            _ => "RISK LEVEL HARMLESS",
        };
        let source = format!(
            "REPORT {program_name}.\n\nCLASS ltc_executor DEFINITION FOR TESTING {risk_level_abap} DURATION SHORT.\n  PUBLIC SECTION.\n    METHODS execute_payload FOR TESTING.\nENDCLASS.\n\nCLASS ltc_executor IMPLEMENTATION.\n  METHOD execute_payload.\n    DATA {return_variable} TYPE string.\n    {code}\n    cl_abap_unit_assert=>fail( msg = |EXEC_RESULT:{{ {return_variable} }}| ).\n  ENDMETHOD.\nENDCLASS.\n"
        );
        let create_result = self
            .handle_create_and_activate_program(
                json!({
                    "program_name": program_name,
                    "description": "Temp program for ExecuteABAP",
                    "package_name": "$TMP",
                    "source": source
                }),
                "CreateAndActivateProgram",
            )
            .await?;
        let object_url = format!(
            "/sap/bc/adt/programs/programs/{}",
            encode_path_segment(program_name.to_ascii_uppercase().as_str())
        );
        let tests = self
            .handle_run_unit_tests(json!({ "objectUrl": object_url }), "RunUnitTests")
            .await?;
        let tests_raw = tests.get("raw").and_then(Value::as_str).unwrap_or_default();
        let mut output = Vec::new();
        let output_regex = Regex::new(r"EXEC_RESULT:([^<\r\n]+)").map_err(|error| {
            NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("failed to compile execution output regex: {error}"),
            }
        })?;
        for capture in output_regex.captures_iter(tests_raw) {
            if let Some(value) = capture.get(1) {
                output.push(value.as_str().trim().to_owned());
            }
        }
        let keep_program = args.keep_program.unwrap_or(false);
        let cleanup = if keep_program {
            json!({"skipped": true})
        } else {
            match self
                .handle_lock_object(
                    json!({ "objectUrl": object_url, "accessMode": "MODIFY" }),
                    "LockObject",
                )
                .await
            {
                Ok(lock) => {
                    if let Some(lock_handle) = extract_json_string(&lock, &["lockHandle"]) {
                        self.handle_delete_object(
                            json!({
                                "objectUrl": object_url,
                                "lockHandle": lock_handle
                            }),
                            "DeleteObject",
                        )
                        .await
                        .unwrap_or_else(|error| json!({ "error": error.to_string() }))
                    } else {
                        json!({ "error": "missing lockHandle" })
                    }
                }
                Err(error) => json!({ "error": error.to_string() }),
            }
        };
        Ok(json!({
            "success": true,
            "programName": program_name,
            "output": output,
            "tests": tests,
            "create": create_result,
            "cleanup": cleanup,
            "message": if output.is_empty() {
                "Executed successfully (no EXEC_RESULT output captured)"
            } else {
                "Executed successfully"
            }
        }))
    }

    async fn handle_list_dependencies(
        &self,
        _arguments: Value,
        _tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        Ok(json!({
            "dependencies": [
                {
                    "name": "abapgit-standalone",
                    "description": "Single program ZABAPGIT",
                    "package": "$ABAPGIT",
                    "available": false
                },
                {
                    "name": "abapgit-dev",
                    "description": "Developer package structure",
                    "package": "$ZGIT_DEV",
                    "available": false
                }
            ],
            "usage": [
                "InstallAbapGit --edition standalone",
                "InstallAbapGit --edition dev"
            ]
        }))
    }

    async fn handle_install_abap_git(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: InstallAbapGitArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let edition = args
            .edition
            .unwrap_or_else(|| "standalone".to_owned())
            .to_ascii_lowercase();
        if edition != "standalone" && edition != "dev" {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "edition must be 'standalone' or 'dev'".to_owned(),
            });
        }
        let package = args.package.unwrap_or_else(|| {
            if edition == "standalone" {
                "$ABAPGIT".to_owned()
            } else {
                "$ZGIT_DEV".to_owned()
            }
        });
        Ok(json!({
            "status": "not_embedded",
            "edition": edition,
            "package": package.to_ascii_uppercase(),
            "check_only": args.check_only.unwrap_or(false),
            "message": "Dependency ZIP embedding is required before deployment. Use ListDependencies for guidance."
        }))
    }

    async fn handle_install_dummy_test(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: InstallDummyTestArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        if args.check_only.unwrap_or(false) {
            return Ok(json!({
                "status": "check_only",
                "package": "$ZADT_INSTALL_TEST",
                "interface": "ZIF_DUMMY_TEST",
                "class": "ZCL_DUMMY_TEST",
                "message": "Would create/update package, interface and class to verify install workflow"
            }));
        }
        let package = self
            .handle_create_package(
                json!({
                    "name": "$ZADT_INSTALL_TEST",
                    "description": "Install Tools Test Package",
                    "parent": "$TMP"
                }),
                "CreatePackage",
            )
            .await
            .unwrap_or_else(|error| json!({ "warning": error.to_string() }));
        let interface_source = "INTERFACE zif_dummy_test\n  PUBLIC.\n\n  METHODS get_value\n    RETURNING VALUE(rv_value) TYPE string.\n\nENDINTERFACE.";
        let class_source = "CLASS zcl_dummy_test DEFINITION\n  PUBLIC\n  FINAL\n  CREATE PUBLIC.\n\n  PUBLIC SECTION.\n    INTERFACES zif_dummy_test.\nENDCLASS.\n\nCLASS zcl_dummy_test IMPLEMENTATION.\n  METHOD zif_dummy_test~get_value.\n    rv_value = 'Dummy Test Passed'.\n  ENDMETHOD.\nENDCLASS.";
        let write_interface = self
            .handle_update_source_by_pattern(
                json!({
                    "name": "ZIF_DUMMY_TEST",
                    "source": interface_source
                }),
                "WriteSource",
                "/sap/bc/adt/oo/interfaces/{name}/source/main",
                &["interfaceName", "interface_name", "name"],
            )
            .await
            .unwrap_or_else(|error| json!({ "warning": error.to_string() }));
        let write_class = self
            .handle_update_source_by_pattern(
                json!({
                    "name": "ZCL_DUMMY_TEST",
                    "source": class_source
                }),
                "WriteSource",
                "/sap/bc/adt/oo/classes/{name}/source/main",
                &["className", "class_name", "name"],
            )
            .await
            .unwrap_or_else(|error| json!({ "warning": error.to_string() }));
        Ok(json!({
            "status": "executed",
            "package": package,
            "interface": write_interface,
            "class": write_class,
            "cleanup_requested": args.cleanup.unwrap_or(false)
        }))
    }

    async fn handle_install_zadtvsp(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: InstallZadtVspArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
            })?;
        let package = args
            .package
            .unwrap_or_else(|| "$ZADT_VSP".to_owned())
            .to_ascii_uppercase();
        if !package.starts_with('$') {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "package must start with $ for local installation".to_owned(),
            });
        }
        if args.check_only.unwrap_or(false) {
            return Ok(json!({
                "status": "check_only",
                "package": package,
                "skip_git_service": args.skip_git_service.unwrap_or(false),
                "message": "Would deploy ZADT_VSP embedded objects (interface + service classes)"
            }));
        }
        Ok(json!({
            "status": "not_embedded",
            "package": package,
            "skip_git_service": args.skip_git_service.unwrap_or(false),
            "message": "Embedded ABAP payloads are required for InstallZADTVSP deployment."
        }))
    }

    async fn handle_ws_request(
        &self,
        arguments: Value,
        tool_name: &str,
    ) -> Result<Value, NeuroMcpError> {
        let args: WsRequestArgs =
            serde_json::from_value(arguments).map_err(|error| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: error.to_string(),
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
    #[serde(
        default,
        alias = "functionName",
        alias = "function_name",
        alias = "name"
    )]
    function_name: Option<String>,
    #[serde(
        default,
        alias = "groupName",
        alias = "group_name",
        alias = "functionGroup"
    )]
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
struct Ui5ListAppsArgs {
    #[serde(default)]
    query: Option<String>,
    #[serde(default, alias = "maxResults", alias = "max_results")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Ui5AppArgs {
    #[serde(default, alias = "appName", alias = "app_name")]
    app_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Ui5FileArgs {
    #[serde(default, alias = "appName", alias = "app_name")]
    app_name: Option<String>,
    #[serde(default, alias = "filePath", alias = "file_path")]
    file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Ui5UploadFileArgs {
    #[serde(default, alias = "appName", alias = "app_name")]
    app_name: Option<String>,
    #[serde(default, alias = "filePath", alias = "file_path")]
    file_path: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default, alias = "contentType", alias = "content_type")]
    content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Ui5CreateAppArgs {
    #[serde(default, alias = "appName", alias = "app_name")]
    app_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "packageName", alias = "package_name")]
    package_name: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Ui5DeleteAppArgs {
    #[serde(default, alias = "appName", alias = "app_name")]
    app_name: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallRfcArgs {
    #[serde(default)]
    function: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct MoveObjectArgs {
    #[serde(default, alias = "objectType", alias = "object_type")]
    object_type: Option<String>,
    #[serde(default, alias = "objectName", alias = "object_name")]
    object_name: Option<String>,
    #[serde(default, alias = "newPackage", alias = "new_package")]
    new_package: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetTypeHierarchyArgs {
    #[serde(default, alias = "sourceUrl", alias = "source_url")]
    source_url: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
    #[serde(default, alias = "superTypes", alias = "super_types")]
    super_types: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GitExportArgs {
    #[serde(default)]
    packages: Option<String>,
    #[serde(default)]
    objects: Option<String>,
    #[serde(default, alias = "includeSubpackages", alias = "include_subpackages")]
    include_subpackages: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GetClassInfoArgs {
    #[serde(default, alias = "class_name", alias = "className", alias = "name")]
    class_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TraceExecutionArgs {
    #[serde(default, alias = "object_uri", alias = "objectUri")]
    object_uri: Option<String>,
    #[serde(default, alias = "max_depth", alias = "maxDepth")]
    max_depth: Option<u32>,
    #[serde(default, alias = "run_tests", alias = "runTests")]
    run_tests: Option<bool>,
    #[serde(default, alias = "test_object_uri", alias = "testObjectUri")]
    test_object_uri: Option<String>,
    #[serde(default, alias = "trace_user", alias = "traceUser")]
    trace_user: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActivatePackageArgs {
    #[serde(default)]
    package: Option<String>,
    #[serde(default, alias = "max_objects", alias = "maxObjects")]
    max_objects: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListDumpsArgs {
    #[serde(default)]
    user: Option<String>,
    #[serde(default, alias = "exception_type", alias = "exceptionType")]
    exception_type: Option<String>,
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default, alias = "date_from", alias = "dateFrom")]
    date_from: Option<String>,
    #[serde(default, alias = "date_to", alias = "dateTo")]
    date_to: Option<String>,
    #[serde(default, alias = "max_results", alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetDumpArgs {
    #[serde(default, alias = "dump_id", alias = "dumpId")]
    dump_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListTracesArgs {
    #[serde(default)]
    user: Option<String>,
    #[serde(default, alias = "process_type", alias = "processType")]
    process_type: Option<String>,
    #[serde(default, alias = "object_type", alias = "objectType")]
    object_type: Option<String>,
    #[serde(default, alias = "max_results", alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetTraceArgs {
    #[serde(default, alias = "trace_id", alias = "traceId")]
    trace_id: Option<String>,
    #[serde(default, alias = "tool_type", alias = "toolType")]
    tool_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListSqlTracesArgs {
    #[serde(default)]
    user: Option<String>,
    #[serde(default, alias = "max_results", alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetAsyncResultArgs {
    #[serde(default, alias = "task_id", alias = "taskId")]
    task_id: Option<String>,
    #[serde(default)]
    wait: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreateAndActivateProgramArgs {
    #[serde(default, alias = "program_name", alias = "programName")]
    program_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "package_name", alias = "packageName")]
    package_name: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateClassWithTestsArgs {
    #[serde(default, alias = "class_name", alias = "className")]
    class_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "package_name", alias = "packageName")]
    package_name: Option<String>,
    #[serde(default, alias = "class_source", alias = "classSource")]
    class_source: Option<String>,
    #[serde(default, alias = "test_source", alias = "testSource")]
    test_source: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTableArgs {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    fields: Option<Value>,
    #[serde(default)]
    transport: Option<String>,
    #[serde(default, alias = "delivery_class", alias = "deliveryClass")]
    delivery_class: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompareSourceArgs {
    #[serde(default)]
    type1: Option<String>,
    #[serde(default)]
    name1: Option<String>,
    #[serde(default)]
    type2: Option<String>,
    #[serde(default)]
    name2: Option<String>,
    #[serde(default)]
    include1: Option<String>,
    #[serde(default)]
    include2: Option<String>,
    #[serde(default)]
    parent1: Option<String>,
    #[serde(default)]
    parent2: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloneObjectArgs {
    #[serde(default, alias = "object_type", alias = "objectType")]
    object_type: Option<String>,
    #[serde(default, alias = "source_name", alias = "sourceName")]
    source_name: Option<String>,
    #[serde(default, alias = "target_name", alias = "targetName")]
    target_name: Option<String>,
    #[serde(
        default,
        alias = "package",
        alias = "package_name",
        alias = "packageName"
    )]
    package: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenameObjectArgs {
    #[serde(
        default,
        alias = "objType",
        alias = "object_type",
        alias = "objectType"
    )]
    obj_type: Option<String>,
    #[serde(default, alias = "oldName", alias = "old_name")]
    old_name: Option<String>,
    #[serde(default, alias = "newName", alias = "new_name")]
    new_name: Option<String>,
    #[serde(default, alias = "packageName", alias = "package_name")]
    package_name: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EditSourceArgs {
    #[serde(default, alias = "object_url", alias = "objectUrl")]
    object_url: Option<String>,
    #[serde(default, alias = "old_string", alias = "oldString")]
    old_string: Option<String>,
    #[serde(default, alias = "new_string", alias = "newString")]
    new_string: Option<String>,
    #[serde(default, alias = "replace_all", alias = "replaceAll")]
    replace_all: Option<bool>,
    #[serde(default, alias = "syntax_check", alias = "syntaxCheck")]
    syntax_check: Option<bool>,
    #[serde(default, alias = "case_insensitive", alias = "caseInsensitive")]
    case_insensitive: Option<bool>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveToFileArgs {
    #[serde(default, alias = "object_type", alias = "objType")]
    object_type: Option<String>,
    #[serde(default, alias = "object_name", alias = "objectName")]
    object_name: Option<String>,
    #[serde(default)]
    include: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default, alias = "outputPath", alias = "output_dir", alias = "output")]
    output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeployFromFileArgs {
    #[serde(default, alias = "file_path", alias = "filePath")]
    file_path: Option<String>,
    #[serde(default, alias = "package_name", alias = "packageName")]
    package_name: Option<String>,
    #[serde(default)]
    transport: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GrepObjectArgs {
    #[serde(default, alias = "object_url", alias = "objectUrl")]
    object_url: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default, alias = "case_insensitive", alias = "caseInsensitive")]
    case_insensitive: Option<bool>,
    #[serde(default, alias = "context_lines", alias = "contextLines")]
    context_lines: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GrepObjectsArgs {
    #[serde(default, alias = "object_urls", alias = "objectUrls")]
    object_urls: Option<Vec<String>>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default, alias = "case_insensitive", alias = "caseInsensitive")]
    case_insensitive: Option<bool>,
    #[serde(default, alias = "context_lines", alias = "contextLines")]
    context_lines: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GrepPackageArgs {
    #[serde(default, alias = "package_name", alias = "packageName")]
    package_name: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default, alias = "case_insensitive", alias = "caseInsensitive")]
    case_insensitive: Option<bool>,
    #[serde(default, alias = "object_types", alias = "objectTypes")]
    object_types: Option<String>,
    #[serde(default, alias = "max_results", alias = "maxResults")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GrepPackagesArgs {
    #[serde(default)]
    packages: Option<Vec<String>>,
    #[serde(default, alias = "include_subpackages", alias = "includeSubpackages")]
    include_subpackages: Option<bool>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default, alias = "case_insensitive", alias = "caseInsensitive")]
    case_insensitive: Option<bool>,
    #[serde(default, alias = "object_types", alias = "objectTypes")]
    object_types: Option<Vec<String>>,
    #[serde(default, alias = "max_results", alias = "maxResults")]
    max_results: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ExecuteAbapArgs {
    #[serde(default)]
    code: Option<String>,
    #[serde(default, alias = "risk_level", alias = "riskLevel")]
    risk_level: Option<String>,
    #[serde(default, alias = "return_variable", alias = "returnVariable")]
    return_variable: Option<String>,
    #[serde(default, alias = "keep_program", alias = "keepProgram")]
    keep_program: Option<bool>,
    #[serde(default, alias = "program_prefix", alias = "programPrefix")]
    program_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstallAbapGitArgs {
    #[serde(default)]
    edition: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default, alias = "check_only", alias = "checkOnly")]
    check_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InstallDummyTestArgs {
    #[serde(default, alias = "check_only", alias = "checkOnly")]
    check_only: Option<bool>,
    #[serde(default)]
    cleanup: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InstallZadtVspArgs {
    #[serde(default)]
    package: Option<String>,
    #[serde(default, alias = "skip_git_service", alias = "skipGitService")]
    skip_git_service: Option<bool>,
    #[serde(default, alias = "check_only", alias = "checkOnly")]
    check_only: Option<bool>,
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

fn build_ui5_file_content_path(app_name: &str, file_path: &str) -> String {
    let app_upper = app_name.trim().to_ascii_uppercase();
    let file_rel = file_path.trim().trim_start_matches('/');
    let full_path = format!("{app_upper}/{file_rel}");
    format!(
        "/sap/bc/adt/filestore/ui5-bsp/objects/{}/content",
        encode_path_segment(full_path.as_str())
    )
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

#[derive(Debug, Clone)]
struct XmlReference {
    name: String,
    object_type: String,
    uri: String,
    parent_uri: Option<String>,
}

#[derive(Debug, Clone)]
struct TableFieldDefinition {
    name: String,
    data_type: String,
    is_key: bool,
    not_null: bool,
}

#[derive(Debug, Clone)]
struct ParsedAbapFileInfo {
    object_type: String,
    object_name: String,
    parent_name: Option<String>,
    include_type: Option<String>,
}

fn parse_xml_references(xml: &str) -> Vec<XmlReference> {
    let tag_regex = match Regex::new(r#"<[A-Za-z0-9:_-]+\s+([^>]+)/?>"#) {
        Ok(regex) => regex,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for captures in tag_regex.captures_iter(xml) {
        let attributes = captures
            .get(1)
            .map(|value| value.as_str())
            .unwrap_or_default();
        let uri = extract_first_attr(
            attributes,
            &["adtcore:uri", "uri", "href", "xlink:href", "link"],
        )
        .unwrap_or_default();
        let name = extract_first_attr(attributes, &["adtcore:name", "name", "obj_name"])
            .unwrap_or_default();
        let object_type = extract_first_attr(attributes, &["adtcore:type", "type", "obj_type"])
            .unwrap_or_default();
        let parent_uri = extract_first_attr(
            attributes,
            &["adtcore:parentUri", "parentUri", "parent_uri", "parent-uri"],
        );
        if uri.is_empty() && name.is_empty() && object_type.is_empty() {
            continue;
        }
        out.push(XmlReference {
            name,
            object_type,
            uri,
            parent_uri,
        });
    }
    out
}

fn extract_first_attr(attributes: &str, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| extract_xml_attr_value(attributes, name))
}

fn object_type_priority(object_type: &str) -> u32 {
    match object_type.trim().to_ascii_uppercase().as_str() {
        "DOMA/DT" => 10,
        "DTEL/DE" => 20,
        "TTYP/TT" => 30,
        "STRU/DS" => 40,
        "TABL/DT" => 50,
        "VIEW/V" => 60,
        "INTF/OI" => 70,
        "CLAS/OC" => 80,
        "PROG/P" => 90,
        "PROG/I" => 100,
        _ => 500,
    }
}

fn extract_first_tag_value(xml: &str, tag_name: &str) -> Option<String> {
    let escaped = regex::escape(tag_name);
    let pattern = format!(
        r"(?is)<(?:[A-Za-z0-9_]+:)?{escaped}\b[^>]*>(?P<value>.*?)</(?:[A-Za-z0-9_]+:)?{escaped}>"
    );
    let regex = Regex::new(pattern.as_str()).ok()?;
    regex
        .captures(xml)
        .and_then(|captures| captures.name("value"))
        .map(|value| value.as_str().trim().to_owned())
}

fn normalize_trace_id(trace_id: &str) -> String {
    trace_id
        .trim()
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn normalize_dump_id(dump_id: &str) -> String {
    dump_id
        .trim()
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn extract_json_string(payload: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = find_json_key(payload, key) {
            if let Some(text) = value.as_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_owned());
                }
            } else if value.is_number() || value.is_boolean() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn extract_json_string_array(payload: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(value) = find_json_key(payload, key) {
            if let Some(items) = value.as_array() {
                return items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::trim))
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
            }
            if let Some(text) = value.as_str() {
                let trimmed = text.trim();
                if trimmed.starts_with('[')
                    && let Ok(decoded) = serde_json::from_str::<Value>(trimmed)
                    && let Some(items) = decoded.as_array()
                {
                    return items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::trim))
                        .filter(|item| !item.is_empty())
                        .map(ToOwned::to_owned)
                        .collect();
                }
                if !trimmed.is_empty() {
                    return trimmed
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(ToOwned::to_owned)
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

fn find_json_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for (name, entry) in map {
                if name.eq_ignore_ascii_case(key) {
                    return Some(entry);
                }
                if let Some(found) = find_json_key(entry, key) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| find_json_key(item, key)),
        _ => None,
    }
}

fn resolve_source_uri(
    object_type: &str,
    object_name: &str,
    parent_name: Option<&str>,
    include: Option<&str>,
) -> Option<String> {
    let object_type = object_type.trim().to_ascii_uppercase();
    let object_name = object_name.trim().to_ascii_uppercase();
    match object_type.as_str() {
        "PROG" | "PROG/P" => Some(format!(
            "/sap/bc/adt/programs/programs/{}/source/main",
            encode_path_segment(object_name.as_str())
        )),
        "PROG/I" | "INCL" | "INCLUDE" => Some(format!(
            "/sap/bc/adt/programs/includes/{}/source/main",
            encode_path_segment(object_name.as_str())
        )),
        "CLAS" | "CLAS/OC" => {
            let include_type = include.unwrap_or("main").trim();
            if include_type.is_empty() || include_type.eq_ignore_ascii_case("main") {
                Some(format!(
                    "/sap/bc/adt/oo/classes/{}/source/main",
                    encode_path_segment(object_name.as_str())
                ))
            } else {
                Some(format!(
                    "/sap/bc/adt/oo/classes/{}/includes/{}",
                    encode_path_segment(object_name.as_str()),
                    include_type.to_ascii_lowercase()
                ))
            }
        }
        "INTF" | "INTF/OI" => Some(format!(
            "/sap/bc/adt/oo/interfaces/{}/source/main",
            encode_path_segment(object_name.as_str())
        )),
        "FUGR/FF" => {
            let parent_name = parent_name?.trim().to_ascii_uppercase();
            Some(format!(
                "/sap/bc/adt/functions/groups/{}/fmodules/{}/source/main",
                encode_path_segment(parent_name.as_str()),
                encode_path_segment(object_name.as_str())
            ))
        }
        "TABL" | "TABL/DT" => Some(format!(
            "/sap/bc/adt/ddic/tables/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        "STRU" | "STRU/DS" => Some(format!(
            "/sap/bc/adt/ddic/structures/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        "DDLS" | "DDLS/DF" => Some(format!(
            "/sap/bc/adt/ddic/ddl/sources/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        "BDEF" | "BDEF/BDO" => Some(format!(
            "/sap/bc/adt/bo/behaviordefinitions/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        "SRVD" | "SRVD/SRV" => Some(format!(
            "/sap/bc/adt/ddic/srvd/sources/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        "SRVB" | "SRVB/SVB" => Some(format!(
            "/sap/bc/adt/businessservices/bindings/{}/source/main",
            encode_path_segment(object_name.to_ascii_lowercase().as_str())
        )),
        _ => None,
    }
}

fn generate_line_diff(old_label: &str, new_label: &str, left: &str, right: &str) -> Value {
    let left_lines = left.lines().collect::<Vec<_>>();
    let right_lines = right.lines().collect::<Vec<_>>();
    let max_len = left_lines.len().max(right_lines.len());
    let mut changes = Vec::new();
    let mut added = 0usize;
    let mut removed = 0usize;
    let mut changed = 0usize;

    for index in 0..max_len {
        let left_line = left_lines.get(index).copied();
        let right_line = right_lines.get(index).copied();
        match (left_line, right_line) {
            (Some(old_line), Some(new_line)) if old_line == new_line => {}
            (Some(old_line), Some(new_line)) => {
                changed += 1;
                changes.push(json!({
                    "line": index + 1,
                    "kind": "changed",
                    "before": old_line,
                    "after": new_line
                }));
            }
            (Some(old_line), None) => {
                removed += 1;
                changes.push(json!({
                    "line": index + 1,
                    "kind": "removed",
                    "before": old_line,
                    "after": Value::Null
                }));
            }
            (None, Some(new_line)) => {
                added += 1;
                changes.push(json!({
                    "line": index + 1,
                    "kind": "added",
                    "before": Value::Null,
                    "after": new_line
                }));
            }
            (None, None) => {}
        }
    }

    json!({
        "left": old_label,
        "right": new_label,
        "equal": changes.is_empty(),
        "stats": {
            "added": added,
            "removed": removed,
            "changed": changed
        },
        "changes": changes
    })
}

fn normalize_creatable_type(object_type: &str) -> Option<&'static str> {
    match object_type.trim().to_ascii_uppercase().as_str() {
        "PROG" | "PROG/P" | "REPORT" => Some("PROG/P"),
        "PROG/I" | "INCL" | "INCLUDE" => Some("PROG/I"),
        "CLAS" | "CLAS/OC" | "CLASS" => Some("CLAS/OC"),
        "INTF" | "INTF/OI" | "INTERFACE" => Some("INTF/OI"),
        "FUGR" | "FUGR/F" => Some("FUGR/F"),
        "FUGR/FF" | "FMOD" | "FUNCTION" => Some("FUGR/FF"),
        "DEVC" | "DEVC/K" | "PACKAGE" => Some("DEVC/K"),
        "DDLS" | "DDLS/DF" => Some("DDLS/DF"),
        "BDEF" | "BDEF/BDO" => Some("BDEF/BDO"),
        "SRVD" | "SRVD/SRV" => Some("SRVD/SRV"),
        "SRVB" | "SRVB/SVB" => Some("SRVB/SVB"),
        _ => None,
    }
}

fn parent_object_uri_from_include(object_url: &str) -> Option<String> {
    let marker = "/sap/bc/adt/oo/classes/";
    let start = object_url.find(marker)?;
    let tail = &object_url[start + marker.len()..];
    let class_name = tail.split('/').next().unwrap_or_default().trim();
    if class_name.is_empty() {
        None
    } else {
        Some(format!("{marker}{class_name}"))
    }
}

fn resolve_output_path(
    output: Option<String>,
    object_type: &str,
    object_name: &str,
    include: &str,
) -> PathBuf {
    let default_name = default_export_filename(object_type, object_name, include);
    if let Some(target) = output {
        let trimmed = target.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            let is_dir_hint = trimmed.ends_with('/') || trimmed.ends_with('\\');
            if is_dir_hint || (path.exists() && path.is_dir()) || path.extension().is_none() {
                return path.join(default_name);
            }
            return path;
        }
    }
    PathBuf::from(default_name)
}

fn default_export_filename(object_type: &str, object_name: &str, include: &str) -> String {
    let name = object_name.trim().to_ascii_lowercase();
    let include = include.trim().to_ascii_lowercase();
    match object_type.trim().to_ascii_uppercase().as_str() {
        "CLAS" | "CLAS/OC" => {
            if include.is_empty() || include == "main" {
                format!("{name}.clas.abap")
            } else {
                format!("{name}.{include}.abap")
            }
        }
        "INTF" | "INTF/OI" => format!("{name}.intf.abap"),
        "PROG" | "PROG/P" => format!("{name}.prog.abap"),
        "PROG/I" | "INCL" | "INCLUDE" => format!("{name}.incl.abap"),
        "TABL" | "TABL/DT" => format!("{name}.tabl.abap"),
        "DDLS" | "DDLS/DF" => format!("{name}.ddls.abap"),
        "BDEF" | "BDEF/BDO" => format!("{name}.bdef.abap"),
        "SRVD" | "SRVD/SRV" => format!("{name}.srvd.abap"),
        "SRVB" | "SRVB/SVB" => format!("{name}.srvb.abap"),
        _ => {
            let typ = object_type
                .trim()
                .to_ascii_lowercase()
                .replace('/', "_")
                .replace(' ', "_");
            format!("{name}.{typ}.abap")
        }
    }
}

fn parse_abap_file_info(
    file_path: &str,
    tool_name: &str,
) -> Result<ParsedAbapFileInfo, NeuroMcpError> {
    let path = Path::new(file_path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: format!("invalid file path `{file_path}`"),
        })?;
    let without_suffix =
        file_name
            .strip_suffix(".abap")
            .ok_or_else(|| NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: "file extension must be .abap".to_owned(),
            })?;
    let mut parts = without_suffix
        .split('.')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "file name must follow <name>.<type>.abap or <class>.<include>.abap"
                .to_owned(),
        });
    }
    let suffix = parts.pop().unwrap_or_default().to_ascii_lowercase();
    let object_name = parts
        .last()
        .map(|value| value.to_ascii_uppercase())
        .unwrap_or_default();
    let prefix_joined = parts.join(".").to_ascii_uppercase();

    let info = match suffix.as_str() {
        "prog" | "program" => ParsedAbapFileInfo {
            object_type: "PROG/P".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "incl" | "include" => ParsedAbapFileInfo {
            object_type: "PROG/I".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "clas" | "class" => ParsedAbapFileInfo {
            object_type: "CLAS/OC".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "intf" | "interface" => ParsedAbapFileInfo {
            object_type: "INTF/OI".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "tabl" | "table" => ParsedAbapFileInfo {
            object_type: "TABL/DT".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "ddls" | "ddl" => ParsedAbapFileInfo {
            object_type: "DDLS/DF".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "srvd" => ParsedAbapFileInfo {
            object_type: "SRVD/SRV".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "srvb" => ParsedAbapFileInfo {
            object_type: "SRVB/SVB".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "fugr" => ParsedAbapFileInfo {
            object_type: "FUGR/F".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: None,
        },
        "fmod" | "func" => {
            if parts.len() < 2 {
                return Err(NeuroMcpError::InvalidArguments {
                    tool: tool_name.to_owned(),
                    message: "function module file should be <group>.<module>.fmod.abap".to_owned(),
                });
            }
            ParsedAbapFileInfo {
                object_type: "FUGR/FF".to_owned(),
                object_name,
                parent_name: Some(parts[parts.len() - 2].to_ascii_uppercase()),
                include_type: None,
            }
        }
        "locals_def" | "locals_imp" | "macros" | "testclasses" => ParsedAbapFileInfo {
            object_type: "CLAS/OC".to_owned(),
            object_name: prefix_joined,
            parent_name: None,
            include_type: Some(suffix),
        },
        _ => {
            return Err(NeuroMcpError::InvalidArguments {
                tool: tool_name.to_owned(),
                message: format!("unsupported ABAP file suffix `{suffix}`"),
            });
        }
    };
    Ok(info)
}

fn parse_table_fields(
    fields: Option<Value>,
    tool_name: &str,
) -> Result<Vec<TableFieldDefinition>, NeuroMcpError> {
    let Some(fields_value) = fields else {
        return Ok(Vec::new());
    };
    match fields_value {
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                match item {
                    Value::Object(map) => {
                        let field_map = map.into_iter().collect::<BTreeMap<String, Value>>();
                        let name = extract_non_empty_string(
                            &field_map,
                            &["name", "fieldName", "field_name", "field"],
                        )
                        .ok_or_else(|| {
                            NeuroMcpError::InvalidArguments {
                                tool: tool_name.to_owned(),
                                message: "table field is missing name".to_owned(),
                            }
                        })?;
                        let data_type = extract_non_empty_string(
                            &field_map,
                            &["type", "dataType", "data_type", "abapType"],
                        )
                        .ok_or_else(|| {
                            NeuroMcpError::InvalidArguments {
                                tool: tool_name.to_owned(),
                                message: format!("field `{name}` is missing type"),
                            }
                        })?;
                        let is_key = field_map
                            .get("key")
                            .or_else(|| field_map.get("isKey"))
                            .and_then(parse_bool_value)
                            .unwrap_or(false);
                        let not_null = field_map
                            .get("not_null")
                            .or_else(|| field_map.get("notNull"))
                            .or_else(|| field_map.get("required"))
                            .and_then(parse_bool_value)
                            .unwrap_or(false);
                        out.push(TableFieldDefinition {
                            name: name.to_ascii_uppercase(),
                            data_type,
                            is_key,
                            not_null,
                        });
                    }
                    Value::String(spec) => {
                        if let Some(parsed) = parse_table_field_spec(spec.as_str()) {
                            out.push(parsed);
                        }
                    }
                    _ => {}
                }
            }
            Ok(out)
        }
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.starts_with('[') {
                let decoded = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                    NeuroMcpError::InvalidArguments {
                        tool: tool_name.to_owned(),
                        message: format!("invalid fields JSON string: {error}"),
                    }
                })?;
                return parse_table_fields(Some(decoded), tool_name);
            }
            Ok(trimmed
                .split(';')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .filter_map(parse_table_field_spec)
                .collect())
        }
        _ => Err(NeuroMcpError::InvalidArguments {
            tool: tool_name.to_owned(),
            message: "fields must be array or string".to_owned(),
        }),
    }
}

fn parse_table_field_spec(spec: &str) -> Option<TableFieldDefinition> {
    let parts = spec
        .split(':')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let mut is_key = false;
    let mut not_null = false;
    for part in parts.iter().skip(2) {
        if part.eq_ignore_ascii_case("key") || part.eq_ignore_ascii_case("pk") {
            is_key = true;
        }
        if part.eq_ignore_ascii_case("notnull") || part.eq_ignore_ascii_case("not_null") {
            not_null = true;
        }
    }
    Some(TableFieldDefinition {
        name: parts[0].to_ascii_uppercase(),
        data_type: parts[1].to_owned(),
        is_key,
        not_null,
    })
}

fn parse_bool_value(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "x" | "1" | "yes" => Some(true),
            "false" | "0" | "no" | "" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn build_table_ddl(
    table_name: &str,
    description: &str,
    delivery_class: &str,
    fields: &[TableFieldDefinition],
) -> String {
    let mut field_lines = Vec::new();
    for field in fields {
        let key = if field.is_key { "key " } else { "" };
        let not_null = if field.not_null || field.is_key {
            " not null"
        } else {
            ""
        };
        field_lines.push(format!(
            "  {key}{} : {}{};",
            field.name.to_ascii_lowercase(),
            field.data_type,
            not_null
        ));
    }
    format!(
        "@EndUserText.label : '{}'\n@AbapCatalog.tableCategory : #TRANSPARENT\n@AbapCatalog.deliveryClass : #{}\n@AbapCatalog.dataMaintenance : #ALLOWED\ndefine table {} {{\n{}\n}}\n",
        description.replace('\'', "''"),
        delivery_class.trim().to_ascii_uppercase(),
        table_name.trim().to_ascii_uppercase(),
        field_lines.join("\n")
    )
}

fn is_source_object_type(object_type: &str) -> bool {
    matches!(
        object_type.trim().to_ascii_uppercase().as_str(),
        "PROG/P"
            | "PROG/I"
            | "CLAS/OC"
            | "INTF/OI"
            | "FUGR/FF"
            | "DDLS/DF"
            | "TABL/DT"
            | "STRU/DS"
            | "BDEF/BDO"
            | "SRVD/SRV"
            | "SRVB/SVB"
    )
}

async fn collect_subpackages(
    facade: &NeuroMcpFacade,
    package: &str,
    seen: &mut BTreeSet<String>,
    expanded: &mut Vec<String>,
) -> Result<(), NeuroMcpError> {
    let root = package.trim().to_ascii_uppercase();
    if root.is_empty() {
        return Ok(());
    }
    let mut queue = Vec::new();
    if seen.insert(root.clone()) {
        expanded.push(root.clone());
        queue.push(root);
    }
    while let Some(current) = queue.pop() {
        let package_data = facade
            .handle_get_package(json!({ "packageName": current }), "GetPackage")
            .await?;
        let raw = package_data
            .get("raw")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        for reference in parse_xml_references(raw.as_str()) {
            if !reference.object_type.eq_ignore_ascii_case("DEVC/K") {
                continue;
            }
            let child = reference.name.trim().to_ascii_uppercase();
            if child.is_empty() {
                continue;
            }
            if seen.insert(child.clone()) {
                expanded.push(child.clone());
                queue.push(child);
            }
        }
    }
    Ok(())
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
    async fn invoke_implemented_tool_reaches_engine() {
        let facade = build_facade().await;
        let error = facade
            .invoke("ActivatePackage", json!({}))
            .await
            .expect_err("implemented tool should attempt runtime call in tests");
        assert!(matches!(error, NeuroMcpError::Engine(_)));
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
