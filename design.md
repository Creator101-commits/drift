# Drift Architecture and System Design

Drift is a real-time autonomous drone mission simulator and telemetry analyzer built as a desktop engineering workstation using Tauri 2, Rust, React, and SQLite.

---

## 1. System Architecture

```
+-------------------------------------------------------------------------+
|                              React Frontend                             |
|                                                                         |
|  +-------------------+  +-----------------------+  +-----------------+  |
|  |   Left Panel      |  |     Center Panel      |  |   Right Panel   |  |
|  | Mission Controls  |  |   Tactical 2D Map     |  | Telemetry Graph |  |
|  | Fault Injection   |  |   (HTML Canvas)       |  | Sensor Matrix   |  |
|  | Script Runner     |  | Raw vs Filtered Trail |  | Safety Alerts   |  |
|  | Mission History   |  | LiDAR Raycasts / NFZ  |  | Event Timeline  |  |
|  +-------------------+  +-----------------------+  +-----------------+  |
+-------------------------------------------------------------------------+
                                    |
          Tauri 2 IPC (Commands & Events @ 20 Hz: telemetry-sample)
                                    v
+-------------------------------------------------------------------------+
|                               Rust Backend                              |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  |                    SimulationEngine (20 Hz, dt = 0.05s)           |  |
|  |  - Seeded ChaCha8 PRNG                                            |  |
|  |  - Kinematic Motion & Wind Aerodynamics                           |  |
|  |  - Battery Power Consumption Model                                |  |
|  +-------------------------------------------------------------------+  |
|         |                     |                    |                    |
|         v                     v                    v                    v
|  +--------------+     +---------------+    +---------------+    +-------+
|  | SensorSuite  |     | StateEstimator|    | OccupancyGrid |    | Alert |
|  | - GPS Neo-M9 |     | - Raw vs      |    | & A* Planner  |    | Engine|
|  | - IMU BMI088 |     |   Filtered    |    | - Smoothing   |    |       |
|  | - BMM150 Mag |     |   Fusion      |    | - Dilation    |    |       |
|  | - DPS310 Baro|     | - Innovation  |    | - Dynamic     |    |       |
|  | - 16-ray Lidar     |   Gating      |    |   Detour      |    |       |
|  +--------------+     +---------------+    +---------------+    +-------+
|                                                                         |
|  +-----------------------------+       +-----------------------------+  |
|  |    TrafficReplayer Engine   |       |   MissionDatabase (SQLite)  |  |
|  |  - 0.5x, 1x, 2x, 5x, Instant|       |   - Scenarios, Missions     |  |
|  |  - JSON, CSV, HTML Reports  |       |   - Telemetry, Sensor Read  |  |
|  +-----------------------------+       +-----------------------------+  |
+-------------------------------------------------------------------------+
```

---

## 2. Coordinate System and Physical Units

- **World Coordinates**: 2.5D Local Cartesian frame.
  - X: East displacement in meters ($+X$ = East).
  - Y: North displacement in meters ($+Y$ = North).
  - Altitude ($Z$): Height above ground level in meters ($+Z$ = Up).
- **Heading**: Degrees $[0.0, 360.0)$ referenced to True North ($0^\circ$ = North, $90^\circ$ = East, $180^\circ$ = South, $270^\circ$ = West).
- **Speeds**: Horizontal speed in $\text{m/s}$, vertical climb rate in $\text{m/s}$.
- **Timing**: Fixed-timestep $\Delta t = 0.05\,\text{s}$ ($20\,\text{Hz}$ simulation rate).
- **Environmental Wind**: Vector $(V_x, V_y)$ in $\text{m/s}$ computed from base speed, compass direction, and sinusoidal gust period.

---

## 3. Core Simulation Engine (`simulation`)

The simulation engine maintains drone state and implements the following flight modes:

- `Disarmed`: Motors locked, stationary on launch pad.
- `Armed`: Motors spinning at idle on ground.
- `Takeoff`: Proportional climb controller climbing to target hover altitude (e.g. 15m) while compensating for ambient crosswinds.
- `Hover`: Station-keeping against wind using velocity feedback.
- `WaypointFollow`: Guidance towards active waypoint along smoothed A* routes at commanded horizontal speed.
- `ReturnToHome`: Three-stage fail-safe:
  1. Climb to safe transit altitude ($20\,\text{m}$ clearance).
  2. Horizontal transit directly to home coordinate $(X_{\text{home}}, Y_{\text{home}})$.
  3. Controlled vertical descent at $-2.0\,\text{m/s}$ until ground touchdown, followed by automatic disarm.
- `EmergencyLand`: Horizontal velocity damping and constant safe descent at $-1.8\,\text{m/s}$.
- `Collision`: Triggered upon physical impact with obstacle or boundary; cuts motor power.

### Battery Consumption Model
The battery discharge rate is calculated continuously:
$$\text{DrainRate} = \text{BaseIdle} + \text{HoverPower} + \left(\frac{v_h}{v_{h,\max}}\right)^2 \cdot C_{\text{drag}} + \left(\frac{v_z}{v_{z,\max}}\right) \cdot C_{\text{climb}}$$

---

## 4. Sensor Suite and Fault Injection (`sensors`)

All sensors derive measurement noise deterministically using a seeded `ChaCha8Rng`:

