//! Fixed-timestep simulation engine orchestrating physics, drone kinematics, and environmental dynamics.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::drone::{Drone, DroneState, FlightMode, Waypoint};
use super::physics::{Boundary, NoFlyZone, Obstacle, Vec2, WindCondition};
use crate::navigation::{
    AStarPlanner, LocalizationState, OccupancyGrid, PathPoint, SafetyMonitor, SafetyStatus,
    StateEstimator,
};
use crate::analysis::{Alert, AlertManager, AnomalyEngine};
use crate::sensors::{FaultType, SensorReading, SensorSuite};

/// Mission event recorded in chronological log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEvent {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
}

/// Complete scenario specification for deterministic execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub seed: u64,
    pub boundary: Boundary,
    pub home_x: f64,
    pub home_y: f64,
    pub obstacles: Vec<Obstacle>,
    pub no_fly_zones: Vec<NoFlyZone>,
    pub waypoints: Vec<Waypoint>,
    pub wind: WindCondition,
}

/// Instantaneous simulation state snapshot dispatched to UI and recorder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSnapshot {
    pub drone: DroneState,
    pub localization: LocalizationState,
    pub sensor_readings: Vec<SensorReading>,
    pub safety: SafetyStatus,
    pub active_alerts: Vec<Alert>,
    pub recent_events: Vec<MissionEvent>,
    pub sim_time_sec: f64,
    pub tick_count: u64,
    pub planned_route: Vec<Vec2>,
    pub raw_trail: Vec<PathPoint>,
    pub filtered_trail: Vec<PathPoint>,
    pub active_faults: Vec<FaultType>,
    pub wind_vector: Vec2,
}

/// 20 Hz Fixed-Timestep Simulation Engine with deterministic PRNG seed.
pub struct SimulationEngine {
    pub drone: Drone,
    pub boundary: Boundary,
    pub obstacles: Vec<Obstacle>,
    pub no_fly_zones: Vec<NoFlyZone>,
    pub waypoints: Vec<Waypoint>,
    pub wind: WindCondition,
    pub sensors: SensorSuite,
    pub estimator: StateEstimator,
    pub occupancy_grid: OccupancyGrid,
    pub alert_manager: AlertManager,
    pub planned_route: Vec<Vec2>,
    pub current_route_index: usize,
    pub events: Vec<MissionEvent>,
    pub rng: ChaCha8Rng,
    pub seed: u64,
    pub sim_time_sec: f64,
    pub tick_count: u64,
    pub is_paused: bool,
    pub time_step: f64, // dt = 0.05s (20 Hz)
    event_counter: usize,
}

impl SimulationEngine {
    pub const DT: f64 = 0.05;

    pub fn new(seed: u64) -> Self {
        let default_boundary = Boundary {
            min_x: 0.0,
            max_x: 400.0,
            min_y: 0.0,
            max_y: 400.0,
            max_altitude: 120.0,
        };

        let home_x = 40.0;
        let home_y = 40.0;
        let rng = ChaCha8Rng::seed_from_u64(seed);

        let occupancy = OccupancyGrid::build(&default_boundary, 2.5, 3.0, &[], &[]);
        Self {
            drone: Drone::new(home_x, home_y),
            boundary: default_boundary,
            obstacles: Vec::new(),
            no_fly_zones: Vec::new(),
            waypoints: Vec::new(),
            planned_route: Vec::new(),
            current_route_index: 0,
            wind: WindCondition::default(),
            sensors: SensorSuite::new(),
            estimator: StateEstimator::new(home_x, home_y),
            occupancy_grid: occupancy,
            alert_manager: AlertManager::new(),
            events: Vec::new(),
            rng,
            seed,
            sim_time_sec: 0.0,
            tick_count: 0,
            is_paused: true,
            time_step: Self::DT,
            event_counter: 0,
        }
    }

    pub fn load_scenario(&mut self, scenario: &ScenarioConfig) {
        self.seed = scenario.seed;
        self.rng = ChaCha8Rng::seed_from_u64(scenario.seed);
        self.boundary = scenario.boundary.clone();
        self.obstacles = scenario.obstacles.clone();
        self.no_fly_zones = scenario.no_fly_zones.clone();
        self.waypoints = scenario.waypoints.clone();
        self.wind = scenario.wind.clone();

        self.occupancy_grid = OccupancyGrid::build(
            &self.boundary,
            2.5,
            3.0,
            &self.obstacles,
            &self.no_fly_zones,
        );
        self.reset_to_home(scenario.home_x, scenario.home_y);
        self.log_event("SCENARIO_LOADED", &format!("Scenario '{}' loaded with seed {}", scenario.name, scenario.seed), None);
    }

