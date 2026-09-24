//! Alert data models, severity classifications, and active alert state management.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert event fired when safety margins, sensor fidelities, or flight envelopes are violated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub timestamp: String,
    pub description: String,
    pub suggested_action: String,
    pub related_mission_event: Option<String>,
    pub acknowledged: bool,
}

/// Alert manager tracking active and historical alerts during simulation.
#[derive(Debug, Clone, Default)]
pub struct AlertManager {
    pub active_alerts: Vec<Alert>,
    pub alert_history: Vec<Alert>,
    next_id: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger_alert(
        &mut self,
        alert_type: &str,
        severity: AlertSeverity,
        timestamp: &str,
        description: &str,
        suggested_action: &str,
        related_mission_event: Option<String>,
    ) -> Option<Alert> {
        // Prevent duplicate spam of identical alert within active list
        if self.active_alerts.iter().any(|a| a.alert_type == alert_type) {
            return None;
        }

        self.next_id += 1;
        let alert = Alert {
            id: format!("ALT-{:04}", self.next_id),
            alert_type: alert_type.to_string(),
            severity,
            timestamp: timestamp.to_string(),
            description: description.to_string(),
            suggested_action: suggested_action.to_string(),
            related_mission_event,
            acknowledged: false,
        };

        self.active_alerts.push(alert.clone());
        self.alert_history.push(alert.clone());
        Some(alert)
    }

    pub fn clear_alert_type(&mut self, alert_type: &str) {
        self.active_alerts.retain(|a| a.alert_type != alert_type);
    }

    pub fn acknowledge(&mut self, alert_id: &str) {
        if let Some(a) = self.active_alerts.iter_mut().find(|a| a.id == alert_id) {
            a.acknowledged = true;
        }
        if let Some(a) = self.alert_history.iter_mut().find(|a| a.id == alert_id) {
            a.acknowledged = true;
        }
    }

    pub fn clear_all(&mut self) {
        self.active_alerts.clear();
        self.alert_history.clear();
    }
}
