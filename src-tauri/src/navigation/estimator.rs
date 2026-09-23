//! Filtered localization system combining GPS, IMU, Compass, Altimeter, and kinematic motion state.
//!
//! Provides simultaneous tracking of both raw sensor telemetry and filtered state estimation.

use serde::{Deserialize, Serialize};
use crate::sensors::gps::GpsValues;
use crate::sensors::imu::ImuValues;
use crate::sensors::SensorReading;

/// Localization output exposing both raw sensor measurements and filtered state estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationState {
    /// Raw uncorrected GPS position X (m)
    pub raw_x: f64,
    /// Raw uncorrected GPS position Y (m)
    pub raw_y: f64,
    /// Raw barometric altitude (m)
    pub raw_altitude: f64,
    /// Raw compass heading (deg)
    pub raw_heading: f64,
    /// Filtered estimated position X (m)
    pub filtered_x: f64,
    /// Filtered estimated position Y (m)
    pub filtered_y: f64,
    /// Filtered estimated altitude (m)
    pub filtered_altitude: f64,
    /// Filtered estimated heading (deg)
    pub filtered_heading: f64,
    /// Estimated velocity X (m/s)
    pub estimated_vx: f64,
    /// Estimated velocity Y (m/s)
    pub estimated_vy: f64,
    /// Estimated vertical velocity (m/s)
    pub estimated_vz: f64,
    /// Spatial divergence between raw GPS and filtered estimate (m)
    pub estimation_error_m: f64,
}

impl Default for LocalizationState {
    fn default() -> Self {
        Self {
            raw_x: 50.0,
            raw_y: 50.0,
            raw_altitude: 0.0,
            raw_heading: 0.0,
            filtered_x: 50.0,
            filtered_y: 50.0,
            filtered_altitude: 0.0,
            filtered_heading: 0.0,
            estimated_vx: 0.0,
            estimated_vy: 0.0,
            estimated_vz: 0.0,
            estimation_error_m: 0.0,
        }
    }
}

/// Point on historical trajectory trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    pub altitude: f64,
}

/// Extended Complementary / Kalman-style Filter Estimator.
#[derive(Debug, Clone)]
pub struct StateEstimator {
    pub state: LocalizationState,
    pub raw_trail: Vec<PathPoint>,
    pub filtered_trail: Vec<PathPoint>,
    pub max_trail_len: usize,
    sample_counter: usize,
}

impl Default for StateEstimator {
    fn default() -> Self {
        Self {
            state: LocalizationState::default(),
            raw_trail: Vec::with_capacity(1200),
            filtered_trail: Vec::with_capacity(1200),
            max_trail_len: 1000,
            sample_counter: 0,
        }
    }
}

impl StateEstimator {
    pub fn new(init_x: f64, init_y: f64) -> Self {
        let mut est = Self::default();
        est.state.raw_x = init_x;
        est.state.raw_y = init_y;
        est.state.filtered_x = init_x;
        est.state.filtered_y = init_y;
        est
    }

    pub fn reset(&mut self, init_x: f64, init_y: f64) {
        self.state = LocalizationState::default();
        self.state.raw_x = init_x;
        self.state.raw_y = init_y;
        self.state.filtered_x = init_x;
        self.state.filtered_y = init_y;
        self.raw_trail.clear();
        self.filtered_trail.clear();
        self.sample_counter = 0;
    }