    pub fn reset_to_home(&mut self, home_x: f64, home_y: f64) {
        self.drone = Drone::new(home_x, home_y);
        self.estimator.reset(home_x, home_y);
        self.sensors.clear_all_faults();
        self.alert_manager.clear_all();
        self.planned_route.clear();
        self.current_route_index = 0;
        self.sim_time_sec = 0.0;
        self.tick_count = 0;
        self.is_paused = true;
        self.rng = ChaCha8Rng::seed_from_u64(self.seed);
    }

    pub fn log_event(&mut self, event_type: &str, description: &str, details: Option<serde_json::Value>) {
        self.event_counter += 1;
        let event = MissionEvent {
            id: format!("EVT-{:04}", self.event_counter),
            timestamp: format!("{:.2}s", self.sim_time_sec),
            event_type: event_type.to_string(),
            description: description.to_string(),
            details,
        };
        self.events.push(event);
    }

    pub fn arm(&mut self) -> Result<(), String> {
        self.drone.arm()?;
        self.log_event("DRONE_ARMED", "Motors armed in ground standby", None);
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.drone.disarm();
        self.log_event("DRONE_DISARMED", "Motors disarmed", None);
    }

    pub fn takeoff(&mut self, target_altitude: f64) -> Result<(), String> {
        self.drone.takeoff(target_altitude)?;
        self.log_event("TAKEOFF_INITIATED", &format!("Ascending to target hover altitude {:.1}m", target_altitude), None);
        Ok(())
    }

    pub fn land(&mut self) {
        self.drone.land();
        self.log_event("LAND_INITIATED", "Commencing descent to ground", None);
    }

    pub fn return_to_home(&mut self) {
        self.drone.return_to_home();
        self.log_event("RTH_INITIATED", "Return-to-home fail-safe activated", None);
    }

    pub fn emergency_land(&mut self) {
        self.drone.emergency_land();
        self.log_event("EMERGENCY_LAND_TRIGGERED", "Emergency landing command issued: descending at max safe rate", None);
    }

    pub fn hover(&mut self) {
        self.drone.hover();
        self.log_event("HOVER_MODE", "Holding station at current coordinates", None);
    }

    pub fn inject_fault(&mut self, fault: FaultType) {
        self.sensors.inject_fault(fault);
        self.log_event("FAULT_INJECTED", &format!("Injected sensor fault: {}", fault), None);
    }

    pub fn clear_fault(&mut self, fault: FaultType) {
        self.sensors.clear_fault(fault);
        self.log_event("FAULT_CLEARED", &format!("Cleared sensor fault: {}", fault), None);
    }

    pub fn clear_all_faults(&mut self) {
        self.sensors.clear_all_faults();
        self.log_event("ALL_FAULTS_CLEARED", "Restored all sensors to nominal state", None);
    }

    pub fn plan_and_follow_route_to(&mut self, goal_x: f64, goal_y: f64, altitude: f64) -> Result<Vec<Vec2>, String> {
        let start_w = Vec2::new(self.drone.state.x, self.drone.state.y);
        let goal_w = Vec2::new(goal_x, goal_y);

        if let Some(path) = AStarPlanner::plan_path(&self.occupancy_grid, &start_w, &goal_w) {
            if path.len() < 2 {
                return Err("Path planning returned trivial path".to_string());
            }
            self.planned_route = path.clone();
            self.current_route_index = 1;
            self.drone.set_waypoint(Waypoint {
                id: 1,
                x: path[1].x,
                y: path[1].y,
                altitude,
                speed_target: self.drone.config.max_horizontal_speed,
            });

            self.log_event(
                "ROUTE_PLANNED",
                &format!("A* path generated with {} waypoints to ({:.1}, {:.1})", path.len(), goal_x, goal_y),
                None,
            );
            Ok(path)
        } else {
            Err("No obstacle-free route found to destination".to_string())
        }
    }

