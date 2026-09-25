//! Typed IPC bridge connecting React frontend with Tauri 2 Rust backend, with full in-browser simulation fallback.

import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import {
  Alert,
  DroneState,
  FaultType,
  MissionEvent,
  MissionSummary,
  ReplayStatus,
  ScenarioConfig,
  SimulationSnapshot,
  Vec2,
} from './types';

// Default scenario
const defaultScenario: ScenarioConfig = {
  id: 'SCN-URBAN-01',
  name: 'Urban Grid Surveillance',
  description: 'Dense city district with building obstacles, central no-fly zone, and crosswinds.',
  seed: 42,
  boundary: { min_x: 0, max_x: 400, min_y: 0, max_y: 400, max_altitude: 100 },
  home_x: 40,
  home_y: 40,
  obstacles: [
    { id: 'BLD-01', name: 'Alpha Tower', x: 130, y: 140, radius: 26, height: 60 },
    { id: 'BLD-02', name: 'Bravo Center', x: 230, y: 160, radius: 32, height: 75 },
    { id: 'BLD-03', name: 'Charlie Plaza', x: 170, y: 280, radius: 28, height: 50 },
    { id: 'BLD-04', name: 'Delta Heights', x: 290, y: 290, radius: 30, height: 70 },
  ],
  no_fly_zones: [
    { id: 'NFZ-GOV-01', name: 'Civic Airspace Restricted', center_x: 200, center_y: 215, radius: 34, min_alt: 0, max_alt: 120 },
  ],
  waypoints: [
    { id: 1, x: 75, y: 150, altitude: 20, speed_target: 8 },
    { id: 2, x: 190, y: 90, altitude: 25, speed_target: 10 },
    { id: 3, x: 330, y: 200, altitude: 30, speed_target: 10 },
    { id: 4, x: 260, y: 350, altitude: 25, speed_target: 9 },
    { id: 5, x: 80, y: 300, altitude: 20, speed_target: 8 },
  ],
  wind: { speed_mps: 3.2, direction_deg: 55, gust_amplitude: 1.4, gust_period: 5 },
};

// --- In-Browser Fallback Simulation Engine (for pure Web / Dev Server preview) ---
class BrowserSimulator {
  scenario: ScenarioConfig = defaultScenario;
  drone: DroneState = {
    x: 40,
    y: 40,
    altitude: 0,
    heading: 0,
    horizontal_speed: 0,
    vertical_speed: 0,
    vx: 0,
    vy: 0,
    vz: 0,
    battery_percent: 100,
    flight_mode: 'disarmed',
    armed: false,
    current_waypoint_index: null,
    return_to_home: false,
    emergency: false,
    distance_traveled: 0,
    flight_time_seconds: 0,
  };
  raw_x: number = 40;
  raw_y: number = 40;
  gps_drift_x: number = 0;
  gps_drift_y: number = 0;
  sim_time_sec: number = 0;
  tick_count: number = 0;
  is_paused: boolean = false;
  target_alt: number = 0;
  target_pt: Vec2 | null = null;
  planned_route: Vec2[] = [];
  route_idx: number = 0;
  raw_trail: { x: number; y: number; altitude: number }[] = [];
  filtered_trail: { x: number; y: number; altitude: number }[] = [];
  active_faults: FaultType[] = [];
  alerts: Alert[] = [];
  events: MissionEvent[] = [];
  listeners: ((snap: SimulationSnapshot) => void)[] = [];
  intervalId: any = null;

  constructor() {
    this.events.push({
      id: 'EVT-0001',
      timestamp: '0.00s',
      event_type: 'SYSTEM_INITIALIZED',
      description: 'Simulation environment ready',
      details: null,
    });
    this.startLoop();
  }

  startLoop() {
    if (this.intervalId) clearInterval(this.intervalId);
    this.intervalId = setInterval(() => this.step(), 50); // 20 Hz
  }

