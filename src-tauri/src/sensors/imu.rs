//! Simulated 6-DOF Inertial Measurement Unit (IMU) with accelerometer and gyroscope models.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use super::{FaultType, SensorReading};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuValues {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub gyro_x: f64,
    pub gyro_y: f64,
    pub gyro_z: f64,
    pub temperature_c: f64,
}

#[derive(Debug, Clone)]
pub struct ImuSensor {
    pub name: String,
    pub nominal_noise_std_dev: f64,
    pub bias_accel_x: f64,
    pub bias_accel_y: f64,
    pub bias_gyro_z: f64,
}

impl Default for ImuSensor {
    fn default() -> Self {
        Self {
            name: "IMU-BMI088".to_string(),
            nominal_noise_std_dev: 0.08,
            bias_accel_x: 0.0,
            bias_accel_y: 0.0,
            bias_gyro_z: 0.0,
        }
    }
}

impl ImuSensor {
    pub fn update(
        &mut self,
        accel_x_true: f64,
        accel_y_true: f64,
        accel_z_true: f64,
        yaw_rate_true: f64,
        time_sec: f64,
        active_faults: &[FaultType],
        rng: &mut ChaCha8Rng,
    ) -> SensorReading {
        let is_bias = active_faults.contains(&FaultType::ImuBias);
        let is_excessive_noise = active_faults.contains(&FaultType::ExcessiveNoise);

        let noise_multiplier = if is_excessive_noise { 6.0 } else { 1.0 };
        let current_noise = self.nominal_noise_std_dev * noise_multiplier;

        if is_bias {
            self.bias_accel_x = 1.45;
            self.bias_accel_y = -0.92;
            self.bias_gyro_z = 7.8;
        } else {
            self.bias_accel_x *= 0.9;
            self.bias_accel_y *= 0.9;
            self.bias_gyro_z *= 0.9;
        }

        let n_ax = rng.gen_range(-current_noise..current_noise);
        let n_ay = rng.gen_range(-current_noise..current_noise);
        let n_az = rng.gen_range(-current_noise..current_noise);
        let n_gz = rng.gen_range(-0.2..0.2) * noise_multiplier;

        // In NED/local frame, hovering accelerometer senses +9.81 m/s^2 upwards normal reaction
        let gravity = 9.80665;

        let measured_ax = accel_x_true + self.bias_accel_x + n_ax;
        let measured_ay = accel_y_true + self.bias_accel_y + n_ay;
        let measured_az = accel_z_true + gravity + n_az;

        let measured_gx = rng.gen_range(-0.1..0.1) * noise_multiplier;
        let measured_gy = rng.gen_range(-0.1..0.1) * noise_multiplier;
        let measured_gz = yaw_rate_true + self.bias_gyro_z + n_gz;

        let active_fault = if is_bias {
            Some(FaultType::ImuBias)
        } else if is_excessive_noise {
            Some(FaultType::ExcessiveNoise)
        } else {
            None
        };

        let confidence: f64 = if is_bias {
            0.45
        } else if is_excessive_noise {
            0.70
        } else {
            0.99
        };

        let values = ImuValues {
            accel_x: (measured_ax * 1000.0).round() / 1000.0,
            accel_y: (measured_ay * 1000.0).round() / 1000.0,
            accel_z: (measured_az * 1000.0).round() / 1000.0,
            gyro_x: (measured_gx * 100.0).round() / 100.0,
            gyro_y: (measured_gy * 100.0).round() / 100.0,
            gyro_z: (measured_gz * 100.0).round() / 100.0,
            temperature_c: 38.4,
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
