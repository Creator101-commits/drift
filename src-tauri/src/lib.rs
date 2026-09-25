//! Tauri 2 backend core for Drift: real-time simulation loop, IPC commands, and database management.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager, State};

pub mod analysis;
pub mod db;
pub mod navigation;
pub mod replay;
pub mod scripting;
pub mod sensors;
pub mod simulation;

use db::{MissionDatabase, MissionSummary, SavedMissionRecord};
use replay::{MissionExporter, ReplayStatus, TrafficReplayer};
use sensors::FaultType;
use simulation::engine::ScenarioConfig;
use simulation::{Boundary, NoFlyZone, Obstacle, SimulationEngine, SimulationSnapshot, Vec2, Waypoint, WindCondition};

pub struct AppState {
    pub engine: Mutex<SimulationEngine>,
    pub replayer: Mutex<TrafficReplayer>,
    pub db: Arc<MissionDatabase>,
    pub recorded_snapshots: Mutex<Vec<SimulationSnapshot>>,
    pub active_scenario: Mutex<ScenarioConfig>,
    pub sim_speed_multiplier: Mutex<f64>,
}

fn create_default_scenario() -> ScenarioConfig {
    ScenarioConfig {
        id: "SCN-URBAN-01".to_string(),
        name: "Urban Grid Surveillance".to_string(),
        description: "Dense city district with building obstacles, central no-fly zone, and moderate crosswinds.".to_string(),
        seed: 42,
        boundary: Boundary {
            min_x: 0.0,
            max_x: 400.0,
            min_y: 0.0,
            max_y: 400.0,
            max_altitude: 100.0,
        },
        home_x: 40.0,
        home_y: 40.0,
        obstacles: vec![
            Obstacle {
                id: "BLD-01".to_string(),
                name: "Alpha Tower".to_string(),
                x: 130.0,
                y: 140.0,
                radius: 26.0,
                height: 60.0,
            },
            Obstacle {
                id: "BLD-02".to_string(),
                name: "Bravo Center".to_string(),
                x: 230.0,
                y: 160.0,
                radius: 32.0,
                height: 75.0,
            },
            Obstacle {
                id: "BLD-03".to_string(),
                name: "Charlie Plaza".to_string(),
                x: 170.0,
                y: 280.0,
                radius: 28.0,
                height: 50.0,
            },
            Obstacle {
                id: "BLD-04".to_string(),
                name: "Delta Heights".to_string(),
                x: 290.0,
                y: 290.0,
                radius: 30.0,
                height: 70.0,
            },
            Obstacle {
                id: "TWR-05".to_string(),
                name: "Communications Mast".to_string(),
                x: 310.0,
                y: 110.0,
                radius: 18.0,
                height: 85.0,
            },
        ],
        no_fly_zones: vec![
            NoFlyZone {
                id: "NFZ-GOV-01".to_string(),
                name: "Civic Airspace Restricted".to_string(),
                center_x: 200.0,
                center_y: 215.0,
                radius: 34.0,
                min_alt: 0.0,
                max_alt: 120.0,
            },
        ],
        waypoints: vec![
            Waypoint { id: 1, x: 75.0, y: 150.0, altitude: 20.0, speed_target: 8.0 },
            Waypoint { id: 2, x: 190.0, y: 90.0, altitude: 25.0, speed_target: 10.0 },
            Waypoint { id: 3, x: 330.0, y: 200.0, altitude: 30.0, speed_target: 10.0 },
            Waypoint { id: 4, x: 260.0, y: 350.0, altitude: 25.0, speed_target: 9.0 },
            Waypoint { id: 5, x: 80.0, y: 300.0, altitude: 20.0, speed_target: 8.0 },
        ],
        wind: WindCondition {
            speed_mps: 3.2,
            direction_deg: 55.0,
            gust_amplitude: 1.4,
            gust_period: 5.0,
        },
    }
}

// --- Tauri Commands ---

#[tauri::command]
fn get_simulation_state(state: State<'_, AppState>) -> SimulationSnapshot {
    let replayer = state.replayer.lock().unwrap();
    if replayer.is_active {
        return replayer.get_current_snapshot();
    }
    let mut engine = state.engine.lock().unwrap();
    engine.step()
}

#[tauri::command]
fn start_simulation(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.is_paused = false;
    let mut replayer = state.replayer.lock().unwrap();
    replayer.stop();
}

