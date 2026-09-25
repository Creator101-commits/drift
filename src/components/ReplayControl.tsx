//! Floating replay bar with variable playback speeds, scrubber slider, and export shortcuts.

import React from 'react';
import {
  Play,
  Pause,
  RotateCcw,
  X,
  FileCode,
  FileSpreadsheet,
  Globe,
} from 'lucide-react';
import {
  pauseReplay,
  resumeReplay,
  resetReplay,
  setReplaySpeed,
  seekReplay,
  stopReplay,
  exportMissionJson,
  exportMissionCsv,
  exportMissionHtml,
} from '../api';
import { ReplayStatus } from '../types';

interface ReplayControlProps {
  replayStatus: ReplayStatus;
  activeMissionId?: string;
  onClose: () => void;
}

export const ReplayControl: React.FC<ReplayControlProps> = ({
  replayStatus,
  activeMissionId,
  onClose,
}) => {
  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const idx = parseInt(e.target.value, 10);
    seekReplay(idx);
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

  const handleExportJson = async () => {
    if (!activeMissionId) return;
    try {
      const json = await exportMissionJson(activeMissionId);
      handleDownloadFile(json, `${activeMissionId}.json`, 'application/json');
    } catch (err) {
      console.error(err);
    }
  };

  const handleExportCsv = async () => {
    if (!activeMissionId) return;
    try {
      const csv = await exportMissionCsv(activeMissionId);
      handleDownloadFile(csv, `${activeMissionId}_telemetry.csv`, 'text/csv');
    } catch (err) {
      console.error(err);
    }
  };

  const handleExportHtml = async () => {
    if (!activeMissionId) return;
    try {
      const html = await exportMissionHtml(activeMissionId);
      handleDownloadFile(html, `${activeMissionId}_report.html`, 'text/html');
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="replay-bar">
      {/* Top Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <span
            style={{
              background: '#ffffff',
              color: '#000000',
              padding: '3px 10px',
              borderRadius: 9999,
              fontSize: 10,
              fontWeight: 800,
              letterSpacing: 1,
              textTransform: 'uppercase',
            }}
          >
            REPLAY MODE
          </span>
          {activeMissionId && (
            <span className="mono" style={{ fontSize: 11, color: '#a1a1aa' }}>
              Mission: {activeMissionId}
            </span>
          )}
        </div>

        {/* Export Shortcuts in Replay */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {activeMissionId && (
            <>
              <button
                className="btn btn-secondary btn-sm"
                style={{ padding: '3px 8px', fontSize: 10, borderRadius: 8 }}
                onClick={handleExportJson}
              >
                <FileCode size={11} /> JSON
              </button>
              <button
                className="btn btn-secondary btn-sm"
                style={{ padding: '3px 8px', fontSize: 10, borderRadius: 8 }}
                onClick={handleExportCsv}
              >
                <FileSpreadsheet size={11} /> CSV
              </button>
              <button
                className="btn btn-secondary btn-sm"
                style={{ padding: '3px 8px', fontSize: 10, borderRadius: 8 }}
                onClick={handleExportHtml}
              >
                <Globe size={11} /> HTML
              </button>
              <div style={{ width: 1, height: 16, background: '#27272a' }} />
            </>
          )}

          <button
            className="btn btn-secondary btn-sm"
            style={{ borderRadius: 8 }}
            onClick={() => {
              stopReplay();
              onClose();
            }}
            title="Exit Replay"
          >
            <X size={13} /> Exit Replay
          </button>
        </div>
      </div>

      {/* Scrubber Slider */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <span className="mono" style={{ fontSize: 11, color: '#ffffff', minWidth: 45 }}>
          {replayStatus.current_time_sec.toFixed(1)}s
        </span>
        <input
          type="range"
          className="replay-slider"
          min={0}
          max={Math.max(replayStatus.total_frames - 1, 0)}
          value={replayStatus.current_index}
          onChange={handleSliderChange}
        />
        <span className="mono" style={{ fontSize: 11, color: '#71717a', minWidth: 45 }}>
          {replayStatus.total_duration_sec.toFixed(1)}s
        </span>
      </div>

      {/* Controls and Speed Bar */}
      <div className="replay-controls">
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {replayStatus.is_playing ? (
            <button className="btn btn-secondary btn-sm" style={{ borderRadius: 8 }} onClick={() => pauseReplay()}>
              <Pause size={13} /> Pause
            </button>
          ) : (
            <button className="btn btn-primary btn-sm" style={{ borderRadius: 8 }} onClick={() => resumeReplay()}>
              <Play size={13} /> Play
            </button>
          )}
          <button className="btn btn-secondary btn-sm" style={{ borderRadius: 8 }} onClick={() => resetReplay()}>
            <RotateCcw size={13} /> Restart
          </button>
        </div>

        {/* Speed Selector */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <span style={{ fontSize: 10, color: '#a1a1aa', marginRight: 4 }}>Speed:</span>
          {[0.5, 1.0, 2.0, 5.0, 25.0].map((spd) => (
            <button
              key={spd}
              className={`btn btn-sm ${replayStatus.playback_speed === spd ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 10, padding: '2px 8px', borderRadius: 9999 }}
              onClick={() => setReplaySpeed(spd)}
            >
              {spd === 25.0 ? 'Instant' : `${spd}x`}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};
