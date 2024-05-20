use notan::draw::*;
use notan::math::{IVec2, Vec2};
use notan::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use crate::draw_arrow;

// ===============================
// NEIGHBOR CACHE
// ===============================
pub type NeighborCacheRef = Rc<RefCell<NeighborCache>>;
#[derive(Clone, Debug)]
pub struct NeighborCache {
    cache: Vec<Vec<(IVec2, i16, bool)>>,
    neighbor_xy_to_increment: HashMap<IVec2, i16>,
}

impl NeighborCache {
    pub fn new(max_increments: u16, arc: u16) -> Self {
        NeighborCache {
            cache: Vec::with_capacity(max_increments as usize),
            neighbor_xy_to_increment: HashMap::new(),
        }
    }
    pub fn new_precomputed(max_increments: u16, arc: u16) -> Self {
        let mut cache = Self::new(max_increments, arc);
        cache.precompute(max_increments, arc);
        cache
    }

    pub fn get(&self, rotation: i16) -> Option<&Vec<(IVec2, i16, bool)>> {
        self.cache.get(rotation as usize)
    }

    pub fn precompute(&mut self, max_increments: u16, arc: u16) {
        // Precompute increments pointing in "cardinal" directions.
        // Those are the increments that go most "straight" to that neighbor.
        let increment_size = std::f32::consts::PI * 2.0 / max_increments as f32;
        let cardinal_directions = vec![
            IVec2::new(0, 1),
            IVec2::new(1, 0),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            // diagonals
            IVec2::new(1, 1),
            IVec2::new(1, -1),
            IVec2::new(-1, 1),
            IVec2::new(-1, -1),
        ];
        // now use dot product to find the closest increment to each direction
        for direction in cardinal_directions {
            let mut closest_increment = 0;
            let mut closest_dot = -1.0;
            for increment in 0..max_increments {
                let angle = increment as f32 * increment_size;
                let rotation_vector = Vec2::from_angle(angle);
                let direction_vector = Vec2::new(direction.x as f32, direction.y as f32);
                let dot = rotation_vector.dot(direction_vector);
                if dot > closest_dot {
                    closest_dot = dot;
                    closest_increment = increment;
                }
            }
            self.neighbor_xy_to_increment
                .insert(direction, closest_increment as i16);
        }
        // now print them pretty
        for (direction, increment) in self.neighbor_xy_to_increment.iter() {
            println!("Direction: {:?} -> Increment: {}", direction, increment);
        }

        // Precompute the neighbors for each rotation.
        for rotation in 0..max_increments as i16 {
            let arc = arc as i16;
            let mut neighbors = Vec::with_capacity((arc * 2 + 1) as usize);

            for i in -arc..=arc {
                let new_rotation = Cell::clamp_rotation(rotation + i, max_increments as i16);
                let cell = Cell::precompute_neighbor(new_rotation, increment_size, false);
                neighbors.push((cell.position, cell.rotation, cell.reverse));
            }

            let reverse_arc = arc * 2;
            for i in -reverse_arc..=reverse_arc {
                let new_rotation = Cell::clamp_rotation(rotation + i, max_increments as i16);
                let cell = Cell::precompute_neighbor(new_rotation, increment_size, true);
                neighbors.push((cell.position, cell.rotation, cell.reverse));
            }

            // filter out the neighbors which don't follow one of the
            // following rules:
            // 1. rotation changed, there was turning
            // 2. rotation didn't change, but going in a "cardinal" direction
            // 3. going in reverse
            neighbors = neighbors
                .into_iter()
                .filter(|(pos, rot, rev)| {
                    let rotation_changed = *rot != rotation;
                    let cardinal = self
                        .neighbor_xy_to_increment
                        .values()
                        .any(|&inc| inc == *rot);
                    rotation_changed || cardinal || *rev
                })
                .collect();

            self.cache.push(neighbors);
        }
    }
}

// ===============================
// COST CACHE
// ===============================
pub type CostCacheRef = Rc<RefCell<CostCache>>;
#[derive(Clone, Debug)]
pub struct CostCache {
    cache: Vec<Vec<u32>>,
}