#[tauri::command]
fn pause_simulation(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.is_paused = true;
}

#[tauri::command]
fn resume_simulation(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.is_paused = false;
}

#[tauri::command]
fn reset_simulation(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    let sc = state.active_scenario.lock().unwrap();
    engine.load_scenario(&sc);
    state.recorded_snapshots.lock().unwrap().clear();
    let mut replayer = state.replayer.lock().unwrap();
    replayer.stop();
}

#[tauri::command]
fn arm_drone(state: State<'_, AppState>) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    engine.arm()
}

#[tauri::command]
fn disarm_drone(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.disarm();
}

#[tauri::command]
fn takeoff(state: State<'_, AppState>, altitude: f64) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    engine.takeoff(altitude)
}

#[tauri::command]
fn land(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.land();
}

#[tauri::command]
fn return_to_home(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.return_to_home();
}

#[tauri::command]
fn emergency_land(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.emergency_land();
}

#[tauri::command]
fn select_waypoint(state: State<'_, AppState>, waypoint_id: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    engine.select_waypoint(waypoint_id)
}

#[tauri::command]
fn plan_route_to_point(state: State<'_, AppState>, x: f64, y: f64, altitude: f64) -> Result<Vec<Vec2>, String> {
    let mut engine = state.engine.lock().unwrap();
    engine.plan_and_follow_route_to(x, y, altitude)
}

#[tauri::command]
fn inject_sensor_fault(state: State<'_, AppState>, fault_type: String) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    let fault = match fault_type.to_lowercase().as_str() {
        "gps_drift" => FaultType::GpsDrift,
        "gps_dropout" => FaultType::GpsDropout,
        "imu_bias" => FaultType::ImuBias,
        "compass_failure" => FaultType::CompassFailure,
        "altimeter_drift" => FaultType::AltimeterDrift,
        "lidar_blind_spot" | "lidar_failure" => FaultType::LidarBlindSpot,
        "excessive_noise" => FaultType::ExcessiveNoise,
        other => return Err(format!("Unknown fault type: {}", other)),
    };
    engine.inject_fault(fault);
    Ok(())
}

#[tauri::command]
fn clear_sensor_fault(state: State<'_, AppState>, fault_type: String) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    let fault = match fault_type.to_lowercase().as_str() {
        "gps_drift" => FaultType::GpsDrift,
        "gps_dropout" => FaultType::GpsDropout,
        "imu_bias" => FaultType::ImuBias,
        "compass_failure" => FaultType::CompassFailure,
        "altimeter_drift" => FaultType::AltimeterDrift,
        "lidar_blind_spot" | "lidar_failure" => FaultType::LidarBlindSpot,
        "excessive_noise" => FaultType::ExcessiveNoise,
        other => return Err(format!("Unknown fault type: {}", other)),
    };
    engine.clear_fault(fault);
    Ok(())
}

#[tauri::command]
fn clear_all_sensor_faults(state: State<'_, AppState>) {
    let mut engine = state.engine.lock().unwrap();
    engine.clear_all_faults();
}

#[tauri::command]
fn set_simulation_speed(state: State<'_, AppState>, speed: f64) {
    let mut s = state.sim_speed_multiplier.lock().unwrap();
    *s = speed.clamp(0.2, 10.0);
}

#[tauri::command]
fn run_mission_script(state: State<'_, AppState>, script_text: String) -> Result<usize, String> {
    let mut engine = state.engine.lock().unwrap();
    let count = engine.script_runner.load_script(&script_text)?;
    engine.log_event("SCRIPT_STARTED", &format!("Mission script initiated with {} commands", count), None);
    engine.is_paused = false;
    Ok(count)
}

#[tauri::command]
fn get_scenario_config(state: State<'_, AppState>) -> ScenarioConfig {
    state.active_scenario.lock().unwrap().clone()
}

