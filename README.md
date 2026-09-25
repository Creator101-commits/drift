# Drift

A real-time autonomous drone mission simulator and telemetry analyzer for desktop that I created with Tauri 2, Rust, and React. Simulate fixed-timestep 20 Hz quadcopter flight physics, watch live sensor telemetry coming into your application in real-time, inspect Kalman-filtered versus noisy raw sensor streams, and store recorded flight runs locally in an SQLite database.

It automatically plans collision-free A* routes around obstacles and restricted airspace, warns about anomalies (geofence breaches, sensor drift, low battery, collision risk), and gives you the ability to replay or even re-inject deterministic sensor faults. And it comes with a basic scripting layer to allow you to run automated mission profiles.

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

For the development process, I used AI to scaffold boilerplate code and UI, and to detect bugs in my code reviews. Everything related to simulation dynamics (fixed-timestep kinematics, deterministic sensor models, Kalman filtering, A* routing, architectural choices, etc.) was done manually by me and was tested.
