//! 2D Canvas tactical map view rendering drone, trajectories, obstacles, NFZ, and LiDAR sweeps.

import React, { useRef, useEffect, useState, useCallback } from 'react';
import {
  ZoomIn,
  ZoomOut,
  Crosshair,
  RotateCcw,
  Layers,
  Radio,
  Eye,
  Wind,
  Grid,
} from 'lucide-react';
import { ScenarioConfig, SimulationSnapshot } from '../types';

interface MapViewProps {
  scenario: ScenarioConfig;
  snapshot: SimulationSnapshot | null;
  onSetTargetPoint: (x: number, y: number) => void;
}

export const MapView: React.FC<MapViewProps> = ({
  scenario,
  snapshot,
  onSetTargetPoint,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Viewport transforms: pan and zoom
  const [zoom, setZoom] = useState<number>(1.1);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 180, y: 80 });
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [dragStart, setDragStart] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [followDrone, setFollowDrone] = useState<boolean>(true);

  // Layer visibility toggles
  const [showLidar, setShowLidar] = useState<boolean>(true);
  const [showRawTrail, setShowRawTrail] = useState<boolean>(true);
  const [showFilteredTrail, setShowFilteredTrail] = useState<boolean>(true);
  const [showGrid, setShowGrid] = useState<boolean>(true);

  // World to canvas coordinate conversion
  // World: (0,0) at bottom-left, X East, Y North
  const worldToCanvas = useCallback(
    (wx: number, wy: number, canvasHeight: number) => {
      const cx = pan.x + wx * zoom;
      const cy = canvasHeight - (pan.y + wy * zoom);
      return { cx, cy };
    },
    [pan, zoom]
  );

  // Canvas to world coordinate conversion
  const canvasToWorld = useCallback(
    (cx: number, cy: number, canvasHeight: number) => {
      const wx = (cx - pan.x) / zoom;
      const wy = (canvasHeight - cy - pan.y) / zoom;
      return { wx, wy };
    },
    [pan, zoom]
  );

  // Follow drone update
  useEffect(() => {
    if (followDrone && snapshot && canvasRef.current) {
      const canvas = canvasRef.current;
      const width = canvas.parentElement?.clientWidth || canvas.clientWidth || 800;
      const height = canvas.parentElement?.clientHeight || canvas.clientHeight || 600;
      const targetX = width / 2 - snapshot.drone.x * zoom;
      const targetY = height / 2 - snapshot.drone.y * zoom;
      setPan((prev) => ({
        x: prev.x + (targetX - prev.x) * 0.15,
        y: prev.y + (targetY - prev.y) * 0.15,
      }));
    }
  }, [followDrone, snapshot?.drone.x, snapshot?.drone.y, zoom]);

  // Main canvas render loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Handle high-DPI retina rendering
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.parentElement?.clientWidth || 800;
    const height = canvas.parentElement?.clientHeight || 600;

    if (canvas.width !== width * dpr || canvas.height !== height * dpr) {
      canvas.width = width * dpr;
      canvas.height = height * dpr;
    }

    ctx.save();
    ctx.scale(dpr, dpr);

    // 1. Clear background
    ctx.fillStyle = '#070a10';
    ctx.fillRect(0, 0, width, height);

    // 2. Render Metric Grid & Coordinates
    if (showGrid) {
      ctx.lineWidth = 1;
      const step = 50; // 50m intervals
      for (let wx = scenario.boundary.min_x; wx <= scenario.boundary.max_x; wx += step) {
        const p1 = worldToCanvas(wx, scenario.boundary.min_y, height);
        const p2 = worldToCanvas(wx, scenario.boundary.max_y, height);
        ctx.strokeStyle = wx % 100 === 0 ? '#1e293b' : '#101726';
        ctx.beginPath();
        ctx.moveTo(p1.cx, p1.cy);
        ctx.lineTo(p2.cx, p2.cy);
        ctx.stroke();

        // Label
        ctx.fillStyle = '#475569';
        ctx.font = '10px ui-monospace, monospace';
        ctx.fillText(`${wx}m`, p1.cx + 2, height - 6);
      }

      for (let wy = scenario.boundary.min_y; wy <= scenario.boundary.max_y; wy += step) {
        const p1 = worldToCanvas(scenario.boundary.min_x, wy, height);
        const p2 = worldToCanvas(scenario.boundary.max_x, wy, height);
        ctx.strokeStyle = wy % 100 === 0 ? '#1e293b' : '#101726';
        ctx.beginPath();
        ctx.moveTo(p1.cx, p1.cy);
        ctx.lineTo(p2.cx, p2.cy);
        ctx.stroke();

        // Label
        ctx.fillStyle = '#475569';
        ctx.font = '10px ui-monospace, monospace';
        ctx.fillText(`${wy}m`, 6, p1.cy - 4);
      }
    }

    // 3. Render Geofence Boundary and Buffer
    {
      const b = scenario.boundary;
      const bl = worldToCanvas(b.min_x, b.min_y, height);
      const tr = worldToCanvas(b.max_x, b.max_y, height);

      // Warning buffer zone (15m inset)
      const bBuf = 15;
      const blBuf = worldToCanvas(b.min_x + bBuf, b.min_y + bBuf, height);
      const trBuf = worldToCanvas(b.max_x - bBuf, b.max_y - bBuf, height);

      ctx.strokeStyle = 'rgba(245, 158, 11, 0.35)';
      ctx.setLineDash([4, 4]);
      ctx.lineWidth = 1;
      ctx.strokeRect(blBuf.cx, trBuf.cy, trBuf.cx - blBuf.cx, blBuf.cy - trBuf.cy);
      ctx.setLineDash([]);

      // Hard Boundary
      ctx.strokeStyle = 'rgba(239, 68, 68, 0.7)';
      ctx.lineWidth = 2;
      ctx.strokeRect(bl.cx, tr.cy, tr.cx - bl.cx, bl.cy - tr.cy);

      // Label
      ctx.fillStyle = '#ef4444';
      ctx.font = '10px ui-monospace, monospace';
      ctx.fillText('GEOFENCE PERIMETER', bl.cx + 6, tr.cy + 14);
    }

    // 4. Render No-Fly Zones
    for (const nfz of scenario.no_fly_zones) {
      const center = worldToCanvas(nfz.center_x, nfz.center_y, height);
      const rPx = nfz.radius * zoom;

      ctx.beginPath();
      ctx.arc(center.cx, center.cy, rPx, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(239, 68, 68, 0.12)';
      ctx.fill();
      ctx.strokeStyle = '#ef4444';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([6, 3]);
      ctx.stroke();
      ctx.setLineDash([]);

      // NFZ Label
      ctx.fillStyle = '#fca5a5';
      ctx.font = 'bold 10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(nfz.name, center.cx, center.cy - 4);
      ctx.font = '9px ui-monospace, monospace';
      ctx.fillStyle = '#f87171';
      ctx.fillText('RESTRICTED AIRSPACE', center.cx, center.cy + 10);
      ctx.textAlign = 'left';
    }

    // 5. Render Physical Obstacles
    for (const obs of scenario.obstacles) {
      const center = worldToCanvas(obs.x, obs.y, height);
      const rPx = obs.radius * zoom;

      // Clearance safety halo
      ctx.beginPath();
      ctx.arc(center.cx, center.cy, rPx + 4 * zoom, 0, Math.PI * 2);
      ctx.strokeStyle = 'rgba(59, 130, 246, 0.25)';
      ctx.lineWidth = 1;
      ctx.stroke();

      // Obstacle body
      ctx.beginPath();
      ctx.arc(center.cx, center.cy, rPx, 0, Math.PI * 2);
      ctx.fillStyle = '#1e293b';
      ctx.fill();
      ctx.strokeStyle = '#475569';
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Obstacle label
      ctx.fillStyle = '#cbd5e1';
      ctx.font = 'bold 10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(obs.name, center.cx, center.cy - 2);
      ctx.font = '9px ui-monospace, monospace';
      ctx.fillStyle = '#94a3b8';
      ctx.fillText(`H: ${obs.height}m`, center.cx, center.cy + 10);
      ctx.textAlign = 'left';
    }

    // 6. Render Scenario Waypoints & Connecting Path
    if (scenario.waypoints.length > 0) {
      ctx.strokeStyle = 'rgba(100, 116, 139, 0.4)';
      ctx.setLineDash([3, 3]);
      ctx.lineWidth = 1;
      ctx.beginPath();
      scenario.waypoints.forEach((wp, idx) => {
        const pt = worldToCanvas(wp.x, wp.y, height);
        if (idx === 0) ctx.moveTo(pt.cx, pt.cy);
        else ctx.lineTo(pt.cx, pt.cy);
      });
      ctx.stroke();
      ctx.setLineDash([]);

      // Waypoint Pins
      scenario.waypoints.forEach((wp) => {
        const pt = worldToCanvas(wp.x, wp.y, height);
        const isActive = snapshot?.drone.current_waypoint_index === wp.id;

        ctx.beginPath();
        ctx.arc(pt.cx, pt.cy, isActive ? 9 : 7, 0, Math.PI * 2);
        ctx.fillStyle = isActive ? '#f59e0b' : '#334155';
        ctx.fill();
        ctx.strokeStyle = isActive ? '#fbbf24' : '#64748b';
        ctx.lineWidth = 2;
        ctx.stroke();

        ctx.fillStyle = isActive ? '#000000' : '#ffffff';
        ctx.font = 'bold 9px ui-monospace, monospace';
        ctx.textAlign = 'center';
        ctx.fillText(wp.id.toString(), pt.cx, pt.cy + 3);
        ctx.textAlign = 'left';
      });
    }

    // 7. Render Planned A* Optimal Route
    if (snapshot && snapshot.planned_route.length > 1) {
      ctx.strokeStyle = '#f59e0b';
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      snapshot.planned_route.forEach((pt, idx) => {
        const c = worldToCanvas(pt.x, pt.y, height);
        if (idx === 0) ctx.moveTo(c.cx, c.cy);
        else ctx.lineTo(c.cx, c.cy);
      });
      ctx.stroke();
    }

    // 8. Render Raw GPS Trajectory Trail
    if (showRawTrail && snapshot && snapshot.raw_trail.length > 1) {
      ctx.strokeStyle = '#ec4899';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      snapshot.raw_trail.forEach((pt, idx) => {
        const c = worldToCanvas(pt.x, pt.y, height);
        if (idx === 0) ctx.moveTo(c.cx, c.cy);
        else ctx.lineTo(c.cx, c.cy);
      });
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // 9. Render Filtered Localization Trajectory Trail
    if (showFilteredTrail && snapshot && snapshot.filtered_trail.length > 1) {
      ctx.strokeStyle = '#06b6d4';
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      snapshot.filtered_trail.forEach((pt, idx) => {
        const c = worldToCanvas(pt.x, pt.y, height);
        if (idx === 0) ctx.moveTo(c.cx, c.cy);
        else ctx.lineTo(c.cx, c.cy);
      });
      ctx.stroke();
    }

    // 10. Render LiDAR Sweeps
    if (showLidar && snapshot) {
      const lidarReading = snapshot.sensor_readings.find((r) =>
        r.sensor_name.startsWith('LiDAR')
      );
      if (lidarReading && lidarReading.values.rays) {
        const dronePos = worldToCanvas(snapshot.drone.x, snapshot.drone.y, height);
        const rays: any[] = lidarReading.values.rays;

        for (const ray of rays) {
          const hitPos = worldToCanvas(ray.hit_x, ray.hit_y, height);

          ctx.beginPath();
          ctx.moveTo(dronePos.cx, dronePos.cy);
          ctx.lineTo(hitPos.cx, hitPos.cy);

          if (ray.hit_detected) {
            ctx.strokeStyle = ray.distance_m < 6.0 ? 'rgba(239, 68, 68, 0.4)' : 'rgba(16, 185, 129, 0.35)';
            ctx.lineWidth = 1;
            ctx.stroke();

            // Hit point dot
            ctx.beginPath();
            ctx.arc(hitPos.cx, hitPos.cy, 2.5, 0, Math.PI * 2);
            ctx.fillStyle = ray.distance_m < 6.0 ? '#ef4444' : '#10b981';
            ctx.fill();
          } else {
            ctx.strokeStyle = 'rgba(6, 182, 212, 0.12)';
            ctx.lineWidth = 0.5;
            ctx.stroke();
          }
        }
      }
    }

    // 11. Render Home Base Marker
    {
      const homePos = worldToCanvas(scenario.home_x, scenario.home_y, height);
      ctx.strokeStyle = '#10b981';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(homePos.cx, homePos.cy, 10, 0, Math.PI * 2);
      ctx.stroke();

      ctx.fillStyle = '#10b981';
      ctx.font = 'bold 10px ui-monospace, monospace';
      ctx.textAlign = 'center';
      ctx.fillText('H', homePos.cx, homePos.cy + 3.5);
      ctx.textAlign = 'left';
    }

    // 12. Render Drone Symbol with Heading and Status
    if (snapshot) {
      const dronePt = worldToCanvas(snapshot.drone.x, snapshot.drone.y, height);
      const headingRad = (snapshot.drone.heading * Math.PI) / 180;

      ctx.save();
      ctx.translate(dronePt.cx, dronePt.cy);

      // Raw vs Filtered Position Offset Indicator (if divergent)
      if (snapshot.localization.estimation_error_m > 2.0) {
        const rawPt = worldToCanvas(snapshot.localization.raw_x, snapshot.localization.raw_y, height);
        const relRawX = rawPt.cx - dronePt.cx;
        const relRawY = rawPt.cy - dronePt.cy;

        // Raw position marker
        ctx.beginPath();
        ctx.arc(relRawX, relRawY, 5, 0, Math.PI * 2);
        ctx.fillStyle = '#ec4899';
        ctx.fill();

        // Stitched divergence line
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(relRawX, relRawY);
        ctx.strokeStyle = '#ec4899';
        ctx.lineWidth = 1;
        ctx.setLineDash([2, 2]);
        ctx.stroke();
        ctx.setLineDash([]);
      }

      // Rotate canvas for drone heading (0 deg North = straight up in canvas)
      ctx.rotate(headingRad);

      // Drone Body (Delta Quadcopter Icon)
      ctx.beginPath();
      ctx.moveTo(0, -14); // Nose
      ctx.lineTo(10, 10);
      ctx.lineTo(0, 6);
      ctx.lineTo(-10, 10);
      ctx.closePath();

      ctx.fillStyle = snapshot.drone.armed ? '#06b6d4' : '#64748b';
      ctx.fill();
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Forward sensor cone
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(-8, -24);
      ctx.lineTo(8, -24);
      ctx.closePath();
      ctx.fillStyle = 'rgba(34, 211, 238, 0.15)';
      ctx.fill();

      ctx.restore();

      // Drone telemetry tag text
      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 10px ui-monospace, monospace';
      ctx.fillText(`ALT: ${snapshot.drone.altitude.toFixed(1)}m`, dronePt.cx + 14, dronePt.cy - 8);
      ctx.fillStyle = '#94a3b8';
      ctx.font = '9px ui-monospace, monospace';
      ctx.fillText(`${snapshot.drone.horizontal_speed.toFixed(1)} m/s`, dronePt.cx + 14, dronePt.cy + 4);
    }

    ctx.restore();
  }, [
    scenario,
    snapshot,
    zoom,
    pan,
    showGrid,
    showLidar,
    showRawTrail,
    showFilteredTrail,
    worldToCanvas,
  ]);

  // Mouse wheel zoom
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 0.87;
    setZoom((z) => Math.min(Math.max(z * factor, 0.4), 6.0));
  };

  // Mouse drag panning
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 0) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
      setFollowDrone(false);
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPan({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Double click or right click to set target waypoint
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    const height = canvas.parentElement?.clientHeight || 600;
    const { wx, wy } = canvasToWorld(cx, cy, height);

    if (
      wx >= scenario.boundary.min_x &&
      wx <= scenario.boundary.max_x &&
      wy >= scenario.boundary.min_y &&
      wy <= scenario.boundary.max_y
    ) {
      onSetTargetPoint(Math.round(wx), Math.round(wy));
    }
  };

  return (
    <div className="map-viewport" onWheel={handleWheel}>
      {/* Map Control Toolbar */}
      <div className="map-toolbar">
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => setZoom((z) => Math.min(z * 1.25, 6.0))}
          title="Zoom In"
        >
          <ZoomIn size={14} />
        </button>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => setZoom((z) => Math.max(z * 0.8, 0.4))}
          title="Zoom Out"
        >
          <ZoomOut size={14} />
        </button>
        <button
          className={`btn btn-sm ${followDrone ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setFollowDrone(!followDrone)}
          title="Lock View on Drone"
        >
          <Crosshair size={14} />
        </button>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => {
            const z = 1.1;
            setZoom(z);
            if (canvasRef.current) {
              const width = canvasRef.current.parentElement?.clientWidth || canvasRef.current.clientWidth || 800;
              const height = canvasRef.current.parentElement?.clientHeight || canvasRef.current.clientHeight || 600;
              setPan({
                x: width / 2 - 200 * z,
                y: height / 2 - 200 * z,
              });
            } else {
              setPan({ x: 180, y: 80 });
            }
            setFollowDrone(false);
          }}
          title="Reset View"
        >
          <RotateCcw size={14} />
        </button>
        <div style={{ width: 1, background: '#334155', margin: '0 4px' }} />
        <button
          className={`btn btn-sm ${showLidar ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setShowLidar(!showLidar)}
          title="Toggle LiDAR Rays"
        >
          <Radio size={14} />
        </button>
        <button
          className={`btn btn-sm ${showFilteredTrail ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setShowFilteredTrail(!showFilteredTrail)}
          title="Toggle Filtered Trail (Cyan)"
        >
          <Eye size={14} />
        </button>
        <button
          className={`btn btn-sm ${showRawTrail ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setShowRawTrail(!showRawTrail)}
          title="Toggle Raw GPS Trail (Pink)"
        >
          <Layers size={14} />
        </button>
        <button
          className={`btn btn-sm ${showGrid ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setShowGrid(!showGrid)}
          title="Toggle Grid Lines"
        >
          <Grid size={14} />
        </button>
      </div>

      {/* Map Legend */}
      <div className="map-legend">
        <div style={{ fontWeight: 700, textTransform: 'uppercase', fontSize: 10, color: '#94a3b8' }}>
          Tactical Map Legend
        </div>
        <div className="legend-item">
          <div className="legend-line filtered" />
          <span>Kalman Filtered Position</span>
        </div>
        <div className="legend-item">
          <div className="legend-line raw" />
          <span>Raw GPS Position (Noise & Drift)</span>
        </div>
        <div className="legend-item">
          <div className="legend-line route" />
          <span>Planned A* Obstacle-Free Route</span>
        </div>
        <div style={{ fontSize: 10, color: '#64748b', marginTop: 4 }}>
          Right-click on map to command GOTO
        </div>
      </div>

      {/* Wind Indicator Badge */}
      {snapshot && (
        <div
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'rgba(15, 23, 42, 0.85)',
            border: '1px solid #2a3b53',
            borderRadius: 6,
            padding: '6px 10px',
            fontSize: 11,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            backdropFilter: 'blur(8px)',
          }}
        >
          <Wind size={14} color="#06b6d4" />
          <div>
            <div style={{ color: '#94a3b8', fontSize: 9, textTransform: 'uppercase' }}>Ambient Wind</div>
            <div className="mono" style={{ fontWeight: 600 }}>
              {snapshot.wind_vector.x.toFixed(1)} m/s E, {snapshot.wind_vector.y.toFixed(1)} m/s N
            </div>
          </div>
        </div>
      )}

      {/* Canvas Element */}
      <canvas
        ref={canvasRef}
        style={{ width: '100%', height: '100%', cursor: isDragging ? 'grabbing' : 'crosshair' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onContextMenu={handleContextMenu}
      />
    </div>
  );
};
