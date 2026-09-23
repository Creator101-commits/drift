//! 2D Occupancy Grid representation for spatial obstacle inflation and route planning.

use serde::{Deserialize, Serialize};
use crate::simulation::{Boundary, NoFlyZone, Obstacle};

/// Discretized 2D grid cell matrix for cost and traversal checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupancyGrid {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub resolution_m: f64,
    pub width_cells: usize,
    pub height_cells: usize,
    /// 1D boolean array where true = occupied/blocked, false = free.
    pub cells: Vec<bool>,
    pub safety_margin_m: f64,
}

impl OccupancyGrid {
    pub fn new(boundary: &Boundary, resolution_m: f64, safety_margin_m: f64) -> Self {
        let width = ((boundary.max_x - boundary.min_x) / resolution_m).ceil() as usize;
        let height = ((boundary.max_y - boundary.min_y) / resolution_m).ceil() as usize;
        let total = width * height;

        Self {
            min_x: boundary.min_x,
            max_x: boundary.max_x,
            min_y: boundary.min_y,
            max_y: boundary.max_y,
            resolution_m,
            width_cells: width,
            height_cells: height,
            cells: vec![false; total],
            safety_margin_m,
        }
    }

    pub fn build(
        boundary: &Boundary,
        resolution_m: f64,
        safety_margin_m: f64,
        obstacles: &[Obstacle],
        no_fly_zones: &[NoFlyZone],
    ) -> Self {
        let mut grid = Self::new(boundary, resolution_m, safety_margin_m);
        grid.rasterize(obstacles, no_fly_zones);
        grid
    }

    /// Rasterize physical obstacles and no-fly zones into the grid with safety expansion.
    pub fn rasterize(&mut self, obstacles: &[Obstacle], no_fly_zones: &[NoFlyZone]) {
        // Clear cells first
        self.cells.fill(false);

        for obs in obstacles {
            let total_radius = obs.radius + self.safety_margin_m;
            let min_gx = ((obs.x - total_radius - self.min_x) / self.resolution_m).floor().max(0.0) as usize;
            let max_gx = ((obs.x + total_radius - self.min_x) / self.resolution_m).ceil().min((self.width_cells - 1) as f64) as usize;
            let min_gy = ((obs.y - total_radius - self.min_y) / self.resolution_m).floor().max(0.0) as usize;
            let max_gy = ((obs.y + total_radius - self.min_y) / self.resolution_m).ceil().min((self.height_cells - 1) as f64) as usize;

            for gy in min_gy..=max_gy {
                for gx in min_gx..=max_gx {
                    let (wx, wy) = self.grid_to_world(gx, gy);
                    let dist = ((wx - obs.x).powi(2) + (wy - obs.y).powi(2)).sqrt();
                    if dist <= total_radius {
                        self.set_occupied(gx, gy, true);
                    }
                }
            }
        }

        for nfz in no_fly_zones {
            let total_radius = nfz.radius + self.safety_margin_m;
            let min_gx = ((nfz.center_x - total_radius - self.min_x) / self.resolution_m).floor().max(0.0) as usize;
            let max_gx = ((nfz.center_x + total_radius - self.min_x) / self.resolution_m).ceil().min((self.width_cells - 1) as f64) as usize;
            let min_gy = ((nfz.center_y - total_radius - self.min_y) / self.resolution_m).floor().max(0.0) as usize;
            let max_gy = ((nfz.center_y + total_radius - self.min_y) / self.resolution_m).ceil().min((self.height_cells - 1) as f64) as usize;

            for gy in min_gy..=max_gy {
                for gx in min_gx..=max_gx {
                    let (wx, wy) = self.grid_to_world(gx, gy);
                    let dist = ((wx - nfz.center_x).powi(2) + (wy - nfz.center_y).powi(2)).sqrt();
                    if dist <= total_radius {
                        self.set_occupied(gx, gy, true);
                    }
                }
            }
        }
    }

    pub fn world_to_grid(&self, wx: f64, wy: f64) -> Option<(usize, usize)> {
        if wx < self.min_x || wx >= self.max_x || wy < self.min_y || wy >= self.max_y {
            return None;
        }
        let gx = ((wx - self.min_x) / self.resolution_m).floor() as usize;
        let gy = ((wy - self.min_y) / self.resolution_m).floor() as usize;
        if gx < self.width_cells && gy < self.height_cells {
            Some((gx, gy))
        } else {
            None
        }
    }

    pub fn grid_to_world(&self, gx: usize, gy: usize) -> (f64, f64) {
        let wx = self.min_x + (gx as f64 + 0.5) * self.resolution_m;
        let wy = self.min_y + (gy as f64 + 0.5) * self.resolution_m;
        (wx, wy)
    }

    pub fn is_occupied(&self, gx: usize, gy: usize) -> bool {
        if gx >= self.width_cells || gy >= self.height_cells {
            return true;
        }
        self.cells[gy * self.width_cells + gx]
    }

    pub fn is_world_point_blocked(&self, wx: f64, wy: f64) -> bool {
        match self.world_to_grid(wx, wy) {
            Some((gx, gy)) => self.is_occupied(gx, gy),
            None => true, // Out of scenario boundaries
        }
    }

    pub fn set_occupied(&mut self, gx: usize, gy: usize, occupied: bool) {
        if gx < self.width_cells && gy < self.height_cells {
            let idx = gy * self.width_cells + gx;
            self.cells[idx] = occupied;
        }
    }
}