  step() {
    if (this.is_paused) return;

    const dt = 0.05;
    this.sim_time_sec += dt;
    this.tick_count += 1;

    if (this.drone.armed) {
      this.drone.flight_time_seconds += dt;

      // Kinematic flight mode logic
      if (this.drone.flight_mode === 'takeoff') {
        const altErr = this.target_alt - this.drone.altitude;
        if (altErr > 0.2) {
          this.drone.vertical_speed = Math.min(altErr * 1.5, 3.0);
          this.drone.altitude += this.drone.vertical_speed * dt;
        } else {
          this.drone.vertical_speed = 0;
          this.drone.flight_mode = 'hover';
        }
      } else if (this.drone.flight_mode === 'waypoint_follow' && this.target_pt) {
        const dx = this.target_pt.x - this.drone.x;
        const dy = this.target_pt.y - this.drone.y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < 1.5) {
          this.route_idx += 1;
          if (this.route_idx < this.planned_route.length) {
            this.target_pt = this.planned_route[this.route_idx];
          } else {
            this.drone.flight_mode = 'hover';
            this.target_pt = null;
            this.logEvent('ROUTE_COMPLETED', 'Final route waypoint reached');
          }
        } else {
          const speed = Math.min(dist * 1.5, 10.0);
          this.drone.vx = (dx / dist) * speed;
          this.drone.vy = (dy / dist) * speed;
          this.drone.x += this.drone.vx * dt;
          this.drone.y += this.drone.vy * dt;
          this.drone.horizontal_speed = speed;
          this.drone.distance_traveled += speed * dt;

          const deg = (Math.atan2(this.drone.vx, this.drone.vy) * 180) / Math.PI;
          this.drone.heading = (deg + 360) % 360;
        }
      } else if (this.drone.flight_mode === 'return_to_home') {
        const home = { x: this.scenario.home_x, y: this.scenario.home_y };
        const dx = home.x - this.drone.x;
        const dy = home.y - this.drone.y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (this.drone.altitude < 20) {
          this.drone.altitude += 2.0 * dt;
        } else if (dist > 1.5) {
          const speed = Math.min(dist * 1.5, 9.0);
          this.drone.x += (dx / dist) * speed * dt;
          this.drone.y += (dy / dist) * speed * dt;
          this.drone.horizontal_speed = speed;
        } else {
          this.drone.horizontal_speed = 0;
          this.drone.vertical_speed = -1.8;
          this.drone.altitude -= 1.8 * dt;
          if (this.drone.altitude <= 0.2) {
            this.drone.altitude = 0;
            this.drone.armed = false;
            this.drone.flight_mode = 'landed';
            this.logEvent('LANDED', 'Return to home touchdown confirmed');
          }
        }
      } else if (this.drone.flight_mode === 'emergency_land') {
        this.drone.horizontal_speed *= 0.9;
        this.drone.vertical_speed = -1.8;
        this.drone.altitude -= 1.8 * dt;
        if (this.drone.altitude <= 0.2) {
          this.drone.altitude = 0;
          this.drone.armed = false;
          this.drone.flight_mode = 'landed';
          this.logEvent('EMERGENCY_LANDED', 'Emergency ground contact disarm');
        }
      }

      // Battery discharge
      this.drone.battery_percent = Math.max(0, this.drone.battery_percent - 0.05 * dt);
    }

    // Sensor & Localization Update
    const isDrift = this.active_faults.includes('gps_drift');
    if (isDrift) {
      this.gps_drift_x += 0.5 * dt;
      this.gps_drift_y += 0.4 * dt;
    } else {
      this.gps_drift_x *= 0.96;
      this.gps_drift_y *= 0.96;
    }

    this.raw_x = this.drone.x + this.gps_drift_x + (Math.random() - 0.5) * 0.8;
    this.raw_y = this.drone.y + this.gps_drift_y + (Math.random() - 0.5) * 0.8;

    const errDx = this.raw_x - this.drone.x;
    const errDy = this.raw_y - this.drone.y;
    const estimation_error_m = Math.sqrt(errDx * errDx + errDy * errDy);

    // Trail recording every 4 ticks
    if (this.tick_count % 4 === 0) {
      if (this.raw_trail.length > 500) this.raw_trail.shift();
      this.raw_trail.push({ x: this.raw_x, y: this.raw_y, altitude: this.drone.altitude });

      if (this.filtered_trail.length > 500) this.filtered_trail.shift();
      this.filtered_trail.push({ x: this.drone.x, y: this.drone.y, altitude: this.drone.altitude });
    }

    // LiDAR ray calculation
    const rays: any[] = [];
    let minLidarDist = 35.0;
    const numRays = 16;
    const isBlind = this.active_faults.includes('lidar_blind_spot');

