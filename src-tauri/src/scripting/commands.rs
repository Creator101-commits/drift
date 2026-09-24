//! Mission script interpreter and automated command execution runner.

use serde::{Deserialize, Serialize};

/// Discrete mission commands executable by the autonomous script runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MissionCommand {
    Takeoff { altitude: f64 },
    Goto { x: f64, y: f64, altitude: f64 },
    Wait { seconds: f64 },
    InjectFault { fault_name: String },
    ClearFaults,
    ReturnHome,
    Land,
    EmergencyLand,
}

/// Parses a line-by-line script into structured mission commands.
pub fn parse_mission_script(script: &str) -> Result<Vec<MissionCommand>, String> {
    let mut commands = Vec::new();

    for (line_idx, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd_name = parts[0].to_uppercase();

        let cmd = match cmd_name.as_str() {
            "TAKEOFF" => {
                let alt = if parts.len() > 1 {
                    parts[1].parse::<f64>().map_err(|_| format!("Line {}: invalid altitude", line_idx + 1))?
                } else {
                    15.0
                };
                MissionCommand::Takeoff { altitude: alt }
            }
            "GOTO" => {
                if parts.len() < 4 {
                    return Err(format!("Line {}: GOTO requires x, y, altitude arguments", line_idx + 1));
                }
                let x = parts[1].parse::<f64>().map_err(|_| format!("Line {}: invalid x", line_idx + 1))?;
                let y = parts[2].parse::<f64>().map_err(|_| format!("Line {}: invalid y", line_idx + 1))?;
                let alt = parts[3].parse::<f64>().map_err(|_| format!("Line {}: invalid altitude", line_idx + 1))?;
                MissionCommand::Goto { x, y, altitude: alt }
            }
            "WAIT" => {
                if parts.len() < 2 {
                    return Err(format!("Line {}: WAIT requires duration in seconds", line_idx + 1));
                }
                let sec = parts[1].parse::<f64>().map_err(|_| format!("Line {}: invalid wait duration", line_idx + 1))?;
                MissionCommand::Wait { seconds: sec }
            }
            "INJECT" => {
                if parts.len() < 2 {
                    return Err(format!("Line {}: INJECT requires fault name (e.g. GPS_DRIFT, LIDAR_FAILURE)", line_idx + 1));
                }
                MissionCommand::InjectFault {
                    fault_name: parts[1].to_uppercase(),
                }
            }
            "CLEAR_FAULTS" => MissionCommand::ClearFaults,
            "RETURN_HOME" | "RTH" => MissionCommand::ReturnHome,
            "LAND" => MissionCommand::Land,
            "EMERGENCY_LAND" => MissionCommand::EmergencyLand,
            other => return Err(format!("Line {}: unrecognized command '{}'", line_idx + 1, other)),
        };

        commands.push(cmd);
    }

    Ok(commands)
}

/// Execution runner that steps through mission commands sequentially.
#[derive(Debug, Clone, Default)]
pub struct CommandRunner {
    pub commands: Vec<MissionCommand>,
    pub current_step: usize,
    pub wait_timer: f64,
    pub is_running: bool,
    pub execution_log: Vec<String>,
}

impl CommandRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_script(&mut self, script: &str) -> Result<usize, String> {
        let cmds = parse_mission_script(script)?;
        let count = cmds.len();
        self.commands = cmds;
        self.current_step = 0;
        self.wait_timer = 0.0;
        self.is_running = !self.commands.is_empty();
        self.execution_log.clear();
        self.execution_log.push(format!("Loaded script with {} commands", count));
        Ok(count)
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn reset(&mut self) {
        self.commands.clear();
        self.current_step = 0;
        self.wait_timer = 0.0;
        self.is_running = false;
        self.execution_log.clear();
    }
}
