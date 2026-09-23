//! Safety monitor, collision risk detection, and geofence boundary enforcement.

use serde::{Deserialize, Serialize};
use crate::simulation::{Boundary, DronePhysicalConfig, DroneState, Obstacle};

/// Status of spatial safety checks evaluated each simulation tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStatus {
    pub collision_detected: bool,
    pub collision_obstacle_id: Option<String>,
    pub collision_risk_level: CollisionRiskLevel,
    pub nearest_obstacle_distance_m: f64,
    pub geofence_status: GeofenceStatus,
    pub altitude_limit_exceeded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionRiskLevel {
    Clear,
    Caution,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeofenceStatus {
    Inside,
    WarningBuffer,
    Breached,
}

pub struct SafetyMonitor;

impl SafetyMonitor {
    pub fn evaluate(
        state: &DroneState,
        config: &DronePhysicalConfig,
        boundary: &Boundary,
        obstacles: &[Obstacle],
        lidar_min_dist: f64,
    ) -> SafetyStatus {
        // 1. Check physical collisions with obstacles
        let mut collision = false;
        let mut coll_id = None;

        for obs in obstacles {
            if obs.is_point_inside(state.x, state.y, state.altitude, config.collision_radius_m) {
                collision = true;
                coll_id = Some(obs.id.clone());
                break;
            }
        }

        // 2. Geofence evaluation
        let geofence = if boundary.is_outside(state.x, state.y, state.altitude) {
            GeofenceStatus::Breached
        } else if boundary.is_near_boundary(state.x, state.y, 15.0) {
            GeofenceStatus::WarningBuffer
        } else {
            GeofenceStatus::Inside
        };

        // 3. Collision risk based on LiDAR proximity and speed
        let risk = if lidar_min_dist <= 2.8 {
            CollisionRiskLevel::Critical
        } else if lidar_min_dist <= 8.0 {
            CollisionRiskLevel::Caution
        } else {
            CollisionRiskLevel::Clear
        };

        let altitude_exceeded = state.altitude > boundary.max_altitude;

        SafetyStatus {
            collision_detected: collision,
            collision_obstacle_id: coll_id,
            collision_risk_level: risk,
            nearest_obstacle_distance_m: lidar_min_dist,
            geofence_status: geofence,
            altitude_limit_exceeded: altitude_exceeded,
        }
    }
}
