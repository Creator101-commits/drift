//! Live timeseries telemetry charts and primary flight display gauges.

import React, { useRef, useEffect, useState } from 'react';
import { Battery, Activity, Compass, Navigation } from 'lucide-react';
import { SimulationSnapshot } from '../types';

interface TelemetryPanelProps {
  snapshot: SimulationSnapshot | null;
}

interface DataHistory {
  times: number[];
  altitude: number[];
  speed: number[];
  battery: number[];
  divergence: number[];
}

export const TelemetryPanel: React.FC<TelemetryPanelProps> = ({ snapshot }) => {
  const chartCanvasRef = useRef<HTMLCanvasElement | null>(null);

  const [history, setHistory] = useState<DataHistory>({
    times: [],
    altitude: [],
    speed: [],
    battery: [],
    divergence: [],
  });

  // Keep rolling 60 samples for charts
  useEffect(() => {
    if (!snapshot) return;

    setHistory((prev) => {
      const maxLen = 60;
      const times = [...prev.times, snapshot.sim_time_sec].slice(-maxLen);
      const altitude = [...prev.altitude, snapshot.drone.altitude].slice(-maxLen);
      const speed = [...prev.speed, snapshot.drone.horizontal_speed].slice(-maxLen);
      const battery = [...prev.battery, snapshot.drone.battery_percent].slice(-maxLen);
      const divergence = [
        ...prev.divergence,
        snapshot.localization.estimation_error_m,
      ].slice(-maxLen);

      return { times, altitude, speed, battery, divergence };
    });
  }, [snapshot?.tick_count]);

  // Render rolling canvas graph
  useEffect(() => {
    const canvas = chartCanvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    ctx.clearRect(0, 0, width, height);

    // Dark background
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, width, height);

    // Draw horizontal grid lines
    ctx.strokeStyle = '#171717';
    ctx.lineWidth = 1;
    for (let y = 0; y <= height; y += height / 4) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    if (history.altitude.length < 2) return;

    const count = history.altitude.length;
    const stepX = width / (count - 1);

    // 1. Draw Altitude series (White)
    const maxAlt = Math.max(...history.altitude, 30.0);
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    history.altitude.forEach((val, i) => {
      const x = i * stepX;
      const y = height - (val / maxAlt) * (height - 20) - 10;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // 2. Draw Speed series (Gray)
    const maxSpd = Math.max(...history.speed, 12.0);
    ctx.strokeStyle = '#a1a1aa';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    history.speed.forEach((val, i) => {
      const x = i * stepX;
      const y = height - (val / maxSpd) * (height - 20) - 10;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // 3. Draw GPS Divergence series (Subtle Muted Gray)
    const maxDiv = Math.max(...history.divergence, 10.0);
    ctx.strokeStyle = '#52525b';
    ctx.lineWidth = 1.5;
    ctx.setLineDash([2, 2]);
    ctx.beginPath();
    history.divergence.forEach((val, i) => {
      const x = i * stepX;
      const y = height - (val / maxDiv) * (height - 20) - 10;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
    ctx.setLineDash([]);
  }, [history]);

  const drone = snapshot?.drone;
  const loc = snapshot?.localization;

  const batteryColor =
    (drone?.battery_percent ?? 100) > 40
      ? '#ffffff'
      : (drone?.battery_percent ?? 100) > 20
      ? '#fbbf24'
      : '#f87171';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: 8 }}>
      {/* Primary Flight Display (PFD) Readouts */}
      <div className="card-section">
        <div className="section-header">
          <span>Flight Dynamics & Telemetry</span>
          <Activity size={13} />
        </div>

        {/* Status Highlights Grid */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <div
            style={{
              background: '#181818',
              border: 'none',
              borderRadius: 12,
              padding: '10px 14px',
            }}
          >
            <div style={{ fontSize: 10, textTransform: 'uppercase', color: '#a1a1aa' }}>
              Altitude (MSL)
            </div>
            <div className="mono" style={{ fontSize: 20, fontWeight: 700, color: '#ffffff' }}>
              {drone ? drone.altitude.toFixed(1) : '0.0'}
              <span style={{ fontSize: 11, color: '#71717a', marginLeft: 4 }}>m</span>
            </div>
            <div style={{ fontSize: 10, color: '#71717a', marginTop: 2 }}>
              Climb: {drone ? drone.vertical_speed.toFixed(1) : '0.0'} m/s
            </div>
          </div>

          <div
            style={{
              background: '#181818',
              border: 'none',
              borderRadius: 12,
              padding: '10px 14px',
            }}
          >
            <div style={{ fontSize: 10, textTransform: 'uppercase', color: '#a1a1aa' }}>
              Ground Speed
            </div>
            <div className="mono" style={{ fontSize: 20, fontWeight: 700, color: '#ffffff' }}>
              {drone ? drone.horizontal_speed.toFixed(1) : '0.0'}
              <span style={{ fontSize: 11, color: '#71717a', marginLeft: 4 }}>m/s</span>
            </div>
            <div style={{ fontSize: 10, color: '#71717a', marginTop: 2 }}>
              Distance: {drone ? drone.distance_traveled.toFixed(0) : '0'} m
            </div>
          </div>
        </div>

        {/* Battery Capacity Gauge */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 5, marginTop: 4 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11 }}>
            <span style={{ color: '#a1a1aa', display: 'flex', alignItems: 'center', gap: 6 }}>
              <Battery size={13} color={batteryColor} /> Battery Energy Reserve
            </span>
            <span className="mono" style={{ fontWeight: 700, color: batteryColor }}>
              {drone ? drone.battery_percent.toFixed(1) : '100.0'}%
            </span>
          </div>
          <div
            style={{
              width: '100%',
              height: 6,
              background: '#222222',
              borderRadius: 9999,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${drone ? drone.battery_percent : 100}%`,
                height: '100%',
                backgroundColor: batteryColor,
                borderRadius: 9999,
                transition: 'width 0.2s linear',
              }}
            />
          </div>
        </div>

        {/* Heading and Compass */}
        <div className="metric-row">
          <span className="metric-label" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <Compass size={13} /> Heading Yaw:
          </span>
          <span className="metric-val">{drone ? drone.heading.toFixed(1) : '0.0'} deg</span>
        </div>

        <div className="metric-row">
          <span className="metric-label">Flight Duration:</span>
          <span className="metric-val">{drone ? drone.flight_time_seconds.toFixed(1) : '0.0'} s</span>
        </div>
      </div>

      {/* Raw vs Filtered Localization Audit */}
      <div className="card-section">
        <div className="section-header">
          <span>Localization Filter Audit</span>
          <Navigation size={13} />
        </div>

        <div className="metric-row">
          <span className="metric-label">Filtered Position (X, Y):</span>
          <span className="metric-val" style={{ color: '#ffffff' }}>
            ({loc ? loc.filtered_x.toFixed(1) : '50.0'}, {loc ? loc.filtered_y.toFixed(1) : '50.0'}) m
          </span>
        </div>

        <div className="metric-row">
          <span className="metric-label">Raw GPS Position (X, Y):</span>
          <span className="metric-val" style={{ color: '#a1a1aa' }}>
            ({loc ? loc.raw_x.toFixed(1) : '50.0'}, {loc ? loc.raw_y.toFixed(1) : '50.0'}) m
          </span>
        </div>

        <div className="metric-row">
          <span className="metric-label">GPS / Inertial Divergence:</span>
          <span
            className="metric-val"
            style={{
              color: (loc?.estimation_error_m ?? 0) > 5.0 ? '#f87171' : '#ffffff',
            }}
          >
            {loc ? loc.estimation_error_m.toFixed(2) : '0.00'} m
          </span>
        </div>
      </div>

      {/* Rolling Telemetry Graph */}
      <div className="card-section">
        <div className="section-header">
          <span>Real-time Telemetry Trend (Last 60 ticks)</span>
        </div>
        <div style={{ display: 'flex', gap: 14, fontSize: 10, marginBottom: 6 }}>
          <span style={{ color: '#ffffff' }}>-- Altitude</span>
          <span style={{ color: '#a1a1aa' }}>-- Speed</span>
          <span style={{ color: '#71717a' }}>.. GPS Divergence</span>
        </div>
        <canvas
          ref={chartCanvasRef}
          width={280}
          height={110}
          style={{ width: '100%', height: 110, borderRadius: 12 }}
        />
      </div>
    </div>
  );
};
