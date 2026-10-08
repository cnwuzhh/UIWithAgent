use agentos_contracts::RuntimeSnapshotDto;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
fn get_runtime_snapshot(
    runtime: State<'_, Mutex<agentos_application::Runtime>>,
) -> RuntimeSnapshotDto {
    runtime.lock().expect("runtime lock poisoned").snapshot()
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(agentos_application::Runtime::demo()))
        .invoke_handler(tauri::generate_handler![get_runtime_snapshot, open_surface])
        .run(tauri::generate_context!())
        .expect("failed to run UI With Agent");
}
