use drift_lib::analysis::{AlertManager, AnomalyEngine};
use drift_lib::db::{MissionDatabase, SavedMissionRecord};
use drift_lib::navigation::{AStarPlanner, OccupancyGrid, SafetyMonitor, StateEstimator};
use drift_lib::replay::MissionExporter;
use drift_lib::scripting::commands::parse_mission_script;
use drift_lib::sensors::{FaultType, SensorSuite};
use drift_lib::simulation::{Boundary, Drone, DronePhysicalConfig, FlightMode, Obstacle, SimulationEngine, Vec2};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn test_drone_kinematics_and_takeoff() {
    let mut drone = Drone::new(50.0, 50.0);
    assert_eq!(drone.state.flight_mode, FlightMode::Disarmed);

    drone.arm().expect("Should arm");
    assert_eq!(drone.state.flight_mode, FlightMode::Armed);

    drone.takeoff(15.0).expect("Should takeoff");
    assert_eq!(drone.state.flight_mode, FlightMode::Takeoff);

    let wind = Vec2::new(0.0, 0.0);
    for _ in 0..100 {
        drone.step(0.05, &wind);
    }

    assert!(drone.state.altitude > 10.0);
    assert!(drone.state.battery_percent < 100.0);
}

#[test]
fn test_deterministic_sensor_reproducibility() {
    let seed = 42;
    let mut rng1 = ChaCha8Rng::seed_from_u64(seed);
    let mut rng2 = ChaCha8Rng::seed_from_u64(seed);

    let mut suite1 = SensorSuite::new();
    let mut suite2 = SensorSuite::new();

    let readings1 = suite1.update(100.0, 100.0, 25.0, 5.0, 90.0, 0.0, 0.1, 0.0, 0.0, 0.0, &[], 10.0, 0.05, &mut rng1);
    let readings2 = suite2.update(100.0, 100.0, 25.0, 5.0, 90.0, 0.0, 0.1, 0.0, 0.0, 0.0, &[], 10.0, 0.05, &mut rng2);

    assert_eq!(readings1.len(), readings2.len());
    for (r1, r2) in readings1.iter().zip(readings2.iter()) {
        assert_eq!(r1.sensor_name, r2.sensor_name);
        assert_eq!(r1.values, r2.values);
        assert_eq!(r1.confidence, r2.confidence);
    }
}

#[test]
fn test_sensor_fault_injection() {
    let mut suite = SensorSuite::new();
    let mut rng = ChaCha8Rng::seed_from_u64(1234);

    suite.inject_fault(FaultType::GpsDropout);
    assert!(suite.has_fault(FaultType::GpsDropout));

    let readings = suite.update(50.0, 50.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &[], 1.0, 0.05, &mut rng);
    let gps = readings.iter().find(|r| r.sensor_name.starts_with("GPS")).unwrap();
    assert!(!gps.validity);
    assert_eq!(gps.confidence, 0.0);

    suite.clear_fault(FaultType::GpsDropout);
    let readings_cleared = suite.update(50.0, 50.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &[], 2.0, 0.05, &mut rng);
    let gps_cleared = readings_cleared.iter().find(|r| r.sensor_name.starts_with("GPS")).unwrap();
    assert!(gps_cleared.validity);
    assert!(gps_cleared.confidence > 0.9);
}

#[test]
fn test_astar_occupancy_planning() {
    let boundary = Boundary { min_x: 0.0, max_x: 200.0, min_y: 0.0, max_y: 200.0, max_altitude: 100.0 };
    let obstacles = vec![
        Obstacle { id: "OBS1".to_string(), name: "Tower".to_string(), x: 100.0, y: 100.0, radius: 25.0, height: 50.0 }
    ];

    let grid = OccupancyGrid::build(&boundary, 2.0, 2.0, &obstacles, &[]);
    let start = Vec2::new(20.0, 100.0);
    let goal = Vec2::new(180.0, 100.0);

    let path = AStarPlanner::plan_path(&grid, &start, &goal).expect("Path should exist around obstacle");
    assert!(path.len() >= 2);

    // Verify path avoids center of obstacle
    for pt in &path {
        let dist = ((pt.x - 100.0).powi(2) + (pt.y - 100.0).powi(2)).sqrt();
        assert!(dist >= 20.0, "Path came too close to obstacle: dist={}", dist);
    }
}

