//! Sensor health table displaying fidelity, noise std dev, active faults, and confidence.

import React from 'react';
import { Radio, CheckCircle2, AlertTriangle, XCircle } from 'lucide-react';
import { SensorReading } from '../types';

interface SensorHealthProps {
  readings: SensorReading[];
}

export const SensorHealth: React.FC<SensorHealthProps> = ({ readings }) => {
  return (
    <div className="card-section">
      <div className="section-header">
        <span>Integrated Sensor Health Matrix</span>
        <Radio size={13} />
      </div>

      <table className="data-table">
        <thead>
          <tr>
            <th>Sensor</th>
            <th>Health</th>
            <th>Metrics</th>
            <th>Fault</th>
            <th>Confidence</th>
          </tr>
        </thead>
        <tbody>
          {readings.map((r) => {
            const isNominal = r.validity && r.confidence > 0.7 && !r.active_fault;
            const isDegraded = r.validity && (r.confidence <= 0.7 || r.active_fault !== null);

            let metricsText = '';
            if (r.sensor_name.startsWith('GPS')) {
              metricsText = `${r.values.satellites ?? 0} sats | HDOP ${r.values.hdop ?? 0}`;
            } else if (r.sensor_name.startsWith('IMU')) {
              metricsText = `Az: ${r.values.accel_z?.toFixed(1) ?? 0} | Gz: ${r.values.gyro_z?.toFixed(1) ?? 0}°/s`;
            } else if (r.sensor_name.startsWith('Compass')) {
              metricsText = `Heading: ${r.values.heading_deg?.toFixed(0) ?? 0}°`;
            } else if (r.sensor_name.startsWith('Baro')) {
              metricsText = `${r.values.altitude_m?.toFixed(1) ?? 0}m | ${r.values.pressure_hpa?.toFixed(0) ?? 0}hPa`;
            } else if (r.sensor_name.startsWith('LiDAR')) {
              metricsText = `Min: ${r.values.min_distance_m?.toFixed(1) ?? 0}m`;
            }

            return (
              <tr key={r.sensor_name}>
                <td style={{ fontWeight: 600, color: '#ffffff' }}>
                  {r.sensor_name.split('-')[0]}
                  <div style={{ fontSize: 9, color: '#71717a' }}>{r.sensor_name}</div>
                </td>
                <td>
                  {isNominal ? (
                    <span style={{ color: '#ffffff', display: 'flex', alignItems: 'center', gap: 5, fontSize: 10 }}>
                      <CheckCircle2 size={12} color="#22c55e" /> NOMINAL
                    </span>
                  ) : isDegraded ? (
                    <span style={{ color: '#fbbf24', display: 'flex', alignItems: 'center', gap: 5, fontSize: 10 }}>
                      <AlertTriangle size={12} color="#fbbf24" /> DEGRADED
                    </span>
                  ) : (
                    <span style={{ color: '#f87171', display: 'flex', alignItems: 'center', gap: 5, fontSize: 10 }}>
                      <XCircle size={12} color="#f87171" /> FAULT
                    </span>
                  )}
                </td>
                <td className="mono" style={{ fontSize: 10, color: '#a1a1aa' }}>
                  {metricsText}
                </td>
                <td>
                  {r.active_fault ? (
                    <span
                      style={{
                        background: '#2b1414',
                        color: '#f87171',
                        border: 'none',
                        padding: '2px 8px',
                        borderRadius: 9999,
                        fontSize: 9,
                        fontWeight: 700,
                        textTransform: 'uppercase',
                        letterSpacing: 0.5,
                      }}
                    >
                      {r.active_fault.replace('_', ' ')}
                    </span>
                  ) : (
                    <span style={{ color: '#52525b', fontSize: 10 }}>--</span>
                  )}
                </td>
                <td>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <div
                      style={{
                        width: 48,
                        height: 5,
                        background: '#222222',
                        borderRadius: 9999,
                        overflow: 'hidden',
                      }}
                    >
                      <div
                        style={{
                          width: `${Math.round(r.confidence * 100)}%`,
                          height: '100%',
                          borderRadius: 9999,
                          background:
                            r.confidence > 0.8
                              ? '#ffffff'
                              : r.confidence > 0.5
                              ? '#fbbf24'
                              : '#f87171',
                        }}
                      />
                    </div>
                    <span className="mono" style={{ fontSize: 10, color: '#ffffff' }}>
                      {Math.round(r.confidence * 100)}%
                    </span>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};
