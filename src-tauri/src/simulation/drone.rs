//! Drone state definitions, kinematics, and flight mode controllers.

use serde::{Deserialize, Serialize};
use super::physics::{DronePhysicalConfig, Vec2};

/// Operational flight modes for autonomous guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightMode {
    Disarmed,
    Armed,
    Takeoff,
    Hover,
    WaypointFollow,
    ReturnToHome,
    EmergencyLand,
    Landed,
    Collision,
}

impl std::fmt::Display for FlightMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlightMode::Disarmed => write!(f, "DISARMED"),
            FlightMode::Armed => write!(f, "ARMED"),
            FlightMode::Takeoff => write!(f, "TAKEOFF"),
            FlightMode::Hover => write!(f, "HOVER"),
            FlightMode::WaypointFollow => write!(f, "WAYPOINT_FOLLOW"),
            FlightMode::ReturnToHome => write!(f, "RETURN_TO_HOME"),
            FlightMode::EmergencyLand => write!(f, "EMERGENCY_LAND"),
            FlightMode::Landed => write!(f, "LANDED"),
            FlightMode::Collision => write!(f, "COLLISION"),
        }
    }
}

/// Instantaneous drone physical and logical state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneState {
    pub x: f64,
    pub y: f64,
    pub altitude: f64,
    pub heading: f64,
    pub horizontal_speed: f64,
    pub vertical_speed: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub battery_percent: f64,
    pub flight_mode: FlightMode,
    pub armed: bool,
    pub current_waypoint_index: Option<usize>,
    pub return_to_home: bool,
    pub emergency: bool,
    pub distance_traveled: f64,
    pub flight_time_seconds: f64,
}

impl Default for DroneState {
    fn default() -> Self {
        Self {
            x: 50.0,
            y: 50.0,
            altitude: 0.0,
            heading: 0.0,
            horizontal_speed: 0.0,
            vertical_speed: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            battery_percent: 100.0,
            flight_mode: FlightMode::Disarmed,
            armed: false,
            current_waypoint_index: None,
            return_to_home: false,
            emergency: false,
            distance_traveled: 0.0,
            flight_time_seconds: 0.0,
        }
    }
}

/// Waypoint in 3D local coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub altitude: f64,
    pub speed_target: f64,
}

/// Drone entity simulating motor thrust, kinematics, and guidance commands.
#[derive(Debug, Clone)]
pub struct Drone {
    pub state: DroneState,
    pub config: DronePhysicalConfig,
    pub target_altitude: f64,
    pub target_position: Option<Vec2>,
    pub home_position: Vec2,
    pub safe_rth_altitude: f64,
}

impl Drone {
    pub fn new(start_x: f64, start_y: f64) -> Self {
        let mut state = DroneState::default();
        state.x = start_x;
        state.y = start_y;

        Self {
            state,
            config: DronePhysicalConfig::default(),
            target_altitude: 0.0,
            target_position: None,
            home_position: Vec2::new(start_x, start_y),
            safe_rth_altitude: 20.0,
        }
    }

    pub fn arm(&mut self) -> Result<(), String> {
        if self.state.battery_percent < 10.0 {
            return Err("Cannot arm drone: Battery critically low (<10%)".to_string());
        }
        if self.state.flight_mode == FlightMode::Collision {
            return Err("Cannot arm drone: Physical collision lock active".to_string());
        }
        self.state.armed = true;
        self.state.flight_mode = FlightMode::Armed;
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.state.armed = false;
        if self.state.altitude <= 0.2 {
            self.state.flight_mode = FlightMode::Landed;
        } else {
            self.state.flight_mode = FlightMode::Disarmed;
        }
    }

    pub fn takeoff(&mut self, target_altitude: f64) -> Result<(), String> {
        if !self.state.armed {
            return Err("Cannot takeoff: Drone is not armed".to_string());
        }
        self.target_altitude = target_altitude.max(2.0);
        self.state.flight_mode = FlightMode::Takeoff;
        self.state.emergency = false;
        Ok(())
    }

    pub fn land(&mut self) {
        self.target_altitude = 0.0;
        self.state.flight_mode = FlightMode::Landed;
    }

