//! Drift desktop application core runtime foundation.

#[tauri::command]
fn load_scenario_manifest(scenario_id: String) -> Result<String, String> {
    Ok(format!("Drift scenario loaded: {}", scenario_id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_scenario_manifest])
        .run(tauri::generate_context!())
        .expect("error while running drift application");
}