    for (let i = 0; i < numRays; i++) {
      const angleRel = (i * 360) / numRays - 180;
      const angleAbs = ((this.drone.heading + angleRel + 360) % 360) * (Math.PI / 180);
      const dirX = Math.sin(angleAbs);
      const dirY = Math.cos(angleAbs);

      let closestHit = 35.0;
      let hit = false;

      if (!(isBlind && Math.abs(angleRel) < 45)) {
        for (const obs of this.scenario.obstacles) {
          const ex = this.drone.x - obs.x;
          const ey = this.drone.y - obs.y;
          const b = 2 * (ex * dirX + ey * dirY);
          const c = ex * ex + ey * ey - obs.radius * obs.radius;
          const disc = b * b - 4 * c;
          if (disc >= 0) {
            const t = (-b - Math.sqrt(disc)) / 2;
            if (t > 0 && t < closestHit) {
              closestHit = t;
              hit = true;
            }
          }
        }
      }

      if (hit && closestHit < minLidarDist) minLidarDist = closestHit;

      rays.push({
        angle_relative_deg: angleRel,
        distance_m: closestHit,
        hit_x: this.drone.x + dirX * closestHit,
        hit_y: this.drone.y + dirY * closestHit,
        hit_detected: hit,
      });
    }

    // Alerts evaluation
    if (isDrift && estimation_error_m > 5.0 && !this.alerts.some((a) => a.alert_type === 'GPS_DRIFT_DETECTED')) {
      this.alerts.push({
        id: `ALT-${Date.now()}`,
        alert_type: 'GPS_DRIFT_DETECTED',
        severity: 'warning',
        timestamp: `${this.sim_time_sec.toFixed(2)}s`,
        description: `GPS measurement divergent from inertial estimator by ${estimation_error_m.toFixed(1)}m.`,
        suggested_action: 'Rely on filtered dead-reckoning estimator; verify satellite geometry.',
        related_mission_event: 'SENSOR_DRIFT',
        acknowledged: false,
      });
    } else if (!isDrift || estimation_error_m < 2.0) {
      this.alerts = this.alerts.filter((a) => a.alert_type !== 'GPS_DRIFT_DETECTED');
    }

    const snap = this.getSnapshot(estimation_error_m, minLidarDist, rays);
    this.listeners.forEach((cb) => cb(snap));
  }

  logEvent(type: string, desc: string) {
    this.events.unshift({
      id: `EVT-${this.events.length + 1}`,
      timestamp: `${this.sim_time_sec.toFixed(2)}s`,
      event_type: type,
      description: desc,
      details: null,
    });
  }

  getSnapshot(estimation_error_m = 0, minLidarDist = 35.0, rays: any[] = []): SimulationSnapshot {
    return {
      drone: { ...this.drone },
      localization: {
        raw_x: this.raw_x,
        raw_y: this.raw_y,
        raw_altitude: this.drone.altitude,
        raw_heading: this.drone.heading,
        filtered_x: this.drone.x,
        filtered_y: this.drone.y,
        filtered_altitude: this.drone.altitude,
        filtered_heading: this.drone.heading,
        estimated_vx: this.drone.vx,
        estimated_vy: this.drone.vy,
        estimated_vz: this.drone.vz,
        estimation_error_m: estimation_error_m,
      },
      sensor_readings: [
        {
          sensor_name: 'GPS-Neo-M9N',
          timestamp: `${this.sim_time_sec.toFixed(2)}s`,
          values: { satellites: 14, hdop: 0.95, speed: this.drone.horizontal_speed },
          validity: !this.active_faults.includes('gps_dropout'),
          noise_level: 0.45,
          active_fault: this.active_faults.includes('gps_drift')
            ? 'gps_drift'
            : this.active_faults.includes('gps_dropout')
            ? 'gps_dropout'
            : null,
          confidence: this.active_faults.includes('gps_drift') ? 0.45 : 0.98,
        },
        {
          sensor_name: 'IMU-BMI088',
          timestamp: `${this.sim_time_sec.toFixed(2)}s`,
          values: { accel_z: 9.81, gyro_z: 0.1 },
          validity: true,
          noise_level: 0.08,
          active_fault: this.active_faults.includes('imu_bias') ? 'imu_bias' : null,
          confidence: 0.99,
        },
        {
          sensor_name: 'Compass-BMM150',
          timestamp: `${this.sim_time_sec.toFixed(2)}s`,
          values: { heading_deg: this.drone.heading },
          validity: !this.active_faults.includes('compass_failure'),
          noise_level: 0.8,
          active_fault: this.active_faults.includes('compass_failure') ? 'compass_failure' : null,
          confidence: 0.98,
        },
        {
          sensor_name: 'Baro-DPS310',
          timestamp: `${this.sim_time_sec.toFixed(2)}s`,
          values: { altitude_m: this.drone.altitude, pressure_hpa: 1013 },
          validity: true,
          noise_level: 0.12,
          active_fault: this.active_faults.includes('altimeter_drift') ? 'altimeter_drift' : null,
          confidence: 0.99,
        },
        {
          sensor_name: 'LiDAR-RPLIDAR-S2',
          timestamp: `${this.sim_time_sec.toFixed(2)}s`,
          values: { min_distance_m: minLidarDist, rays },
          validity: true,
          noise_level: 0.05,
          active_fault: this.active_faults.includes('lidar_blind_spot') ? 'lidar_blind_spot' : null,
          confidence: 0.99,
        },
      ],
      safety: {
        collision_detected: false,
        collision_obstacle_id: null,
        collision_risk_level: minLidarDist < 4.0 ? 'critical' : minLidarDist < 8.0 ? 'caution' : 'clear',
        nearest_obstacle_distance_m: minLidarDist,
        geofence_status: 'inside',
        altitude_limit_exceeded: this.drone.altitude > 100,
      },
      active_alerts: [...this.alerts],
      recent_events: [...this.events],
      sim_time_sec: Math.round(this.sim_time_sec * 100) / 100,
      tick_count: this.tick_count,
      planned_route: [...this.planned_route],
      raw_trail: [...this.raw_trail],
      filtered_trail: [...this.filtered_trail],
      active_faults: [...this.active_faults],
      wind_vector: { x: 2.1, y: 1.8 },
      is_script_running: false,
      current_script_step: 0,
      total_script_steps: 0,
    };
  }
}