    pub fn return_to_home(&mut self) {
        self.state.return_to_home = true;
        self.state.flight_mode = FlightMode::ReturnToHome;
        // Ascend to safe clearance altitude before traversing horizontally
        self.target_altitude = self.state.altitude.max(self.safe_rth_altitude);
        self.target_position = Some(self.home_position);
    }

    pub fn emergency_land(&mut self) {
        self.state.emergency = true;
        self.state.flight_mode = FlightMode::EmergencyLand;
        self.target_altitude = 0.0;
        self.target_position = None;
    }

    pub fn hover(&mut self) {
        if self.state.armed && self.state.altitude > 0.5 {
            self.state.flight_mode = FlightMode::Hover;
            self.target_position = Some(Vec2::new(self.state.x, self.state.y));
            self.target_altitude = self.state.altitude;
        }
    }

    pub fn set_waypoint(&mut self, wp: Waypoint) {
        self.target_position = Some(Vec2::new(wp.x, wp.y));
        self.target_altitude = wp.altitude;
        self.state.flight_mode = FlightMode::WaypointFollow;
        self.state.current_waypoint_index = Some(wp.id);
    }

    /// Step simulation physics forward by dt seconds with environmental wind.
    pub fn step(&mut self, dt: f64, wind_vector: &Vec2) {
        if !self.state.armed && self.state.flight_mode != FlightMode::Collision {
            // Natural ground rest
            self.state.vx = 0.0;
            self.state.vy = 0.0;
            self.state.vz = 0.0;
            self.state.horizontal_speed = 0.0;
            self.state.vertical_speed = 0.0;
            return;
        }

        self.state.flight_time_seconds += dt;

        // Guidance & Control Logic by flight mode
        match self.state.flight_mode {
            FlightMode::Disarmed | FlightMode::Landed => {
                self.state.vx = 0.0;
                self.state.vy = 0.0;
                self.state.vz = 0.0;
            }
            FlightMode::Armed => {
                // Spinning up on ground
                self.state.vx = 0.0;
                self.state.vy = 0.0;
                self.state.vz = 0.0;
            }
            FlightMode::Takeoff => {
                let alt_err = self.target_altitude - self.state.altitude;
                if alt_err > 0.2 {
                    self.state.vz = (alt_err * 1.5).clamp(0.5, self.config.max_climb_speed);
                } else {
                    self.state.vz = 0.0;
                    self.state.flight_mode = FlightMode::Hover;
                    self.target_position = Some(Vec2::new(self.state.x, self.state.y));
                }
                self.compensate_wind(wind_vector, 0.4);
            }
            FlightMode::Hover => {
                // Maintain position against wind
                if let Some(target) = self.target_position {
                    self.guide_towards(&target, 0.0, dt);
                }
                self.adjust_altitude(self.target_altitude, dt);
                self.compensate_wind(wind_vector, 0.9);
            }
            FlightMode::WaypointFollow => {
                if let Some(target) = self.target_position {
                    let dist = target.distance_to(&Vec2::new(self.state.x, self.state.y));
                    if dist < 1.5 {
                        // Waypoint reached
                        self.state.flight_mode = FlightMode::Hover;
                    } else {
                        self.guide_towards(&target, self.config.max_horizontal_speed, dt);
                    }
                }
                self.adjust_altitude(self.target_altitude, dt);
                self.compensate_wind(wind_vector, 0.8);
            }
            FlightMode::ReturnToHome => {
                let dist_to_home = self.home_position.distance_to(&Vec2::new(self.state.x, self.state.y));
                if self.state.altitude < (self.safe_rth_altitude - 0.5) {
                    // Stage 1: Climb to safe transit altitude first
                    self.adjust_altitude(self.safe_rth_altitude, dt);
                    self.state.vx *= 0.5;
                    self.state.vy *= 0.5;
                } else if dist_to_home > 1.5 {
                    // Stage 2: Transit horizontally towards home
                    let home = self.home_position;
                    let max_speed = self.config.max_horizontal_speed;
                    self.guide_towards(&home, max_speed, dt);
                    self.adjust_altitude(self.safe_rth_altitude, dt);
                } else {
                    // Stage 3: Above home, descend vertically
                    self.state.vx = 0.0;
                    self.state.vy = 0.0;
                    self.state.vz = -self.config.max_descent_speed;
                    if self.state.altitude <= 0.15 {
                        self.state.altitude = 0.0;
                        self.state.vz = 0.0;
                        self.disarm();
                        self.state.return_to_home = false;
                    }
                }
                self.compensate_wind(wind_vector, 0.85);
            }
            FlightMode::EmergencyLand => {
                // Cut horizontal velocity and descend rapidly but safely
                self.state.vx *= 0.85;
                self.state.vy *= 0.85;
                self.state.vz = -1.8;
                if self.state.altitude <= 0.15 {
                    self.state.altitude = 0.0;
                    self.state.vz = 0.0;
                    self.disarm();
                }
            }
            FlightMode::Collision => {
                // Drone has struck an obstacle or boundary
                self.state.vx = 0.0;
                self.state.vy = 0.0;
                self.state.vz = -2.5;
                if self.state.altitude <= 0.05 {
                    self.state.altitude = 0.0;
                    self.state.vz = 0.0;
                    self.state.armed = false;
                }
            }
        }

        // Kinematic integration
        let dx = self.state.vx * dt;
        let dy = self.state.vy * dt;
        let dz = self.state.vz * dt;

        self.state.x += dx;
        self.state.y += dy;
        self.state.altitude = (self.state.altitude + dz).max(0.0);

        let step_dist = (dx * dx + dy * dy).sqrt();
        self.state.distance_traveled += step_dist;
        self.state.horizontal_speed = (self.state.vx.powi(2) + self.state.vy.powi(2)).sqrt();
        self.state.vertical_speed = self.state.vz;

        // Update heading smoothly to track motion vector if moving
        if self.state.horizontal_speed > 0.3 {
            // Heading in degrees: 0 = North (+y), 90 = East (+x)
            let desired_rad = self.state.vx.atan2(self.state.vy);
            let mut desired_deg = desired_rad.to_degrees();
            if desired_deg < 0.0 {
                desired_deg += 360.0;
            }
            let diff = (desired_deg - self.state.heading + 540.0) % 360.0 - 180.0;
            let max_turn = self.config.max_yaw_rate_deg_s * dt;
            self.state.heading = (self.state.heading + diff.clamp(-max_turn, max_turn) + 360.0) % 360.0;
        }

        // Battery drainage model
        self.drain_battery(dt);
    }

