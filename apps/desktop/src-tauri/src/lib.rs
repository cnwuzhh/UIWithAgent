use agentos_contracts::RuntimeSnapshotDto;

#[tauri::command]
fn get_runtime_snapshot() -> RuntimeSnapshotDto {
    agentos_application::initial_runtime_snapshot()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_runtime_snapshot])
        .run(tauri::generate_context!())
        .expect("failed to run UI With Agent");
}
