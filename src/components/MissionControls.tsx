//! Left Panel: Mission controls, scenario selection, sensor fault injection, and script runner.

import React, { useState } from 'react';
import {
  Play,
  Pause,
  RotateCcw,
  ArrowUpCircle,
  ArrowDownCircle,
  Home,
  AlertOctagon,
  Terminal,
  Zap,
  CheckCircle2,
  AlertTriangle,
  Sliders,
  Settings,
} from 'lucide-react';
import {
  armDrone,
  disarmDrone,
  emergencyLand,
  land,
  returnToHome,
  takeoff,
  selectWaypoint,
  injectSensorFault,
  clearSensorFault,
  clearAllSensorFaults,
  setSimulationSpeed,
  runMissionScript,
  loadScenarioById,
  resetSimulation,
  startSimulation,
  pauseSimulation,
} from '../api';
import { FaultType, ScenarioConfig, SimulationSnapshot } from '../types';

interface MissionControlsProps {
  scenario: ScenarioConfig;
  snapshot: SimulationSnapshot | null;
  onScenarioChange: (scenario: ScenarioConfig) => void;
}

export const MissionControls: React.FC<MissionControlsProps> = ({
  scenario,
  snapshot,
  onScenarioChange,
}) => {
  const [takeoffAlt, setTakeoffAlt] = useState<number>(15);
  const [activeSpeed, setActiveSpeed] = useState<number>(1.0);
  const [scriptText, setScriptText] = useState<string>(
    `# Autonomous Mission Script
TAKEOFF 20
WAIT 3
GOTO 190 90 25
WAIT 4
INJECT GPS_DRIFT
WAIT 6
GOTO 330 200 30
WAIT 4
CLEAR_FAULTS
RETURN_HOME`
  );
  const [scriptStatus, setScriptStatus] = useState<string>('');

  const isArmed = snapshot?.drone.armed ?? false;
  const activeFaults = snapshot?.active_faults || [];

  const handleScenarioSelect = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    try {
      const sc = await loadScenarioById(e.target.value);
      onScenarioChange(sc);
    } catch (err) {
      console.error(err);
    }
  };

  const handleSpeedChange = (spd: number) => {
    setActiveSpeed(spd);
    setSimulationSpeed(spd);
  };

  const toggleFault = async (fault: FaultType) => {
    if (activeFaults.includes(fault)) {
      await clearSensorFault(fault);
    } else {
      await injectSensorFault(fault);
    }
  };

  const handleRunScript = async () => {
    try {
      const count = await runMissionScript(scriptText);
      setScriptStatus(`Running script (${count} steps)`);
    } catch (err: any) {
      setScriptStatus(`Error: ${err}`);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: 8 }}>
      {/* 1. Scenario Selector */}
      <div className="card-section">
        <div className="section-header">
          <span>Mission Scenario</span>
          <span className="mono" style={{ fontSize: 10 }}>SEED: {scenario.seed}</span>
        </div>
        <select
          className="form-input"
          value={scenario.id}
          onChange={handleScenarioSelect}
          style={{ cursor: 'pointer' }}
        >
          <option value="SCN-URBAN-01">Urban Grid Surveillance (400m)</option>
          <option value="SCN-CANYON-02">Canyon Wind Obstacle Challenge</option>
        </select>
        <div style={{ fontSize: 11, color: '#a1a1aa', lineHeight: 1.4 }}>
          {scenario.description}
        </div>
      </div>

      {/* 2. Simulation Execution Controls */}
      <div className="card-section">
        <div className="section-header">
          <span>Simulation Engine</span>
          <span className="mono">{activeSpeed}x SPEED</span>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 6 }}>
          <button className="btn btn-primary btn-sm" onClick={startSimulation}>
            <Play size={12} /> Start
          </button>
          <button className="btn btn-secondary btn-sm" onClick={pauseSimulation}>
            <Pause size={12} /> Pause
          </button>
          <button className="btn btn-secondary btn-sm" onClick={resetSimulation}>
            <RotateCcw size={12} /> Reset
          </button>
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          {[0.5, 1.0, 2.0, 5.0].map((spd) => (
            <button
              key={spd}
              className={`btn btn-sm ${activeSpeed === spd ? 'btn-primary' : 'btn-secondary'}`}
              style={{ flex: 1, borderRadius: 9999, fontSize: 10, padding: '3px 0' }}
              onClick={() => handleSpeedChange(spd)}
            >
              {spd}x
            </button>
          ))}
        </div>
      </div>

      {/* 3. Flight Commands */}
      <div className="card-section">
        <div className="section-header">
          <span>Flight Control Guidance</span>
          <span className="mono" style={{ color: isArmed ? '#ffffff' : '#71717a' }}>
            {isArmed ? 'ARMED' : 'DISARMED'}
          </span>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
          {!isArmed ? (
            <button className="btn btn-secondary" onClick={() => armDrone()}>
              <Zap size={14} /> Arm Motors
            </button>
          ) : (
            <button className="btn btn-secondary" onClick={() => disarmDrone()}>
              <Zap size={14} /> Disarm
            </button>
          )}

          <button
            className="btn btn-primary"
            onClick={async () => {
              if (!isArmed) {
                await armDrone();
              }
              await takeoff(takeoffAlt);
            }}
          >
            <ArrowUpCircle size={14} /> Takeoff
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 11, color: '#a1a1aa' }}>Target Alt:</span>
          <input
            type="number"
            className="form-input"
            style={{ width: 70, padding: '4px 8px' }}
            value={takeoffAlt}
            min={5}
            max={80}
            onChange={(e) => setTakeoffAlt(Number(e.target.value))}
          />
          <span style={{ fontSize: 11, color: '#71717a' }}>meters</span>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
          <button className="btn btn-secondary btn-sm" onClick={() => land()}>
            <ArrowDownCircle size={13} /> Land
          </button>
          <button className="btn btn-secondary btn-sm" onClick={() => returnToHome()}>
            <Home size={13} /> Return Home
          </button>
        </div>

        <button className="btn btn-danger btn-sm" onClick={() => emergencyLand()}>
          <AlertOctagon size={14} /> Emergency Land
        </button>
      </div>

      {/* 4. Waypoints Navigation */}
      <div className="card-section">
        <div className="section-header">
          <span>Scenario Waypoints ({scenario.waypoints.length})</span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
          {scenario.waypoints.map((wp) => {
            const isActive = snapshot?.drone.current_waypoint_index === wp.id;
            return (
              <div
                key={wp.id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '7px 10px',
                  background: isActive ? '#ffffff' : '#181818',
                  color: isActive ? '#000000' : '#ffffff',
                  borderRadius: 10,
                }}
              >
                <div>
                  <span className="mono" style={{ fontWeight: 700, color: isActive ? '#000000' : '#ffffff' }}>
                    WP #{wp.id}
                  </span>
                  <span style={{ fontSize: 10, color: isActive ? '#3f3f46' : '#71717a', marginLeft: 8 }}>
                    ({wp.x}m, {wp.y}m, {wp.altitude}m)
                  </span>
                </div>
                <button
                  className={`btn btn-sm ${isActive ? 'btn-secondary' : 'btn-primary'}`}
                  onClick={() => selectWaypoint(wp.id)}
                >
                  Fly A*
                </button>
              </div>
            );
          })}
        </div>
      </div>

      {/* 5. Sensor Fault Injection Matrix */}
      <div className="card-section">
        <div className="section-header">
          <span>Deterministic Fault Injection</span>
          {activeFaults.length > 0 && (
            <span className="mono" style={{ color: '#ffffff' }}>
              {activeFaults.length} ACTIVE
            </span>
          )}
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
          {[
            { id: 'gps_drift' as FaultType, label: 'GPS Drift' },
            { id: 'gps_dropout' as FaultType, label: 'GPS Dropout' },
            { id: 'imu_bias' as FaultType, label: 'IMU Bias' },
            { id: 'compass_failure' as FaultType, label: 'Compass Failure' },
            { id: 'altimeter_drift' as FaultType, label: 'Altimeter Drift' },
            { id: 'lidar_blind_spot' as FaultType, label: 'LiDAR Blind Spot' },
            { id: 'excessive_noise' as FaultType, label: 'Excessive Noise' },
          ].map((f) => {
            const isActive = activeFaults.includes(f.id);
            return (
              <button
                key={f.id}
                className={`btn btn-sm ${isActive ? 'btn-danger' : 'btn-secondary'}`}
                onClick={() => toggleFault(f.id)}
                style={{ textAlign: 'left', justifyContent: 'flex-start' }}
              >
                {isActive ? <AlertTriangle size={12} /> : <Sliders size={12} />}
                <span>{f.label}</span>
              </button>
            );
          })}
        </div>

        <button
          className="btn btn-secondary btn-sm"
          onClick={() => clearAllSensorFaults()}
          disabled={activeFaults.length === 0}
        >
          <CheckCircle2 size={13} /> Clear All Faults
        </button>
      </div>

      {/* 6. Script Runner */}
      <div className="card-section">
        <div className="section-header">
          <span>Mission Command Script Runner</span>
          <Terminal size={13} />
        </div>
        <textarea
          className="form-input mono textarea-input"
          style={{ fontSize: 11, lineHeight: 1.4 }}
          value={scriptText}
          onChange={(e) => setScriptText(e.target.value)}
        />
        {scriptStatus && (
          <div style={{ fontSize: 11, color: '#ffffff' }}>{scriptStatus}</div>
        )}
        <div style={{ display: 'flex', gap: 6 }}>
          <button className="btn btn-primary btn-sm" style={{ flex: 1 }} onClick={handleRunScript}>
            <Play size={12} /> Execute Script
          </button>
          <button
            className="btn btn-secondary btn-sm"
            onClick={() =>
              setScriptText(`TAKEOFF 15\nWAIT 3\nGOTO 190 90 20\nINJECT LIDAR_BLIND_SPOT\nWAIT 5\nRETURN_HOME`)
            }
          >
            Preset 2
          </button>
        </div>
      </div>

      {/* 7. Drone Configuration Specifications */}
      <div className="card-section">
        <div className="section-header">
          <span>Quadcopter Specifications</span>
          <Settings size={13} />
        </div>
        <div className="metric-row">
          <span className="metric-label">Mass (All-Up):</span>
          <span className="metric-val">1.85 kg</span>
        </div>
        <div className="metric-row">
          <span className="metric-label">Max Horiz Speed:</span>
          <span className="metric-val">12.0 m/s</span>
        </div>
        <div className="metric-row">
          <span className="metric-label">Max Climb Rate:</span>
          <span className="metric-val">3.5 m/s</span>
        </div>
        <div className="metric-row">
          <span className="metric-label">Battery Capacity:</span>
          <span className="metric-val">5200 mAh (4S)</span>
        </div>
        <div className="metric-row">
          <span className="metric-label">Collision Radius:</span>
          <span className="metric-val">0.75 m</span>
        </div>
      </div>
    </div>
  );
};
