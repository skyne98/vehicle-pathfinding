use std::hash::Hash;
use std::time::Instant;

use agent::Agent;
use notan::draw::*;
use notan::math::{IVec2, Vec2};
use notan::prelude::*;
use pathfinding::directed::astar::astar;

pub mod agent;

#[derive(AppState)]
struct State {
    font: Font,
    grid: Grid,
    agent: Agent,
    mouse_pos: (f32, f32),
    path: Option<Vec<Cell>>,
}

struct Grid {
    cell_size: f32,
    size: (i32, i32),
    cells: Vec<bool>,
}
impl Grid {
    fn new(cell_size: f32, width: i32, height: i32) -> Self {
        let size = (width / cell_size as i32, height / cell_size as i32);
        let cells = vec![false; (size.0 * size.1) as usize];
        Self {
            cell_size,
            size,
            cells,
        }
    }

    fn index(&self, x: i32, y: i32) -> usize {
        (y * self.size.0 + x) as usize
    }

    fn xy(&self, index: usize) -> (i32, i32) {
        let x = index as i32 % self.size.0;
        let y = index as i32 / self.size.0;
        (x, y)
    }

    fn is_cell_blocked(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= self.size.0 || y < 0 || y >= self.size.1 {
            return true;
        }
        self.cells[self.index(x, y)]
    }

    fn toggle_cell(&mut self, x: i32, y: i32) {
        let index = self.index(x, y);
        self.cells[index] = !self.cells[index];
    }
}

#[derive(Clone, Debug)]
struct Cell {
    rotation: i16,
    position: IVec2,
}
impl Cell {
    pub fn new(rotation: i16, start: IVec2) -> Self {
        Self {
            rotation,
            position: start,
        }
    }
    pub fn neighbor(&self, rotation: i16, increment_size: f32) -> Self {
        // Convert rotation increments into an angle in radians.
        let angle = rotation as f32 * increment_size;

        // Generate the rotation vector from the angle. Assuming Vec2::from_angle constructs
        // a vector from an angle in radians and points in the direction of the angle.
        let rotation_vector = Vec2::from_angle(angle);

        // Since we want to move to a direct neighbor, ensure the vector is normalized
        // and then rounded to get a unit vector pointing to the closest grid direction.
        let direction_vector = Vec2::new(rotation_vector.x.round(), rotation_vector.y.round());

        // Calculate the new position by adding the direction vector to the current position.
        // This assumes `self.position` is represented in grid coordinates (e.g., IVec2).
        let new_position = self.position + direction_vector.as_ivec2();

        // Assert that the new position is different from the old position to ensure movement.
        assert_ne!(
            self.position, new_position,
            "The position should change; direction_vector: {:?}, old_position: {:?}, new_position: {:?}",
            direction_vector, self.position, new_position
        );

        // Return a new instance of the struct with the updated position.
        Self::new(rotation, new_position)
    }
    pub fn neighbors(&self, arc: u16, max_increments: u16) -> Vec<Self> {
        let arc = arc as i16;
        let mut neighbors = Vec::new();
        let increment_size = std::f32::consts::PI * 2.0 / max_increments as f32;
        for i in -arc..=arc {
            let new_rotation = self.rotation + i as i16;
            let new_rotation = if new_rotation < 0 {
                max_increments as i16 + new_rotation
            } else if new_rotation >= max_increments as i16 {
                new_rotation - max_increments as i16
            } else {
                new_rotation
            };
            let cell = self.neighbor(new_rotation, increment_size);
            neighbors.push(cell);
        }
        neighbors
    }
    pub fn opposite_rotation(&self, max_increments: u16) -> u16 {
        let current_rotation = self.rotation as i32;
        let max_rotation = max_increments as i32;
        let opposite_rotation = (current_rotation + max_rotation / 2) % max_rotation;
        opposite_rotation as u16
    }
    pub fn cost(&self) -> u32 {
        1
    }

    pub fn draw(&self, draw: &mut Draw, font: &Font, cell_size: f32, max_increments: u16) {
        let color = Color::BLUE;
        let half_cell = cell_size / 2.0;
        let start = Vec2::new(self.position.x as f32, self.position.y as f32);
        let end = start
            + Vec2::from_angle(
                self.rotation as f32 * 2.0 * std::f32::consts::PI / max_increments as f32,
            );
        draw_arrow(
            draw,
            start * cell_size + half_cell,
            end * cell_size + half_cell,
            color,
        );

        // write the data
        let text = format!("R: {}", self.rotation);
        let text_position = self.position.as_vec2();
        draw.text(&font, &text)
            .translate(text_position.x * cell_size, text_position.y * cell_size)
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
    }
}

#[notan_main]
fn main() -> Result<(), String> {
    let window_config = WindowConfig::new().set_vsync(true).set_size(1920, 1080);
    notan::init_with(setup)
        .add_config(DrawConfig)
        .add_config(window_config)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
    let font = gfx
        .create_font(include_bytes!("assets/quicksand.ttf"))
        .expect("Error loading font");
    let cell_size = 60.0;
    let grid = Grid::new(cell_size, 1920, 1080);
    State {
        font,
        grid,
        agent: Agent::new(IVec2::new(3, 3), Vec2::new(1.5, 0.75), 0, 16),
        mouse_pos: (0.0, 0.0),
        path: None,
    }
}

