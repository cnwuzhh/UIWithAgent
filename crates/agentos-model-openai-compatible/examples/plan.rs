use agentos_agent_ledger::InMemoryAgentLedger;
use agentos_application::{PlanningAgent, Runtime};
use agentos_model_openai_compatible::OpenAiCompatiblePlanningAgent;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let request = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if request.trim().is_empty() {
        eprintln!(
            "usage: cargo run -p agentos-model-openai-compatible --example plan -- <request>"
        );
        std::process::exit(2);
    }

    let ledger = Arc::new(InMemoryAgentLedger::default());
    let agent = OpenAiCompatiblePlanningAgent::from_env(ledger);
    let result = match Runtime::demo().planning_request("surface-desktop", &request) {
        Ok(planning_request) => agent.plan(planning_request).await,
        Err(error) => Err(error),
    };

    match result {
        Ok(plan) => println!("{}", serde_json::to_string_pretty(&plan).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
