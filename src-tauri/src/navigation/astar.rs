//! A-star (A*) optimal path planning and waypoint route smoothing over occupancy grids.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use super::occupancy::OccupancyGrid;
use crate::simulation::Vec2;

#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f64,
    x: usize,
    y: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* route planner that computes obstacle-free paths and performs line-of-sight path smoothing.
pub struct AStarPlanner;

impl AStarPlanner {
    /// Plan an optimal smooth path from start (sx, sy) to goal (gx, gy) in world coordinates.
    pub fn plan_path(
        grid: &OccupancyGrid,
        start_w: &Vec2,
        goal_w: &Vec2,
    ) -> Option<Vec<Vec2>> {
        let start_cell = match grid.world_to_grid(start_w.x, start_w.y) {
            Some(c) => c,
            None => return None,
        };

        let mut goal_cell = match grid.world_to_grid(goal_w.x, goal_w.y) {
            Some(c) => c,
            None => return None,
        };

        // If goal is occupied, search outward for closest accessible free cell
        if grid.is_occupied(goal_cell.0, goal_cell.1) {
            if let Some(free_goal) = find_nearest_free_cell(grid, goal_cell.0, goal_cell.1) {
                goal_cell = free_goal;
            } else {
                return None;
            }
        }

        let (sx, sy) = start_cell;
        let (gx, gy) = goal_cell;

        if sx == gx && sy == gy {
            return Some(vec![*start_w, *goal_w]);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_score: HashMap<(usize, usize), f64> = HashMap::new();

        g_score.insert((sx, sy), 0.0);
        let h_start = heuristic(sx, sy, gx, gy);
        open_set.push(State {
            cost: h_start,
            x: sx,
            y: sy,
        });

        // 8-direction movements: (dx, dy, cost)
        let sqrt2 = std::f64::consts::SQRT_2;
        let directions: [(isize, isize, f64); 8] = [
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (1, 1, sqrt2),
            (1, -1, sqrt2),
            (-1, 1, sqrt2),
            (-1, -1, sqrt2),
        ];

        let mut reached = false;

        while let Some(State { cost: _, x, y }) = open_set.pop() {
            if x == gx && y == gy {
                reached = true;
                break;
            }

            let current_g = match g_score.get(&(x, y)) {
                Some(&g) => g,
                None => continue,
            };

            for &(dx, dy, step_cost) in &directions {
                let nx_i = x as isize + dx;
                let ny_i = y as isize + dy;

                if nx_i < 0 || ny_i < 0 {
                    continue;
                }
                let nx = nx_i as usize;
                let ny = ny_i as usize;

                if nx >= grid.width_cells || ny >= grid.height_cells {
                    continue;
                }

                if grid.is_occupied(nx, ny) {
                    continue;
                }

                // Prevent cutting through corners of obstacles diagonally
                if dx != 0 && dy != 0 {
                    if grid.is_occupied(x, ny) || grid.is_occupied(nx, y) {
                        continue;
                    }
                }

                let tentative_g = current_g + step_cost;
                let neighbor_g = g_score.get(&(nx, ny)).copied().unwrap_or(f64::INFINITY);

                if tentative_g < neighbor_g {
                    came_from.insert((nx, ny), (x, y));
                    g_score.insert((nx, ny), tentative_g);
                    let f = tentative_g + heuristic(nx, ny, gx, gy);
                    open_set.push(State {
                        cost: f,
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        if !reached {
            return None;
        }

        // Reconstruct grid path
        let mut path_cells = Vec::new();
        let mut curr = (gx, gy);
        path_cells.push(curr);

        while let Some(&prev) = came_from.get(&curr) {
            path_cells.push(prev);
            curr = prev;
        }
        path_cells.reverse();

        // Convert grid cells to world points
        let mut raw_world_path: Vec<Vec2> = path_cells
            .iter()
            .map(|&(cx, cy)| {
                let (wx, wy) = grid.grid_to_world(cx, cy);
                Vec2::new(wx, wy)
            })
            .collect();

        // Pin exact start and goal coordinates
        if let Some(first) = raw_world_path.first_mut() {
            *first = *start_w;
        }
        if let Some(last) = raw_world_path.last_mut() {
            *last = *goal_w;
        }

        // Apply line-of-sight shortcutting / path smoothing
        let smoothed_path = smooth_path(grid, &raw_world_path);
        Some(smoothed_path)
    }
}

fn heuristic(x1: usize, y1: usize, x2: usize, y2: usize) -> f64 {
    let dx = (x1 as f64 - x2 as f64).abs();
    let dy = (y1 as f64 - y2 as f64).abs();
    // Octile distance heuristic for 8-connected grid
    let sqrt2 = std::f64::consts::SQRT_2;
    (dx + dy) + (sqrt2 - 2.0) * dx.min(dy)
}

fn find_nearest_free_cell(grid: &OccupancyGrid, cx: usize, cy: usize) -> Option<(usize, usize)> {
    for r in 1..20 {
        let r_i = r as isize;
        for dy in -r_i..=r_i {
            for dx in -r_i..=r_i {
                let nx = cx as isize + dx;
                let ny = cy as isize + dy;
                if nx >= 0 && ny >= 0 {
                    let unx = nx as usize;
                    let uny = ny as usize;
                    if unx < grid.width_cells && uny < grid.height_cells && !grid.is_occupied(unx, uny) {
                        return Some((unx, uny));
                    }
                }
            }
        }
    }
    None
}

/// Line-of-sight ray tracing smoothing to remove discrete grid stepping artifacts.
fn smooth_path(grid: &OccupancyGrid, path: &[Vec2]) -> Vec<Vec2> {
    if path.len() <= 2 {
        return path.to_vec();
    }

    let mut smoothed = Vec::new();
    smoothed.push(path[0]);

    let mut anchor_idx = 0;

    while anchor_idx < path.len() - 1 {
        let mut furthest_idx = anchor_idx + 1;

        for test_idx in (anchor_idx + 2)..path.len() {
            if has_line_of_sight(grid, &path[anchor_idx], &path[test_idx]) {
                furthest_idx = test_idx;
            } else {
                break;
            }
        }

        smoothed.push(path[furthest_idx]);
        anchor_idx = furthest_idx;
    }

    smoothed
}

/// Test whether a straight line between two world points is free of obstacles.
fn has_line_of_sight(grid: &OccupancyGrid, p1: &Vec2, p2: &Vec2) -> bool {
    let dist = p1.distance_to(p2);
    let steps = (dist / (grid.resolution_m * 0.4)).ceil() as usize;
    if steps == 0 {
        return true;
    }

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let wx = p1.x + (p2.x - p1.x) * t;
        let wy = p1.y + (p2.y - p1.y) * t;
        if grid.is_world_point_blocked(wx, wy) {
            return false;
        }
    }
    true
}
