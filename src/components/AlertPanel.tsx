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
          <ShieldAlert size={13} color={alerts.length > 0 ? '#f87171' : '#ffffff'} />
          <span className="mono">{alerts.length}</span>
        </div>
      </div>

      {alerts.length === 0 ? (
        <div style={{ padding: '16px 8px', textAlign: 'center', color: '#71717a', fontSize: 11 }}>
          All flight safety rules nominal. No active alerts.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 220, overflowY: 'auto' }}>
          {alerts.map((a) => {
            const isAck = acknowledgedIds.includes(a.id) || a.acknowledged;
            const isCritical = a.severity === 'critical';
            const isWarning = a.severity === 'warning';

            const severityColor = isCritical ? '#f87171' : isWarning ? '#fbbf24' : '#ffffff';
            const bgColor = isCritical ? '#241212' : isWarning ? '#221910' : '#181818';

            return (
              <div
                key={a.id}
                style={{
                  background: bgColor,
                  border: 'none',
                  borderRadius: 12,
                  padding: '10px 12px',
                  opacity: isAck ? 0.5 : 1.0,
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
                      gap: 6,
                      fontWeight: 700,
                      fontSize: 10,
                      color: severityColor,
                      textTransform: 'uppercase',
                    }}
                  >
                    {isCritical || isWarning ? (
                      <AlertTriangle size={12} color={severityColor} />
                    ) : (
                      <Info size={12} color={severityColor} />
                    )}
                    {a.alert_type.replace(/_/g, ' ')}
                  </span>
                  <span className="mono" style={{ fontSize: 9, color: '#71717a' }}>
                    {a.timestamp}
                  </span>
                </div>

                <div style={{ fontSize: 11, color: '#ffffff' }}>{a.description}</div>

                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 4 }}>
                  <div style={{ fontSize: 10, color: '#a1a1aa' }}>
                    Action: {a.suggested_action}
                  </div>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ padding: '2px 8px', fontSize: 9, borderRadius: 9999 }}
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
