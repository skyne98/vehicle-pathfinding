use notan::draw::*;
use notan::math::{IVec2, Vec2};
use notan::prelude::*;
use std::hash::Hash;

use crate::draw_arrow;

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
    pub fn neighbor(&self, rotation: i16, increment_size: f32, reverse: bool) -> Self {
        // Convert rotation increments into an angle in radians.
        let angle = rotation as f32 * increment_size;
        let angle = if reverse {
            angle + std::f32::consts::PI
        } else {
            angle
        };
        let rotation_vector = Vec2::from_angle(angle);

        // Calculate the x and y components of the direction vector.
        let x = rotation_vector.x.round() as i32;
        let y = rotation_vector.y.round() as i32;

        // Ensure that the direction vector is within the valid range.
        let direction_vector = Vec2::new(x.clamp(-1, 1) as f32, y.clamp(-1, 1) as f32);

        // Calculate the new position by adding the direction vector to the current position.
        let new_position = self.position + direction_vector.as_ivec2();

        // Assert that the new position is different from the old position to ensure movement.
        assert_ne!(
            self.position, new_position,
            "The position should change; direction_vector: {:?}, old_position: {:?}, new_position: {:?}",
            direction_vector, self.position, new_position
        );

        // Return a new instance of the struct with the updated position.
        Self::new(rotation, new_position, reverse)
    }
    pub fn neighbors(&self, arc: u16, max_increments: u16) -> Vec<Self> {
        let arc = arc as i16;
        let mut neighbors = Vec::with_capacity((arc * 2 + 1) as usize);
        let increment_size = std::f32::consts::PI * 2.0 / max_increments as f32;
        for i in -arc..=arc {
            let new_rotation = self.rotation + i as i16;
            let new_rotation = Self::clamp_rotation(new_rotation, max_increments as i16);
            let cell = self.neighbor(new_rotation, increment_size, false);
            neighbors.push(cell);
        }

        // now reverse, which has 2x the arc and 2x the cost
        let reverse_arc = arc * 2;
        for i in -reverse_arc..=reverse_arc {
            let new_rotation = self.rotation + i as i16;
            let new_rotation = Self::clamp_rotation(new_rotation, max_increments as i16);
            let cell = self.neighbor(new_rotation, increment_size, true);
            neighbors.push(cell);
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
    pub fn cost(&self, from: Option<Cell>, max_increments: u16) -> u32 {
        let reverse_cost = if self.reverse { 4 } else { 1 };
        if let Some(from) = from {
            let direction = (self.position - from.position).as_vec2().normalize();
            let angle = direction.y.atan2(direction.x);
            let expected_rotation =
                (angle / (2.0 * std::f32::consts::PI) * max_increments as f32).round() as i16;

            let rotation_difference = (from.rotation - expected_rotation).abs() as u32;
            let rotation_difference_fraction = rotation_difference * 100 / max_increments as u32;

            let expected_to_final_fraction = (self.rotation - expected_rotation).abs() as u32;
            let expected_to_final_fraction =
                expected_to_final_fraction * 100 / max_increments as u32;

            let distance = self
                .position
                .as_vec2()
                .distance_squared(from.position.as_vec2());
            let distance = (distance * 100.0) as u32;

            (rotation_difference_fraction + expected_to_final_fraction + distance) * reverse_cost
        } else {
            0
        }
    }
    pub fn heuristic(&self, to: IVec2) -> u32 {
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
