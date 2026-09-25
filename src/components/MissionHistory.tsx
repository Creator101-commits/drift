//! Mission history browser with inspect, replay, export, and delete confirmation.

import React, { useEffect, useState } from 'react';
import {
  History,
  Play,
  Trash2,
  RefreshCw,
  FileCode,
  FileSpreadsheet,
  Globe,
  AlertTriangle,
} from 'lucide-react';
import {
  listSavedMissions,
  deleteSavedMission,
  exportMissionJson,
  exportMissionCsv,
  exportMissionHtml,
} from '../api';
import { MissionSummary } from '../types';

interface MissionHistoryProps {
  onStartReplay: (missionId: string) => void;
}

export const MissionHistory: React.FC<MissionHistoryProps> = ({ onStartReplay }) => {
  const [missions, setMissions] = useState<MissionSummary[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  const loadMissions = async () => {
    setIsLoading(true);
    try {
      const list = await listSavedMissions();
      setMissions(list);
    } catch (err) {
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadMissions();
  }, []);

  const handleDelete = async () => {
    if (!confirmDeleteId) return;
    try {
      await deleteSavedMission(confirmDeleteId);
      setConfirmDeleteId(null);
      await loadMissions();
    } catch (err) {
      console.error(err);
    }
  };

  const handleDownloadFile = (content: string, filename: string, type: string) => {
    const blob = new Blob([content], { type });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleExportJson = async (id: string) => {
    try {
      const json = await exportMissionJson(id);
      handleDownloadFile(json, `${id}.json`, 'application/json');
    } catch (err) {
      console.error(err);
    }
  };

  const handleExportCsv = async (id: string) => {
    try {
      const csv = await exportMissionCsv(id);
      handleDownloadFile(csv, `${id}_telemetry.csv`, 'text/csv');
    } catch (err) {
      console.error(err);
    }
  };

  const handleExportHtml = async (id: string) => {
    try {
      const html = await exportMissionHtml(id);
      handleDownloadFile(html, `${id}_report.html`, 'text/html');
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="card-section">
      <div className="section-header">
        <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <History size={13} /> Saved Mission History ({missions.length})
        </span>
        <button
          className="btn btn-secondary btn-sm"
          style={{ padding: '2px 6px' }}
          onClick={loadMissions}
          title="Refresh List"
        >
          <RefreshCw size={11} className={isLoading ? 'animate-spin' : ''} />
        </button>
      </div>

      {missions.length === 0 ? (
        <div style={{ padding: '16px 8px', textAlign: 'center', color: '#71717a', fontSize: 11 }}>
          No recorded missions saved in SQLite yet. Complete a flight and click "Save Mission".
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, maxHeight: 250, overflowY: 'auto' }}>
          {missions.map((m) => (
            <div
              key={m.id}
              style={{
                background: '#181818',
                border: 'none',
                borderRadius: 12,
                padding: '12px 14px',
                display: 'flex',
                flexDirection: 'column',
                gap: 8,
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <div style={{ fontWeight: 700, color: '#ffffff', fontSize: 12 }}>{m.name}</div>
                  <div className="mono" style={{ fontSize: 9, color: '#71717a' }}>
                    {m.id} &bull; {m.start_time}
                  </div>
                </div>
                <span
                  style={{
                    background: '#27272a',
                    color: '#ffffff',
                    padding: '2px 8px',
                    borderRadius: 9999,
                    fontSize: 9,
                    fontWeight: 700,
                    textTransform: 'uppercase',
                  }}
                >
                  {m.status}
                </span>
              </div>

              {/* Metrics Summary */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 4, fontSize: 10 }}>
                <div style={{ color: '#71717a' }}>
                  Dist: <span className="mono" style={{ color: '#ffffff' }}>{m.total_distance.toFixed(0)}m</span>
                </div>
                <div style={{ color: '#71717a' }}>
                  Peak: <span className="mono" style={{ color: '#ffffff' }}>{m.max_altitude.toFixed(1)}m</span>
                </div>
                <div style={{ color: '#71717a' }}>
                  Bat: <span className="mono" style={{ color: '#ffffff' }}>{m.battery_consumed.toFixed(1)}%</span>
                </div>
              </div>

              {/* Action Buttons */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 4 }}>
                <button
                  className="btn btn-primary btn-sm"
                  style={{ padding: '4px 10px', fontSize: 10, borderRadius: 8 }}
                  onClick={() => onStartReplay(m.id)}
                >
                  <Play size={11} /> Replay
                </button>

                <div style={{ display: 'flex', gap: 4 }}>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ padding: '4px 8px', borderRadius: 8 }}
                    onClick={() => handleExportJson(m.id)}
                    title="Export JSON"
                  >
                    <FileCode size={11} /> JSON
                  </button>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ padding: '4px 8px', borderRadius: 8 }}
                    onClick={() => handleExportCsv(m.id)}
                    title="Export CSV Telemetry"
                  >
                    <FileSpreadsheet size={11} /> CSV
                  </button>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ padding: '4px 8px', borderRadius: 8 }}
                    onClick={() => handleExportHtml(m.id)}
                    title="Export Standalone HTML Report"
                  >
                    <Globe size={11} /> HTML
                  </button>
                  <button
                    className="btn btn-danger btn-sm"
                    style={{ padding: '4px 8px', borderRadius: 8 }}
                    onClick={() => setConfirmDeleteId(m.id)}
                    title="Delete Saved Mission"
                  >
                    <Trash2 size={11} />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Explicit Confirmation Modal for Deleting Saved Mission */}
      {confirmDeleteId && (
        <div className="modal-overlay">
          <div className="modal-content">
            <div className="modal-title" style={{ display: 'flex', alignItems: 'center', gap: 8, color: '#ef4444' }}>
              <AlertTriangle size={18} /> Confirm Permanent Deletion
            </div>
            <div style={{ fontSize: 13, color: '#cbd5e1', lineHeight: 1.5 }}>
              Are you sure you want to permanently delete mission <strong>{confirmDeleteId}</strong>?
              <br />
              All recorded telemetry points, sensor readings, alerts, and replay frames will be removed from SQLite.
            </div>
            <div className="modal-actions">
              <button
                className="btn btn-secondary"
                onClick={() => setConfirmDeleteId(null)}
              >
                Cancel
              </button>
              <button
                className="btn btn-danger"
                onClick={handleDelete}
              >
                Delete Permanently
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
