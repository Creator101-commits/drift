//! Deterministic mission replay engine with variable playback speeds and scrubber controls.

use serde::{Deserialize, Serialize};
use crate::analysis::Alert;
use crate::simulation::engine::MissionEvent;
use crate::simulation::SimulationSnapshot;

/// Current status of the replay playback controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStatus {
    pub is_active: bool,
    pub is_playing: bool,
    pub current_index: usize,
    pub total_frames: usize,
    pub playback_speed: f64,
    pub current_time_sec: f64,
    pub total_duration_sec: f64,
}

impl Default for ReplayStatus {
    fn default() -> Self {
        Self {
            is_active: false,
            is_playing: false,
            current_index: 0,
            total_frames: 0,
            playback_speed: 1.0,
            current_time_sec: 0.0,
            total_duration_sec: 0.0,
        }
    }
}

/// In-memory replay engine delivering recorded frames at calibrated playback rates.
#[derive(Debug, Clone, Default)]
pub struct TrafficReplayer {
    pub snapshots: Vec<SimulationSnapshot>,
    pub events: Vec<MissionEvent>,
    pub alerts: Vec<Alert>,
    pub current_index: usize,
    pub is_active: bool,
    pub is_playing: bool,
    pub playback_speed: f64,
    accumulated_time: f64,
}

impl TrafficReplayer {
    pub fn new() -> Self {
        Self {
            playback_speed: 1.0,
            ..Default::default()
        }
    }

    pub fn load_mission(
        &mut self,
        snapshots: Vec<SimulationSnapshot>,
        events: Vec<MissionEvent>,
        alerts: Vec<Alert>,
    ) {
        self.snapshots = snapshots;
        self.events = events;
        self.alerts = alerts;
        self.current_index = 0;
        self.is_active = !self.snapshots.is_empty();
        self.is_playing = false;
        self.accumulated_time = 0.0;
    }

    pub fn play(&mut self) {
        if self.is_active && !self.snapshots.is_empty() {
            self.is_playing = true;
        }
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        self.play();
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
        self.is_playing = false;
        self.accumulated_time = 0.0;
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.playback_speed = speed.max(0.1);
    }

    pub fn seek(&mut self, index: usize) -> Option<SimulationSnapshot> {
        if self.snapshots.is_empty() {
            return None;
        }
        self.current_index = index.min(self.snapshots.len() - 1);
        Some(self.get_current_snapshot())
    }

    pub fn stop(&mut self) {
        self.is_active = false;
        self.is_playing = false;
        self.current_index = 0;
    }

    pub fn get_status(&self) -> ReplayStatus {
        let total_frames = self.snapshots.len();
        let curr_time = self.snapshots.get(self.current_index).map(|s| s.sim_time_sec).unwrap_or(0.0);
        let duration = self.snapshots.last().map(|s| s.sim_time_sec).unwrap_or(0.0);

        ReplayStatus {
            is_active: self.is_active,
            is_playing: self.is_playing,
            current_index: self.current_index,
            total_frames,
            playback_speed: self.playback_speed,
            current_time_sec: curr_time,
            total_duration_sec: duration,
        }
    }

    pub fn get_current_snapshot(&self) -> SimulationSnapshot {
        if self.snapshots.is_empty() {
            return SimulationSnapshot {
                drone: crate::simulation::DroneState::default(),
                localization: crate::navigation::LocalizationState::default(),
                sensor_readings: Vec::new(),
                safety: crate::navigation::SafetyStatus {
                    collision_detected: false,
                    collision_obstacle_id: None,
                    collision_risk_level: crate::navigation::CollisionRiskLevel::Clear,
                    nearest_obstacle_distance_m: 35.0,
                    geofence_status: crate::navigation::GeofenceStatus::Inside,
                    altitude_limit_exceeded: false,
                },
                active_alerts: Vec::new(),
                recent_events: Vec::new(),
                sim_time_sec: 0.0,
                tick_count: 0,
                planned_route: Vec::new(),
                raw_trail: Vec::new(),
                filtered_trail: Vec::new(),
                active_faults: Vec::new(),
                wind_vector: crate::simulation::Vec2::new(0.0, 0.0),
                is_script_running: false,
                current_script_step: 0,
                total_script_steps: 0,
            };
        }

        let mut snap = self.snapshots[self.current_index].clone();
        let curr_time = snap.sim_time_sec;

        // Filter events up to current replay time
        snap.recent_events = self
            .events
            .iter()
            .filter(|e| {
                e.timestamp
                    .trim_end_matches('s')
                    .parse::<f64>()
                    .map(|t| t <= curr_time)
                    .unwrap_or(true)
            })
            .rev()
            .take(40)
            .cloned()
            .collect();

        // Filter alerts up to current replay time
        snap.active_alerts = self
            .alerts
            .iter()
            .filter(|a| {
                a.timestamp
                    .trim_end_matches('s')
                    .parse::<f64>()
                    .map(|t| t <= curr_time)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        snap
    }

    /// Advance replay state by dt seconds.
    pub fn step(&mut self, dt: f64) -> Option<SimulationSnapshot> {
        if !self.is_active || !self.is_playing || self.snapshots.is_empty() {
            return None;
        }

        // Instant mode (high speed e.g. 50x)
        if self.playback_speed >= 20.0 {
            self.current_index = self.snapshots.len() - 1;
            self.is_playing = false;
            return Some(self.get_current_snapshot());
        }

        // Nominal simulation dt is 0.05s (20 Hz)
        self.accumulated_time += dt * self.playback_speed;
        let frames_to_advance = (self.accumulated_time / 0.05).floor() as usize;

        if frames_to_advance > 0 {
            self.accumulated_time -= frames_to_advance as f64 * 0.05;
            self.current_index += frames_to_advance;

            if self.current_index >= self.snapshots.len() {
                self.current_index = self.snapshots.len() - 1;
                self.is_playing = false;
            }

            Some(self.get_current_snapshot())
        } else {
            None
        }
    }
}
