//! Sensor suite simulation and deterministic fault injection subsystem.

pub mod altimeter;
pub mod compass;
pub mod gps;
pub mod imu;
pub mod lidar;

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

pub use altimeter::AltimeterSensor;
pub use compass::CompassSensor;
pub use gps::GpsSensor;
pub use imu::ImuSensor;
pub use lidar::LidarSensor;

use crate::simulation::Obstacle;

/// Fault modes supported across the sensor suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultType {
    GpsDrift,
    GpsDropout,
    ImuBias,
    CompassFailure,
    AltimeterDrift,
    LidarBlindSpot,
    ExcessiveNoise,
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultType::GpsDrift => write!(f, "GPS Drift"),
            FaultType::GpsDropout => write!(f, "GPS Dropout"),
            FaultType::ImuBias => write!(f, "IMU Bias"),
            FaultType::CompassFailure => write!(f, "Compass Failure"),
            FaultType::AltimeterDrift => write!(f, "Altimeter Drift"),
            FaultType::LidarBlindSpot => write!(f, "LiDAR Blind Spot"),
            FaultType::ExcessiveNoise => write!(f, "Excessive Sensor Noise"),
        }
    }
}

/// Standardized telemetry reading output by every simulated sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_name: String,
    pub timestamp: String,
    pub values: serde_json::Value,
    pub validity: bool,
    pub noise_level: f64,
    pub active_fault: Option<FaultType>,
    pub confidence: f64,
}

/// Complete integrated sensor suite with deterministic PRNG and fault injection.
#[derive(Debug, Clone)]
pub struct SensorSuite {
    pub gps: GpsSensor,
    pub imu: ImuSensor,
    pub compass: CompassSensor,
    pub altimeter: AltimeterSensor,
    pub lidar: LidarSensor,
    pub active_faults: Vec<FaultType>,
}

impl Default for SensorSuite {
    fn default() -> Self {
        Self {
            gps: GpsSensor::default(),
            imu: ImuSensor::default(),
            compass: CompassSensor::default(),
            altimeter: AltimeterSensor::default(),
            lidar: LidarSensor::default(),
            active_faults: Vec::new(),
        }
    }
}

impl SensorSuite {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject_fault(&mut self, fault: FaultType) {
        if !self.active_faults.contains(&fault) {
            self.active_faults.push(fault);
        }
    }

    pub fn clear_fault(&mut self, fault: FaultType) {
        self.active_faults.retain(|&f| f != fault);
    }

    pub fn clear_all_faults(&mut self) {
        self.active_faults.clear();
    }

    pub fn has_fault(&self, fault: FaultType) -> bool {
        self.active_faults.contains(&fault)
    }

    pub fn update(
        &mut self,
        drone_x: f64,
        drone_y: f64,
        drone_alt: f64,
        drone_speed: f64,
        drone_heading: f64,
        drone_vz: f64,
        accel_x: f64,
        accel_y: f64,
        accel_z: f64,
        yaw_rate: f64,
        obstacles: &[Obstacle],
        time_sec: f64,
        dt: f64,
        rng: &mut ChaCha8Rng,
    ) -> Vec<SensorReading> {
        let gps_reading = self.gps.update(
            drone_x,
            drone_y,
            drone_alt,
            drone_speed,
            drone_heading,
            time_sec,
            dt,
            &self.active_faults,
            rng,
        );

        let imu_reading = self.imu.update(
            accel_x,
            accel_y,
            accel_z,
            yaw_rate,
            time_sec,
            &self.active_faults,
            rng,
        );

        let compass_reading = self.compass.update(
            drone_heading,
            time_sec,
            dt,
            &self.active_faults,
            rng,
        );

        let altimeter_reading = self.altimeter.update(
            drone_alt,
            drone_vz,
            time_sec,
            dt,
            &self.active_faults,
            rng,
        );

        let lidar_reading = self.lidar.update(
            drone_x,
            drone_y,
            drone_alt,
            drone_heading,
            obstacles,
            time_sec,
            &self.active_faults,
            rng,
        );

        vec![
            gps_reading,
            imu_reading,
            compass_reading,
            altimeter_reading,
            lidar_reading,
        ]
    }
}
