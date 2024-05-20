use notan::{
    app::Color,
    draw::{Draw, DrawShapes, DrawTransform},
    math::{Affine2, IVec2, Mat3, Vec2},
};

fn aabb_rect_collision(
    aabb_x: f32,
    aabb_y: f32, // AABB upper-left corner
    rect_center_x: f32,
    rect_center_y: f32,
    rect_half_extents_x: f32,
    rect_half_extents_y: f32,
    rect_angle: f32,
) -> bool {
    // Helper functions
    fn dot(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
        ax * bx + ay * by
    }

    fn rotate(x: f32, y: f32, angle: f32) -> (f32, f32) {
        let cos_theta = angle.cos();
        let sin_theta = angle.sin();
        (x * cos_theta - y * sin_theta, x * sin_theta + y * cos_theta)
    }

    fn get_axes(rect_angle: f32) -> [(f32, f32); 2] {
        // The axes are the normals to the rectangle's sides
        let (x1, y1) = rotate(1.0, 0.0, rect_angle); // First axis
        let (x2, y2) = rotate(0.0, 1.0, rect_angle); // Second axis
        [(x1, y1), (x2, y2)]
    }

    fn project_onto_axis(vertices: &[(f32, f32)], axis_x: f32, axis_y: f32) -> (f32, f32) {
        let mut min = dot(vertices[0].0, vertices[0].1, axis_x, axis_y);
        let mut max = min;
        for &(vx, vy) in &vertices[1..] {
            let projection = dot(vx, vy, axis_x, axis_y);
            if projection < min {
                min = projection;
            }
            if projection > max {
                max = projection;
            }
        }
        (min, max)
    }

    // Get the vertices of the rotated rect
    let half_extents = [
        (rect_half_extents_x, rect_half_extents_y),
        (rect_half_extents_x, -rect_half_extents_y),
        (-rect_half_extents_x, rect_half_extents_y),
        (-rect_half_extents_x, -rect_half_extents_y),
    ];
    let mut rect_vertices = [(0.0, 0.0); 4];
    for i in 0..4 {
        let (hx, hy) = half_extents[i];
        let (rx, ry) = rotate(hx, hy, rect_angle);
        rect_vertices[i] = (rect_center_x + rx, rect_center_y + ry);
    }

    // Get the vertices of the AABB
    let aabb_vertices = [
        (aabb_x, aabb_y),
        (aabb_x + 1.0, aabb_y),
        (aabb_x, aabb_y + 1.0),
        (aabb_x + 1.0, aabb_y + 1.0),
    ];

    // Axes to test
    let rect_axes = get_axes(rect_angle);
    let aabb_axes = [(1.0, 0.0), (0.0, 1.0)];

    // Check for separation on each axis
    for &(axis_x, axis_y) in rect_axes.iter().chain(aabb_axes.iter()) {
        let (rect_min, rect_max) = project_onto_axis(&rect_vertices, axis_x, axis_y);
        let (aabb_min, aabb_max) = project_onto_axis(&aabb_vertices, axis_x, axis_y);
        if rect_max < aabb_min || aabb_max < rect_min {
            return false;
        }
    }

    true
}

pub struct Agent {
    pub position: IVec2,
    pub size: Vec2,
    /// Rotation in increments.
    pub rotation: i16,
    pub max_increments: u16,

    footprints_cache: Vec<Vec<IVec2>>,
}

impl Agent {
    pub fn new(position: IVec2, size: Vec2, rotation: i16, max_increments: u16) -> Self {
        let mut footprints_cache = Vec::with_capacity(max_increments as usize);
        let half_width = size.x / 2.0;
        let half_height = size.y / 2.0;

        for increment in 0..max_increments {
            let angle = 2.0 * std::f32::consts::PI * (increment as f32) / (max_increments as f32);
            let transform = Affine2::from_angle(angle);

            // Define the corners of the rectangle
            let corners = [
                Vec2::new(-half_width, -half_height),
                Vec2::new(half_width, -half_height),
                Vec2::new(half_width, half_height),
                Vec2::new(-half_width, half_height),
            ];

            // Add 0.5 to the corners to center them
            let transformed_corners: Vec<Vec2> = corners
                .iter()
                .map(|&corner| transform.transform_point2(corner))
                .collect();

            // Calculate bounding box of the transformed rectangle
            let (min_x, max_x) = transformed_corners.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min_x, max_x), corner| (min_x.min(corner.x), max_x.max(corner.x)),
            );
            let (min_y, max_y) = transformed_corners.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min_y, max_y), corner| (min_y.min(corner.y), max_y.max(corner.y)),
            );

            // Create a list of all the points inside the bounding box
            let mut footprint = Vec::new();
            for x in min_x.round() as i32..=max_x.round() as i32 {
                for y in min_y.round() as i32..=max_y.round() as i32 {
                    // test if the cell collides with the agent
                    if !aabb_rect_collision(
                        x as f32,
                        y as f32,
                        0.5,
                        0.5,
                        half_width,
                        half_height,
                        angle,
                    ) {
                        continue;
                    }

                    let point = IVec2::new(x, y);
                    footprint.push(point);
                }
            }

            footprints_cache.push(footprint);
        }

        Self {
            position,
            size,
            rotation,
            max_increments,
            footprints_cache,
        }
    }

    pub fn rotation_footprint(&self, rotation: i16) -> &Vec<IVec2> {
        &self.footprints_cache[rotation as usize]
    }
    pub fn footprint(&self, position: IVec2, rotation: i16) -> Vec<IVec2> {
        let footprint = &self.footprints_cache[rotation as usize];
        footprint
            .iter()
            .map(|footprint| *footprint + position)
            .collect()
    }
    pub fn current_footprint(&self) -> Vec<IVec2> {
        self.footprint(self.position, self.rotation)
    }

    pub fn draw(&mut self, draw: &mut Draw, color: Color, cell_size: f32) {
        let (x_grid, y_grid) = (
            (self.position.x as f32 + 0.5) * cell_size,
            (self.position.y as f32 + 0.5) * cell_size,
        );
        let (width_grid, height_grid) = (self.size.x * cell_size, self.size.y * cell_size);

        let half_width = width_grid / 2.0;
        let half_height = height_grid / 2.0;

        let increment_size = std::f32::consts::PI * 2.0 / self.max_increments as f32;
        let rotation = self.rotation as f32 * increment_size;
        let transform = Affine2::from_translation(Vec2::new(x_grid, y_grid))
            * Affine2::from_angle(rotation)
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
                (
                    footprint.x as f32 * cell_size,
                    footprint.y as f32 * cell_size,
                ),
                (cell_size, cell_size),
            )
            .color(color);
        }
    }
}
