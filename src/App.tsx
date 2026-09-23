import React, { useState, useEffect } from 'react';
import { MapView } from './components/MapView';
import { SensorHealth } from './components/SensorHealth';
import { ScenarioConfig, SimulationSnapshot, SensorReading } from './types';
import urbanScenario from '../scenarios/urban_surveillance.json';
import canyonScenario from '../scenarios/canyon_wind_obstacle.json';
import './App.css';

export function App() {
  const [selectedScenarioId, setSelectedScenarioId] = useState<'urban' | 'canyon'>('urban');
  const scenario = (selectedScenarioId === 'urban' ? urbanScenario : canyonScenario) as unknown as ScenarioConfig;
  const [snapshot, setSnapshot] = useState<SimulationSnapshot | null>(null);
  const [isRunning, setIsRunning] = useState<boolean>(false);

  // Simulation tick loop (20 Hz) with simulated sensor suite
  useEffect(() => {
    if (!isRunning) return;
    const interval = setInterval(() => {
      setSnapshot((prev) => {
        const readings: SensorReading[] = [
          { sensor_name: 'GPS', timestamp: new Date().toISOString(), values: { lat: 37.7749, lon: -122.4194, alt: 15.0 }, valid: true, noise_level: 0.05, active_fault: null, confidence: 0.98 },
          { sensor_name: 'IMU', timestamp: new Date().toISOString(), values: { ax: 0.02, ay: -0.01, az: 9.81, gx: 0.0, gy: 0.0, gz: 0.01 }, valid: true, noise_level: 0.02, active_fault: null, confidence: 0.99 },
          { sensor_name: 'Compass', timestamp: new Date().toISOString(), values: { heading_deg: 45.2 }, valid: true, noise_level: 0.01, active_fault: null, confidence: 0.97 },
          { sensor_name: 'Altimeter', timestamp: new Date().toISOString(), values: { baro_alt: 15.04 }, valid: true, noise_level: 0.03, active_fault: null, confidence: 0.96 },
          { sensor_name: 'LiDAR', timestamp: new Date().toISOString(), values: { min_range: 32.4, beams_valid: 16 }, valid: true, noise_level: 0.01, active_fault: null, confidence: 0.99 }
        ];

        if (!prev) {
          return {
            drone: {
              x: scenario.home_x,
              y: scenario.home_y,
              altitude: 15.0,
              heading: 45.0,
              velocity_x: 1.2,
              velocity_y: 0.8,
              vertical_speed: 0.0,
              battery_pct: 99.8,
              flight_mode: 'InFlight',
              armed: true,
              current_waypoint_idx: 0,
              return_to_home: false,
              emergency: false,
            },
            localization: {
              raw_gps: { x: scenario.home_x, y: scenario.home_y, altitude: 15.0 },
              filtered: { x: scenario.home_x, y: scenario.home_y, altitude: 15.0 },
              confidence: 0.95,
            },
            sensor_readings: readings,
            safety: { is_geofence_safe: true, is_collision_safe: true, min_obstacle_dist: 32.4 },
            active_alerts: [],
            recent_events: [],
            sim_time_sec: 0.05,
            tick_count: 1,
            planned_route: [],
            raw_trail: [],
            filtered_trail: [],
            active_faults: [],
            wind_vector: { x: 2.0, y: 1.0 },
            is_script_running: false,
            current_script_step: 0,
            total_script_steps: 0,
          };
        }
        return {
          ...prev,
          drone: {
            ...prev.drone,
            x: prev.drone.x + 0.1,
            y: prev.drone.y + 0.05,
            battery_pct: Math.max(0, prev.drone.battery_pct - 0.01),
          },
          sensor_readings: readings,
          sim_time_sec: prev.sim_time_sec + 0.05,
          tick_count: prev.tick_count + 1,
        };
      });
    }, 50);
    return () => clearInterval(interval);
  }, [isRunning, scenario]);

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="header-left">
          <span className="app-title">DRIFT</span>
          <span className="app-badge">v0.2.0</span>
          <span className="app-status">{isRunning ? 'SENSORS ACTIVE (20 Hz)' : 'SIMULATION PAUSED'}</span>
        </div>
        <div className="header-right" style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <button className="primary-btn" onClick={() => setIsRunning(!isRunning)}>
            {isRunning ? 'Pause' : 'Start'}
          </button>
          <button className="secondary-btn" onClick={() => { setIsRunning(false); setSnapshot(null); }}>
            Reset
          </button>
          <select
            value={selectedScenarioId}
            onChange={(e) => setSelectedScenarioId(e.target.value as 'urban' | 'canyon')}
            className="scenario-select"
          >
            <option value="urban">Urban Grid Surveillance</option>
            <option value="canyon">Canyon Wind Obstacle Challenge</option>
          </select>
        </div>
      </header>
      <main className="main-viewport" style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <div style={{ flex: 1, position: 'relative' }}>
          <MapView
            scenario={scenario}
            snapshot={snapshot}
            onSetTargetPoint={(x, y) => console.log('Target set:', x, y)}
          />
        </div>
        <div style={{ width: '320px', borderLeft: '1px solid var(--border-color)', overflowY: 'auto' }}>
          <SensorHealth
            readings={snapshot?.sensor_readings || []}
            activeFaults={snapshot?.active_faults || []}
          />
        </div>
      </main>
    </div>
  );
}

export default App;