fn pathfind(state: &mut State, to: IVec2) {
    let start = Instant::now();
    let start_action = Cell::new(state.agent.rotation, state.agent.position);

    let result = astar(
        &start_action,
        |action| {
            let to_check = action
                .neighbors(1, 16)
                .into_iter()
                .map(|neigh| (neigh.clone(), neigh.cost()))
                .collect::<Vec<(Cell, u32)>>();
            to_check
                .into_iter()
                .filter(|(action, _)| {
                    let footprint = state.agent.footprint(action.position, action.rotation);
                    footprint
                        .iter()
                        .all(|cell| !state.grid.is_cell_blocked(cell.x as i32, cell.y as i32))
                })
                .collect::<Vec<(Cell, u32)>>()
        },
        |action| ((to - action.position).length_squared()) as u32,
        |action| {
            let (x, y) = (action.position.x as i32, action.position.y as i32);
            let (goal_x, goal_y) = (to.x as i32, to.y as i32);
            x == goal_x && y == goal_y
        },
    );

    if let Some((path, _)) = result {
        state.path = Some(path.iter().map(|a| a.clone()).collect());
    } else {
        state.path = None;
    }

    println!("Pathfinding took: {:?}", start.elapsed());
}

fn update(app: &mut App, state: &mut State) {
    let (x, y) = app.mouse.position();
    state.mouse_pos = (x, y);
    if app.mouse.was_pressed(MouseButton::Left) {
        let grid_x = (x / state.grid.cell_size) as i32;
        let grid_y = (y / state.grid.cell_size) as i32;
        state.grid.toggle_cell(grid_x, grid_y);
    }
    if app.mouse.was_pressed(MouseButton::Middle) {
        state.agent.position = IVec2::new(
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        );
    }
    if app.mouse.was_pressed(MouseButton::Right) {
        let to = (
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        );
        pathfind(state, IVec2::new(to.0, to.1));
    }
}

fn draw_selection(draw: &mut Draw, position: (i32, i32), size: f32, color: Color) {
    let (x, y) = position;
    let (x, y) = (x as f32, y as f32);
    let (x_grid, y_grid) = (x * size, y * size);
    let rect = |draw: &mut Draw, x, y, w, h| {
        draw.rect((x, y), (w, h)).color(color);
    };
    let line_width = size / 10.0;
    rect(draw, x_grid, y_grid, size, line_width);
    rect(draw, x_grid, y_grid, line_width, size);
    rect(draw, x_grid + size - line_width, y_grid, line_width, size);
    rect(draw, x_grid, y_grid + size - line_width, size, line_width);
}
fn draw_arrow(draw: &mut Draw, from: Vec2, to: Vec2, color: Color) {
    if from.is_finite() == false || to.is_finite() == false {
        return;
    }

    let dir = to - from;
    let length = dir.length();
    let dir = dir.normalize();
    let angle = dir.y.atan2(dir.x);
    let arrow_size = 10.0;
    let arrow_end = from + dir * length;
    let arrow_start = arrow_end
        - Vec2::new(arrow_size, 0.0).rotate(Vec2::from_angle(angle + std::f32::consts::PI / 6.0));
    let arrow_end2 = arrow_end
        - Vec2::new(arrow_size, 0.0).rotate(Vec2::from_angle(angle - std::f32::consts::PI / 6.0));

    // check for NaN and infinite values
    if arrow_start.is_finite() == false
        || arrow_end.is_finite() == false
        || arrow_end2.is_finite() == false
    {
        return;
    }

    draw.line((from.x, from.y), (arrow_end.x, arrow_end.y))
        .color(color);
    draw.line((arrow_end.x, arrow_end.y), (arrow_start.x, arrow_start.y))
        .color(color);
    draw.line((arrow_end.x, arrow_end.y), (arrow_end2.x, arrow_end2.y))
        .color(color);
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    // Draw grid lines
    for x in 0..state.grid.size.0 {
        draw.line(
            (x as f32 * state.grid.cell_size, 0.0),
            (x as f32 * state.grid.cell_size, 1080.0),
        )
        .color(Color::GRAY);
    }
    for y in 0..state.grid.size.1 {
        draw.line(
            (0.0, y as f32 * state.grid.cell_size),
            (1920.0, y as f32 * state.grid.cell_size),
        )
        .color(Color::GRAY);
    }

    // Draw the grid
    for (i, cell) in state.grid.cells.iter().enumerate() {
        if *cell {
            let (x, y) = state.grid.xy(i);
            draw.rect(
                (
                    x as f32 * state.grid.cell_size,
                    y as f32 * state.grid.cell_size,
                ),
                (state.grid.cell_size, state.grid.cell_size),
            )
            .color(Color::WHITE);
        }
    }

    // Draw the agent
    state
        .agent
        .draw(&mut draw, Color::RED, state.grid.cell_size);

    // Draw the path
    if let Some(path) = &state.path {
        for action in path {
            action.draw(&mut draw, &state.font, state.grid.cell_size, 16);
        }
    }

    // Draw the selection
    let (x, y) = state.mouse_pos;
    draw_selection(
        &mut draw,
        (
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        ),
        state.grid.cell_size,
        Color::GREEN,
    );

    gfx.render(&draw);
}
