//! TypeScript type definitions for Drift IPC commands, events, and telemetry state.

export type FlightMode =
  | 'disarmed'
  | 'armed'
  | 'takeoff'
  | 'hover'
  | 'waypoint_follow'
  | 'return_to_home'
  | 'emergency_land'
  | 'landed'
  | 'collision';

export interface Vec2 {
  x: number;
  y: number;
}

export interface PathPoint {
  x: number;
  y: number;
  altitude: number;
}

export interface Obstacle {
  id: string;
  name: string;
  x: number;
  y: number;
  radius: number;
  height: number;
}

export interface NoFlyZone {
  id: string;
  name: string;
  center_x: number;
  center_y: number;
  radius: number;
  min_alt: number;
  max_alt: number;
}

export interface Boundary {
  min_x: number;
  max_x: number;
  min_y: number;
  max_y: number;
  max_altitude: number;
}

export interface Waypoint {
  id: number;
  x: number;
  y: number;
  altitude: number;
  speed_target: number;
}

export interface WindCondition {
  speed_mps: number;
  direction_deg: number;
  gust_amplitude: number;
  gust_period: number;
}

export interface DroneState {
  x: number;
  y: number;
  altitude: number;
  heading: number;
  horizontal_speed: number;
  vertical_speed: number;
  vx: number;
  vy: number;
  vz: number;
  battery_percent: number;
  flight_mode: FlightMode;
  armed: boolean;
  current_waypoint_index: number | null;
  return_to_home: boolean;
  emergency: boolean;
  distance_traveled: number;
  flight_time_seconds: number;
}

export interface LocalizationState {
  raw_x: number;
  raw_y: number;
  raw_altitude: number;
  raw_heading: number;
  filtered_x: number;
  filtered_y: number;
  filtered_altitude: number;
  filtered_heading: number;
  estimated_vx: number;
  estimated_vy: number;
  estimated_vz: number;
  estimation_error_m: number;
}

export type FaultType =
  | 'gps_drift'
  | 'gps_dropout'
  | 'imu_bias'
  | 'compass_failure'
  | 'altimeter_drift'
  | 'lidar_blind_spot'
  | 'excessive_noise';

export interface SensorReading {
  sensor_name: string;
  timestamp: string;
  values: Record<string, any>;
  validity: boolean;
  noise_level: number;
  active_fault: FaultType | null;
  confidence: number;
}

export type AlertSeverity = 'info' | 'warning' | 'critical';

export interface Alert {
  id: string;
  alert_type: string;
  severity: AlertSeverity;
  timestamp: string;
  description: string;
  suggested_action: string;
  related_mission_event: string | null;
  acknowledged: boolean;
}

export interface MissionEvent {
  id: string;
  timestamp: string;
  event_type: string;
  description: string;
  details: any | null;
}

export interface SafetyStatus {
  collision_detected: boolean;
  collision_obstacle_id: string | null;
  collision_risk_level: 'clear' | 'caution' | 'critical';
  nearest_obstacle_distance_m: number;
  geofence_status: 'inside' | 'warning_buffer' | 'breached';
  altitude_limit_exceeded: boolean;
}

export interface SimulationSnapshot {
  drone: DroneState;
  localization: LocalizationState;
  sensor_readings: SensorReading[];
  safety: SafetyStatus;
  active_alerts: Alert[];
  recent_events: MissionEvent[];
  sim_time_sec: number;
  tick_count: number;
  planned_route: Vec2[];
  raw_trail: PathPoint[];
  filtered_trail: PathPoint[];
  active_faults: FaultType[];
  wind_vector: Vec2;
  is_script_running: boolean;
  current_script_step: number;
  total_script_steps: number;
}

export interface ScenarioConfig {
  id: string;
  name: string;
  description: string;
  seed: number;
  boundary: Boundary;
  home_x: number;
  home_y: number;
  obstacles: Obstacle[];
  no_fly_zones: NoFlyZone[];
  waypoints: Waypoint[];
  wind: WindCondition;
}

export interface MissionSummary {
  id: string;
  scenario_id: string;
  name: string;
  start_time: string;
  end_time: string;
  status: string;
  total_distance: number;
  max_altitude: number;
  battery_consumed: number;
  total_samples: number;
  total_alerts: number;
}

export interface ReplayStatus {
  is_active: boolean;
  is_playing: boolean;
  current_index: number;
  total_frames: number;
  playback_speed: number;
  current_time_sec: number;
  total_duration_sec: number;
}
