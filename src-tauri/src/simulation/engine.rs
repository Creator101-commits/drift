//! Fixed-timestep simulation engine orchestrating physics, sensors, navigation, and alerts.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::drone::{Drone, DroneState, FlightMode, Waypoint};
use super::physics::{Boundary, NoFlyZone, Obstacle, Vec2, WindCondition};
use crate::analysis::{Alert, AlertManager, AnomalyEngine};
use crate::navigation::{
    AStarPlanner, LocalizationState, OccupancyGrid, PathPoint, SafetyMonitor, SafetyStatus,
    StateEstimator,
};
use crate::scripting::{CommandRunner, MissionCommand};
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
    pub is_script_running: bool,
    pub current_script_step: usize,
    pub total_script_steps: usize,
}

/// 20 Hz Fixed-Timestep Simulation Engine with deterministic PRNG seed.
pub struct SimulationEngine {
    pub drone: Drone,
    pub boundary: Boundary,
    pub obstacles: Vec<Obstacle>,
    pub no_fly_zones: Vec<NoFlyZone>,
    pub waypoints: Vec<Waypoint>,
    pub planned_route: Vec<Vec2>,
    pub current_route_index: usize,
    pub wind: WindCondition,
    pub sensors: SensorSuite,
    pub estimator: StateEstimator,
    pub occupancy_grid: OccupancyGrid,
    pub alert_manager: AlertManager,
    pub script_runner: CommandRunner,
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

        let occupancy = OccupancyGrid::build(&default_boundary, 2.5, 3.0, &[], &[]);
        let rng = ChaCha8Rng::seed_from_u64(seed);

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
            script_runner: CommandRunner::new(),
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
        self.script_runner.reset();
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

