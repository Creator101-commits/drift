//! Drift desktop application core runtime, simulation, sensors, and navigation.

pub mod navigation;
pub mod sensors;
pub mod simulation;

use sensors::FaultType;

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

#[tauri::command]
fn inject_sensor_fault(fault: FaultType) -> Result<String, String> {
    Ok(format!("Injected sensor fault: {:?}", fault))
}

#[tauri::command]
fn clear_sensor_faults() -> Result<String, String> {
    Ok("Cleared all active sensor faults".into())
}

#[tauri::command]
fn plan_route_to(target_x: f64, target_y: f64) -> Result<String, String> {
    Ok(format!("Planned A* path to ({:.1}, {:.1})", target_x, target_y))
}

#[tauri::command]
fn return_to_home() -> Result<String, String> {
    Ok("Return to home commanded".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_scenario_manifest,
            start_simulation,
            step_simulation,
            inject_sensor_fault,
            clear_sensor_faults,
            plan_route_to,
            return_to_home
        ])
        .run(tauri::generate_context!())
        .expect("error while running drift application");
}