    pub fn select_waypoint(&mut self, waypoint_id: usize) -> Result<(), String> {
        if let Some(wp) = self.waypoints.iter().find(|w| w.id == waypoint_id).cloned() {
            self.plan_and_follow_route_to(wp.x, wp.y, wp.altitude)?;
            self.drone.state.current_waypoint_index = Some(waypoint_id);
            self.log_event("WAYPOINT_SELECTED", &format!("Targeting scenario waypoint #{}", waypoint_id), None);
            Ok(())
        } else {
            Err(format!("Waypoint #{} not found in scenario", waypoint_id))
        }
    }

    /// Single simulation step at dt = 0.05 seconds (20 Hz).
    pub fn step(&mut self) -> SimulationSnapshot {
        let dt = self.time_step;
        self.sim_time_sec += dt;
        self.tick_count += 1;

        // 1. Wind simulation
        let wind_vec = self.wind.velocity_at(self.sim_time_sec);

        // 2. Physics & kinematics integration
        self.drone.update_kinematics(&wind_vec, dt);

        // 3. Boundary containment checks
        if !self.boundary.contains_point(self.drone.state.x, self.drone.state.y) {
            self.log_event("BOUNDARY_BREACH", "Drone exceeded scenario map boundaries", None);
        }

        // 4. Obstacle collision detection
        for obstacle in &self.obstacles {
            if obstacle.collides_2d(self.drone.state.x, self.drone.state.y, 1.0)
                && obstacle.collides_altitude(self.drone.state.altitude)
            {
                self.drone.state.flight_mode = FlightMode::Emergency;
                self.log_event(
                    "COLLISION_DETECTED",
                    &format!("Physical collision with obstacle '{}'", obstacle.id),
                    None,
                );
            }
        }

        // Sample sensor suite with seeded ChaCha8 PRNG
        let readings = self.sensors.sample(&self.drone.state, &self.boundary, &self.obstacles, &mut self.rng);
        let active_faults = self.sensors.get_active_faults();

        // 5. Waypoint progression along planned route
        if !self.planned_route.is_empty() && self.current_route_index < self.planned_route.len() {
            let target = &self.planned_route[self.current_route_index];
            let dist = ((self.drone.state.x - target.x).powi(2) + (self.drone.state.y - target.y).powi(2)).sqrt();
            if dist < 3.0 {
                self.current_route_index += 1;
                if self.current_route_index < self.planned_route.len() {
                    let next = &self.planned_route[self.current_route_index];
                    self.drone.set_waypoint(Waypoint {
                        id: self.current_route_index,
                        x: next.x,
                        y: next.y,
                        altitude: self.drone.state.altitude,
                        speed_target: self.drone.config.max_horizontal_speed,
                    });
                }
            }
        }

        // 6. Localization estimator
        let (raw_gps, imu_accel, compass_hdg, baro_alt) = self.sensors.get_nav_signals();
        let loc = self.estimator.predict_and_update(
            dt,
            imu_accel,
            compass_hdg,
            baro_alt,
            raw_gps,
            self.drone.state.flight_mode == FlightMode::Grounded,
        );

        // 7. Safety monitor
        let safety = SafetyMonitor::evaluate(&self.drone.state, &self.boundary, &self.obstacles, &self.no_fly_zones);

        // 8. Anomaly detection and alert updates
        let anomalies = AnomalyEngine::detect_anomalies(
            &self.drone.state,
            &self.sensors,
            &self.estimator,
            &safety,
            self.sim_time_sec,
        );
        for anomaly in anomalies {
            if let Some(alert) = self.alert_manager.fire_alert(anomaly) {
                self.log_event("ALERT_TRIGGERED", &alert.description, None);
            }
        }
        self.alert_manager.prune_resolved(self.sim_time_sec);

        SimulationSnapshot {
            drone: self.drone.state.clone(),
            localization: loc,
            sensor_readings: readings,
            safety,
            active_alerts: self.alert_manager.get_active_alerts(),
            recent_events: self.events.iter().rev().take(20).cloned().collect(),
            sim_time_sec: self.sim_time_sec,
            tick_count: self.tick_count,
            planned_route: self.planned_route.clone(),
            raw_trail: self.estimator.get_raw_trail(),
            filtered_trail: self.estimator.get_filtered_trail(),
            active_faults,
            wind_vector: wind_vec,
        }
    }
}
