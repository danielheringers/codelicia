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
    "GetStructure",
    "GetTransaction",
    "GetTypeInfo",
    "GetCDSDependencies",
    "GetClassInclude",
    "UpdateClassInclude",
    "GetInstalledComponents",
    "GetConnectionInfo",
    "GetFeatures",
    "PrettyPrint",
    "GetPrettyPrinterSettings",
    "SetPrettyPrinterSettings",
    "FindDefinition",
    "FindReferences",
    "CodeCompletion",
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
            "GetInstalledComponents" => {
                self.handle_get_installed_components(arguments, tool_name).await
            }
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
struct PrettyPrintArgs {
    source: String,
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
            .invoke("CreateObject", json!({}))
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
