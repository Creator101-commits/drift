//! Export generation for JSON mission packages, CSV telemetry matrices, and standalone HTML reports.

use crate::db::SavedMissionRecord;

pub struct MissionExporter;

impl MissionExporter {
    /// Export complete mission dataset as structured JSON.
    pub fn to_json(record: &SavedMissionRecord) -> Result<String, String> {
        serde_json::to_string_pretty(record).map_err(|e| e.to_string())
    }

    /// Export telemetry timeseries samples as formatted CSV.
    pub fn to_csv(record: &SavedMissionRecord) -> String {
        let mut csv = String::new();
        csv.push_str("step,sim_time_sec,x_m,y_m,altitude_m,heading_deg,horizontal_speed_mps,vertical_speed_mps,battery_percent,raw_x_m,raw_y_m,raw_altitude_m,raw_heading_deg,filtered_x_m,filtered_y_m,filtered_altitude_m,filtered_heading_deg,flight_mode\n");

        for (idx, s) in record.snapshots.iter().enumerate() {
            csv.push_str(&format!(
                "{},{:.2},{:.2},{:.2},{:.2},{:.1},{:.2},{:.2},{:.1},{:.2},{:.2},{:.2},{:.1},{:.2},{:.2},{:.2},{:.1},{}\n",
                idx,
                s.sim_time_sec,
                s.drone.x,
                s.drone.y,
                s.drone.altitude,
                s.drone.heading,
                s.drone.horizontal_speed,
                s.drone.vertical_speed,
                s.drone.battery_percent,
                s.localization.raw_x,
                s.localization.raw_y,
                s.localization.raw_altitude,
                s.localization.raw_heading,
                s.localization.filtered_x,
                s.localization.filtered_y,
                s.localization.filtered_altitude,
                s.localization.filtered_heading,
                s.drone.flight_mode,
            ));
        }

        csv
    }