// ===============================
// CELL
// ===============================
#[derive(Clone, Debug)]
pub struct Cell {
    pub rotation: i16,
    pub position: IVec2,
    pub reverse: bool,
}
impl Cell {
    pub fn new(rotation: i16, start: IVec2, reverse: bool) -> Self {
        Self {
            rotation,
            position: start,
            reverse,
        }
    }
    pub fn precompute_neighbor(rotation: i16, increment_size: f32, reverse: bool) -> Self {
        let angle = rotation as f32 * increment_size;
        let angle = if reverse {
            angle + std::f32::consts::PI
        } else {
            angle
        };
        let rotation_vector = Vec2::from_angle(angle);
        let x = rotation_vector.x.round() as i32;
        let y = rotation_vector.y.round() as i32;
        let direction_vector = Vec2::new(x.clamp(-1, 1) as f32, y.clamp(-1, 1) as f32);
        let new_position = IVec2::new(direction_vector.x as i32, direction_vector.y as i32);

        Self {
            position: new_position,
            rotation,
            reverse,
        }
    }
    pub fn neighbors(&self, cache: &NeighborCacheRef, arc: u16, max_increments: u16) -> Vec<Self> {
        let mut neighbors = Vec::new();
        if let Some(cached) = cache.borrow().get(self.rotation) {
            neighbors = Vec::with_capacity(cached.len());
            for (position, rotation, reverse) in cached {
                let new_position = self.position + *position;
                let new_rotation = *rotation;
                let new_reverse = *reverse;
                neighbors.push(Self {
                    position: new_position,
                    rotation: new_rotation,
                    reverse: new_reverse,
                });
            }
        }
        neighbors
    }
    pub fn opposite_rotation(&self, rotation: i16, max_increments: i16) -> i16 {
        let current_rotation = rotation as i32;
        let max_rotation = max_increments as i32;
        let opposite_rotation = current_rotation + max_rotation / 2;
        Self::clamp_rotation(opposite_rotation as i16, max_increments)
    }
    pub fn clamp_rotation(rotation: i16, max_increments: i16) -> i16 {
        let max_rotation = max_increments as i16;
        if rotation < 0 {
            max_rotation + rotation
        } else if rotation >= max_rotation {
            rotation - max_rotation
        } else {
            rotation
        }
    }
    pub fn cost(&self, from: Option<Cell>, arc: u16, max_increments: u16) -> u32 {
        fn angle_difference(angle1: f32, angle2: f32) -> f32 {
            let pi = std::f32::consts::PI;
            let two_pi = 2.0 * pi;
            let mut diff = (angle2 - angle1) % two_pi;

            if diff > pi {
                diff -= two_pi;
            } else if diff < -pi {
                diff += two_pi;
            }

            diff
        }
        let speed_loss_fraction = |turn_angle_rad: f32| -> f32 {
            let initial_speed = 40.0 * 0.27778; // 40 km/h in m/s
            let friction_coefficient = 0.75;
            let g = 9.81;

            // Calculate the maximum safe speed
            let max_safe_speed = (friction_coefficient * g * (1.0 / turn_angle_rad)).sqrt();

            // If max_safe_speed is greater than initial_speed, cap it at initial_speed
            let capped_max_safe_speed = if max_safe_speed > initial_speed {
                initial_speed
            } else {
                max_safe_speed
            };

            // Calculate the speed loss fraction
            let speed_loss = (initial_speed - capped_max_safe_speed) / initial_speed;

            // Ensure the fraction is between 0 and 1
            speed_loss.max(0.0).min(1.0)
        };

        let reverse_cost = if self.reverse { 10 } else { 1 };
        if let Some(from) = from {
            let angle1 = self.rotation as f32 * 2.0 * std::f32::consts::PI / max_increments as f32;
            let angle2 = from.rotation as f32 * 2.0 * std::f32::consts::PI / max_increments as f32;
            let angle_diff = angle_difference(angle1, angle2).abs();
            let speed_loss = speed_loss_fraction(angle_diff);
            let angle_cost = (speed_loss * 1000.0) as u32;

            let distance = self
                .position
                .as_vec2()
                .distance_squared(from.position.as_vec2());
            let distance_cost = (distance * 1000.0) as u32;

            (angle_cost + distance_cost) * reverse_cost
        } else {
            0
        }
    }
    pub fn heuristic(&self, to: IVec2, max_increments: u16) -> u32 {
        let distance = self.position.as_vec2().distance_squared(to.as_vec2());
        (distance * 10.0) as u32
    }

    pub fn draw(&self, draw: &mut Draw, font: &Font, cell_size: f32, max_increments: u16) {
        // Define the color based on the reverse flag
        let color = if self.reverse {
            Color::RED
        } else {
            Color::BLUE
        };

        // Calculate the center of the current cell as the starting point
        let center = Vec2::new(
            self.position.x as f32 * cell_size + cell_size / 2.0,
            self.position.y as f32 * cell_size + cell_size / 2.0,
        );

        // Determine the rotation angle, adjusting for reverse if necessary
        let rotation_angle = if self.reverse {
            self.opposite_rotation(self.rotation, max_increments as i16)
        } else {
            self.rotation
        } as f32
            * 2.0
            * std::f32::consts::PI
            / max_increments as f32;

        // Calculate the end point of the arrow based on the rotation angle
        // Ensuring it remains visually centered within the cell
        let arrow_length = cell_size / 2.0; // Adjust this value to change the arrow's length
        let end = center + Vec2::from_angle(rotation_angle).normalize() * arrow_length;

        // Draw the arrow from center to the calculated end point
        draw_arrow(draw, center, end, color);

        // Write the data (rotation) below the arrow
        let text = format!("R: {}", self.rotation);
        let text_position = Vec2::new(
            self.position.x as f32 * cell_size,
            (self.position.y as f32 + 1.0) * cell_size - 20.0, // Adjust this to position the text below the cell
        );
        draw.text(&font, &text)
            .translate(text_position.x, text_position.y)
            .size(15.0)
            .color(Color::WHITE);
    }
}
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.rotation == other.rotation
    }
}
impl Eq for Cell {}
impl Hash for Cell {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position.hash(state);
        self.rotation.hash(state);
        self.reverse.hash(state);
    }
}
