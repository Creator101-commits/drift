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
                <td style={{ fontWeight: 600, color: '#f8fafc' }}>
                  {r.sensor_name.split('-')[0]}
                  <div style={{ fontSize: 9, color: '#64748b' }}>{r.sensor_name}</div>
                </td>
                <td>
                  {isNominal ? (
                    <span style={{ color: '#10b981', display: 'flex', alignItems: 'center', gap: 4 }}>
                      <CheckCircle2 size={12} /> NOMINAL
                    </span>
                  ) : isDegraded ? (
                    <span style={{ color: '#f59e0b', display: 'flex', alignItems: 'center', gap: 4 }}>
                      <AlertTriangle size={12} /> DEGRADED
                    </span>
                  ) : (
                    <span style={{ color: '#ef4444', display: 'flex', alignItems: 'center', gap: 4 }}>
                      <XCircle size={12} /> FAULT
                    </span>
                  )}
                </td>
                <td className="mono" style={{ fontSize: 10, color: '#94a3b8' }}>
                  {metricsText}
                </td>
                <td>
                  {r.active_fault ? (
                    <span
                      style={{
                        background: 'rgba(239, 68, 68, 0.15)',
                        color: '#f87171',
                        border: '1px solid #ef4444',
                        padding: '1px 5px',
                        borderRadius: 3,
                        fontSize: 9,
                        textTransform: 'uppercase',
                        fontFamily: 'monospace',
                      }}
                    >
                      {r.active_fault.replace('_', ' ')}
                    </span>
                  ) : (
                    <span style={{ color: '#64748b', fontSize: 10 }}>--</span>
                  )}
                </td>
                <td>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <div
                      style={{
                        width: 45,
                        height: 4,
                        background: '#1e293b',
                        borderRadius: 2,
                        overflow: 'hidden',
                      }}
                    >
                      <div
                        style={{
                          width: `${Math.round(r.confidence * 100)}%`,
                          height: '100%',
                          background:
                            r.confidence > 0.8
                              ? '#10b981'
                              : r.confidence > 0.5
                              ? '#f59e0b'
                              : '#ef4444',
                        }}
                      />
                    </div>
                    <span className="mono" style={{ fontSize: 10 }}>
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
