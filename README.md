<!-- ![Banner](./image.png) -->
# Drift
> Real-time autonomous drone mission simulator and telemetry analyzer.

![GitHub release (latest by date including pre-releases)](https://img.shields.io/github/v/release/Creator101-commits/drift?include_prereleases)
![GitHub last commit](https://img.shields.io/github/last-commit/Creator101-commits/drift)
![GitHub issues](https://img.shields.io/github/issues-raw/Creator101-commits/drift)
![GitHub pull requests](https://img.shields.io/github/issues-pr/Creator101-commits/drift)
![GitHub](https://img.shields.io/github/license/Creator101-commits/drift)

Drift is a real-time autonomous drone mission simulator and telemetry analysis workstation built on Tauri 2, Rust, React, and SQLite. It provides deterministic 20 Hz simulation with altitude, flight modes, battery drain, crosswind dynamics, obstacle avoidance, no-fly zones, and sensor fault injection.

The platform includes an 8-beam to 16-beam ray-casting LiDAR sensor, barometric altimeter, 6-DOF IMU, digital compass, and multi-constellation GPS with deterministic seeded noise. An integrated Kalman-style state estimator tracks both raw and filtered positioning in real time alongside automated A* route planning, safety monitoring, mission script execution, SQLite persistence, deterministic replay, and multi-format exports (JSON, CSV, and HTML reports).

## Table of Contents
- [Drift](#drift)
- [Quickstart / Demo](#quickstart--demo)
- [Installation](#installation)
- [Usage](#usage)
- [Development](#development)
- [Contributing](#contributing)
- [Release History](#release-history)
- [License](#license)
- [Meta](#meta)

## Quickstart / Demo
[(Back to top)](#table-of-contents)

Drift supports an end-to-end autonomous mission demonstration workflow:

1. **Load Scenario**: Select "Urban Grid Surveillance" or "Canyon Wind Obstacle Challenge".
2. **Arm and Takeoff**: Arm the quadcopter motors and command vertical climb to 15m hover altitude.
3. **Select Waypoint**: Click any scenario waypoint or right-click directly on the tactical map to generate a collision-free A* route avoiding obstacles and restricted airspace.
4. **Follow Route**: The autonomous guidance controller traverses the path while real-time telemetry streams at 20 Hz.
5. **Inject Sensor Faults**: Toggle GPS Drift, GPS Dropout, IMU Bias, Compass Failure, or LiDAR Blind Spots.
6. **Observe Filtered Localization**: The map simultaneously renders the noisy raw GPS breadcrumb path and the stable Kalman-filtered localization estimate.
7. **Fires Alerts**: The anomaly engine flags sensor drift, geofence buffer proximity, and collision risks.
8. **Return to Home / Emergency Landing**: Command automated RTH transit or immediate emergency descent.
9. **Save Run**: Persist the completed mission telemetry and alert audit trail into SQLite.
10. **Replay & Export**: Replay the mission at variable speeds (0.5x, 1x, 2x, 5x, Instant) and export to JSON, CSV, or standalone HTML reports.

## Installation
[(Back to top)](#table-of-contents)

> **Note**: For longer README files, a "Back to top" link like the one above makes it easy to navigate.

Prerequisites:
- Node.js (v18+) and npm
- Rust toolchain (cargo 1.77+)

**macOS & Linux**

```sh
# Clone repository
git clone https://github.com/Creator101-commits/drift.git
cd drift

# Install frontend dependencies
npm install

# Run backend unit and integration test suite
cd src-tauri && cargo test && cd ..
```

**Windows**

```sh
git clone https://github.com/Creator101-commits/drift.git
cd drift
npm install
cd src-tauri && cargo test && cd ..
```

## Usage
[(Back to top)](#table-of-contents)

### Running the Desktop Application

```sh
# Start development desktop application with hot-reloading
npm run tauri dev
```

### Building Release Executable

```sh
# Build optimized desktop binary package
npm run tauri build
```

### Scripting Engine Commands

Drift includes a built-in mission command runner. You can execute custom script files using syntax such as:

```text
TAKEOFF 20
WAIT 3
GOTO 190 90 25
INJECT GPS_DRIFT
WAIT 5
GOTO 330 200 30
CLEAR_FAULTS
RETURN_HOME
```

## Development
[(Back to top)](#table-of-contents)

Instructions for setting up a local development environment:

```sh
git clone https://github.com/Creator101-commits/drift.git
cd drift

# Install dependencies
npm install

# Run frontend build
npm run build

# Run Rust cargo check and tests
cd src-tauri
cargo check
cargo test
cd ..

# Run in desktop development mode
npm run tauri dev
```

## Contributing
[(Back to top)](#table-of-contents)

Contributions are welcome. To propose a change:

1. Fork it (<https://github.com/Creator101-commits/drift/fork>)
2. Create your feature branch (`git checkout -b feature/fooBar`)
3. Commit your changes (`git commit -am 'Add some fooBar'`)
4. Push to the branch (`git push origin feature/fooBar`)
5. Open a new Pull Request

Please make sure tests pass and the code is formatted before opening a PR.

## Release History
[(Back to top)](#table-of-contents)

* 0.1.0
    * Initial release: fixed-timestep 20 Hz simulation engine
    * Deterministic sensor suite with GPS, IMU, Compass, Altimeter, and 16-beam LiDAR
    * A* occupancy grid path planner with line-of-sight path smoothing
    * Filtered state estimator rendering raw vs filtered telemetry paths
    * Anomaly and alert engine with configurable safety thresholds
    * SQLite persistence with rusqlite
    * Multi-speed deterministic replayer (0.5x to instant)
    * Multi-format exports: JSON, CSV, and standalone HTML reports
    * Script command runner for automated mission execution

## License
[(Back to top)](#table-of-contents)

Distributed under the MIT License. See [`LICENSE`](./LICENSE) for more information.

## Meta
[(Back to top)](#table-of-contents)

Sreeharsha Kannegundla – [@Creator101-commits](https://github.com/Creator101-commits)

Project link: [https://github.com/Creator101-commits/drift](https://github.com/Creator101-commits/drift)
