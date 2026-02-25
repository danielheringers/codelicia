use std::collections::BTreeMap;

use clap::{Parser, Subcommand};
use neuro_engine::NeuroEngine;
use neuro_types::{
    AdtAuth, AdtHttpConfig, AdtHttpEndpoints, NeuroEngineConfig, SafetyPolicy, WsClientConfig,
};

#[derive(Debug, Parser)]
#[command(name = "neuro-cli")]
#[command(about = "Neuro orchestration CLI for ADT/WS runtime operations")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:8000")]
    adt_base_url: String,
    #[arg(long, default_value_t = 30)]
    adt_timeout_secs: u64,
    #[arg(long, default_value = "/sap/bc/adt")]
    adt_csrf_fetch_path: String,
    #[arg(long, default_value = "/sap/bc/adt/discovery/search")]
    adt_search_path: String,

    #[arg(long, conflicts_with = "adt_cookie", requires = "adt_password")]
    adt_user: Option<String>,
    #[arg(long, conflicts_with = "adt_cookie", requires = "adt_user")]
    adt_password: Option<String>,
    #[arg(long, conflicts_with_all = ["adt_user", "adt_password"])]
    adt_cookie: Option<String>,

    #[arg(long)]
    ws_url: Option<String>,
    #[arg(long, default_value_t = 15)]
    ws_timeout_secs: u64,

    #[arg(long, default_value_t = false)]
    read_only: bool,
    #[arg(long = "blocked-pattern")]
    blocked_patterns: Vec<String>,
    #[arg(long = "allowed-ws-domain")]
    allowed_ws_domains: Vec<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Diagnose,
    Search {
        query: String,
        #[arg(long)]
        max_results: Option<u32>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = build_engine_config(&cli);
    let engine = NeuroEngine::new(config).await?;

    match cli.command {
        Command::Diagnose => {
            let report = engine.diagnose().await;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Search { query, max_results } => {
            let objects = engine.search(query.as_str(), max_results).await?;
            println!("{}", serde_json::to_string_pretty(&objects)?);
        }
    }

    Ok(())
}

fn build_engine_config(cli: &Cli) -> NeuroEngineConfig {
    let adt_auth = match (&cli.adt_user, &cli.adt_password, &cli.adt_cookie) {
        (_, _, Some(cookie)) => AdtAuth::Cookie {
            cookie: cookie.clone(),
        },
        (Some(username), Some(password), None) => AdtAuth::Basic {
            username: username.clone(),
            password: password.clone(),
        },
        _ => AdtAuth::Anonymous,
    };

    let ws_config = cli.ws_url.as_ref().map(|url| WsClientConfig {
        url: url.clone(),
        request_timeout_secs: cli.ws_timeout_secs,
        connect_headers: BTreeMap::new(),
    });

    NeuroEngineConfig {
        adt: AdtHttpConfig {
            base_url: cli.adt_base_url.clone(),
            auth: adt_auth,
            timeout_secs: cli.adt_timeout_secs,
            csrf_fetch_path: cli.adt_csrf_fetch_path.clone(),
            endpoints: AdtHttpEndpoints {
                search_objects_path: cli.adt_search_path.clone(),
            },
            insecure_tls: false,
            sap_client: None,
            sap_language: None,
        },
        ws: ws_config,
        safety: SafetyPolicy {
            read_only: cli.read_only,
            blocked_source_patterns: cli.blocked_patterns.clone(),
            allowed_ws_domains: cli.allowed_ws_domains.clone(),
            require_etag_for_updates: false,
        },
    }
}
