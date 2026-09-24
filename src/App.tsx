import { useEffect, useState } from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import {
  Activity,
  Battery,
  Clock,
  Navigation,
  Radio,
} from 'lucide-react';
import {
  getScenarioConfig,
  getSimulationState,
  onDroneStateChanged,
  onMissionEvent,
  onTelemetrySample,
  planRouteToPoint,
} from './api';
import { EventLog } from './components/EventLog';
import { MapView } from './components/MapView';
import { MissionControls } from './components/MissionControls';
import { SensorHealth } from './components/SensorHealth';
import { TelemetryPanel } from './components/TelemetryPanel';
import {
  ScenarioConfig,
  SimulationSnapshot,
} from './types';
import './App.css';

export function App() {
  const [scenario, setScenario] = useState<ScenarioConfig | null>(null);
  const [snapshot, setSnapshot] = useState<SimulationSnapshot | null>(null);
  const [rightTab, setRightTab] = useState<'telemetry' | 'sensors' | 'events'>('telemetry');

  useEffect(() => {
    let isMounted = true;
    getScenarioConfig('urban').then((cfg) => {
      if (isMounted) setScenario(cfg);
    });

    const unlistenState = onDroneStateChanged(() => {});
    const unlistenTelem = onTelemetrySample((s) => {
      if (isMounted) setSnapshot(s);
    });
    const unlistenEvent = onMissionEvent(() => {});

    return () => {
      isMounted = false;
      unlistenState.then((u) => u());
      unlistenTelem.then((u) => u());
      unlistenEvent.then((u) => u());
    };
  }, []);

  const handleSetTargetPoint = async (x: number, y: number) => {
    try {
      await planRouteToPoint(x, y, snapshot?.drone.altitude || 15.0);
    } catch (err) {
      console.error('Failed to set target point:', err);
    }
  };

  const drone = snapshot?.drone;
  const simTimeSec = snapshot?.sim_time_sec || 0;

  return (
    <div className="drift-app">
      <header className="drift-header">
        <div className="header-brand">
          <div className="brand-logo">
            <Navigation className="icon" size={20} />
          </div>
          <div>
            <h1 className="brand-title">DRIFT</h1>
            <p className="brand-subtitle">Autonomous Drone Mission Simulator</p>
          </div>
        </div>

        <div className="header-indicators">
          <div className="status-badge live">
            <span className="status-dot"></span>
            LIVE 20 HZ
          </div>
          <div className="indicator-chip">
            <Clock size={14} />
            <span>{simTimeSec.toFixed(1)}s</span>
          </div>
          <div className="indicator-chip">
            <Battery size={14} />
            <span>{drone?.battery_pct.toFixed(0) || 100}%</span>
          </div>
          <div className="indicator-chip">
            <Activity size={14} />
            <span>{drone?.flight_mode || 'Grounded'}</span>
          </div>
        </div>
      </header>

      <main className="drift-main">
        {scenario && (
          <PanelGroup direction="horizontal">
            <Panel defaultSize={22} minSize={16} maxSize={30} className="panel left-panel">
              <MissionControls
                scenario={scenario}
                snapshot={snapshot}
                onSelectScenario={(s) => setScenario(s)}
              />
            </Panel>

            <PanelResizeHandle className="resize-handle" />

            <Panel defaultSize={52} minSize={30} className="panel center-panel">
              <MapView
                scenario={scenario}
                snapshot={snapshot}
                onSetTargetPoint={handleSetTargetPoint}
              />
            </Panel>

            <PanelResizeHandle className="resize-handle" />

            <Panel defaultSize={26} minSize={20} maxSize={35} className="panel right-panel">
              <div className="tab-nav">
                <button
                  className={`tab-btn ${rightTab === 'telemetry' ? 'active' : ''}`}
                  onClick={() => setRightTab('telemetry')}
                >
                  Telemetry
                </button>
                <button
                  className={`tab-btn ${rightTab === 'sensors' ? 'active' : ''}`}
                  onClick={() => setRightTab('sensors')}
                >
                  Sensors
                </button>
                <button
                  className={`tab-btn ${rightTab === 'events' ? 'active' : ''}`}
                  onClick={() => setRightTab('events')}
                >
                  Events
                </button>
              </div>

              <div className="tab-body">
                {rightTab === 'telemetry' && <TelemetryPanel snapshot={snapshot} />}
                {rightTab === 'sensors' && (
                  <SensorHealth
                    readings={snapshot?.sensor_readings || []}
                    activeFaults={snapshot?.active_faults || []}
                  />
                )}
                {rightTab === 'events' && <EventLog events={snapshot?.recent_events || []} />}
              </div>
            </Panel>
          </PanelGroup>
        )}
      </main>
    </div>
  );
}

export default App;