    /// Update filter with latest sensor suite readings and timestep dt.
    pub fn update(
        &mut self,
        readings: &[SensorReading],
        dt: f64,
    ) {
        let mut gps_val: Option<GpsValues> = None;
        let mut gps_valid = false;
        let mut gps_conf = 0.0;

        let mut imu_val: Option<ImuValues> = None;
        let mut baro_alt = self.state.filtered_altitude;
        let mut compass_head = self.state.filtered_heading;

        for r in readings {
            match r.sensor_name.as_str() {
                s if s.starts_with("GPS") => {
                    gps_valid = r.validity;
                    gps_conf = r.confidence;
                    if let Ok(v) = serde_json::from_value::<GpsValues>(r.values.clone()) {
                        gps_val = Some(v);
                    }
                }
                s if s.starts_with("IMU") => {
                    if let Ok(v) = serde_json::from_value::<ImuValues>(r.values.clone()) {
                        imu_val = Some(v);
                    }
                }
                s if s.starts_with("Compass") => {
                    if let Some(h) = r.values.get("heading_deg").and_then(|v| v.as_f64()) {
                        compass_head = h;
                    }
                }
                s if s.starts_with("Baro") => {
                    if let Some(a) = r.values.get("altitude_m").and_then(|v| v.as_f64()) {
                        baro_alt = a;
                    }
                }
                _ => {}
            }
        }

        // 1. Extract Raw Sensor Positions
        if let Some(ref gps) = gps_val {
            if gps_valid {
                self.state.raw_x = gps.x;
                self.state.raw_y = gps.y;
            }
        }
        self.state.raw_altitude = baro_alt;
        self.state.raw_heading = compass_head;

        // 2. Filter Prediction Step (Inertial dead-reckoning using IMU)
        let mut pred_vx = self.state.estimated_vx;
        let mut pred_vy = self.state.estimated_vy;
        let mut pred_vz = self.state.estimated_vz;
        let pred_heading = self.state.filtered_heading;

        if let Some(ref imu) = imu_val {
            // Transform body acceleration to world frame using heading
            let rad = pred_heading.to_radians();
            let cos_h = rad.cos();
            let sin_h = rad.sin();

            // Heading 0 = North (+y), 90 = East (+x)
            let world_ax = imu.accel_x * cos_h + imu.accel_y * sin_h;
            let world_ay = -imu.accel_x * sin_h + imu.accel_y * cos_h;
            let world_az = imu.accel_z - 9.80665;

            // Velocity integration with dampening
            pred_vx = (pred_vx + world_ax * dt) * 0.985;
            pred_vy = (pred_vy + world_ay * dt) * 0.985;
            pred_vz = (pred_vz + world_az * dt) * 0.98;
        }

        let mut pred_x = self.state.filtered_x + pred_vx * dt;
        let mut pred_y = self.state.filtered_y + pred_vy * dt;
        let pred_alt = (self.state.filtered_altitude + pred_vz * dt).max(0.0);

        // 3. Filter Measurement Update Step (GPS & Altimeter Fusion)
        if gps_valid {
            if let Some(ref gps) = gps_val {
                let dx = gps.x - pred_x;
                let dy = gps.y - pred_y;
                let innovation_dist = (dx * dx + dy * dy).sqrt();

                // Innovation gating: reject extreme jumps if confidence is degraded
                let gain = if innovation_dist > 12.0 {
                    0.05 * gps_conf // heavily damped when diverging
                } else {
                    0.20 * gps_conf
                };

                pred_x += dx * gain;
                pred_y += dy * gain;

                // Adjust velocity estimates towards GPS measurements
                let vel_gain = 0.12 * gps_conf;
                pred_vx += (gps.horizontal_speed * (gps.ground_course.to_radians()).sin() - pred_vx) * vel_gain;
                pred_vy += (gps.horizontal_speed * (gps.ground_course.to_radians()).cos() - pred_vy) * vel_gain;
            }
        }

        // Altimeter fusion
        let alt_gain = 0.35;
        let est_alt = pred_alt + (baro_alt - pred_alt) * alt_gain;

        // Heading fusion
        let head_diff = (compass_head - pred_heading + 540.0) % 360.0 - 180.0;
        let est_head = (pred_heading + head_diff * 0.20 + 360.0) % 360.0;

        self.state.filtered_x = (pred_x * 100.0).round() / 100.0;
        self.state.filtered_y = (pred_y * 100.0).round() / 100.0;
        self.state.filtered_altitude = (est_alt * 100.0).round() / 100.0;
        self.state.filtered_heading = (est_head * 10.0).round() / 10.0;
        self.state.estimated_vx = (pred_vx * 100.0).round() / 100.0;
        self.state.estimated_vy = (pred_vy * 100.0).round() / 100.0;
        self.state.estimated_vz = (pred_vz * 100.0).round() / 100.0;

        let err_dx = self.state.raw_x - self.state.filtered_x;
        let err_dy = self.state.raw_y - self.state.filtered_y;
        self.state.estimation_error_m = ((err_dx * err_dx + err_dy * err_dy).sqrt() * 100.0).round() / 100.0;

        // 4. Record breadcrumb trail for UI visualization (subsampled every 4 ticks = 5 Hz)
        self.sample_counter += 1;
        if self.sample_counter % 4 == 0 {
            if self.raw_trail.len() >= self.max_trail_len {
                self.raw_trail.remove(0);
            }
            self.raw_trail.push(PathPoint {
                x: self.state.raw_x,
                y: self.state.raw_y,
                altitude: self.state.raw_altitude,
            });

            if self.filtered_trail.len() >= self.max_trail_len {
                self.filtered_trail.remove(0);
            }
            self.filtered_trail.push(PathPoint {
                x: self.state.filtered_x,
                y: self.state.filtered_y,
                altitude: self.state.filtered_altitude,
            });
        }
    }
}
