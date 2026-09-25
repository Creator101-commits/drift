//! Chronological mission event log with filtering and auto-scroll.

import React, { useState } from 'react';
import { FileText } from 'lucide-react';
import { MissionEvent } from '../types';

interface EventLogProps {
  events: MissionEvent[];
}

export const EventLog: React.FC<EventLogProps> = ({ events }) => {
  const [filter, setFilter] = useState<string>('ALL');

  const filteredEvents = events.filter((e) => {
    if (filter === 'ALL') return true;
    if (filter === 'FAULTS') return e.event_type.includes('FAULT');
    if (filter === 'NAVIGATION') return e.event_type.includes('ROUTE') || e.event_type.includes('WAYPOINT');
    if (filter === 'COMMANDS') return e.event_type.includes('INITIATED') || e.event_type.includes('SCRIPT');
    return true;
  });

  return (
    <div className="card-section" style={{ flex: 1, minHeight: 180 }}>
      <div className="section-header">
        <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <FileText size={13} /> Mission Event Stream
        </span>
        <div style={{ display: 'flex', gap: 4 }}>
          {['ALL', 'FAULTS', 'NAVIGATION', 'COMMANDS'].map((f) => (
            <button
              key={f}
              className={`btn btn-sm ${filter === f ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 9, padding: '2px 8px', borderRadius: 9999 }}
              onClick={() => setFilter(f)}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
          overflowY: 'auto',
          maxHeight: 220,
        }}
      >
        {filteredEvents.length === 0 ? (
          <div style={{ padding: '12px 8px', textAlign: 'center', color: '#71717a', fontSize: 11 }}>
            No mission events logged yet.
          </div>
        ) : (
          filteredEvents.map((e) => {
            const isFault = e.event_type.includes('FAULT') || e.event_type.includes('COLLISION');
            return (
              <div
                key={e.id}
                style={{
                  display: 'flex',
                  alignItems: 'baseline',
                  gap: 8,
                  padding: '6px 10px',
                  background: '#161616',
                  border: 'none',
                  borderRadius: 10,
                  fontSize: 11,
                }}
              >
                <span className="mono" style={{ fontSize: 9, color: '#71717a', flexShrink: 0 }}>
                  {e.timestamp}
                </span>
                <span
                  className="mono"
                  style={{
                    fontSize: 9,
                    fontWeight: 700,
                    color: isFault ? '#f87171' : '#ffffff',
                    flexShrink: 0,
                  }}
                >
                  [{e.event_type}]
                </span>
                <span style={{ color: '#d4d4d8', wordBreak: 'break-word' }}>{e.description}</span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
