//! Anomaly detection engine inspecting flight telemetry, sensor fidelity, and spatial safety rules.

use super::alerts::{Alert, AlertManager, AlertSeverity};
use crate::navigation::{CollisionRiskLevel, GeofenceStatus, LocalizationState, SafetyStatus};
use crate::sensors::SensorReading;
use crate::simulation::{Boundary, DroneState, FlightMode, Vec2};

pub struct AnomalyEngine;

impl AnomalyEngine {
    pub fn evaluate(
        alert_mgr: &mut AlertManager,
        drone: &DroneState,
        localization: &LocalizationState,
        sensor_readings: &[SensorReading],
        safety: &SafetyStatus,
        boundary: &Boundary,
        home_pos: &Vec2,
        time_sec: f64,
    ) -> Vec<Alert> {
        let mut new_alerts = Vec::new();
        let ts = format!("{:.2}s", time_sec);

        // 1. Battery Anomaly Checks
        if drone.battery_percent <= 12.0 {
            if let Some(a) = alert_mgr.trigger_alert(
                "LOW_BATTERY_CRITICAL",
                AlertSeverity::Critical,
                &ts,
                &format!("Battery critically depleted ({:.1}%). Emergency landing imminent.", drone.battery_percent),
                "Execute emergency landing immediately or initiate direct RTH.",
                Some("BATTERY_CRITICAL".to_string()),
            ) {
                new_alerts.push(a);
            }
        } else if drone.battery_percent <= 25.0 {
            if let Some(a) = alert_mgr.trigger_alert(
                "LOW_BATTERY_WARNING",
                AlertSeverity::Warning,
                &ts,
                &format!("Battery reserve low ({:.1}%). Plan return to home.", drone.battery_percent),
                "Abort distant waypoints and command return-to-home.",
                Some("BATTERY_LOW".to_string()),
            ) {
                new_alerts.push(a);
            }
        } else {
            alert_mgr.clear_alert_type("LOW_BATTERY_WARNING");
            alert_mgr.clear_alert_type("LOW_BATTERY_CRITICAL");
        }

        // 2. GPS Drift and Sensor Divergence Checks
        if localization.estimation_error_m >= 6.0 {
            if let Some(a) = alert_mgr.trigger_alert(
                "GPS_DRIFT_DETECTED",
                AlertSeverity::Warning,
                &ts,
                &format!("GPS measurement divergent from inertial estimator by {:.1}m.", localization.estimation_error_m),
                "Rely on filtered dead-reckoning estimator; verify satellite geometry.",
                Some("SENSOR_DRIFT".to_string()),
            ) {
                new_alerts.push(a);
            }
        } else if localization.estimation_error_m < 2.5 {
            alert_mgr.clear_alert_type("GPS_DRIFT_DETECTED");
        }

        // 3. Sensor Dropout and Degraded Health
        for reading in sensor_readings {
            if !reading.validity {
                let alert_key = format!("DROPOUT_{}", reading.sensor_name);
                if let Some(a) = alert_mgr.trigger_alert(
                    &alert_key,
                    AlertSeverity::Critical,
                    &ts,
                    &format!("Total signal loss on {}: Fix invalid.", reading.sensor_name),
                    "Switch flight controller to backup navigation sensors.",
                    Some("SENSOR_FAILURE".to_string()),
                ) {
                    new_alerts.push(a);
                }
            } else if reading.confidence < 0.40 {
                let alert_key = format!("DEGRADED_{}", reading.sensor_name);
                if let Some(a) = alert_mgr.trigger_alert(
                    &alert_key,
                    AlertSeverity::Warning,
                    &ts,
                    &format!("Sensor {} confidence degraded to {:.0}%.", reading.sensor_name, reading.confidence * 100.0),
                    "Cross-check secondary telemetry for bias or interference.",
                    Some("SENSOR_DEGRADED".to_string()),
                ) {
                    new_alerts.push(a);
                }
            }
        }

        // 4. Geofence Violations
        match safety.geofence_status {
            GeofenceStatus::Breached => {
                if let Some(a) = alert_mgr.trigger_alert(
                    "GEOFENCE_BREACH",
                    AlertSeverity::Critical,
                    &ts,
                    &format!("Airspace boundary breached at ({:.1}m, {:.1}m).", drone.x, drone.y),
                    "Autonomous fail-safe: Triggering return-to-home or hover stop.",
                    Some("GEOFENCE_BREACH".to_string()),
                ) {
                    new_alerts.push(a);
                }
            }
            GeofenceStatus::WarningBuffer => {
                if let Some(a) = alert_mgr.trigger_alert(
                    "GEOFENCE_WARNING",
                    AlertSeverity::Warning,
                    &ts,
                    &format!("Drone within 15m safety buffer of perimeter boundary."),
                    "Alter course inward to maintain safe clearance.",
                    Some("GEOFENCE_WARNING".to_string()),
                ) {
                    new_alerts.push(a);
                }
            }
            GeofenceStatus::Inside => {
                alert_mgr.clear_alert_type("GEOFENCE_WARNING");
                alert_mgr.clear_alert_type("GEOFENCE_BREACH");
            }
        }

        // 5. Collision Risk & Obstacle Proximity
        if safety.collision_detected {
            if let Some(a) = alert_mgr.trigger_alert(
                "COLLISION_DETECTED",
                AlertSeverity::Critical,
                &ts,
                &format!("Impact detected with obstacle ID {:?}", safety.collision_obstacle_id),
                "Emergency disarm and ground recovery required.",
                Some("COLLISION".to_string()),
            ) {
                new_alerts.push(a);
            }
        } else {
            match safety.collision_risk_level {
                CollisionRiskLevel::Critical => {
                    if let Some(a) = alert_mgr.trigger_alert(
                        "COLLISION_RISK_CRITICAL",
                        AlertSeverity::Critical,
                        &ts,
                        &format!("Obstacle detected within {:.1}m. High collision probability.", safety.nearest_obstacle_distance_m),
                        "Halt forward movement immediately; recalculate detour route.",
                        Some("COLLISION_AVOIDANCE".to_string()),
                    ) {
                        new_alerts.push(a);
                    }
                }
                CollisionRiskLevel::Caution => {
                    if let Some(a) = alert_mgr.trigger_alert(
                        "COLLISION_RISK_CAUTION",
                        AlertSeverity::Warning,
                        &ts,
                        &format!("LiDAR proximity warning: Obstacle at {:.1}m.", safety.nearest_obstacle_distance_m),
                        "Reduce horizontal velocity and scan lateral clearance.",
                        Some("PROXIMITY_CAUTION".to_string()),
                    ) {
                        new_alerts.push(a);
                    }
                }
                CollisionRiskLevel::Clear => {
                    alert_mgr.clear_alert_type("COLLISION_RISK_CAUTION");
                    alert_mgr.clear_alert_type("COLLISION_RISK_CRITICAL");
                }
            }
        }

        // 6. Excessive Altitude
        if safety.altitude_limit_exceeded {
            if let Some(a) = alert_mgr.trigger_alert(
                "EXCESSIVE_ALTITUDE",
                AlertSeverity::Warning,
                &ts,
                &format!("Current altitude ({:.1}m) exceeds ceiling limit ({:.1}m).", drone.altitude, boundary.max_altitude),
                "Command immediate altitude descent to authorized ceiling.",
                Some("AIRSPACE_CEILING".to_string()),
            ) {
                new_alerts.push(a);
            }
        } else {
            alert_mgr.clear_alert_type("EXCESSIVE_ALTITUDE");
        }

        // 7. Failed Return-to-Home Energy Audit
        let dist_to_home = ((drone.x - home_pos.x).powi(2) + (drone.y - home_pos.y).powi(2)).sqrt();
        let est_energy_needed = (dist_to_home / 8.0) * 0.15; // approximate % battery to transit
        if drone.flight_mode != FlightMode::Landed && drone.flight_mode != FlightMode::Disarmed {
            if drone.battery_percent < est_energy_needed + 3.0 && dist_to_home > 30.0 {
                if let Some(a) = alert_mgr.trigger_alert(
                    "FAILED_RTH_ENERGY_DEFICIT",
                    AlertSeverity::Critical,
                    &ts,
                    &format!("Remaining battery ({:.1}%) insufficient for safe RTH transit ({:.1}m required).", drone.battery_percent, dist_to_home),
                    "Abort RTH; perform controlled field landing at current position.",
                    Some("ENERGY_DEFICIT".to_string()),
                ) {
                    new_alerts.push(a);
                }
            }
        }

        // 8. Unexpected Flight-State Changes (e.g. rapid uncommanded descent while hovering)
        if drone.flight_mode == FlightMode::Hover && drone.vz < -1.2 {
            if let Some(a) = alert_mgr.trigger_alert(
                "UNEXPECTED_FLIGHT_STATE",
                AlertSeverity::Warning,
                &ts,
                &format!("Uncommanded descent detected during hover ({:.1} m/s).", drone.vz),
                "Verify motor thrust output and aerodynamic vortex ring state.",
                Some("STABILITY_WARNING".to_string()),
            ) {
                new_alerts.push(a);
            }
        }

        new_alerts
    }
}
