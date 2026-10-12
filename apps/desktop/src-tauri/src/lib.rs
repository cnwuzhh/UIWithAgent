use agentos_agent_ledger::SqliteAgentLedger;
use agentos_application::PlanningAgent;
use agentos_contracts::{AgentPlanDto, BuildSubmissionDto, BuildTaskDto, RuntimeSnapshotDto};
use agentos_generated_app_runtime::{
    BuildCommandExecution, GeneratedAppPackageDraft, build_package, materialize_package,
};
use agentos_model_openai_compatible::OpenAiCompatiblePlanningAgent;
use agentos_toolbox::ToolboxDescriptor;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, Window};

static NEXT_PLAN_ID: AtomicU64 = AtomicU64::new(1);

struct ApprovedPlans(Mutex<BTreeMap<String, AgentPlanDto>>);
struct ApprovedPackages(Mutex<BTreeMap<String, GeneratedAppPackageDraft>>);
struct PackageMaterializationLock(Mutex<()>);
struct GeneratedAppWorkspaceRoot(PathBuf);

fn next_package_revision(workspace_root: &Path, app_id: &str) -> Result<u32, String> {
    let app_root = workspace_root.join(app_id);
    if !app_root.exists() {
        return Ok(1);
    }

    let mut latest_revision = 0;
    for entry in fs::read_dir(&app_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().map_err(|error| error.to_string())?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(revision) = name
            .to_str()
            .and_then(|name| name.strip_prefix("revision-"))
            .and_then(|revision| revision.parse::<u32>().ok())
        else {
            continue;
        };
        latest_revision = latest_revision.max(revision);
    }
    latest_revision
        .checked_add(1)
        .ok_or_else(|| format!("package revision overflow for {app_id}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewedAgentPlanDto {
    plan_id: String,
    plan: AgentPlanDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentBuildResultDto {
    app_id: String,
    revision: u32,
    workspace: PathBuf,
    files: Vec<String>,
    commands: Vec<BuildCommandExecution>,
}

#[tauri::command]
fn get_runtime_snapshot(
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> RuntimeSnapshotDto {
    runtime.lock().expect("runtime lock poisoned").snapshot()
}

#[tauri::command]
fn get_toolbox_descriptor(
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> ToolboxDescriptor {
    runtime
        .lock()
        .expect("runtime lock poisoned")
        .toolbox_descriptor()
}

#[tauri::command]
fn open_surface(
    surface_id: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<RuntimeSnapshotDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .open_surface(&surface_id)
}

#[tauri::command]
fn reposition_element(
    surface_id: String,
    element_id: String,
    x: u8,
    y: u8,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<RuntimeSnapshotDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .reposition_element(&surface_id, &element_id, x, y)
}

#[tauri::command]
fn resize_element(
    surface_id: String,
    element_id: String,
    width: u8,
    height: u8,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<RuntimeSnapshotDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .resize_element(&surface_id, &element_id, width, height)
}

#[tauri::command]
fn add_time_panel(
    surface_id: String,
    title: String,
    timezone: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<RuntimeSnapshotDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .add_time_panel(&surface_id, &title, &timezone)
}

#[tauri::command]
fn remove_element(
    surface_id: String,
    element_id: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<RuntimeSnapshotDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .remove_element(&surface_id, &element_id)
}

#[tauri::command]
fn submit_build(
    surface_id: String,
    request: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<BuildSubmissionDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .submit_build(&surface_id, &request)
}

#[tauri::command]
fn get_build_task(
    build_task_id: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> Result<BuildTaskDto, String> {
    runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .build_task(&build_task_id)
}

#[tauri::command]
async fn plan_agent(
    surface_id: String,
    request: String,
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
    agent: State<'_, OpenAiCompatiblePlanningAgent>,
    approved_plans: State<'_, ApprovedPlans>,
) -> Result<ReviewedAgentPlanDto, String> {
    let planning_request = runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?
        .planning_request(&surface_id, &request)?;
    let plan = agent.plan(planning_request).await?;
    let plan_id = format!("plan-{}", NEXT_PLAN_ID.fetch_add(1, Ordering::Relaxed));
    approved_plans
        .0
        .lock()
        .map_err(|_| "approved plan lock poisoned".to_string())?
        .insert(plan_id.clone(), plan.clone());
    Ok(ReviewedAgentPlanDto { plan_id, plan })
}

#[tauri::command]
async fn generate_agent_package(
    plan_id: String,
    agent: State<'_, OpenAiCompatiblePlanningAgent>,
    approved_plans: State<'_, ApprovedPlans>,
    approved_packages: State<'_, ApprovedPackages>,
) -> Result<GeneratedAppPackageDraft, String> {
    let plan = approved_plans
        .0
        .lock()
        .map_err(|_| "approved plan lock poisoned".to_string())?
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| "已审核方案不存在或桌面应用已重启，请重新生成方案".to_string())?;
    let package = agent.generate_package(&plan.request, &plan).await?;
    approved_packages
        .0
        .lock()
        .map_err(|_| "approved package lock poisoned".to_string())?
        .insert(plan_id, package.clone());
    Ok(package)
}

#[tauri::command]
async fn build_agent_package(
    plan_id: String,
    approved_packages: State<'_, ApprovedPackages>,
    workspace_root: State<'_, GeneratedAppWorkspaceRoot>,
    materialization_lock: State<'_, PackageMaterializationLock>,
    ledger: State<'_, Arc<SqliteAgentLedger>>,
) -> Result<AgentBuildResultDto, String> {
    let package = approved_packages
        .0
        .lock()
        .map_err(|_| "approved package lock poisoned".to_string())?
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| "已审核代码包不存在或桌面应用已重启，请重新生成代码包".to_string())?;
    let materialized = {
        let _guard = materialization_lock
            .0
            .lock()
            .map_err(|_| "package materialization lock poisoned".to_string())?;
        let revision = next_package_revision(&workspace_root.0, &package.manifest.app_id)?;
        materialize_package(&workspace_root.0, revision, &package)?
    };
    let revision = materialized.revision;
    let execution = build_package(
        ledger.inner().as_ref(),
        &materialized.workspace,
        revision,
        &package.manifest,
        &BTreeMap::new(),
    )
    .await?;
    Ok(AgentBuildResultDto {
        app_id: materialized.app_id,
        revision: materialized.revision,
        workspace: materialized.workspace,
        files: materialized.files,
        commands: execution.commands,
    })
}

#[tauri::command]
fn is_fullscreen(window: Window) -> Result<bool, String> {
    window.is_fullscreen().map_err(|error| error.to_string())
}

#[tauri::command]
fn toggle_fullscreen(window: Window) -> Result<bool, String> {
    let fullscreen = window.is_fullscreen().map_err(|error| error.to_string())?;
    let next = !fullscreen;
    window
        .set_fullscreen(next)
        .map_err(|error| error.to_string())?;
    Ok(next)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(agentos_application::Runtime::demo()))
        .manage(ApprovedPlans(Mutex::new(BTreeMap::new())))
        .manage(ApprovedPackages(Mutex::new(BTreeMap::new())))
        .manage(PackageMaterializationLock(Mutex::new(())))
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let ledger_path = app_data_dir.join("agent-ledger.sqlite3");
            let ledger = Arc::new(SqliteAgentLedger::open(ledger_path)?);
            let agent = OpenAiCompatiblePlanningAgent::from_env(ledger.clone());
            app.manage(GeneratedAppWorkspaceRoot(
                app_data_dir.join("generated-apps"),
            ));
            app.manage(ledger);
            app.manage(agent);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_runtime_snapshot,
            get_toolbox_descriptor,
            open_surface,
            reposition_element,
            resize_element,
            add_time_panel,
            remove_element,
            submit_build,
            get_build_task,
            plan_agent,
            generate_agent_package,
            build_agent_package,
            is_fullscreen,
            toggle_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("failed to run UI With Agent");
}

#[cfg(test)]
mod tests {
    use super::next_package_revision;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn allocates_after_the_highest_materialized_revision() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("agentos-revision-test-{nonce}"));
        let app_root = root.join("finance-news");
        fs::create_dir_all(app_root.join("revision-1")).unwrap();
        fs::create_dir_all(app_root.join("revision-3")).unwrap();
        fs::create_dir_all(app_root.join(".revision-4.staging")).unwrap();

        assert_eq!(next_package_revision(&root, "finance-news").unwrap(), 4);
        assert_eq!(next_package_revision(&root, "new-app").unwrap(), 1);

        fs::remove_dir_all(root).unwrap();
    }
}