#[test]
fn test_state_estimator_filtering() {
    let mut estimator = StateEstimator::new(50.0, 50.0);
    let mut suite = SensorSuite::new();
    let mut rng = ChaCha8Rng::seed_from_u64(999);

    // Run normal step
    let readings = suite.update(50.0, 50.0, 15.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &[], 1.0, 0.05, &mut rng);
    estimator.update(&readings, 0.05);

    assert!((estimator.state.filtered_x - 50.0).abs() < 2.0);

    // Inject GPS drift
    suite.inject_fault(FaultType::GpsDrift);
    for i in 1..=40 {
        let r = suite.update(50.0, 50.0, 15.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &[], 1.0 + i as f64 * 0.05, 0.05, &mut rng);
        estimator.update(&r, 0.05);
    }

    // Raw GPS has drifted, divergence should be detected
    assert!(estimator.state.estimation_error_m > 0.0);
}

#[test]
fn test_safety_and_anomaly_alerts() {
    let mut alert_mgr = AlertManager::new();
    let boundary = Boundary { min_x: 0.0, max_x: 100.0, min_y: 0.0, max_y: 100.0, max_altitude: 50.0 };
    let obstacles = vec![Obstacle { id: "B1".to_string(), name: "Wall".to_string(), x: 50.0, y: 50.0, radius: 10.0, height: 40.0 }];
    let config = DronePhysicalConfig::default();

    let mut state = drift_lib::simulation::DroneState::default();
    state.x = 95.0; // Near boundary
    state.y = 50.0;
    state.altitude = 10.0;
    state.battery_percent = 20.0; // Low battery

    let safety = SafetyMonitor::evaluate(&state, &config, &boundary, &obstacles, 5.0);
    let loc = drift_lib::navigation::LocalizationState::default();
    let home = Vec2::new(10.0, 10.0);

    let alerts = AnomalyEngine::evaluate(&mut alert_mgr, &state, &loc, &[], &safety, &boundary, &home, 10.0);
    assert!(!alerts.is_empty());
    assert!(alerts.iter().any(|a| a.alert_type == "LOW_BATTERY_WARNING" || a.alert_type == "COLLISION_RISK_CAUTION" || a.alert_type == "GEOFENCE_WARNING"));
}

#[test]
fn test_script_parser() {
    let script = r#"
        TAKEOFF 20
        WAIT 5
        GOTO 150 180 25
        INJECT GPS_DRIFT
        WAIT 3
        RETURN_HOME
    "#;

    let cmds = parse_mission_script(script).expect("Script parsing should succeed");
    assert_eq!(cmds.len(), 6);
}

#[test]
fn test_database_persistence_and_export() {
    let db = MissionDatabase::new_in_memory().expect("In-memory database should initialize");

    let record = SavedMissionRecord {
        id: "MSN-TEST-01".to_string(),
        scenario_id: "SCN-01".to_string(),
        name: "Test Mission".to_string(),
        start_time: "0.00s".to_string(),
        end_time: "10.00s".to_string(),
        status: "Hover".to_string(),
        total_distance: 85.4,
        max_altitude: 20.0,
        battery_consumed: 4.5,
        notes: "Unit test recording".to_string(),
        snapshots: Vec::new(),
        events: Vec::new(),
        alerts: Vec::new(),
    };

    db.save_mission(&record).expect("Should save mission");
    let list = db.list_missions().expect("Should list missions");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "MSN-TEST-01");

    let json_str = MissionExporter::to_json(&record).expect("JSON export should succeed");
    assert!(json_str.contains("MSN-TEST-01"));

    let csv_str = MissionExporter::to_csv(&record);
    assert!(csv_str.starts_with("step,sim_time_sec"));

    let html_str = MissionExporter::to_html_report(&record);
    assert!(html_str.contains("<!DOCTYPE html>"));
    assert!(html_str.contains("Test Mission"));

    let deleted = db.delete_mission("MSN-TEST-01").expect("Should delete");
    assert!(deleted);
    let after_delete = db.list_missions().unwrap();
    assert_eq!(after_delete.len(), 0);
}

#[test]
fn test_simulation_engine_autonomous_routing_and_snapshot() {
    let mut engine = SimulationEngine::new(42);
    assert!(!engine.is_paused);
    assert_eq!(engine.drone.state.flight_mode, FlightMode::Disarmed);

    // Initial snapshot should be readable immediately
    let initial_snap = engine.current_snapshot();
    assert_eq!(initial_snap.drone.altitude, 0.0);
    assert_eq!(initial_snap.tick_count, 0);

    // Plan route should auto-arm and begin following route
    let route = engine.plan_and_follow_route_to(100.0, 100.0, 20.0).expect("Should plan route");
    assert!(route.len() >= 2);
    assert!(engine.drone.state.armed);
    assert!(matches!(engine.drone.state.flight_mode, FlightMode::WaypointFollow | FlightMode::Takeoff));

    // Step simulation
    for _ in 0..100 {
        let snap = engine.step();
        assert!(snap.tick_count > 0);
    }

    assert!(engine.drone.state.altitude > 10.0);
}
