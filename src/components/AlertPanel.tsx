//! Alert list component with severity badges, descriptions, and mitigation actions.

import React, { useState } from 'react';
import { ShieldAlert, AlertTriangle, Info, Check } from 'lucide-react';
import { Alert } from '../types';

interface AlertPanelProps {
  alerts: Alert[];
}

export const AlertPanel: React.FC<AlertPanelProps> = ({ alerts }) => {
  const [acknowledgedIds, setAcknowledgedIds] = useState<string[]>([]);

  const toggleAck = (id: string) => {
    setAcknowledgedIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  return (
    <div className="card-section">
      <div className="section-header">
        <span>Active Diagnostic & Safety Alerts</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <ShieldAlert size={13} color={alerts.length > 0 ? '#f59e0b' : '#10b981'} />
          <span className="mono">{alerts.length}</span>
        </div>
      </div>

      {alerts.length === 0 ? (
        <div style={{ padding: '16px 8px', textAlign: 'center', color: '#64748b', fontSize: 11 }}>
          All flight safety rules nominal. No active alerts.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 220, overflowY: 'auto' }}>
          {alerts.map((a) => {
            const isAck = acknowledgedIds.includes(a.id) || a.acknowledged;
            const isCritical = a.severity === 'critical';
            const isWarning = a.severity === 'warning';

            const borderColor = isCritical ? '#ef4444' : isWarning ? '#f59e0b' : '#3b82f6';
            const bgColor = isCritical
              ? 'rgba(239, 68, 68, 0.1)'
              : isWarning
              ? 'rgba(245, 158, 11, 0.1)'
              : 'rgba(59, 130, 246, 0.1)';

            return (
              <div
                key={a.id}
                style={{
                  background: bgColor,
                  border: `1px solid ${borderColor}`,
                  borderRadius: 5,
                  padding: '8px 10px',
                  opacity: isAck ? 0.6 : 1.0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      fontWeight: 700,
                      fontSize: 10,
                      color: borderColor,
                      textTransform: 'uppercase',
                    }}
                  >
                    {isCritical ? (
                      <AlertTriangle size={12} />
                    ) : isWarning ? (
                      <AlertTriangle size={12} />
                    ) : (
                      <Info size={12} />
                    )}
                    {a.alert_type.replace(/_/g, ' ')}
                  </span>
                  <span className="mono" style={{ fontSize: 9, color: '#94a3b8' }}>
                    {a.timestamp}
                  </span>
                </div>

                <div style={{ fontSize: 11, color: '#f8fafc' }}>{a.description}</div>

                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 2 }}>
                  <div style={{ fontSize: 10, color: '#94a3b8', fontStyle: 'italic' }}>
                    Action: {a.suggested_action}
                  </div>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ padding: '1px 6px', fontSize: 9 }}
                    onClick={() => toggleAck(a.id)}
                  >
                    <Check size={10} /> {isAck ? 'Acked' : 'Ack'}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
