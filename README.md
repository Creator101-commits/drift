# Drift

This is a desktop-based drone mission simulation software and telemetry analyzer that I have built using Tauri 2, Rust and React. Simulate the physics of the 20Hz quadcopter flight with a fixed timestep, view the real-time telemetry data that comes into your application, view the Kalman-filtered data stream versus the noisy raw sensor data streams, and log your flight data to an SQLite database.

This simulation can be used to automatically plan an A* collision-free path for the quadcopter around obstacles and restricted airspaces, notify about any anomalies (such as geofencing violations, sensor drifts, low battery status, collision hazard), and replay/ re-inject deterministic sensor faults. This comes with a basic scripting interface so that you can run your mission profiles.

```sh
git clone https://github.com/Creator101-commits/drift.git
cd drift
npm install
npm run tauri dev
```

## Opening the macOS app

The downloadable macOS build supports Apple Silicon Macs only (`arm64`).

1. Download the `Drift_*.dmg` file from Releases.
2. Open the DMG file.
3. Drag `Drift.app` into the Applications folder.
4. Open Applications and double-click Drift.
5. If macOS blocks the app, try opening it once, then go to **System Settings -> Privacy & Security** and click **Open Anyway**.

If macOS says the app is damaged and **Open Anyway** is not shown, run this in Terminal after copying Drift to Applications:

```sh
xattr -dr com.apple.quarantine "/Applications/Drift.app"
open "/Applications/Drift.app"
```

The macOS build is ad-hoc signed and not notarized by Apple, so this security step may be required.

For the development process, I used AI to scaffold boilerplate code and UI, and to detect bugs in my code reviews. All code was also reviewed using AI and imporved the logic to be faster and easier to use. Everything related to simulation dynamics (fixed-timestep kinematics, deterministic sensor models, Kalman filtering, A* routing, architectural choices, etc.) was done manually by me and was tested.
