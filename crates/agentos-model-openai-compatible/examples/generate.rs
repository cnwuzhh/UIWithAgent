use agentos_agent_ledger::SqliteAgentLedger;
use agentos_application::{PlanningAgent, Runtime};
use agentos_generated_app_runtime::materialize_package;
use agentos_model_openai_compatible::OpenAiCompatiblePlanningAgent;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let request = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if request.trim().is_empty() {
        eprintln!(
            "usage: cargo run -p agentos-model-openai-compatible --example generate -- <request>"
        );
        std::process::exit(2);
    }

    let workspace_root = std::env::var_os("AGENTOS_GENERATED_APPS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("agentos-generated-apps"));
    if let Err(error) = std::fs::create_dir_all(&workspace_root) {
        eprintln!("cannot create generated apps directory: {error}");
        std::process::exit(1);
    }
    let ledger_path = workspace_root.join("agent-ledger.sqlite3");
    let ledger = match SqliteAgentLedger::open(&ledger_path) {
        Ok(ledger) => Arc::new(ledger),
        Err(error) => {
            eprintln!("cannot open Agent Ledger: {error}");
            std::process::exit(1);
        }
    };
    let agent = OpenAiCompatiblePlanningAgent::from_env(ledger.clone());
    let result = async {
        let planning_request = Runtime::demo().planning_request("surface-desktop", &request)?;
        let plan = agent.plan(planning_request).await?;
        let package = agent.generate_package(&request, &plan).await?;
        let materialized = materialize_package(&workspace_root, 1, &package)?;
        Ok::<_, String>((plan, package, materialized))
    }
    .await;

    match result {
        Ok((plan, package, materialized)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "reviewReady",
                    "staticReview": "passed",
                    "request": request,
                    "plan": plan,
                    "manifest": package.manifest,
                    "materialized": materialized,
                    "ledgerPath": ledger_path,
                    "execution": "notStarted",
                    "surfaceMutation": "none"
                }))
                .unwrap()
            );
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
