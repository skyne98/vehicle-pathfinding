use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use agent::Agent;
use bitarray::BitArray;
use geo::{Simplify, SimplifyIdx};
use notan::app::crevice::std140::WriteStd140;
use notan::draw::*;
use notan::math::{IVec2, Vec2};
use notan::prelude::*;
use pathfinding::directed::astar::astar;

pub mod agent;
pub mod bitarray;
pub mod cell;
pub mod pathfind;

use cell::Cell;

use mimalloc::MiMalloc;

use crate::pathfind::optimized_astar;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const ARC: u16 = 1;
const MAX_INCREMENTS: u16 = 32;
const CELL_SIZE: f32 = 16.0;
const SCREEN_SIZE: (u32, u32) = (1600, 800);
const CELL_COUNT: (i32, i32) = (
    SCREEN_SIZE.0 as i32 / CELL_SIZE as i32,
    SCREEN_SIZE.1 as i32 / CELL_SIZE as i32,
);
const PATHFIND_STATE_SIZE: usize =
    CELL_COUNT.0 as usize * CELL_COUNT.1 as usize * MAX_INCREMENTS as usize;

#[derive(AppState)]
pub struct State {
    font: Option<Font>,
    grid: Grid,
    agent: Agent,
    mouse_pos: (f32, f32),
    path: Option<Vec<Cell>>,
    neighbor_cache: cell::NeighborCacheRef,
}

struct Grid {
    cell_size: f32,
    size: (i32, i32),
    cells: BitArray,
}
impl Grid {
    fn new(cell_size: f32, width: i32, height: i32) -> Self {
        let size = (width / cell_size as i32, height / cell_size as i32);
        println!("Grid size: {:?}", size);
        let cells = BitArray::new((size.0 * size.1) as usize);
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
        self.cells.get_bool(self.index(x, y))
    }

    fn toggle_cell(&mut self, x: i32, y: i32) {
        let index = self.index(x, y);
        let existing = self.cells.get_bool(index);
        self.cells.set_bool(index, !existing);
    }
}

#[notan_main]
fn main() -> Result<(), String> {
    let window_config = WindowConfig::new()
        .set_vsync(true)
        .set_size(SCREEN_SIZE.0, SCREEN_SIZE.1);
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
    let cell_size = CELL_SIZE;
    let grid = Grid::new(cell_size, SCREEN_SIZE.0 as i32, SCREEN_SIZE.1 as i32);
    State {
        font: Some(font),
        grid,
        agent: Agent::new(IVec2::new(3, 3), Vec2::new(2.35, 1.75), 0, MAX_INCREMENTS),
        mouse_pos: (0.0, 0.0),
        path: None,
        neighbor_cache: Rc::new(RefCell::new(cell::NeighborCache::new_precomputed(
            MAX_INCREMENTS,
            ARC,
        ))),
    }
}

