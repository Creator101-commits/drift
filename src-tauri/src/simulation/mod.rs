//! Core simulation engine, drone kinematics, and environmental physics.

pub mod drone;
pub mod engine;
pub mod physics;

pub use drone::{Drone, DroneState, FlightMode, Waypoint};
pub use engine::{MissionEvent, ScenarioConfig, SimulationEngine, SimulationSnapshot};
pub use physics::{Boundary, DronePhysicalConfig, NoFlyZone, Obstacle, Vec2, WindCondition};
