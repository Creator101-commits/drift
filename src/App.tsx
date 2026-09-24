import { useEffect, useState } from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import {
  Activity,
  Battery,
  Clock,
  History,
  Navigation,
  Radio,
  Save,
  ShieldAlert,
} from 'lucide-react';
import {
  getScenarioConfig,
  getSimulationState,
  onAlertFired,
  onDroneStateChanged,
  onMissionEvent,
  onReplayStateChanged,
  onTelemetrySample,
  planRouteToPoint,
  saveCurrentMission,
  startReplay,
} from './api';
import { AlertPanel } from './components/AlertPanel';
import { EventLog } from './components/EventLog';
import { MapView } from './components/MapView';
import { MissionControls } from './components/MissionControls';
import { MissionHistory } from './components/MissionHistory';
import { ReplayControl } from './components/ReplayControl';
import { SensorHealth } from './components/SensorHealth';
import { TelemetryPanel } from './components/TelemetryPanel';
import {
  ReplayStatus,
  ScenarioConfig,
  SimulationSnapshot,
} from './types';
import './App.css';

export function App() {
  const [scenario, setScenario] = useState<ScenarioConfig | null>(null);
  const [snapshot, setSnapshot] = useState<SimulationSnapshot | null>(null);
  const [leftTab, setLeftTab] = useState<'controls' | 'history'>('controls');
  const [rightTab, setRightTab] = useState<'telemetry' | 'sensors' | 'alerts' | 'events'>('telemetry');

  // Replay state
  const [replayStatus, setReplayStatus] = useState<ReplayStatus>({
    is_active: false,
    is_playing: false,
    current_index: 0,
    total_frames: 0,
    playback_speed: 1.0,
    current_time_sec: 0,
    total_duration_sec: 0,
  });
  const [activeReplayMissionId, setActiveReplayMissionId] = useState<string | undefined>(undefined);

  // Save Mission Modal
  const [showSaveModal, setShowSaveModal] = useState<boolean>(false);
  const [saveMissionName, setSaveMissionName] = useState<string>('');
  const [saveMissionNotes, setSaveMissionNotes] = useState<string>('');
  const [saveSuccessMsg, setSaveSuccessMsg] = useState<string>('');

  // Initial load
  useEffect(() => {
    getScenarioConfig().then((sc) => setScenario(sc));
    getSimulationState().then((s) => {
      if (s) setSnapshot(s);
    });

    // Subscriptions to Tauri events
    let unlistenSample: (() => void) | undefined;
    let unlistenDrone: (() => void) | undefined;
    let unlistenEvent: (() => void) | undefined;
    let unlistenAlert: (() => void) | undefined;
    let unlistenReplay: (() => void) | undefined;

    onTelemetrySample((snap) => setSnapshot(snap)).then((u) => (unlistenSample = u));
    onDroneStateChanged((d) => {
      setSnapshot((prev) => (prev ? { ...prev, drone: d } : prev));
    }).then((u) => (unlistenDrone = u));

    onMissionEvent((evt) => {
      setSnapshot((prev) => {
        if (!prev) return prev;
        const exists = prev.recent_events.some((e) => e.id === evt.id);
        if (exists) return prev;
        return { ...prev, recent_events: [evt, ...prev.recent_events] };
      });
    }).then((u) => (unlistenEvent = u));

    onAlertFired((alrt) => {
      setSnapshot((prev) => {
        if (!prev) return prev;
        const exists = prev.active_alerts.some((a) => a.id === alrt.id);
        if (exists) return prev;
        return { ...prev, active_alerts: [alrt, ...prev.active_alerts] };
      });
    }).then((u) => (unlistenAlert = u));

    onReplayStateChanged((rep) => setReplayStatus(rep)).then((u) => (unlistenReplay = u));

    return () => {
      unlistenSample?.();
      unlistenDrone?.();
      unlistenEvent?.();
      unlistenAlert?.();
      unlistenReplay?.();
    };
  }, []);

  const handleStartReplay = async (missionId: string) => {
    try {
      setActiveReplayMissionId(missionId);
      const rep = await startReplay(missionId);
      setReplayStatus(rep);
    } catch (err) {
      console.error(err);
    }
  };

  const handleTargetPoint = async (x: number, y: number) => {
    try {
      await planRouteToPoint(x, y, snapshot?.drone.altitude ?? 20);
    } catch (err) {
      console.error(err);
    }
  };

  const handleSaveMission = async () => {
    try {
      const id = await saveCurrentMission(saveMissionName, saveMissionNotes);
      setSaveSuccessMsg(`Saved as ${id}`);
      setTimeout(() => {
        setShowSaveModal(false);
        setSaveSuccessMsg('');
        setSaveMissionName('');
        setSaveMissionNotes('');
      }, 1200);
    } catch (err: any) {
      setSaveSuccessMsg(`Error: ${err}`);
    }
  };

  const flightMode = snapshot?.drone.flight_mode ?? 'disarmed';
  const batteryPct = snapshot?.drone.battery_percent ?? 100;
  const simTime = snapshot?.sim_time_sec ?? 0;

  if (!scenario) {
    return (
      <div style={{ padding: 40, color: '#06b6d4', fontFamily: 'monospace' }}>
        Loading Drift Mission Simulator...
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', width: '100vw' }}>
      {/* 1. Header Toolbar */}
      <header className="app-header">
        <div className="brand-section">
          <div className="brand-title">Drift</div>
          <div className="brand-subtitle">Autonomous Drone Telemetry Analyzer</div>
        </div>

        <div className="header-status-group">
          {/* Flight Mode Badge */}
          <div className={`mode-badge mode-${flightMode}`}>
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                backgroundColor: 'currentColor',
              }}
            />
            {flightMode.replace(/_/g, ' ')}
          </div>

          {/* Clock */}
          <div className="header-clock" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <Clock size={14} color="#06b6d4" />
            <span>T+{simTime.toFixed(1)}s</span>
          </div>

          {/* Battery */}
          <div
            className="mono"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 12,
              color: batteryPct > 30 ? '#10b981' : '#ef4444',
            }}
          >
            <Battery size={14} />
            <span>{batteryPct.toFixed(1)}%</span>
          </div>

          {/* Save Mission Button */}
          <button
            className="btn btn-primary btn-sm"
            onClick={() => setShowSaveModal(true)}
            disabled={!snapshot || snapshot.sim_time_sec <= 0}
          >
            <Save size={13} /> Save Run
          </button>
        </div>
      </header>

      {/* 2. Main Three-Panel Resizable Layout */}
      <div className="main-layout">
        <PanelGroup direction="horizontal">
          {/* Left Panel: Controls & History */}
          <Panel defaultSize={26} minSize={20} maxSize={38}>
            <div className="panel-container">
              {/* Left Tab Switcher */}
              <div
                style={{
                  display: 'flex',
                  borderBottom: '1px solid #1e293b',
                  background: '#0a0f18',
                }}
              >
                <button
                  className={`btn btn-sm ${leftTab === 'controls' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0 }}
                  onClick={() => setLeftTab('controls')}
                >
                  <Navigation size={12} /> Controls
                </button>
                <button
                  className={`btn btn-sm ${leftTab === 'history' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0 }}
                  onClick={() => setLeftTab('history')}
                >
                  <History size={12} /> History
                </button>
              </div>

              {leftTab === 'controls' ? (
                <MissionControls
                  scenario={scenario}
                  snapshot={snapshot}
                  onScenarioChange={(newSc) => setScenario(newSc)}
                />
              ) : (
                <MissionHistory onStartReplay={handleStartReplay} />
              )}
            </div>
          </Panel>

          <PanelResizeHandle className="panel-resize-handle" />

          {/* Center Panel: Tactical 2D Map */}
          <Panel defaultSize={48} minSize={32}>
            <div className="panel-container" style={{ position: 'relative' }}>
              <MapView
                scenario={scenario}
                snapshot={snapshot}
                onSetTargetPoint={handleTargetPoint}
              />

              {/* Floating Replay Bar */}
              {replayStatus.is_active && (
                <ReplayControl
                  replayStatus={replayStatus}
                  activeMissionId={activeReplayMissionId}
                  onClose={() => {
                    setReplayStatus((prev) => ({ ...prev, is_active: false }));
                    setActiveReplayMissionId(undefined);
                  }}
                />
              )}
            </div>
          </Panel>

          <PanelResizeHandle className="panel-resize-handle" />

          {/* Right Panel: Telemetry, Sensors, Alerts, Event Log */}
          <Panel defaultSize={26} minSize={22} maxSize={40}>
            <div className="panel-container">
              {/* Right Tab Switcher */}
              <div
                style={{
                  display: 'flex',
                  borderBottom: '1px solid #1e293b',
                  background: '#0a0f18',
                }}
              >
                <button
                  className={`btn btn-sm ${rightTab === 'telemetry' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0, padding: '6px 2px', fontSize: 11 }}
                  onClick={() => setRightTab('telemetry')}
                  title="Live Telemetry"
                >
                  <Activity size={11} /> Telemetry
                </button>
                <button
                  className={`btn btn-sm ${rightTab === 'sensors' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0, padding: '6px 2px', fontSize: 11 }}
                  onClick={() => setRightTab('sensors')}
                  title="Sensor Health"
                >
                  <Radio size={11} /> Sensors
                </button>
                <button
                  className={`btn btn-sm ${rightTab === 'alerts' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0, padding: '6px 2px', fontSize: 11 }}
                  onClick={() => setRightTab('alerts')}
                  title="Safety Alerts"
                >
                  <ShieldAlert size={11} /> Alerts ({snapshot?.active_alerts.length ?? 0})
                </button>
                <button
                  className={`btn btn-sm ${rightTab === 'events' ? 'btn-primary' : 'btn-secondary'}`}
                  style={{ flex: 1, borderRadius: 0, padding: '6px 2px', fontSize: 11 }}
                  onClick={() => setRightTab('events')}
                  title="Event Log"
                >
                  Log
                </button>
              </div>

              {/* Active Tab View */}
              {rightTab === 'telemetry' && <TelemetryPanel snapshot={snapshot} />}
              {rightTab === 'sensors' && (
                <SensorHealth readings={snapshot?.sensor_readings ?? []} />
              )}
              {rightTab === 'alerts' && (
                <AlertPanel alerts={snapshot?.active_alerts ?? []} />
              )}
              {rightTab === 'events' && (
                <EventLog events={snapshot?.recent_events ?? []} />
              )}
            </div>
          </Panel>
        </PanelGroup>
      </div>

      {/* 3. Save Mission to SQLite Dialog Modal */}
      {showSaveModal && (
        <div className="modal-overlay">
          <div className="modal-content">
            <div className="modal-title">Save Mission Run to SQLite Database</div>
            <div>
              <label style={{ fontSize: 11, color: '#94a3b8', display: 'block', marginBottom: 4 }}>
                Mission Name
              </label>
              <input
                type="text"
                className="form-input"
                placeholder="e.g. Urban Patrol Trial 1"
                value={saveMissionName}
                onChange={(e) => setSaveMissionName(e.target.value)}
              />
            </div>
            <div>
              <label style={{ fontSize: 11, color: '#94a3b8', display: 'block', marginBottom: 4 }}>
                Engineering Notes / Description
              </label>
              <textarea
                className="form-input textarea-input"
                placeholder="Observed sensor drift recovery, geofence compliance, etc."
                value={saveMissionNotes}
                onChange={(e) => setSaveMissionNotes(e.target.value)}
              />
            </div>

            {saveSuccessMsg && (
              <div style={{ fontSize: 12, color: '#38bdf8' }}>{saveSuccessMsg}</div>
            )}

            <div className="modal-actions">
              <button
                className="btn btn-secondary"
                onClick={() => setShowSaveModal(false)}
              >
                Cancel
              </button>
              <button className="btn btn-primary" onClick={handleSaveMission}>
                <Save size={14} /> Commit to SQLite
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
