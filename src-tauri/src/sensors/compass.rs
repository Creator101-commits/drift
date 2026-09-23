//! Simulated 3-axis magnetometer / digital compass with heading computation.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use super::{FaultType, SensorReading};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompassValues {
    pub heading_deg: f64,
    pub mag_x_ut: f64,
    pub mag_y_ut: f64,
    pub mag_z_ut: f64,
    pub field_strength_ut: f64,
    pub calibration_level: u8,
}

#[derive(Debug, Clone)]
pub struct CompassSensor {
    pub name: String,
    pub nominal_noise_std_dev: f64,
    pub spin_offset_deg: f64,
}

impl Default for CompassSensor {
    fn default() -> Self {
        Self {
            name: "Compass-BMM150".to_string(),
            nominal_noise_std_dev: 0.8,
            spin_offset_deg: 0.0,
        }
    }
}

impl CompassSensor {
    pub fn update(
        &mut self,
        true_heading: f64,
        time_sec: f64,
        dt: f64,
        active_faults: &[FaultType],
        rng: &mut ChaCha8Rng,
    ) -> SensorReading {
        let is_failed = active_faults.contains(&FaultType::CompassFailure);
        let is_excessive_noise = active_faults.contains(&FaultType::ExcessiveNoise);

        let noise_multiplier = if is_excessive_noise { 8.0 } else { 1.0 };
        let current_noise = self.nominal_noise_std_dev * noise_multiplier;

        let (measured_heading, validity, confidence, cal_level): (f64, bool, f64, u8) = if is_failed {
            // Spinning failure mode
            self.spin_offset_deg = (self.spin_offset_deg + 45.0 * dt) % 360.0;
            let val = (true_heading + self.spin_offset_deg + 180.0) % 360.0;
            (val, false, 0.15, 0)
        } else {
            self.spin_offset_deg = 0.0;
            let n = rng.gen_range(-current_noise..current_noise);
            let val = (true_heading + n + 360.0) % 360.0;
            let conf = if is_excessive_noise { 0.60 } else { 0.98 };
            (val, true, conf, 3)
        };

        let rad = measured_heading.to_radians();
        let field_total = 48.5; // Earth's ambient field in microteslas
        let mag_x = field_total * rad.cos();
        let mag_y = field_total * rad.sin();
        let mag_z = 32.0;

        let active_fault = if is_failed {
            Some(FaultType::CompassFailure)
        } else if is_excessive_noise {
            Some(FaultType::ExcessiveNoise)
        } else {
            None
        };

        let values = CompassValues {
            heading_deg: (measured_heading * 10.0).round() / 10.0,
            mag_x_ut: (mag_x * 10.0).round() / 10.0,
            mag_y_ut: (mag_y * 10.0).round() / 10.0,
            mag_z_ut: mag_z,
            field_strength_ut: field_total,
            calibration_level: cal_level,
        };

        SensorReading {
            sensor_name: self.name.clone(),
            timestamp: format!("{:.2}s", time_sec),
            values: serde_json::to_value(values).unwrap_or_default(),
            validity,
            noise_level: (current_noise * 10.0).round() / 10.0,
            active_fault,
            confidence: (confidence * 100.0).round() / 100.0,
        }
    }
}
