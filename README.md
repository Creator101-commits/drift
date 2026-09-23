<!-- ![Banner](./image.png) -->
# Drift
> Real-time autonomous drone mission simulator and telemetry analyzer.

![GitHub release (latest by date including pre-releases)](https://img.shields.io/github/v/release/Creator101-commits/drift?include_prereleases)
![GitHub last commit](https://img.shields.io/github/last-commit/Creator101-commits/drift)
![GitHub issues](https://img.shields.io/github/issues-raw/Creator101-commits/drift)
![GitHub pull requests](https://img.shields.io/github/issues-pr/Creator101-commits/drift)
![GitHub](https://img.shields.io/github/license/Creator101-commits/drift)

Drift is a real-time autonomous drone mission simulator and telemetry analysis workstation built on Tauri 2, Rust, React, and SQLite. It provides deterministic 20 Hz simulation with altitude, flight modes, battery drain, crosswind dynamics, obstacle avoidance, no-fly zones, and sensor fault injection.

## Table of Contents
- [Drift](#drift)
- [Quickstart / Demo](#quickstart--demo)
- [Installation](#installation)
- [Architecture Overview](#architecture-overview)
- [Development](#development)
- [Contributing](#contributing)
- [Release History](#release-history)
- [License](#license)
- [Meta](#meta)

## Quickstart / Demo
[(Back to top)](#table-of-contents)

Drift supports an end-to-end autonomous mission demonstration workflow:

1. **Load Scenario**: Select "Urban Grid Surveillance" or "Canyon Wind Obstacle Challenge".
2. **Interactive Map**: View flight bounds, obstacles, no-fly zones, and mission waypoints rendered on an HTML5 canvas.

## Installation
[(Back to top)](#table-of-contents)

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

# Run frontend build
npm run build
```

**Windows**

```sh
git clone https://github.com/Creator101-commits/drift.git
cd drift
npm install
npm run build
```

## Architecture Overview
[(Back to top)](#table-of-contents)

- **Frontend**: React 18, TypeScript, Vite, HTML5 Canvas 2D tactical mission map.
- **Desktop Runtime**: Tauri 2 native desktop bridge.
- **Simulation Engine**: Fixed-timestep 20 Hz loop (dt = 0.05s) simulating aerodynamic drag, crosswind, battery consumption, and obstacle boundary collisions.

## Drone Dynamics & 20 Hz Simulation Engine
[(Back to top)](#table-of-contents)

Drift features a fixed-timestep 20 Hz (dt = 0.05s) simulation engine written in Rust:
- **Kinematics**: 2.5D position (x, y, altitude), horizontal/vertical velocity vectors, heading orientation.
- **Environmental Physics**: Continuous crosswind and gust velocity vectors affecting drone trajectory.
- **Battery Dynamics**: Base avionics power drain plus quadratic rotor thrust discharge during climb and acceleration.
- **Safety Boundaries**: Physical map bounds, spherical obstacle collision boundaries, and polygonal no-fly zones.

## Development
[(Back to top)](#table-of-contents)

```sh
# Install dependencies
npm install

# Run frontend build
npm run build

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

* 0.3.0
    * Occupancy grid construction with obstacle dilation for safety buffers
    * A* autonomous route planner with dynamic obstacle avoidance
    * Kalman-style state estimator tracking dual raw GPS vs filtered positions
    * Geofence boundary violation warnings, collision risk alerts, and automated RTH
* 0.2.0
    * Simulated flight sensor suite: GPS, 6-DOF IMU, Digital Compass, Barometer, 16-beam LiDAR
    * Deterministic seeded noise via ChaCha8 PRNG for repeatable test runs
    * Real-time fault injection: GPS drift/dropout, IMU bias, compass lock, altimeter drift, LiDAR blind spots
    * Live sensor-health telemetry status monitoring in frontend
* 0.1.0
    * Foundation release: Tauri 2, Rust simulation core, React + TypeScript frontend
    * Interactive 2D tactical mission map rendering boundaries, obstacles, and waypoints
    * Seeded scenario loading (Urban Grid Surveillance, Canyon Wind Challenge)
    * Real-time telemetry data models for autonomous flight state

## License
[(Back to top)](#table-of-contents)

Distributed under the MIT License. See [`LICENSE`](./LICENSE) for more information.

## Meta
[(Back to top)](#table-of-contents)

Sreeharsha Kannegundla – [@Creator101-commits](https://github.com/Creator101-commits)

Project link: [https://github.com/Creator101-commits/drift](https://github.com/Creator101-commits/drift)


## Simulated Flight Sensor Suite & Fault Injection
[(Back to top)](#table-of-contents)

Each sensor produces timestamped readings generated from a seeded ChaCha8 PRNG:
- **GPS Receiver**: Longitude, latitude, horizontal speed, fix quality, dilution of precision.
- **6-DOF IMU**: 3-axis accelerometer and 3-axis angular gyroscope rates with temperature compensation.
- **Digital Compass**: Magnetometer heading in degrees [0, 360) with magnetic declination offset.
- **Barometric Altimeter**: Atmospheric pressure sensor measuring relative altitude with altitude drift models.
- **16-Beam LiDAR**: Radial ray-casting scanner measuring distances to scenario obstacles and terrain.

### Deterministic Sensor Faults
Drift supports on-the-fly fault injection to simulate real-world hardware degradation:
- **GPS Drift**: Systematic wander in estimated coordinates.
- **GPS Dropout**: Complete loss of satellite fix.
- **IMU Bias**: Steady acceleration / gyroscope offset causing attitude drift.
- **Compass Lock**: Heading lock or magnetic interference.
- **Altimeter Drift**: Barometric bias causing vertical tracking error.
- **LiDAR Blind Spots**: Laser emitter occlusions.


## Autonomous Navigation & State Estimation
[(Back to top)](#table-of-contents)

- **Occupancy Grid**: Continuous obstacle map discretized into a 2D occupancy grid with configurable safety dilation margins.
- **A* Route Planning**: 8-directional heuristic search with line-of-sight path smoothing avoiding obstacles and restricted airspace.
- **Kalman-Style State Estimator**: Combines noisy GPS measurements with high-rate IMU dead reckoning and barometric altitude. The map renders both raw GPS breadcrumbs and stable filtered trajectories simultaneously.
- **Safety Monitor**: Real-time evaluation of geofence margins, collision proximity risks, automated Return-to-Home (RTH), and emergency descent.
