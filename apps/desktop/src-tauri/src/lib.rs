use agentos_contracts::RuntimeSnapshotDto;
use std::sync::Mutex;
use tauri::{State, Window};

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
        .invoke_handler(tauri::generate_handler![
            get_runtime_snapshot,
            open_surface,
            reposition_element,
            is_fullscreen,
            toggle_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("failed to run UI With Agent");
}
