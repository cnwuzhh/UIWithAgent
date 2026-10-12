use agentos_agent_ledger::SqliteAgentLedger;
use agentos_generated_app_runtime::{GeneratedAppManifest, build_package};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: build <workspace> <revision> <ledger-path>");
        std::process::exit(2);
    }
    let workspace = PathBuf::from(&args[0]);
    let revision = match args[1].parse::<u32>() {
        Ok(revision) if revision > 0 => revision,
        _ => {
            eprintln!("revision must be a positive integer");
            std::process::exit(2);
        }
    };
    let manifest_bytes = match std::fs::read(workspace.join("MANIFEST.json")) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read materialized manifest: {error}");
            std::process::exit(1);
        }
    };
    let manifest = match serde_json::from_slice::<GeneratedAppManifest>(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("cannot parse materialized manifest: {error}");
            std::process::exit(1);
        }
    };
    let ledger = match SqliteAgentLedger::open(&args[2]) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("cannot open Agent Ledger: {error}");
            std::process::exit(1);
        }
    };
    match build_package(&ledger, &workspace, revision, &manifest, &BTreeMap::new()).await {
        Ok(execution) => println!("{}", serde_json::to_string_pretty(&execution).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