fn pathfind(state: &mut State, to: IVec2, arc: u16, max_increment: u16) {
    let start = Instant::now();
    let start_action = Cell::new(state.agent.rotation, state.agent.position);
    let neighbors_cache = state.neighbor_cache.clone();

    // let result = astar(
    //     &start_action,
    //     |action| {
    //         let mut result = Vec::with_capacity(128);

    //         for neigh in action.neighbors(&neighbors_cache, arc, max_increment) {
    //             if !state
    //                 .grid
    //                 .is_cell_blocked(neigh.position.x as i32, neigh.position.y as i32)
    //             {
    //                 let cost = neigh.cost(Some(action.clone()), arc, max_increment);
    //                 let rotation_footprint = state.agent.rotation_footprint(neigh.rotation);
    //                 if rotation_footprint.iter().all(|cell| {
    //                     !state.grid.is_cell_blocked(
    //                         cell.x as i32 + neigh.position.x,
    //                         cell.y as i32 + neigh.position.y,
    //                     )
    //                 }) {
    //                     result.push((neigh, cost));
    //                 }
    //             }
    //         }

    //         result
    //     },
    //     |action| action.heuristic(to, max_increment),
    //     |action| {
    //         let (x, y) = (action.position.x as i32, action.position.y as i32);
    //         let (goal_x, goal_y) = (to.x as i32, to.y as i32);
    //         x == goal_x && y == goal_y
    //     },
    // );
    let result = optimized_astar(
        start_action,
        PATHFIND_STATE_SIZE,
        |action| {
            let mut result = Vec::with_capacity(128);

            for neigh in action.neighbors(&neighbors_cache, arc, max_increment) {
                if !state
                    .grid
                    .is_cell_blocked(neigh.position.x as i32, neigh.position.y as i32)
                {
                    let cost = neigh.cost(Some(action.clone()), arc, max_increment);
                    let rotation_footprint = state.agent.rotation_footprint(neigh.rotation);
                    if rotation_footprint.iter().all(|cell| {
                        !state.grid.is_cell_blocked(
                            cell.x as i32 + neigh.position.x,
                            cell.y as i32 + neigh.position.y,
                        )
                    }) {
                        result.push((neigh, cost));
                    }
                }
            }

            result
        },
        |action| action.heuristic(to, MAX_INCREMENTS),
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
        pathfind(state, IVec2::new(to.0, to.1), ARC, MAX_INCREMENTS);
    }
    if app.keyboard.is_down(KeyCode::Space) {
        state.agent.rotation = (state.agent.rotation + 1) % MAX_INCREMENTS as i16;
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
fn draw_path_spline(draw: &mut Draw, path: &[Cell], color: Color, cell_size: f32) {
    use geo::{Coord, LineString};
    use splines::{Interpolation, Key, Spline};

    let line_string = LineString::new(
        path.iter()
            .map(|cell| Coord {
                x: cell.position.x as f64,
                y: cell.position.y as f64,
            })
            .collect(),
    );
    let simplified_path = line_string.simplify_idx(&0.5);
    // add back direction nodes, where reverse
    // switches to the opposite direction and back
    let mut reverse_keys = Vec::new();
    let simplified_path = path
        .iter()
        .enumerate()
        .filter(|(i, cell)| {
            let (i, cell) = (*i, *cell);
            if i == 0 || i == path.len() - 1 {
                return true;
            }

            let next = &path[i + 1];
            let reverse = next.is_reverse_to(cell, MAX_INCREMENTS as i16);
            if reverse {
                reverse_keys.push(i);
                return true;
            }

            simplified_path.contains(&i)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    let mut keys = Vec::with_capacity(path.len());
    for (i, cell_i) in simplified_path.iter().enumerate() {
        let cell = &path[*cell_i];
        let x = (cell.position.x as f32 + 0.5) * cell_size;
        let y = (cell.position.y as f32 + 0.5) * cell_size;
        let xy = Vec2::new(x, y);

        let angle = (cell.rotation as f32 / MAX_INCREMENTS as f32) * std::f32::consts::PI * 2.0;
        let angle_vector = Vec2::from_angle(angle);
        let tangent = xy + angle_vector * cell_size;
        let reverse = reverse_keys.contains(cell_i);
        let interpolation = if reverse {
            Interpolation::Linear
        } else {
            Interpolation::Bezier(tangent)
        };
        keys.push(Key::new(i as f32, xy, interpolation));
    }

    let spline = Spline::from_vec(keys);
    // now sample and draw the spline at a higher resolution
    let mut last = None;
    for i in 0..path.len() * 10 {
        let t = i as f32 / 10.0;
        let point = spline
            .clamped_sample(t)
            .expect(format!("Failed to sample x at {} (len: {})", t, spline.len()).as_str());
        if let Some((last_x, last_y)) = last {
            draw.line((last_x, last_y), (point.x, point.y)).color(color);
        }
        last = Some((point.x, point.y));
    }
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    // Draw the footprint
    state
        .agent
        .draw_current_footprint(&mut draw, Color::YELLOW, state.grid.cell_size);

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
    for y in 0..state.grid.size.1 {
        for x in 0..state.grid.size.0 {
            if state.grid.is_cell_blocked(x, y) {
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
    }

    // Draw the agent
    state
        .agent
        .draw(&mut draw, Color::RED, state.grid.cell_size);

    // Draw the path
    if let Some(path) = &state.path {
        let mut last = None;
        for action in path {
            action.draw(
                last,
                &mut draw,
                &state.font.unwrap(),
                state.grid.cell_size,
                MAX_INCREMENTS,
            );
            last = Some(action);
        }
    }
    // Draw the path as a spline
    if let Some(path) = &state.path {
        draw_path_spline(&mut draw, path, Color::GREEN, state.grid.cell_size);
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

// ===== TESTS =====
#[cfg(test)]
mod tests {
    use std::path;

    use super::*;

    fn setup_state(max_increment: u16, arc: u16) -> State {
        let cell_size = CELL_SIZE;
        let grid = Grid::new(cell_size, SCREEN_SIZE.0 as i32, SCREEN_SIZE.1 as i32);
        let agent = Agent::new(IVec2::new(0, 0), Vec2::new(0.01, 0.01), 0, max_increment);
        State {
            font: None,
            grid,
            agent,
            mouse_pos: (0.0, 0.0),
            path: None,
            neighbor_cache: Rc::new(RefCell::new(cell::NeighborCache::new_precomputed(
                max_increment,
                arc,
            ))),
        }
    }
    fn default_state() -> State {
        setup_state(8, 1)
    }

    #[test]
    fn test_pathfind_straight() {
        let mut state = default_state();
        pathfind(&mut state, IVec2::new(5, 0), 1, 8);
        assert!(state.path.is_some());
        for i in 0..5 {
            let action = state.path.as_ref().unwrap().get(i).unwrap();
            assert_eq!(action.position, IVec2::new(i as i32, 0));
        }
    }
    #[test]
    fn test_pathfind_diagonal() {
        let mut state = default_state();
        state.agent.rotation = 1;
        pathfind(&mut state, IVec2::new(5, 5), 1, 8);
        assert!(state.path.is_some());
        for i in 0..5 {
            let action = state.path.as_ref().unwrap().get(i).unwrap();
            assert_eq!(action.position, IVec2::new(i as i32, i as i32));
        }
    }
    #[test]
    fn test_pathfind_blocked() {
        let mut state = default_state();
        state.grid.toggle_cell(1, 0);
        state.grid.toggle_cell(1, 1);
        state.grid.toggle_cell(0, 1);
        pathfind(&mut state, IVec2::new(5, 5), 1, 8);
        assert!(state.path.is_none());
    }
    #[test]
    fn test_pathfind_turn() {
        let mut state = default_state();
        pathfind(&mut state, IVec2::new(5, 5), 1, 8);
        assert!(state.path.is_some());
        let path = state.path.as_ref().unwrap();

        let action = path.get(0).unwrap();
        assert_eq!(action.position, IVec2::new(0, 0));
        assert_eq!(action.rotation, 0);
        let action = path.get(1).unwrap();
        assert_eq!(action.position, IVec2::new(1, 1));
        assert_eq!(action.rotation, 1);
        let action = path.get(2).unwrap();
        assert_eq!(action.position, IVec2::new(2, 2));
        assert_eq!(action.rotation, 1);
        let action = path.get(3).unwrap();
        assert_eq!(action.position, IVec2::new(3, 3));
        assert_eq!(action.rotation, 1);
        let action = path.get(4).unwrap();
        assert_eq!(action.position, IVec2::new(4, 4));
        assert_eq!(action.rotation, 1);
    }
    #[test]
    fn test_pathfind_reverse_better_arc() {
        let mut state = default_state();
        state.agent.position = IVec2::new(5, 5);
        pathfind(&mut state, IVec2::new(5, 6), 1, 8);
        assert!(state.path.is_some());
        let path = state.path.as_ref().unwrap();

        let action = path.get(0).unwrap();
        assert_eq!(action.position, IVec2::new(5, 5));
        assert_eq!(action.rotation, 0);
        let action = path.get(1).unwrap();
        assert_eq!(action.position, IVec2::new(5, 6));
        assert_eq!(action.rotation, 6);
    }
}
