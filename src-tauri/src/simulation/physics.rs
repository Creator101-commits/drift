//! Physics engine and environmental dynamics for drone flight.
//!
//! Units and Conventions:
//! - Coordinates: X = East (m), Y = North (m), Altitude Z = Height above ground (m).
//! - Heading: Degrees [0.0, 360.0), where 0 = North, 90 = East, 180 = South, 270 = West.
//! - Speeds: Horizontal speed in m/s, Vertical speed (climb rate) in m/s.
//! - Timestep: Fixed dt = 0.05 seconds (20 Hz simulation frequency).

use serde::{Deserialize, Serialize};

/// 2D vector for spatial coordinates and velocities.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance_to(&self, other: &Vec2) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self { x: 0.0, y: 0.0 }
        }
    }
}

/// Circular physical obstacle with height clearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub height: f64,
}

impl Obstacle {
    pub fn is_point_inside(&self, x: f64, y: f64, alt: f64, drone_radius: f64) -> bool {
        if alt > self.height {
            return false;
        }
        let dist = ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt();
        dist <= (self.radius + drone_radius)
    }

    pub fn distance_to_point(&self, x: f64, y: f64) -> f64 {
        let center_dist = ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt();
        (center_dist - self.radius).max(0.0)
    }
}

/// Circular restricted airspace (No-Fly Zone).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoFlyZone {
    pub id: String,
    pub name: String,
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub min_alt: f64,
    pub max_alt: f64,
}

impl NoFlyZone {
    pub fn contains(&self, x: f64, y: f64, alt: f64) -> bool {
        if alt < self.min_alt || alt > self.max_alt {
            return false;
        }
        let dist = ((x - self.center_x).powi(2) + (y - self.center_y).powi(2)).sqrt();
        dist <= self.radius
    }
}

/// Scenario boundaries (geofence perimeter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boundary {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub max_altitude: f64,
}

impl Boundary {
    pub fn is_outside(&self, x: f64, y: f64, alt: f64) -> bool {
        x < self.min_x
            || x > self.max_x
            || y < self.min_y
            || y > self.max_y
            || alt > self.max_altitude
    }

    pub fn is_near_boundary(&self, x: f64, y: f64, buffer: f64) -> bool {
        x < (self.min_x + buffer)
            || x > (self.max_x - buffer)
            || y < (self.min_y + buffer)
            || y > (self.max_y - buffer)
    }
}

/// Wind conditions affecting the drone body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindCondition {
    /// Wind speed in meters per second.
    pub speed_mps: f64,
    /// Wind direction in degrees (direction wind is blowing TOWARDS, 0 = North, 90 = East).
    pub direction_deg: f64,
    /// Periodic gust amplitude in meters per second.
    pub gust_amplitude: f64,
    /// Gust period in seconds.
    pub gust_period: f64,
}

impl Default for WindCondition {
    fn default() -> Self {
        Self {
            speed_mps: 2.5,
            direction_deg: 45.0,
            gust_amplitude: 1.0,
            gust_period: 6.0,
        }
    }
}

impl WindCondition {
    /// Calculate instantaneous wind velocity vector (vx, vy) in m/s at time t.
    pub fn velocity_at(&self, time_sec: f64) -> Vec2 {
        let gust = if self.gust_period > 0.0 {
            let phase = 2.0 * std::f64::consts::PI * (time_sec / self.gust_period);
            phase.sin() * self.gust_amplitude
        } else {
            0.0
        };
        let effective_speed = (self.speed_mps + gust).max(0.0);
        let rad = self.direction_deg.to_radians();
        Vec2 {
            x: effective_speed * rad.sin(),
            y: effective_speed * rad.cos(),
        }
    }
}

/// Physical constraints and mass properties of the quadcopter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DronePhysicalConfig {
    pub mass_kg: f64,
    pub max_horizontal_speed: f64,
    pub max_climb_speed: f64,
    pub max_descent_speed: f64,
    pub max_tilt_angle_deg: f64,
    pub max_yaw_rate_deg_s: f64,
    pub collision_radius_m: f64,
    pub battery_capacity_mah: f64,
}

impl Default for DronePhysicalConfig {
    fn default() -> Self {
        Self {
            mass_kg: 1.85,
            max_horizontal_speed: 12.0,
            max_climb_speed: 3.5,
            max_descent_speed: 2.0,
            max_tilt_angle_deg: 35.0,
            max_yaw_rate_deg_s: 90.0,
            collision_radius_m: 0.75,
            battery_capacity_mah: 5200.0,
        }
    }
}