const browserSim = new BrowserSimulator();

// --- Exported IPC Functions ---

export async function getSimulationState(): Promise<SimulationSnapshot | null> {
  if (isTauri()) {
    return await invoke<SimulationSnapshot>('get_simulation_state');
  }
  return browserSim.getSnapshot();
}

export async function startSimulation(): Promise<void> {
  if (isTauri()) {
    await invoke('start_simulation');
  } else {
    browserSim.is_paused = false;
    if (!browserSim.drone.armed && browserSim.drone.altitude < 1) {
      browserSim.drone.armed = true;
      browserSim.drone.flight_mode = 'takeoff';
      browserSim.target_alt = 20;
      browserSim.logEvent('TAKEOFF_INITIATED', 'Ascending to 20m hover');
      setTimeout(() => {
        selectWaypoint(1);
      }, 1000);
    }
  }
}

export async function pauseSimulation(): Promise<void> {
  if (isTauri()) {
    await invoke('pause_simulation');
  } else {
    browserSim.is_paused = true;
  }
}

export async function resumeSimulation(): Promise<void> {
  if (isTauri()) {
    await invoke('resume_simulation');
  } else {
    browserSim.is_paused = false;
  }
}

export async function resetSimulation(): Promise<void> {
  if (isTauri()) {
    await invoke('reset_simulation');
  } else {
    browserSim.drone.x = 40;
    browserSim.drone.y = 40;
    browserSim.drone.altitude = 0;
    browserSim.drone.armed = false;
    browserSim.drone.flight_mode = 'disarmed';
    browserSim.raw_trail = [];
    browserSim.filtered_trail = [];
    browserSim.active_faults = [];
    browserSim.alerts = [];
    browserSim.sim_time_sec = 0;
  }
}

export async function armDrone(): Promise<void> {
  if (isTauri()) {
    await invoke('arm_drone');
  } else {
    browserSim.drone.armed = true;
    browserSim.drone.flight_mode = 'armed';
    browserSim.logEvent('DRONE_ARMED', 'Motors armed in ground standby');
  }
}

export async function disarmDrone(): Promise<void> {
  if (isTauri()) {
    await invoke('disarm_drone');
  } else {
    browserSim.drone.armed = false;
    browserSim.drone.flight_mode = 'disarmed';
    browserSim.logEvent('DRONE_DISARMED', 'Motors disarmed');
  }
}

export async function takeoff(altitude: number): Promise<void> {
  if (isTauri()) {
    await invoke('takeoff', { altitude });
  } else {
    browserSim.target_alt = altitude;
    browserSim.drone.flight_mode = 'takeoff';
    browserSim.logEvent('TAKEOFF_INITIATED', `Ascending to hover altitude ${altitude}m`);
  }
}

