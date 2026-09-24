//! SQLite persistence layer for scenarios, missions, telemetry recordings, and replay metadata.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

use crate::analysis::Alert;
use crate::simulation::engine::MissionEvent;
use crate::simulation::{ScenarioConfig, SimulationSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSummary {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub total_distance: f64,
    pub max_altitude: f64,
    pub battery_consumed: f64,
    pub total_samples: usize,
    pub total_alerts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMissionRecord {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub total_distance: f64,
    pub max_altitude: f64,
    pub battery_consumed: f64,
    pub notes: String,
    pub snapshots: Vec<SimulationSnapshot>,
    pub events: Vec<MissionEvent>,
    pub alerts: Vec<Alert>,
}

pub struct MissionDatabase {
    conn: Mutex<Connection>,
}

impl MissionDatabase {
    pub fn new<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn new_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS scenarios (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                seed INTEGER NOT NULL,
                config_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS missions (
                id TEXT PRIMARY KEY,
                scenario_id TEXT NOT NULL,
                name TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                status TEXT NOT NULL,
                total_distance REAL NOT NULL DEFAULT 0.0,
                max_altitude REAL NOT NULL DEFAULT 0.0,
                battery_consumed REAL NOT NULL DEFAULT 0.0,
                notes TEXT
            );

            CREATE TABLE IF NOT EXISTS telemetry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mission_id TEXT NOT NULL,
                step_index INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                sim_time_sec REAL NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                altitude REAL NOT NULL,
                heading REAL NOT NULL,
                h_speed REAL NOT NULL,
                v_speed REAL NOT NULL,
                battery REAL NOT NULL,
                raw_x REAL NOT NULL,
                raw_y REAL NOT NULL,
                raw_alt REAL NOT NULL,
                raw_heading REAL NOT NULL,
                filtered_x REAL NOT NULL,
                filtered_y REAL NOT NULL,
                filtered_alt REAL NOT NULL,
                filtered_heading REAL NOT NULL,
                flight_mode TEXT NOT NULL,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mission_id TEXT NOT NULL,
                step_index INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                sensor_name TEXT NOT NULL,
                values_json TEXT NOT NULL,
                validity INTEGER NOT NULL,
                noise_level REAL NOT NULL,
                active_fault TEXT,
                confidence REAL NOT NULL,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS faults (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mission_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                fault_type TEXT NOT NULL,
                active INTEGER NOT NULL,
                details TEXT,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS alerts (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                alert_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                description TEXT NOT NULL,
                suggested_action TEXT,
                acknowledged INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS mission_events (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                details_json TEXT,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS replay_metadata (
                mission_id TEXT PRIMARY KEY,
                total_steps INTEGER NOT NULL,
                duration_seconds REAL NOT NULL,
                seed INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );
            "#,
        )?;

        Ok(())
    }

    // --- Scenario Operations ---

    pub fn save_scenario(&self, scenario: &ScenarioConfig) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let config_json = serde_json::to_string(scenario).unwrap_or_default();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO scenarios (id, name, description, seed, config_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                seed = excluded.seed,
                config_json = excluded.config_json;
            "#,
            params![
                scenario.id,
                scenario.name,
                scenario.description,
                scenario.seed as i64,
                config_json,
                created_at,
            ],
        )?;

        Ok(())
    }

    pub fn list_scenarios(&self) -> SqlResult<Vec<ScenarioConfig>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT config_json FROM scenarios ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })?;

        let mut list = Vec::new();
        for r in rows {
            if let Ok(js) = r {
                if let Ok(sc) = serde_json::from_str::<ScenarioConfig>(&js) {
                    list.push(sc);
                }
            }
        }
        Ok(list)
    }

    // --- Mission Save & Browse ---

    pub fn save_mission(&self, record: &SavedMissionRecord) -> SqlResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO missions (id, scenario_id, name, start_time, end_time, status, total_distance, max_altitude, battery_consumed, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                end_time = excluded.end_time,
                status = excluded.status,
                total_distance = excluded.total_distance,
                max_altitude = excluded.max_altitude,
                battery_consumed = excluded.battery_consumed,
                notes = excluded.notes;
            "#,
            params![
                record.id,
                record.scenario_id,
                record.name,
                record.start_time,
                record.end_time,
                record.status,
                record.total_distance,
                record.max_altitude,
                record.battery_consumed,
                record.notes,
            ],
        )?;

        // Telemetry batch insert
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO telemetry (
                    mission_id, step_index, timestamp, sim_time_sec,
                    x, y, altitude, heading, h_speed, v_speed, battery,
                    raw_x, raw_y, raw_alt, raw_heading,
                    filtered_x, filtered_y, filtered_alt, filtered_heading,
                    flight_mode
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                "#,
            )?;

            for (idx, snap) in record.snapshots.iter().enumerate() {
                stmt.execute(params![
                    record.id,
                    idx as i64,
                    format!("{:.2}s", snap.sim_time_sec),
                    snap.sim_time_sec,
                    snap.drone.x,
                    snap.drone.y,
                    snap.drone.altitude,
                    snap.drone.heading,
                    snap.drone.horizontal_speed,
                    snap.drone.vertical_speed,
                    snap.drone.battery_percent,
                    snap.localization.raw_x,
                    snap.localization.raw_y,
                    snap.localization.raw_altitude,
                    snap.localization.raw_heading,
                    snap.localization.filtered_x,
                    snap.localization.filtered_y,
                    snap.localization.filtered_altitude,
                    snap.localization.filtered_heading,
                    format!("{:?}", snap.drone.flight_mode),
                ])?;
            }
        }

        // Alerts insert
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR REPLACE INTO alerts (id, mission_id, timestamp, alert_type, severity, description, suggested_action, acknowledged)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )?;

            for a in &record.alerts {
                stmt.execute(params![
                    a.id,
                    record.id,
                    a.timestamp,
                    a.alert_type,
                    format!("{:?}", a.severity),
                    a.description,
                    a.suggested_action,
                    if a.acknowledged { 1 } else { 0 },
                ])?;
            }
        }

        // Mission events insert
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR REPLACE INTO mission_events (id, mission_id, timestamp, event_type, description, details_json)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;

            for e in &record.events {
                let det = e.details.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
                stmt.execute(params![
                    e.id,
                    record.id,
                    e.timestamp,
                    e.event_type,
                    e.description,
                    det,
                ])?;
            }
        }

        // Replay metadata insert
        let duration = record.snapshots.last().map(|s| s.sim_time_sec).unwrap_or(0.0);
        tx.execute(
            r#"
            INSERT OR REPLACE INTO replay_metadata (mission_id, total_steps, duration_seconds, seed, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                record.id,
                record.snapshots.len() as i64,
                duration,
                42 as i64,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_missions(&self) -> SqlResult<Vec<MissionSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                m.id, m.scenario_id, m.name, m.start_time, COALESCE(m.end_time, ''),
                m.status, m.total_distance, m.max_altitude, m.battery_consumed,
                (SELECT COUNT(*) FROM telemetry t WHERE t.mission_id = m.id) as sample_count,
                (SELECT COUNT(*) FROM alerts a WHERE a.mission_id = m.id) as alert_count
            FROM missions m
            ORDER BY m.start_time DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(MissionSummary {
                id: row.get(0)?,
                scenario_id: row.get(1)?,
                name: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                status: row.get(5)?,
                total_distance: row.get(6)?,
                max_altitude: row.get(7)?,
                battery_consumed: row.get(8)?,
                total_samples: row.get::<_, i64>(9)? as usize,
                total_alerts: row.get::<_, i64>(10)? as usize,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_mission(&self, mission_id: &str) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        // foreign keys cascade automatically
        let deleted = conn.execute("DELETE FROM missions WHERE id = ?1", params![mission_id])?;
        Ok(deleted > 0)
    }

    pub fn get_mission_snapshots(&self, mission_id: &str) -> SqlResult<Vec<SimulationSnapshot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                step_index, sim_time_sec,
                x, y, altitude, heading, h_speed, v_speed, battery,
                raw_x, raw_y, raw_alt, raw_heading,
                filtered_x, filtered_y, filtered_alt, filtered_heading,
                flight_mode
            FROM telemetry
            WHERE mission_id = ?1
            ORDER BY step_index ASC
            "#,
        )?;

        let rows = stmt.query_map(params![mission_id], |row| {
            let sim_time_sec: f64 = row.get(1)?;
            let x: f64 = row.get(2)?;
            let y: f64 = row.get(3)?;
            let alt: f64 = row.get(4)?;
            let head: f64 = row.get(5)?;
            let h_speed: f64 = row.get(6)?;
            let v_speed: f64 = row.get(7)?;
            let battery: f64 = row.get(8)?;

            let raw_x: f64 = row.get(9)?;
            let raw_y: f64 = row.get(10)?;
            let raw_alt: f64 = row.get(11)?;
            let raw_head: f64 = row.get(12)?;

            let fil_x: f64 = row.get(13)?;
            let fil_y: f64 = row.get(14)?;
            let fil_alt: f64 = row.get(15)?;
            let fil_head: f64 = row.get(16)?;

            let mode_str: String = row.get(17)?;

            let mode = match mode_str.as_str() {
                "Takeoff" => crate::simulation::FlightMode::Takeoff,
                "Hover" => crate::simulation::FlightMode::Hover,
                "WaypointFollow" => crate::simulation::FlightMode::WaypointFollow,
                "ReturnToHome" => crate::simulation::FlightMode::ReturnToHome,
                "EmergencyLand" => crate::simulation::FlightMode::EmergencyLand,
                "Landed" => crate::simulation::FlightMode::Landed,
                "Collision" => crate::simulation::FlightMode::Collision,
                _ => crate::simulation::FlightMode::Armed,
            };

            let drone = crate::simulation::DroneState {
                x,
                y,
                altitude: alt,
                heading: head,
                horizontal_speed: h_speed,
                vertical_speed: v_speed,
                vx: 0.0,
                vy: 0.0,
                vz: v_speed,
                battery_percent: battery,
                flight_mode: mode,
                armed: true,
                current_waypoint_index: None,
                return_to_home: false,
                emergency: false,
                distance_traveled: 0.0,
                flight_time_seconds: sim_time_sec,
            };

            let localization = crate::navigation::LocalizationState {
                raw_x,
                raw_y,
                raw_altitude: raw_alt,
                raw_heading: raw_head,
                filtered_x: fil_x,
                filtered_y: fil_y,
                filtered_altitude: fil_alt,
                filtered_heading: fil_head,
                estimated_vx: 0.0,
                estimated_vy: 0.0,
                estimated_vz: 0.0,
                estimation_error_m: ((raw_x - fil_x).powi(2) + (raw_y - fil_y).powi(2)).sqrt(),
            };

            let safety = crate::navigation::SafetyStatus {
                collision_detected: false,
                collision_obstacle_id: None,
                collision_risk_level: crate::navigation::CollisionRiskLevel::Clear,
                nearest_obstacle_distance_m: 35.0,
                geofence_status: crate::navigation::GeofenceStatus::Inside,
                altitude_limit_exceeded: false,
            };

            Ok(SimulationSnapshot {
                drone,
                localization,
                sensor_readings: Vec::new(),
                safety,
                active_alerts: Vec::new(),
                recent_events: Vec::new(),
                sim_time_sec,
                tick_count: row.get::<_, i64>(0)? as u64,
                planned_route: Vec::new(),
                raw_trail: Vec::new(),
                filtered_trail: Vec::new(),
                active_faults: Vec::new(),
                wind_vector: crate::simulation::Vec2::new(0.0, 0.0),
                is_script_running: false,
                current_script_step: 0,
                total_script_steps: 0,
            })
        })?;

        let mut snaps = Vec::new();
        for r in rows {
            snaps.push(r?);
        }
        Ok(snaps)
    }
}
