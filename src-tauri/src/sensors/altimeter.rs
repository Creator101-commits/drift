//! Simulated Barometric Altimeter and vertical velocity estimation.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use super::{FaultType, SensorReading};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltimeterValues {
    pub altitude_m: f64,
    pub pressure_hpa: f64,
    pub climb_rate_mps: f64,
    pub temperature_c: f64,
}

#[derive(Debug, Clone)]
pub struct AltimeterSensor {
    pub name: String,
    pub nominal_noise_std_dev: f64,
    pub drift_bias_m: f64,
    pub sea_level_pressure_hpa: f64,
}

impl Default for AltimeterSensor {
    fn default() -> Self {
        Self {
            name: "Baro-DPS310".to_string(),
            nominal_noise_std_dev: 0.12,
            drift_bias_m: 0.0,
            sea_level_pressure_hpa: 1013.25,
        }
    }
}

impl AltimeterSensor {
    pub fn update(
        &mut self,
        true_alt: f64,
        true_vz: f64,
        time_sec: f64,
        dt: f64,
        active_faults: &[FaultType],
        rng: &mut ChaCha8Rng,
    ) -> SensorReading {
        let is_drift = active_faults.contains(&FaultType::AltimeterDrift);
        let is_excessive_noise = active_faults.contains(&FaultType::ExcessiveNoise);

        let noise_multiplier = if is_excessive_noise { 6.0 } else { 1.0 };
        let current_noise = self.nominal_noise_std_dev * noise_multiplier;

        if is_drift {
            self.drift_bias_m += 0.35 * dt;
        } else {
            self.drift_bias_m *= 0.95;
        }

        let n_alt = rng.gen_range(-current_noise..current_noise);
        let measured_alt = (true_alt + self.drift_bias_m + n_alt).max(0.0);
        let measured_vz = true_vz + rng.gen_range(-0.05..0.05) * noise_multiplier;

        // Barometric formula: P = P0 * (1 - 2.25577e-5 * h)^5.25588
        let pressure = self.sea_level_pressure_hpa * (1.0 - 2.25577e-5 * measured_alt).powf(5.25588);

        let active_fault = if is_drift {
            Some(FaultType::AltimeterDrift)
        } else if is_excessive_noise {
            Some(FaultType::ExcessiveNoise)
        } else {
            None
        };

        let confidence = if is_drift {
            (1.0 - (self.drift_bias_m.abs() / 15.0)).clamp(0.3, 0.99)
        } else if is_excessive_noise {
            0.70
        } else {
            0.99
        };

        let values = AltimeterValues {
            altitude_m: (measured_alt * 100.0).round() / 100.0,
            pressure_hpa: (pressure * 100.0).round() / 100.0,
            climb_rate_mps: (measured_vz * 100.0).round() / 100.0,
            temperature_c: 21.5,
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