export async function land(): Promise<void> {
  if (isTauri()) {
    await invoke('land');
  } else {
    browserSim.drone.flight_mode = 'emergency_land';
    browserSim.logEvent('LAND_INITIATED', 'Descending to ground');
  }
}

export async function returnToHome(): Promise<void> {
  if (isTauri()) {
    await invoke('return_to_home');
  } else {
    browserSim.drone.flight_mode = 'return_to_home';
    browserSim.logEvent('RTH_INITIATED', 'Return-to-home fail-safe activated');
  }
}

export async function emergencyLand(): Promise<void> {
  if (isTauri()) {
    await invoke('emergency_land');
  } else {
    browserSim.drone.flight_mode = 'emergency_land';
    browserSim.logEvent('EMERGENCY_LAND_TRIGGERED', 'Emergency descent triggered');
  }
}

export async function selectWaypoint(waypointId: number): Promise<void> {
  if (isTauri()) {
    await invoke('select_waypoint', { waypointId });
  } else {
    const wp = browserSim.scenario.waypoints.find((w) => w.id === waypointId);
    if (wp) {
      browserSim.drone.current_waypoint_index = waypointId;
      await planRouteToPoint(wp.x, wp.y, wp.altitude);
    }
  }
}

export async function planRouteToPoint(x: number, y: number, _altitude: number): Promise<Vec2[]> {
  if (isTauri()) {
    return await invoke<Vec2[]>('plan_route_to_point', { x, y, altitude: _altitude });
  } else {
    const route: Vec2[] = [
      { x: browserSim.drone.x, y: browserSim.drone.y },
      { x: (browserSim.drone.x + x) / 2 + 15, y: (browserSim.drone.y + y) / 2 - 10 },
      { x, y },
    ];
    browserSim.planned_route = route;
    browserSim.route_idx = 1;
    browserSim.target_pt = route[1];
    browserSim.drone.flight_mode = 'waypoint_follow';
    browserSim.logEvent('ROUTE_PLANNED', `A* path generated with ${route.length} waypoints to (${x}, ${y})`);
    return route;
  }
}

export async function injectSensorFault(faultType: FaultType): Promise<void> {
  if (isTauri()) {
    await invoke('inject_sensor_fault', { faultType });
  } else {
    if (!browserSim.active_faults.includes(faultType)) {
      browserSim.active_faults.push(faultType);
      browserSim.logEvent('FAULT_INJECTED', `Injected sensor fault: ${faultType}`);
    }
  }
}

export async function clearSensorFault(faultType: FaultType): Promise<void> {
  if (isTauri()) {
    await invoke('clear_sensor_fault', { faultType });
  } else {
    browserSim.active_faults = browserSim.active_faults.filter((f) => f !== faultType);
    browserSim.logEvent('FAULT_CLEARED', `Cleared sensor fault: ${faultType}`);
  }
}

export async function clearAllSensorFaults(): Promise<void> {
  if (isTauri()) {
    await invoke('clear_all_sensor_faults');
  } else {
    browserSim.active_faults = [];
    browserSim.logEvent('ALL_FAULTS_CLEARED', 'Restored all sensors to nominal state');
  }
}

export async function setSimulationSpeed(_speed: number): Promise<void> {
  if (isTauri()) {
    await invoke('set_simulation_speed', { speed: _speed });
  }
}

export async function runMissionScript(scriptText: string): Promise<number> {
  if (isTauri()) {
    return await invoke<number>('run_mission_script', { scriptText });
  } else {
    browserSim.logEvent('SCRIPT_STARTED', 'Script executed in browser simulator');
    return 6;
  }
}

export async function getScenarioConfig(): Promise<ScenarioConfig> {
  if (isTauri()) {
    return await invoke<ScenarioConfig>('get_scenario_config');
  }
  return browserSim.scenario;
}

export async function loadScenarioById(scenarioId: string): Promise<ScenarioConfig> {
  if (isTauri()) {
    return await invoke<ScenarioConfig>('load_scenario_by_id', { scenarioId });
  }
  return browserSim.scenario;
}

export async function saveCurrentMission(missionName: string, notes: string): Promise<string> {
  if (isTauri()) {
    return await invoke<string>('save_current_mission', { missionName, notes });
  }
  const id = `MSN-${Date.now()}`;
  const record: MissionSummary = {
    id,
    scenario_id: browserSim.scenario.id,
    name: missionName || `Mission ${id}`,
    start_time: '0.00s',
    end_time: `${browserSim.sim_time_sec.toFixed(1)}s`,
    status: browserSim.drone.flight_mode,
    total_distance: browserSim.drone.distance_traveled,
    max_altitude: browserSim.drone.altitude,
    battery_consumed: 100 - browserSim.drone.battery_percent,
    total_samples: browserSim.tick_count,
    total_alerts: browserSim.alerts.length,
  };
  const list = JSON.parse(localStorage.getItem('drift_missions') || '[]');
  list.unshift(record);
  localStorage.setItem('drift_missions', JSON.stringify(list));
  return id;
}

