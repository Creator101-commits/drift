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
- **Simulation Models**: Explicit typed structs for drone state, flight modes, waypoints, obstacles, and geofence bounds.

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
