//! Fixed-timestep simulation engine orchestrating physics, drone kinematics, and environmental dynamics.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use super::drone::{Drone, DroneState, FlightMode, Waypoint};
use super::physics::{Boundary, NoFlyZone, Obstacle, Vec2, WindCondition};

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
    pub sim_time_sec: f64,
    pub tick_count: u64,
    pub wind_vector: Vec2,
    pub recent_events: Vec<MissionEvent>,
}

/// 20 Hz Fixed-Timestep Simulation Engine with deterministic PRNG seed.
pub struct SimulationEngine {
    pub drone: Drone,
    pub boundary: Boundary,
    pub obstacles: Vec<Obstacle>,
    pub no_fly_zones: Vec<NoFlyZone>,
    pub waypoints: Vec<Waypoint>,
    pub wind: WindCondition,
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

        Self {
            drone: Drone::new(home_x, home_y),
            boundary: default_boundary,
            obstacles: Vec::new(),
            no_fly_zones: Vec::new(),
            waypoints: Vec::new(),
            wind: WindCondition::default(),
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

        self.reset_to_home(scenario.home_x, scenario.home_y);
        self.log_event("SCENARIO_LOADED", &format!("Scenario '{}' loaded with seed {}", scenario.name, scenario.seed), None);
    }

    pub fn reset_to_home(&mut self, home_x: f64, home_y: f64) {
        self.drone = Drone::new(home_x, home_y);
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

        SimulationSnapshot {
            drone: self.drone.state.clone(),
            sim_time_sec: self.sim_time_sec,
            tick_count: self.tick_count,
            wind_vector: wind_vec,
            recent_events: self.events.iter().rev().take(20).cloned().collect(),
        }
    }
}