1. **GPS (`GpsSensor`)**:
   - Outputs: Position $(X, Y)$, Altitude, Horizontal Speed, Ground Course, HDOP, Satellites, Fix Quality.
   - Faults:
     - `GpsDrift`: Progressive cumulative positional drift offset ($\sim 0.8\,\text{m/s}$).
     - `GpsDropout`: Loss of satellite lock ($\text{validity} = \text{false}$, $\text{confidence} = 0.0$, $\text{satellites} = 2$, $\text{HDOP} = 99.9$).
2. **IMU (`ImuSensor`)**:
   - Outputs: 3-axis Accelerometer $(A_x, A_y, A_z)$ with gravity bias $+9.81\,\text{m/s}^2$ on $Z$, 3-axis Gyroscope $(G_x, G_y, G_z)$.
   - Faults: `ImuBias` (adds constant sensor offsets), `ExcessiveNoise`.
3. **Digital Compass (`CompassSensor`)**:
   - Outputs: Yaw heading degrees, 3-axis magnetic field strength in microteslas ($\mu\text{T}$).
   - Faults: `CompassFailure` (uncontrolled continuous spinning yaw), `ExcessiveNoise`.
4. **Barometric Altimeter (`AltimeterSensor`)**:
   - Outputs: Atmospheric pressure ($\text{hPa}$), barometric altitude ($\text{m}$), climb rate.
   - Faults: `AltimeterDrift` (accumulating altitude bias), `ExcessiveNoise`.
5. **360-Degree LiDAR Scanner (`LidarSensor`)**:
   - Outputs: 16 radial raycasts across $360^\circ$ ($22.5^\circ$ angular resolution), obstacle intersection distances, minimum distance to obstacle.
   - Faults: `LidarBlindSpot` (failure to detect obstacles within forward $90^\circ$ sector).

---

## 5. Navigation and Localization (`navigation`)

- **Occupancy Grid**: Discretized 2D grid matrix ($2.5\,\text{m}$ resolution) covering the $400\,\text{m} \times 400\,\text{m}$ area. Static obstacles and restricted no-fly zones are dilated by drone safety margin ($3.0\,\text{m}$).
- **A\* Path Planning**: 8-connected search algorithm using octile distance heuristic. Produces collision-free path, which is subsequently optimized using line-of-sight ray-tracing string pulling to eliminate grid staircasing.
- **Filtered State Estimator**: Extended complementary Kalman filter fusing:
  - IMU accelerometer dead-reckoning integration for high-frequency motion prediction.
  - GPS absolute position updates modulated by innovation gating (rejects wild drift jumps).
  - Barometric altitude and magnetic compass heading fusion.
  - Generates both `raw` and `filtered` position streams for side-by-side visualization.
- **Safety Monitor**: Real-time evaluation of time-to-collision (TTC), geofence warning perimeter ($15\,\text{m}$ buffer), hard boundary breaches, and ceiling limits.

---

## 6. Alerts and Anomaly Engine (`analysis`)

Evaluates rules at every simulation tick:
- `LOW_BATTERY_WARNING` ($< 25\%$) and `LOW_BATTERY_CRITICAL` ($< 12\%$).
- `GPS_DRIFT_DETECTED`: Triggered when discrepancy between raw GPS and filtered dead-reckoning exceeds $6.0\,\text{m}$.
- `DROPOUT_<SENSOR>`: Fired when sensor validity is lost.
- `GEOFENCE_WARNING` / `GEOFENCE_BREACH`: Perimeter boundary proximity.
- `COLLISION_RISK_CAUTION` ($< 8.0\,\text{m}$) and `COLLISION_RISK_CRITICAL` ($< 2.8\,\text{m}$).
- `EXCESSIVE_ALTITUDE`: Exceeding max authorized ceiling.
- `FAILED_RTH_ENERGY_DEFICIT`: Insufficient battery remaining to complete RTH transit.

---

## 7. SQLite Persistence and Export (`db` and `replay`)

- **Database Engine**: `rusqlite` with bundled SQLite library.
- **Schema**:
  - `scenarios`: Scenario definitions, obstacle layouts, NFZ polygons, wind parameters.
  - `missions`: Master mission runs, distance, duration, battery consumed, flight status.
  - `telemetry`: Timestamped samples with raw and filtered coordinates.
  - `sensor_readings`: Individual sensor telemetry outputs.
  - `faults` & `alerts`: Audit log of all injected faults and triggered safety alerts.
  - `mission_events`: Chronological lifecycle events.
  - `replay_metadata`: Replay frame indices and durations.
- **Replay Modes**: Synchronous frame delivery at $0.5\times$, $1\times$, $2\times$, $5\times$, and Instant playback with timeline scrubber.
- **Exports**:
  - Structured JSON mission package.
  - Formatted CSV telemetry matrix.
  - Standalone, self-contained HTML report featuring embedded SVG flight trajectories, KPI metric cards, and alert audit tables.

---

## 8. Mission Scripting Engine (`scripting`)

Supports text-based command sequences executed by an automated queue runner:
- `TAKEOFF <altitude>`
- `GOTO <x> <y> <altitude>`
- `WAIT <seconds>`
- `INJECT <fault_name>`
- `CLEAR_FAULTS`
- `RETURN_HOME`
- `LAND`
- `EMERGENCY_LAND`