    // --- Command APIs ---

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
        self.log_event(
            "TAKEOFF_INITIATED",
            &format!("Ascending to target hover altitude {:.1}m", target_altitude),
            None,
        );
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
        self.log_event(
            "EMERGENCY_LAND_TRIGGERED",
            "Emergency landing command issued: descending at max safe rate",
            None,
        );
    }

    pub fn hover(&mut self) {
        self.drone.hover();
        self.log_event("HOVER_MODE", "Holding station at current coordinates", None);
    }

    pub fn plan_and_follow_route_to(&mut self, goal_x: f64, goal_y: f64, altitude: f64) -> Result<Vec<Vec2>, String> {
        let start_w = Vec2::new(self.drone.state.x, self.drone.state.y);
        let goal_w = Vec2::new(goal_x, goal_y);

        if let Some(path) = AStarPlanner::plan_path(&self.occupancy_grid, &start_w, &goal_w) {
            if path.len() < 2 {
                return Err("Path planning returned trivial path".to_string());
            }
            self.planned_route = path.clone();
            self.current_route_index = 1; // First segment target
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
                Some(serde_json::json!({ "waypoints_count": path.len() })),
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

    pub fn inject_fault(&mut self, fault: FaultType) {
        self.sensors.inject_fault(fault);
        self.log_event(
            "FAULT_INJECTED",
            &format!("Injected sensor fault: {}", fault),
            Some(serde_json::json!({ "fault": format!("{:?}", fault) })),
        );
    }

    pub fn clear_fault(&mut self, fault: FaultType) {
        self.sensors.clear_fault(fault);
        self.log_event(
            "FAULT_CLEARED",
            &format!("Cleared sensor fault: {}", fault),
            Some(serde_json::json!({ "fault": format!("{:?}", fault) })),
        );
    }

    pub fn clear_all_faults(&mut self) {
        self.sensors.clear_all_faults();
        self.log_event("ALL_FAULTS_CLEARED", "Restored all sensors to nominal state", None);
    }

    /// Single simulation step at dt = 0.05 seconds (20 Hz).
    pub fn step(&mut self) -> SimulationSnapshot {
        let dt = self.time_step;
        self.sim_time_sec += dt;
        self.tick_count += 1;

        // 1. Process automated script commands if active
        self.step_script(dt);

        // 2. Wind simulation
        let wind_vec = self.wind.velocity_at(self.sim_time_sec);

        // 3. Update multi-point route progression
        self.update_route_progression();

        // 4. Step drone kinematics and controllers
        self.drone.step(dt, &wind_vec);

        // 5. Update sensors with deterministic seeded noise
        let sensor_readings = self.sensors.update(
            self.drone.state.x,
            self.drone.state.y,
            self.drone.state.altitude,
            self.drone.state.horizontal_speed,
            self.drone.state.heading,
            self.drone.state.vertical_speed,
            self.drone.state.vx,
            self.drone.state.vy,
            self.drone.state.vz,
            0.0,
            &self.obstacles,
            self.sim_time_sec,
            dt,
            &mut self.rng,
        );

        // 6. Update Kalman / complementary state estimator
        self.estimator.update(&sensor_readings, dt);

        // Extract LiDAR nearest distance for safety monitor
        let lidar_min_dist = sensor_readings
            .iter()
            .find(|r| r.sensor_name.starts_with("LiDAR"))
            .and_then(|r| r.values.get("min_distance_m").and_then(|v| v.as_f64()))
            .unwrap_or(35.0);

        // 7. Evaluate spatial safety and boundaries
        let safety = SafetyMonitor::evaluate(
            &self.drone.state,
            &self.drone.config,
            &self.boundary,
            &self.obstacles,
            lidar_min_dist,
        );

        if safety.collision_detected && self.drone.state.flight_mode != FlightMode::Collision {
            self.drone.state.flight_mode = FlightMode::Collision;
            self.log_event(
                "COLLISION",
                &format!("Drone impacted obstacle ID {:?}", safety.collision_obstacle_id),
                None,
            );
        }

        // 8. Evaluate alerts and anomalies
        let _new_alerts = AnomalyEngine::evaluate(
            &mut self.alert_manager,
            &self.drone.state,
            &self.estimator.state,
            &sensor_readings,
            &safety,
            &self.boundary,
            &self.drone.home_position,
            self.sim_time_sec,
        );

        // 9. Construct and return snapshot
        SimulationSnapshot {
            drone: self.drone.state.clone(),
            localization: self.estimator.state.clone(),
            sensor_readings,
            safety,
            active_alerts: self.alert_manager.active_alerts.clone(),
            recent_events: self.events.iter().rev().take(50).cloned().collect(),
            sim_time_sec: (self.sim_time_sec * 100.0).round() / 100.0,
            tick_count: self.tick_count,
            planned_route: self.planned_route.clone(),
            raw_trail: self.estimator.raw_trail.clone(),
            filtered_trail: self.estimator.filtered_trail.clone(),
            active_faults: self.sensors.active_faults.clone(),
            wind_vector: wind_vec,
            is_script_running: self.script_runner.is_running,
            current_script_step: self.script_runner.current_step,
            total_script_steps: self.script_runner.commands.len(),
        }
    }

    fn update_route_progression(&mut self) {
        if self.planned_route.is_empty() || self.current_route_index >= self.planned_route.len() {
            return;
        }

        let target_pt = self.planned_route[self.current_route_index];
        let drone_pos = Vec2::new(self.drone.state.x, self.drone.state.y);
        let dist = drone_pos.distance_to(&target_pt);

        if dist < 2.0 {
            // Reached this path point
            self.current_route_index += 1;
            if self.current_route_index < self.planned_route.len() {
                let next_pt = self.planned_route[self.current_route_index];
                self.drone.set_waypoint(Waypoint {
                    id: self.current_route_index,
                    x: next_pt.x,
                    y: next_pt.y,
                    altitude: self.drone.target_altitude,
                    speed_target: self.drone.config.max_horizontal_speed,
                });
            } else {
                // Route complete
                self.drone.hover();
                self.log_event("ROUTE_COMPLETED", "Final route waypoint reached, holding station", None);
            }
        }
    }

    fn step_script(&mut self, dt: f64) {
        if !self.script_runner.is_running || self.script_runner.current_step >= self.script_runner.commands.len() {
            if self.script_runner.is_running {
                self.script_runner.is_running = false;
                self.log_event("SCRIPT_COMPLETED", "Mission script sequence execution finished", None);
            }
            return;
        }

        let cmd = self.script_runner.commands[self.script_runner.current_step].clone();

        match cmd {
            MissionCommand::Takeoff { altitude } => {
                if !self.drone.state.armed {
                    let _ = self.arm();
                }
                let _ = self.takeoff(altitude);
                self.script_runner.current_step += 1;
            }
            MissionCommand::Goto { x, y, altitude } => {
                let _ = self.plan_and_follow_route_to(x, y, altitude);
                self.script_runner.current_step += 1;
            }
            MissionCommand::Wait { seconds } => {
                self.script_runner.wait_timer += dt;
                if self.script_runner.wait_timer >= seconds {
                    self.script_runner.wait_timer = 0.0;
                    self.script_runner.current_step += 1;
                }
            }
            MissionCommand::InjectFault { fault_name } => {
                let fault = match fault_name.as_str() {
                    "GPS_DRIFT" => FaultType::GpsDrift,
                    "GPS_DROPOUT" => FaultType::GpsDropout,
                    "IMU_BIAS" => FaultType::ImuBias,
                    "COMPASS_FAILURE" => FaultType::CompassFailure,
                    "ALTIMETER_DRIFT" => FaultType::AltimeterDrift,
                    "LIDAR_FAILURE" | "LIDAR_BLIND_SPOT" => FaultType::LidarBlindSpot,
                    "EXCESSIVE_NOISE" => FaultType::ExcessiveNoise,
                    _ => FaultType::GpsDrift,
                };
                self.inject_fault(fault);
                self.script_runner.current_step += 1;
            }
            MissionCommand::ClearFaults => {
                self.clear_all_faults();
                self.script_runner.current_step += 1;
            }
            MissionCommand::ReturnHome => {
                self.return_to_home();
                self.script_runner.current_step += 1;
            }
            MissionCommand::Land => {
                self.land();
                self.script_runner.current_step += 1;
            }
            MissionCommand::EmergencyLand => {
                self.emergency_land();
                self.script_runner.current_step += 1;
            }
        }
    }
}
