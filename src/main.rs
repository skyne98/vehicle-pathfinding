use std::hash::Hash;
use std::time::Instant;

use agent::Agent;
use notan::draw::*;
use notan::math::Vec2;
use notan::prelude::*;
use pathfinding::directed::astar::astar;

pub mod agent;

#[derive(AppState)]
struct State {
    font: Font,
    grid: Grid,
    agent: Agent,
    mouse_pos: (f32, f32),
    path: Option<Vec<Action>>,
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
struct Action {
    movement: f32,
    rotation: f32,

    start: Vec2,
    start_rotation: f32,
}
impl Action {
    pub fn new(movement: f32, rotation: f32, start: Vec2, start_rotation: f32) -> Self {
        Self {
            movement,
            rotation,
            start,
            start_rotation,
        }
    }
    pub fn end(&self) -> Vec2 {
        self.start + Vec2::from_angle(self.start_rotation + self.rotation) * self.movement
    }
    pub fn end_rotation(&self) -> f32 {
        self.start_rotation + self.rotation
    }
    pub fn neighbors(
        &self,
        move_increments: f32,
        move_limit: f32,
        turn_increments: f32,
        turn_limit: f32,
    ) -> Vec<Self> {
        let mut neighbors = Vec::new();
        let mut turn = 0.0;
        let mut movement;
        while turn <= turn_limit {
            movement = 0.0;
            while movement <= move_limit {
                // positive
                let turn_abs = turn.abs();
                let turn_fraction = turn_abs / turn_limit;
                let turn_fraction = turn_fraction * turn_fraction * turn_fraction;
                let actual_movement = movement * (1.0 - turn_fraction);
                neighbors.push(Self::new(
                    actual_movement,
                    turn,
                    self.end(),
                    self.end_rotation(),
                ));
                neighbors.push(Self::new(
                    actual_movement,
                    -turn,
                    self.end(),
                    self.end_rotation(),
                ));
                neighbors.push(Self::new(
                    -actual_movement,
                    turn,
                    self.end(),
                    self.end_rotation(),
                ));
                neighbors.push(Self::new(
                    -actual_movement,
                    -turn,
                    self.end(),
                    self.end_rotation(),
                ));

                movement += move_increments;
            }
            turn += turn_increments;
        }

        neighbors
    }
    pub fn cost(&self) -> u32 {
        1
    }

    pub fn draw(&self, draw: &mut Draw, font: &Font, cell_size: f32) {
        let color = if self.movement.signum() > 0.0 {
            Color::BLUE
        } else {
            Color::RED
        };
        draw_arrow(draw, self.start * cell_size, self.end() * cell_size, color);

        // write the data
        let text = format!("M: {:.2} R: {:.2}", self.movement, self.rotation);
        let text_position = self.start + (self.end() - self.start) / 2.0 + Vec2::new(0.0, 0.5);
        draw.text(&font, &text)
            .translate(text_position.x * cell_size, text_position.y * cell_size)
            .size(15.0)
            .color(Color::WHITE);
    }
}
impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        let start_x = self.start.x as i32 == other.start.x as i32;
        let start_y = self.start.y as i32 == other.start.y as i32;
        let end_x = self.end().x as i32 == other.end().x as i32;
        let end_y = self.end().y as i32 == other.end().y as i32;
        start_x && start_y && end_x && end_y
    }
}
impl Eq for Action {}
impl Hash for Action {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.start.x) as u32, (self.start.y) as u32).hash(state);
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
        agent: Agent::new(Vec2::new(3.0, 3.0), Vec2::new(1.5, 0.75)),
        mouse_pos: (0.0, 0.0),
        path: None,
    }
}

fn pathfind(state: &mut State, to: Vec2) {
    let start = Instant::now();
    let start_action = Action::new(0.0, 0.0, state.agent.position, state.agent.rotation);

    let result = astar(
        &start_action,
        |action| {
            let to_check = action
                .neighbors(
                    0.1,
                    1.0,
                    std::f32::consts::PI / 128.0,
                    std::f32::consts::PI / 4.0,
                )
                .into_iter()
                .map(|neigh| (neigh.clone(), neigh.cost()))
                .collect::<Vec<(Action, u32)>>();
            to_check
                .into_iter()
                .filter(|(action, _)| {
                    // out of bounds
                    let (x, y) = (action.end().x as i32, action.end().y as i32);
                    if x < 0 || x >= state.grid.size.0 || y < 0 || y >= state.grid.size.1 {
                        println!("Out of bounds: {:?}", action.end());
                        return false;
                    }
                    // blocked
                    state.grid.is_cell_blocked(x, y) == false
                })
                .filter(|(action, _)| {
                    let footprint = state.agent.footprint(action.end(), action.end_rotation());
                    footprint
                        .iter()
                        .all(|cell| !state.grid.is_cell_blocked(cell.x as i32, cell.y as i32))
                })
                .collect::<Vec<(Action, u32)>>()
        },
        |action| ((to - action.end()).length() * 10.0) as u32,
        |action| {
            let (x, y) = (action.end().x as i32, action.end().y as i32);
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
        state.agent.position = Vec2::new(x / state.grid.cell_size, y / state.grid.cell_size);
    }
    if app.mouse.was_pressed(MouseButton::Right) {
        let to = (
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        );
        pathfind(state, Vec2::new(to.0 as f32, to.1 as f32));
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
            action.draw(&mut draw, &state.font, state.grid.cell_size);
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
