import React, { useState } from 'react';
import { MapView } from './components/MapView';
import { ScenarioConfig, SimulationSnapshot } from './types';
import urbanScenario from '../scenarios/urban_surveillance.json';
import canyonScenario from '../scenarios/canyon_wind_obstacle.json';
import './App.css';

export function App() {
  const [selectedScenarioId, setSelectedScenarioId] = useState<'urban' | 'canyon'>('urban');
  const scenario = (selectedScenarioId === 'urban' ? urbanScenario : canyonScenario) as unknown as ScenarioConfig;
  const [snapshot] = useState<SimulationSnapshot | null>(null);

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="header-left">
          <span className="app-title">DRIFT</span>
          <span className="app-badge">v0.1.0</span>
          <span className="app-status">SIMULATOR IDLE</span>
        </div>
        <div className="header-right">
          <label className="scenario-label">Scenario:</label>
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
      <main className="main-viewport" style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
        <MapView
          scenario={scenario}
          snapshot={snapshot}
          onSetTargetPoint={(x, y) => console.log('Target set:', x, y)}
        />
      </main>
    </div>
  );
}

export default App;
