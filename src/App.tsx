import React from 'react';
import './App.css';

export function App() {
  return (
    <div className="drift-app">
      <header className="drift-header">
        <h1 className="brand-title">DRIFT</h1>
        <p className="brand-subtitle">Autonomous Drone Mission Simulator</p>
      </header>
      <main style={{ padding: '24px' }}>
        <p>Initializing mission workspace...</p>
      </main>
    </div>
  );
}

export default App;