#[tauri::command]
fn load_scenario_by_id(state: State<'_, AppState>, scenario_id: String) -> Result<ScenarioConfig, String> {
    let default_sc = create_default_scenario();
    let sc = if scenario_id == "SCN-URBAN-01" {
        default_sc
    } else if scenario_id == "SCN-CANYON-02" {
        ScenarioConfig {
            id: "SCN-CANYON-02".to_string(),
            name: "Canyon Wind Obstacle Challenge".to_string(),
            description: "Narrow corridor flanked by rock pillars with high turbulence and gusts.".to_string(),
            seed: 1337,
            boundary: Boundary { min_x: 0.0, max_x: 400.0, min_y: 0.0, max_y: 400.0, max_altitude: 120.0 },
            home_x: 30.0,
            home_y: 200.0,
            obstacles: vec![
                Obstacle { id: "OBS-01".to_string(), name: "North Ridge".to_string(), x: 140.0, y: 280.0, radius: 45.0, height: 90.0 },
                Obstacle { id: "OBS-02".to_string(), name: "South Ridge".to_string(), x: 140.0, y: 120.0, radius: 45.0, height: 90.0 },
                Obstacle { id: "OBS-03".to_string(), name: "Center Pillar".to_string(), x: 230.0, y: 200.0, radius: 22.0, height: 80.0 },
                Obstacle { id: "OBS-04".to_string(), name: "East Wall".to_string(), x: 320.0, y: 240.0, radius: 35.0, height: 90.0 },
            ],
            no_fly_zones: vec![],
            waypoints: vec![
                Waypoint { id: 1, x: 110.0, y: 200.0, altitude: 25.0, speed_target: 9.0 },
                Waypoint { id: 2, x: 230.0, y: 270.0, altitude: 30.0, speed_target: 10.0 },
                Waypoint { id: 3, x: 340.0, y: 160.0, altitude: 25.0, speed_target: 8.0 },
            ],
            wind: WindCondition { speed_mps: 6.2, direction_deg: 90.0, gust_amplitude: 2.8, gust_period: 4.0 },
        }
    } else {
        match state.db.list_scenarios() {
            Ok(list) => {
                if let Some(s) = list.into_iter().find(|s| s.id == scenario_id) {
                    s
                } else {
                    return Err(format!("Scenario {} not found", scenario_id));
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    };

    let mut active = state.active_scenario.lock().unwrap();
    *active = sc.clone();
    let mut engine = state.engine.lock().unwrap();
    engine.load_scenario(&sc);
    state.recorded_snapshots.lock().unwrap().clear();
    let mut replayer = state.replayer.lock().unwrap();
    replayer.stop();

    Ok(sc)
}

#[tauri::command]
fn save_current_mission(state: State<'_, AppState>, mission_name: String, notes: String) -> Result<String, String> {
    let snaps = state.recorded_snapshots.lock().unwrap().clone();
    if snaps.is_empty() {
        return Err("Cannot save mission: No telemetry recorded yet. Start simulation first.".to_string());
    }

    let engine = state.engine.lock().unwrap();
    let sc = state.active_scenario.lock().unwrap();

    let mission_id = format!("MSN-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let start_time = snaps.first().map(|s| format!("{:.2}s", s.sim_time_sec)).unwrap_or_default();
    let end_time = snaps.last().map(|s| format!("{:.2}s", s.sim_time_sec)).unwrap_or_default();

    let last_snap = snaps.last().unwrap();
    let total_dist = last_snap.drone.distance_traveled;
    let max_alt = snaps.iter().map(|s| s.drone.altitude).fold(0.0f64, f64::max);
    let battery_consumed = 100.0 - last_snap.drone.battery_percent;

    let record = SavedMissionRecord {
        id: mission_id.clone(),
        scenario_id: sc.id.clone(),
        name: if mission_name.is_empty() { format!("Mission {}", mission_id) } else { mission_name },
        start_time,
        end_time,
        status: format!("{:?}", last_snap.drone.flight_mode),
        total_distance: (total_dist * 10.0).round() / 10.0,
        max_altitude: (max_alt * 10.0).round() / 10.0,
        battery_consumed: (battery_consumed * 10.0).round() / 10.0,
        notes,
        snapshots: snaps,
        events: engine.events.clone(),
        alerts: engine.alert_manager.alert_history.clone(),
    };

    state.db.save_mission(&record).map_err(|e| e.to_string())?;
    Ok(mission_id)
}

#[tauri::command]
fn list_saved_missions(state: State<'_, AppState>) -> Result<Vec<MissionSummary>, String> {
    state.db.list_missions().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_saved_mission(state: State<'_, AppState>, mission_id: String) -> Result<bool, String> {
    state.db.delete_mission(&mission_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_replay(state: State<'_, AppState>, mission_id: String) -> Result<ReplayStatus, String> {
    let snaps = state.db.get_mission_snapshots(&mission_id).map_err(|e| e.to_string())?;
    if snaps.is_empty() {
        return Err("No telemetry frames found for selected mission".to_string());
    }

    let mut engine = state.engine.lock().unwrap();
    engine.is_paused = true; // Pause live simulation during replay

    let mut replayer = state.replayer.lock().unwrap();
    replayer.load_mission(snaps, Vec::new(), Vec::new());
    replayer.play();

    Ok(replayer.get_status())
}

#[tauri::command]
fn pause_replay(state: State<'_, AppState>) -> ReplayStatus {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.pause();
    replayer.get_status()
}

#[tauri::command]
fn resume_replay(state: State<'_, AppState>) -> ReplayStatus {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.resume();
    replayer.get_status()
}

#[tauri::command]
fn reset_replay(state: State<'_, AppState>) -> ReplayStatus {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.reset();
    replayer.get_status()
}

#[tauri::command]
fn set_replay_speed(state: State<'_, AppState>, speed: f64) -> ReplayStatus {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.set_speed(speed);
    replayer.get_status()
}

#[tauri::command]
fn seek_replay(state: State<'_, AppState>, index: usize) -> ReplayStatus {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.seek(index);
    replayer.get_status()
}

#[tauri::command]
fn stop_replay(state: State<'_, AppState>) {
    let mut replayer = state.replayer.lock().unwrap();
    replayer.stop();
}

#[tauri::command]
fn export_mission_json(state: State<'_, AppState>, mission_id: String) -> Result<String, String> {
    let snaps = state.db.get_mission_snapshots(&mission_id).map_err(|e| e.to_string())?;
    let record = SavedMissionRecord {
        id: mission_id,
        scenario_id: "SCN-EXPORT".to_string(),
        name: "Exported Mission".to_string(),
        start_time: "0.00s".to_string(),
        end_time: format!("{:.2}s", snaps.last().map(|s| s.sim_time_sec).unwrap_or(0.0)),
        status: "Completed".to_string(),
        total_distance: snaps.last().map(|s| s.drone.distance_traveled).unwrap_or(0.0),
        max_altitude: snaps.iter().map(|s| s.drone.altitude).fold(0.0, f64::max),
        battery_consumed: 100.0 - snaps.last().map(|s| s.drone.battery_percent).unwrap_or(100.0),
        notes: String::new(),
        snapshots: snaps,
        events: Vec::new(),
        alerts: Vec::new(),
    };
    MissionExporter::to_json(&record)
}

#[tauri::command]
fn export_mission_csv(state: State<'_, AppState>, mission_id: String) -> Result<String, String> {
    let snaps = state.db.get_mission_snapshots(&mission_id).map_err(|e| e.to_string())?;
    let record = SavedMissionRecord {
        id: mission_id,
        scenario_id: "SCN-EXPORT".to_string(),
        name: "Exported Mission".to_string(),
        start_time: "0.00s".to_string(),
        end_time: format!("{:.2}s", snaps.last().map(|s| s.sim_time_sec).unwrap_or(0.0)),
        status: "Completed".to_string(),
        total_distance: snaps.last().map(|s| s.drone.distance_traveled).unwrap_or(0.0),
        max_altitude: snaps.iter().map(|s| s.drone.altitude).fold(0.0, f64::max),
        battery_consumed: 100.0 - snaps.last().map(|s| s.drone.battery_percent).unwrap_or(100.0),
        notes: String::new(),
        snapshots: snaps,
        events: Vec::new(),
        alerts: Vec::new(),
    };
    Ok(MissionExporter::to_csv(&record))
}

#[tauri::command]
fn export_mission_html(state: State<'_, AppState>, mission_id: String) -> Result<String, String> {
    let snaps = state.db.get_mission_snapshots(&mission_id).map_err(|e| e.to_string())?;
    let record = SavedMissionRecord {
        id: mission_id,
        scenario_id: "SCN-EXPORT".to_string(),
        name: "Mission Execution Analysis Report".to_string(),
        start_time: "0.00s".to_string(),
        end_time: format!("{:.2}s", snaps.last().map(|s| s.sim_time_sec).unwrap_or(0.0)),
        status: "Completed".to_string(),
        total_distance: snaps.last().map(|s| s.drone.distance_traveled).unwrap_or(0.0),
        max_altitude: snaps.iter().map(|s| s.drone.altitude).fold(0.0, f64::max),
        battery_consumed: 100.0 - snaps.last().map(|s| s.drone.battery_percent).unwrap_or(100.0),
        notes: String::new(),
        snapshots: snaps,
        events: Vec::new(),
        alerts: Vec::new(),
    };
    Ok(MissionExporter::to_html_report(&record))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = if std::path::Path::new("../src-tauri").exists() {
        "../drift_data.db"
    } else {
        "drift_data.db"
    };
    let db = match MissionDatabase::new(db_path) {
        Ok(d) => Arc::new(d),
        Err(_) => Arc::new(MissionDatabase::new_in_memory().expect("Failed to initialize database")),
    };

    let default_scenario = create_default_scenario();
    let _ = db.save_scenario(&default_scenario);

    let mut engine = SimulationEngine::new(default_scenario.seed);
    engine.load_scenario(&default_scenario);

    let state = AppState {
        engine: Mutex::new(engine),
        replayer: Mutex::new(TrafficReplayer::new()),
        db,
        recorded_snapshots: Mutex::new(Vec::with_capacity(3600)),
        active_scenario: Mutex::new(default_scenario),
        sim_speed_multiplier: Mutex::new(1.0),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_simulation_state,
            start_simulation,
            pause_simulation,
            resume_simulation,
            reset_simulation,
            arm_drone,
            disarm_drone,
            takeoff,
            land,
            return_to_home,
            emergency_land,
            select_waypoint,
            plan_route_to_point,
            inject_sensor_fault,
            clear_sensor_fault,
            clear_all_sensor_faults,
            set_simulation_speed,
            run_mission_script,
            get_scenario_config,
            load_scenario_by_id,
            save_current_mission,
            list_saved_missions,
            delete_saved_mission,
            start_replay,
            pause_replay,
            resume_replay,
            reset_replay,
            set_replay_speed,
            seek_replay,
            stop_replay,
            export_mission_json,
            export_mission_csv,
            export_mission_html,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background Simulation & Replay Loop (20 Hz nominal rate = 50ms timestep)
            std::thread::spawn(move || {
                let tick_duration = Duration::from_millis(50);
                let mut last_event_count = 0;
                let mut last_alert_count = 0;

                loop {
                    std::thread::sleep(tick_duration);

                    let state: State<AppState> = app_handle.state();

                    // Check Replay loop
                    {
                        let mut replayer = state.replayer.lock().unwrap();
                        if replayer.is_active {
                            if let Some(snapshot) = replayer.step(0.05) {
                                let status = replayer.get_status();
                                let _ = app_handle.emit("telemetry-sample", &snapshot);
                                let _ = app_handle.emit("drone-state-changed", &snapshot.drone);
                                let _ = app_handle.emit("replay-state-changed", &status);
                            }
                            continue;
                        }
                    }

                    // Live Simulation Loop
                    let (snapshot, new_events, new_alerts) = {
                        let mut engine = state.engine.lock().unwrap();
                        if engine.is_paused {
                            continue;
                        }

                        let snap = engine.step();

                        // Accumulate telemetry for persistence/export (subsampled to 5 Hz to save memory)
                        if snap.tick_count % 4 == 0 {
                            let mut recorder = state.recorded_snapshots.lock().unwrap();
                            if recorder.len() < 5000 {
                                recorder.push(snap.clone());
                            }
                        }

                        let total_evts = engine.events.len();
                        let new_evts = if total_evts > last_event_count {
                            let evts = engine.events[last_event_count..total_evts].to_vec();
                            last_event_count = total_evts;
                            evts
                        } else {
                            Vec::new()
                        };

                        let total_alrts = engine.alert_manager.active_alerts.len();
                        let new_alrts = if total_alrts > last_alert_count {
                            let alrts = engine.alert_manager.active_alerts[last_alert_count..total_alrts].to_vec();
                            last_alert_count = total_alrts;
                            alrts
                        } else {
                            Vec::new()
                        };

                        (snap, new_evts, new_alrts)
                    };

                    // Emit Tauri events to frontend
                    let _ = app_handle.emit("telemetry-sample", &snapshot);
                    let _ = app_handle.emit("drone-state-changed", &snapshot.drone);

                    for evt in new_events {
                        let _ = app_handle.emit("mission-event", &evt);
                    }

                    for alrt in new_alerts {
                        let _ = app_handle.emit("alert-fired", &alrt);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