    fn guide_towards(&mut self, target: &Vec2, max_speed: f64, _dt: f64) {
        let current = Vec2::new(self.state.x, self.state.y);
        let diff = Vec2::new(target.x - current.x, target.y - current.y);
        let dist = diff.length();

        if dist < 0.2 {
            self.state.vx = 0.0;
            self.state.vy = 0.0;
            return;
        }

        // Proportional velocity controller with saturation
        let speed = (dist * 1.5).clamp(0.5, max_speed.max(2.0));
        let norm = diff.normalized();
        self.state.vx = norm.x * speed;
        self.state.vy = norm.y * speed;
    }

    fn adjust_altitude(&mut self, target_alt: f64, _dt: f64) {
        let err = target_alt - self.state.altitude;
        if err.abs() < 0.1 {
            self.state.vz = 0.0;
        } else if err > 0.0 {
            self.state.vz = (err * 1.2).clamp(0.2, self.config.max_climb_speed);
        } else {
            self.state.vz = -(err.abs() * 1.2).clamp(0.2, self.config.max_descent_speed);
        }
    }

    fn compensate_wind(&mut self, wind: &Vec2, factor: f64) {
        // Wind adds a force; quadcopter tilt compensates by 'factor'
        let drift_x = wind.x * (1.0 - factor);
        let drift_y = wind.y * (1.0 - factor);
        self.state.vx += drift_x;
        self.state.vy += drift_y;
    }

    fn drain_battery(&mut self, dt: f64) {
        if !self.state.armed {
            return;
        }
        // Base consumption: 0.02% per second while armed
        let mut rate = 0.02;

        if self.state.altitude > 0.2 {
            // Hover consumption: 0.05% per second
            rate += 0.05;

            // Speed factor: drag forces consume motor power
            let speed_factor = (self.state.horizontal_speed / self.config.max_horizontal_speed).powi(2) * 0.05;
            let climb_factor = (self.state.vertical_speed.max(0.0) / self.config.max_climb_speed) * 0.04;
            rate += speed_factor + climb_factor;
        }

        self.state.battery_percent = (self.state.battery_percent - rate * dt).max(0.0);
    }
}
