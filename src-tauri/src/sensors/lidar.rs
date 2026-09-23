//! Simulated 16-beam 360-degree LiDAR scanner with geometric ray-obstacle intersection.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use super::{FaultType, SensorReading};
use crate::simulation::Obstacle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarRayHit {
    pub angle_relative_deg: f64,
    pub distance_m: f64,
    pub hit_x: f64,
    pub hit_y: f64,
    pub hit_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarValues {
    pub min_distance_m: f64,
    pub closest_ray_angle_deg: f64,
    pub rays: Vec<LidarRayHit>,
    pub max_range_m: f64,
}

#[derive(Debug, Clone)]
pub struct LidarSensor {
    pub name: String,
    pub num_rays: usize,
    pub max_range_m: f64,
    pub nominal_noise_std_dev: f64,
}

impl Default for LidarSensor {
    fn default() -> Self {
        Self {
            name: "LiDAR-RPLIDAR-S2".to_string(),
            num_rays: 16,
            max_range_m: 35.0,
            nominal_noise_std_dev: 0.05,
        }
    }
}

impl LidarSensor {
    pub fn update(
        &mut self,
        drone_x: f64,
        drone_y: f64,
        drone_alt: f64,
        drone_heading_deg: f64,
        obstacles: &[Obstacle],
        time_sec: f64,
        active_faults: &[FaultType],
        rng: &mut ChaCha8Rng,
    ) -> SensorReading {
        let is_blind_spot = active_faults.contains(&FaultType::LidarBlindSpot);
        let is_excessive_noise = active_faults.contains(&FaultType::ExcessiveNoise);

        let noise_multiplier = if is_excessive_noise { 8.0 } else { 1.0 };
        let current_noise = self.nominal_noise_std_dev * noise_multiplier;

        let angle_step = 360.0 / (self.num_rays as f64);
        let mut ray_hits = Vec::with_capacity(self.num_rays);
        let mut min_dist = self.max_range_m;
        let mut closest_angle = 0.0;

        for i in 0..self.num_rays {
            let rel_angle = (i as f64 * angle_step + 180.0) % 360.0 - 180.0; // [-180, 180)
            let abs_angle = (drone_heading_deg + rel_angle + 360.0) % 360.0;
            let rad = abs_angle.to_radians();
            let ray_dir_x = rad.sin(); // Heading 0 = North (+y), 90 = East (+x)
            let ray_dir_y = rad.cos();

            // Blind spot fault hides obstacles in the forward 90-degree cone (-45 to +45 deg)
            let in_blind_zone = is_blind_spot && rel_angle.abs() <= 50.0;

            let mut closest_hit_dist = self.max_range_m;
            let mut hit_found = false;

            if !in_blind_zone {
                for obs in obstacles {
                    // Check if drone altitude is within obstacle height
                    if drone_alt > obs.height {
                        continue;
                    }
                    if let Some(dist) = ray_circle_intersect(drone_x, drone_y, ray_dir_x, ray_dir_y, obs.x, obs.y, obs.radius) {
                        if dist >= 0.0 && dist < closest_hit_dist {
                            closest_hit_dist = dist;
                            hit_found = true;
                        }
                    }
                }
            }

            // Apply measurement noise
            let measured_dist = if hit_found {
                let n = rng.gen_range(-current_noise..current_noise);
                (closest_hit_dist + n).clamp(0.1, self.max_range_m)
            } else {
                self.max_range_m
            };

            let hit_x = drone_x + ray_dir_x * measured_dist;
            let hit_y = drone_y + ray_dir_y * measured_dist;

            if measured_dist < min_dist {
                min_dist = measured_dist;
                closest_angle = rel_angle;
            }

            ray_hits.push(LidarRayHit {
                angle_relative_deg: (rel_angle * 10.0).round() / 10.0,
                distance_m: (measured_dist * 100.0).round() / 100.0,
                hit_x: (hit_x * 100.0).round() / 100.0,
                hit_y: (hit_y * 100.0).round() / 100.0,
                hit_detected: hit_found,
            });
        }

        let active_fault = if is_blind_spot {
            Some(FaultType::LidarBlindSpot)
        } else if is_excessive_noise {
            Some(FaultType::ExcessiveNoise)
        } else {
            None
        };

        let confidence: f64 = if is_blind_spot {
            0.50
        } else if is_excessive_noise {
            0.75
        } else {
            0.99
        };

        let values = LidarValues {
            min_distance_m: (min_dist * 100.0).round() / 100.0,
            closest_ray_angle_deg: (closest_angle * 10.0).round() / 10.0,
            rays: ray_hits,
            max_range_m: self.max_range_m,
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

/// Computes positive ray-circle intersection distance.
fn ray_circle_intersect(
    ox: f64,
    oy: f64,
    dx: f64,
    dy: f64,
    cx: f64,
    cy: f64,
    radius: f64,
) -> Option<f64> {
    let ex = ox - cx;
    let ey = oy - cy;

    // Quadratic equation: |e + t*d|^2 = r^2 -> t^2 + 2*(e . d)*t + (|e|^2 - r^2) = 0
    let a = dx * dx + dy * dy; // Normalized direction = 1.0
    let b = 2.0 * (ex * dx + ey * dy);
    let c = (ex * ex + ey * ey) - (radius * radius);

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    if t1 >= 0.0 {
        Some(t1)
    } else if t2 >= 0.0 {
        Some(t2)
    } else {
        None
    }
}
