use agent::Agent;
use notan::draw::*;
use notan::math::Vec2;
use notan::prelude::*;

pub mod agent;

#[derive(AppState)]
struct State {
    font: Font,
    grid: Grid,
    agent: Agent,
    mouse_pos: (f32, f32),
    path: Option<Vec<(i32, i32)>>,
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
        self.cells[self.index(x, y)]
    }

    fn toggle_cell(&mut self, x: i32, y: i32) {
        let index = self.index(x, y);
        self.cells[index] = !self.cells[index];
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
        agent: Agent { position: (0, 0) },
        mouse_pos: (0.0, 0.0),
        path: None,
    }
}

fn pathfind(state: &mut State, to: (i32, i32)) {
    let start_time = std::time::Instant::now();
    let start = state.agent.position;
    let grid = &state.grid;
    let result = pathfinding::directed::astar::astar(
        &start,
        |p| {
            let mut neighbors = Vec::new();
            let (x, y) = *p;
            for (dx, dy) in &[
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ] {
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0
                    && nx < grid.size.0
                    && ny >= 0
                    && ny < grid.size.1
                    && !grid.is_cell_blocked(nx, ny)
                {
                    // additional check for diagonal movement
                    let diagonal = dx.abs() + dy.abs() == 2;
                    if diagonal {
                        if grid.is_cell_blocked(nx, y) || grid.is_cell_blocked(x, ny) {
                            continue;
                        }
                        neighbors.push(((nx, ny), 14));
                    } else {
                        neighbors.push(((nx, ny), 10));
                    }
                }
            }
            neighbors
        },
        |p| {
            let (x, y) = *p;
            let from = Vec2::new(x as f32, y as f32);
            let to = Vec2::new(to.0 as f32, to.1 as f32);
            ((to - from).length() * 10.0) as i32
        },
        |p| *p == to,
    );
    state.path = result.map(|(path, _)| path);
    println!("Pathfinding took: {:?}", start_time.elapsed());
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
        state.agent.position = (
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        );
    }
    if app.mouse.was_pressed(MouseButton::Right) {
        let to = (
            (x / state.grid.cell_size) as i32,
            (y / state.grid.cell_size) as i32,
        );
        pathfind(state, to);
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
    let (x, y) = state.agent.position;
    draw.rect(
        (
            x as f32 * state.grid.cell_size,
            y as f32 * state.grid.cell_size,
        ),
        (state.grid.cell_size, state.grid.cell_size),
    )
    .color(Color::RED);

    // Draw the path
    if let Some(path) = &state.path {
        for cell in path {
            draw_selection(&mut draw, *cell, state.grid.cell_size, Color::BLUE);
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
