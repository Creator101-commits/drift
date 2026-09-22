//! Drift desktop application core runtime and simulation engine.

pub mod simulation;

#[tauri::command]
fn load_scenario_manifest(scenario_id: String) -> Result<String, String> {
    Ok(format!("Drift scenario loaded: {}", scenario_id))
}

#[tauri::command]
fn start_simulation() -> Result<String, String> {
    Ok("Simulation started at 20 Hz".into())
}

#[tauri::command]
fn step_simulation() -> Result<String, String> {
    Ok("Step completed".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_scenario_manifest,
            start_simulation,
            step_simulation
        ])
        .run(tauri::generate_context!())
        .expect("error while running drift application");
}
