//! Simulated Global Positioning System (GPS) sensor with deterministic noise and fault modes.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use super::{FaultType, SensorReading};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsValues {
    pub x: f64,
    pub y: f64,
    pub altitude: f64,
    pub horizontal_speed: f64,
    pub ground_course: f64,
    pub hdop: f64,
    pub satellites: u32,
    pub fix_quality: String,
}

#[derive(Debug, Clone)]
pub struct GpsSensor {
    pub name: String,
    pub nominal_noise_std_dev: f64,
    pub drift_offset_x: f64,
    pub drift_offset_y: f64,
}

impl Default for GpsSensor {
    fn default() -> Self {
        Self {
            name: "GPS-Neo-M9N".to_string(),
            nominal_noise_std_dev: 0.45,
            drift_offset_x: 0.0,
            drift_offset_y: 0.0,
        }
    }
}

impl GpsSensor {
    pub fn update(
        &mut self,
        true_x: f64,
        true_y: f64,
        true_alt: f64,
        true_speed: f64,
        true_heading: f64,
        time_sec: f64,
        dt: f64,
        active_faults: &[FaultType],
        rng: &mut ChaCha8Rng,
    ) -> SensorReading {
        let is_dropout = active_faults.contains(&FaultType::GpsDropout);
        let is_drift = active_faults.contains(&FaultType::GpsDrift);
        let is_excessive_noise = active_faults.contains(&FaultType::ExcessiveNoise);

        let noise_multiplier = if is_excessive_noise { 6.5 } else { 1.0 };
        let current_noise = self.nominal_noise_std_dev * noise_multiplier;

        if is_dropout {
            let values = GpsValues {
                x: 0.0,
                y: 0.0,
                altitude: 0.0,
                horizontal_speed: 0.0,
                ground_course: 0.0,
                hdop: 99.9,
                satellites: 2,
                fix_quality: "NO_FIX".to_string(),
            };
            return SensorReading {
                sensor_name: self.name.clone(),
                timestamp: format!("{:.2}s", time_sec),
                values: serde_json::to_value(values).unwrap_or_default(),
                validity: false,
                noise_level: current_noise,
                active_fault: Some(FaultType::GpsDropout),
                confidence: 0.0,
            };
        }

        if is_drift {
            // Accumulate gradual drift at approx 0.8 m/s along 35 deg angle
            self.drift_offset_x += 0.55 * dt;
            self.drift_offset_y += 0.40 * dt;
        } else {
            // Slowly decay drift back to zero if fault was cleared
            self.drift_offset_x *= 0.95;
            self.drift_offset_y *= 0.95;
        }

        // Gaussian noise via Box-Muller transform
        let u1: f64 = rng.gen_range(1e-6..1.0);
        let u2: f64 = rng.gen_range(0.0..std::f64::consts::TAU);
        let mag = (-2.0 * u1.ln()).sqrt() * current_noise;
        let noise_x = mag * u2.cos();
        let noise_y = mag * u2.sin();
        let noise_z = rng.gen_range(-0.3..0.3) * noise_multiplier;

        let measured_x = true_x + self.drift_offset_x + noise_x;
        let measured_y = true_y + self.drift_offset_y + noise_y;
        let measured_alt = (true_alt + noise_z).max(0.0);
        let measured_speed = (true_speed + rng.gen_range(-0.1..0.1) * noise_multiplier).max(0.0);
        let measured_course = (true_heading + rng.gen_range(-1.5..1.5) * noise_multiplier + 360.0) % 360.0;

        let active_fault = if is_drift {
            Some(FaultType::GpsDrift)
        } else if is_excessive_noise {
            Some(FaultType::ExcessiveNoise)
        } else {
            None
        };

        let hdop = if is_excessive_noise { 3.8 } else { 0.95 };
        let sats = if is_excessive_noise { 7 } else { 16 };
        let confidence = if is_drift {
            let drift_dist = (self.drift_offset_x.powi(2) + self.drift_offset_y.powi(2)).sqrt();
            (1.0 - (drift_dist / 25.0)).clamp(0.2, 0.98)
        } else if is_excessive_noise {
            0.65
        } else {
            0.98
        };

        let values = GpsValues {
            x: (measured_x * 100.0).round() / 100.0,
            y: (measured_y * 100.0).round() / 100.0,
            altitude: (measured_alt * 100.0).round() / 100.0,
            horizontal_speed: (measured_speed * 10.0).round() / 10.0,
            ground_course: (measured_course * 10.0).round() / 10.0,
            hdop,
            satellites: sats,
            fix_quality: "3D_RTK_FLOAT".to_string(),
        };

        SensorReading {
            sensor_name: self.name.clone(),
            timestamp: format!("{:.2}s", time_sec),
            values: serde_json::to_value(values).unwrap_or_default(),
            validity: true,
            noise_level: (current_noise * 100.0).round() / 100.0,
            active_fault,
            confidence: (confidence * 100.0).round() / 100.0,
        }
    }
}