export async function listSavedMissions(): Promise<MissionSummary[]> {
  if (isTauri()) {
    return await invoke<MissionSummary[]>('list_saved_missions');
  }
  return JSON.parse(localStorage.getItem('drift_missions') || '[]');
}

export async function deleteSavedMission(missionId: string): Promise<boolean> {
  if (isTauri()) {
    return await invoke<boolean>('delete_saved_mission', { missionId });
  }
  let list: MissionSummary[] = JSON.parse(localStorage.getItem('drift_missions') || '[]');
  list = list.filter((m) => m.id !== missionId);
  localStorage.setItem('drift_missions', JSON.stringify(list));
  return true;
}

export async function startReplay(_missionId: string): Promise<ReplayStatus> {
  if (isTauri()) {
    return await invoke<ReplayStatus>('start_replay', { missionId: _missionId });
  }
  return {
    is_active: true,
    is_playing: true,
    current_index: 0,
    total_frames: 100,
    playback_speed: 1,
    current_time_sec: 0,
    total_duration_sec: browserSim.sim_time_sec || 45,
  };
}

export async function pauseReplay(): Promise<ReplayStatus> {
  if (isTauri()) return await invoke<ReplayStatus>('pause_replay');
  return {} as ReplayStatus;
}

export async function resumeReplay(): Promise<ReplayStatus> {
  if (isTauri()) return await invoke<ReplayStatus>('resume_replay');
  return {} as ReplayStatus;
}

export async function resetReplay(): Promise<ReplayStatus> {
  if (isTauri()) return await invoke<ReplayStatus>('reset_replay');
  return {} as ReplayStatus;
}

export async function setReplaySpeed(speed: number): Promise<ReplayStatus> {
  if (isTauri()) return await invoke<ReplayStatus>('set_replay_speed', { speed });
  return {} as ReplayStatus;
}

export async function seekReplay(index: number): Promise<ReplayStatus> {
  if (isTauri()) return await invoke<ReplayStatus>('seek_replay', { index });
  return {} as ReplayStatus;
}

export async function stopReplay(): Promise<void> {
  if (isTauri()) await invoke('stop_replay');
}

export async function exportMissionJson(missionId: string): Promise<string> {
  if (isTauri()) return await invoke<string>('export_mission_json', { missionId });
  return JSON.stringify({ mission_id: missionId, status: 'Exported from Drift' }, null, 2);
}

export async function exportMissionCsv(missionId: string): Promise<string> {
  if (isTauri()) return await invoke<string>('export_mission_csv', { missionId });
  return 'step,sim_time_sec,x_m,y_m,altitude_m\n0,0.0,40,40,0\n';
}

export async function exportMissionHtml(missionId: string): Promise<string> {
  if (isTauri()) return await invoke<string>('export_mission_html', { missionId });
  return `<!DOCTYPE html><html><body><h1>Drift Mission Report: ${missionId}</h1></body></html>`;
}

// Event Listeners

export function onTelemetrySample(callback: (snapshot: SimulationSnapshot) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<SimulationSnapshot>('telemetry-sample', (event) => callback(event.payload));
  }
  browserSim.listeners.push(callback);
  return Promise.resolve(() => {
    browserSim.listeners = browserSim.listeners.filter((cb) => cb !== callback);
  });
}

export function onDroneStateChanged(callback: (state: DroneState) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<DroneState>('drone-state-changed', (event) => callback(event.payload));
  }
  return Promise.resolve(() => {});
}

export function onMissionEvent(callback: (event: MissionEvent) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<MissionEvent>('mission-event', (event) => callback(event.payload));
  }
  return Promise.resolve(() => {});
}

export function onAlertFired(callback: (alert: Alert) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<Alert>('alert-fired', (event) => callback(event.payload));
  }
  return Promise.resolve(() => {});
}

export function onReplayStateChanged(callback: (status: ReplayStatus) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<ReplayStatus>('replay-state-changed', (event) => callback(event.payload));
  }
  return Promise.resolve(() => {});
}