    /// Export a self-contained, offline HTML mission report with embedded SVG map and metrics.
    pub fn to_html_report(record: &SavedMissionRecord) -> String {
        let duration = record.snapshots.last().map(|s| s.sim_time_sec).unwrap_or(0.0);

        // Generate SVG polyline path for raw and filtered trajectories
        let mut raw_points = String::new();
        let mut filtered_points = String::new();

        for s in &record.snapshots {
            // Coordinate mapping: 0..400 meters to 0..600 SVG canvas (Y flipped)
            let rx = (s.localization.raw_x / 400.0) * 600.0;
            let ry = 600.0 - (s.localization.raw_y / 400.0) * 600.0;
            raw_points.push_str(&format!("{:.1},{:.1} ", rx, ry));

            let fx = (s.localization.filtered_x / 400.0) * 600.0;
            let fy = 600.0 - (s.localization.filtered_y / 400.0) * 600.0;
            filtered_points.push_str(&format!("{:.1},{:.1} ", fx, fy));
        }

        // Generate event log table rows
        let mut event_rows = String::new();
        for e in &record.events {
            event_rows.push_str(&format!(
                "<tr><td class=\"mono\">{}</td><td><span class=\"badge badge-event\">{}</span></td><td>{}</td></tr>\n",
                e.timestamp, e.event_type, e.description
            ));
        }

        // Generate alert table rows
        let mut alert_rows = String::new();
        for a in &record.alerts {
            let badge_class = match a.severity {
                crate::analysis::AlertSeverity::Critical => "badge-critical",
                crate::analysis::AlertSeverity::Warning => "badge-warning",
                crate::analysis::AlertSeverity::Info => "badge-info",
            };
            alert_rows.push_str(&format!(
                "<tr><td class=\"mono\">{}</td><td><span class=\"badge {}\">{}</span></td><td>{}</td><td class=\"subtle\">{}</td></tr>\n",
                a.timestamp, badge_class, a.alert_type, a.description, a.suggested_action
            ));
        }

        let template = r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Drift Mission Report: __NAME__</title>
<style>
  :root {
    --bg-primary: #0b0f19;
    --bg-secondary: #111827;
    --bg-card: #1e293b;
    --border: #334155;
    --text-primary: #f8fafc;
    --text-secondary: #94a3b8;
    --cyan: #06b6d4;
    --amber: #f59e0b;
    --emerald: #10b981;
    --red: #ef4444;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    padding: 32px;
    line-height: 1.5;
  }
  .container { max-width: 1200px; margin: 0 auto; }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border);
    padding-bottom: 24px;
    margin-bottom: 32px;
  }
  h1 { font-size: 28px; font-weight: 700; letter-spacing: -0.5px; }
  .tag { font-size: 13px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 1px; }
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
    margin-bottom: 32px;
  }
  .metric-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
  }
  .metric-title { font-size: 12px; text-transform: uppercase; color: var(--text-secondary); margin-bottom: 6px; }
  .metric-value { font-size: 26px; font-weight: 700; font-family: ui-monospace, monospace; color: var(--cyan); }
  .section {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    margin-bottom: 32px;
  }
  .section-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 10px;
  }
  .map-wrapper {
    display: flex;
    justify-content: center;
    background: #06090e;
    border-radius: 6px;
    padding: 16px;
  }
  svg { max-width: 100%; height: auto; }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
    margin-top: 8px;
  }
  th, td {
    padding: 10px 14px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  th { color: var(--text-secondary); font-size: 12px; text-transform: uppercase; }
  .mono { font-family: ui-monospace, monospace; font-size: 13px; color: var(--text-secondary); }
  .subtle { color: var(--text-secondary); font-size: 13px; }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .badge-event { background: rgba(6, 182, 212, 0.15); color: var(--cyan); border: 1px solid var(--cyan); }
  .badge-critical { background: rgba(239, 68, 68, 0.15); color: var(--red); border: 1px solid var(--red); }
  .badge-warning { background: rgba(245, 158, 11, 0.15); color: var(--amber); border: 1px solid var(--amber); }
  .badge-info { background: rgba(16, 185, 129, 0.15); color: var(--emerald); border: 1px solid var(--emerald); }
</style>
</head>
<body>
<div class="container">
  <header>
    <div>
      <div class="tag">Drift Telemetry Analysis Platform</div>
      <h1>Mission Report: __NAME__</h1>
    </div>
    <div style="text-align: right;">
      <div class="mono">ID: __ID__</div>
      <div class="subtle">Started: __START_TIME__</div>
    </div>
  </header>

  <div class="metrics-grid">
    <div class="metric-card">
      <div class="metric-title">Flight Duration</div>
      <div class="metric-value">__DURATION__s</div>
    </div>
    <div class="metric-card">
      <div class="metric-title">Distance Traveled</div>
      <div class="metric-value">__DISTANCE__m</div>
    </div>
    <div class="metric-card">
      <div class="metric-title">Peak Altitude</div>
      <div class="metric-value">__MAX_ALT__m</div>
    </div>
    <div class="metric-card">
      <div class="metric-title">Battery Consumed</div>
      <div class="metric-value">__BATTERY__%</div>
    </div>
    <div class="metric-card">
      <div class="metric-title">Total Alerts</div>
      <div class="metric-value" style="color: var(--amber);">__ALERT_COUNT__</div>
    </div>
  </div>

  <div class="section">
    <div class="section-title">2D Trajectory Vector Map (400m x 400m Airspace)</div>
    <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
      <span style="color: #ef4444;">-- Pink / Red Dashed:</span> Raw GPS Position Path &nbsp;|&nbsp;
      <span style="color: #06b6d4;">-- Cyan Solid:</span> Kalman Filtered Localization Path
    </div>
    <div class="map-wrapper">
      <svg width="600" height="600" viewBox="0 0 600 600" xmlns="http://www.w3.org/2000/svg">
        <rect width="600" height="600" fill="#090d16" stroke="#334155" stroke-width="2"/>
        <line x1="150" y1="0" x2="150" y2="600" stroke="#1e293b" stroke-dasharray="4"/>
        <line x1="300" y1="0" x2="300" y2="600" stroke="#1e293b" stroke-dasharray="4"/>
        <line x1="450" y1="0" x2="450" y2="600" stroke="#1e293b" stroke-dasharray="4"/>
        <line x1="0" y1="150" x2="600" y2="150" stroke="#1e293b" stroke-dasharray="4"/>
        <line x1="0" y1="300" x2="600" y2="300" stroke="#1e293b" stroke-dasharray="4"/>
        <line x1="0" y1="450" x2="600" y2="450" stroke="#1e293b" stroke-dasharray="4"/>
        <polyline points="__RAW_POINTS__" fill="none" stroke="#ec4899" stroke-width="1.5" stroke-dasharray="3,3" opacity="0.75"/>
        <polyline points="__FILTERED_POINTS__" fill="none" stroke="#06b6d4" stroke-width="2.5"/>
      </svg>
    </div>
  </div>

  <div class="section">
    <div class="section-title">Mission Safety & Diagnostic Alerts</div>
    <table>
      <thead>
        <tr>
          <th>Timestamp</th>
          <th>Alert Type</th>
          <th>Description</th>
          <th>Mitigation Action</th>
        </tr>
      </thead>
      <tbody>
        __ALERT_ROWS__
      </tbody>
    </table>
  </div>

  <div class="section">
    <div class="section-title">Chronological Event Timeline</div>
    <table>
      <thead>
        <tr>
          <th>Timestamp</th>
          <th>Event Type</th>
          <th>Description</th>
        </tr>
      </thead>
      <tbody>
        __EVENT_ROWS__
      </tbody>
    </table>
  </div>
</div>
</body>
</html>"###;

        template
            .replace("__NAME__", &record.name)
            .replace("__ID__", &record.id)
            .replace("__START_TIME__", &record.start_time)
            .replace("__DURATION__", &format!("{:.1}", duration))
            .replace("__DISTANCE__", &format!("{:.1}", record.total_distance))
            .replace("__MAX_ALT__", &format!("{:.1}", record.max_altitude))
            .replace("__BATTERY__", &format!("{:.1}", record.battery_consumed))
            .replace("__ALERT_COUNT__", &record.alerts.len().to_string())
            .replace("__RAW_POINTS__", &raw_points)
            .replace("__FILTERED_POINTS__", &filtered_points)
            .replace("__ALERT_ROWS__", &alert_rows)
            .replace("__EVENT_ROWS__", &event_rows)
    }
}
