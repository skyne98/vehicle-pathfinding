use notan::{
    app::Color,
    draw::{Draw, DrawShapes, DrawTransform},
    math::{Affine2, Mat3, Vec2},
};

pub struct Agent {
    pub position: Vec2,
    pub size: Vec2,
    /// Rotation in radians.
    pub rotation: f32,
}

impl Agent {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
            rotation: 0.0,
        }
    }

    pub fn footprint(&self, position: Vec2, rotation: f32) -> Vec<Vec2> {
        let half_width = self.size.x / 2.0;
        let half_height = self.size.y / 2.0;
        let (x, y) = (position.x, position.y);
        let corners = [
            Vec2::new(
                -half_width,
                -half_height, // top-left
            ),
            Vec2::new(
                half_width,
                -half_height, // top-right
            ),
            Vec2::new(
                half_width,
                half_height, // bottom-right
            ),
            Vec2::new(
                -half_width,
                half_height, // bottom-left
            ),
        ];

        let transform = Affine2::from_translation(Vec2::new(x, y)) * Affine2::from_angle(rotation);
        let corners_transformed: Vec<Vec2> = corners
            .iter()
            .map(|corner| transform.transform_point2(*corner))
            .collect();

        let min_x = corners_transformed
            .iter()
            .map(|corner| corner.x)
            .fold(f32::INFINITY, |acc, x| x.min(acc));
        let min_y = corners_transformed
            .iter()
            .map(|corner| corner.y)
            .fold(f32::INFINITY, |acc, y| y.min(acc));
        let max_x = corners_transformed
            .iter()
            .map(|corner| corner.x)
            .fold(f32::NEG_INFINITY, |acc, x| x.max(acc));
        let max_y = corners_transformed
            .iter()
            .map(|corner| corner.y)
            .fold(f32::NEG_INFINITY, |acc, y| y.max(acc));

        let mut footprint = Vec::new();
        for x in (min_x as i32)..=(max_x as i32) {
            for y in (min_y as i32)..=(max_y as i32) {
                footprint.push(Vec2::new(x as f32, y as f32));
            }
        }

        footprint
    }
    pub fn current_footprint(&self) -> Vec<Vec2> {
        self.footprint(self.position, self.rotation)
    }

    pub fn draw(&mut self, draw: &mut Draw, color: Color, cell_size: f32) {
        let (x_grid, y_grid) = (self.position.x * cell_size, self.position.y * cell_size);
        let (width_grid, height_grid) = (self.size.x * cell_size, self.size.y * cell_size);

        let half_width = width_grid / 2.0;
        let half_height = height_grid / 2.0;

        let transform = Affine2::from_translation(Vec2::new(x_grid, y_grid))
            * Affine2::from_angle(self.rotation)
            * Affine2::from_translation(-Vec2::new(half_width, half_height));
        draw.rect((0.0, 0.0), (width_grid, height_grid))
            .color(color)
            .transform(transform.into());

        // draw the front
        draw.line(
            (half_width, half_height),
            (half_width + half_width, half_height),
        )
        .color(Color::YELLOW)
        .transform(transform.into());
    }
    pub fn draw_current_footprint(&mut self, draw: &mut Draw, color: Color, cell_size: f32) {
        for footprint in self.current_footprint() {
            draw.rect(
                (footprint.x * cell_size, footprint.y * cell_size),
                (cell_size, cell_size),
            )
            .color(color);
        }
    }
}
