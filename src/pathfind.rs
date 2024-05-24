use binary_heap_plus::BinaryHeap;
use std::cmp::Ordering;
use std::collections::HashMap;
use typed_arena::Arena;

#[derive(Debug, Clone)]
pub struct AStarNode<T> {
    state: T,
    g_cost: u32,
    f_cost: u32,
}

impl<T> Ord for AStarNode<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost.cmp(&self.f_cost)
    }
}
impl<T> PartialOrd for AStarNode<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> PartialEq for AStarNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}
impl<T> Eq for AStarNode<T> {}

impl<T> AStarNode<T> {
    fn new(state: T, g_cost: u32, f_cost: u32) -> Self {
        AStarNode {
            state,
            g_cost,
            f_cost,
        }
    }
}

pub fn optimized_astar<T, F, H, G>(
    start: T,
    max_states: usize,
    neighbors_fn: F,
    heuristic_fn: H,
    goal_fn: G,
) -> Option<(Vec<T>, u32)>
where
    T: Eq + Clone + std::hash::Hash,
    F: Fn(&T) -> Vec<(T, u32)>,
    H: Fn(&T) -> u32,
    G: Fn(&T) -> bool,
{
    let arena = Arena::new();
    let mut open_set = BinaryHeap::with_capacity(max_states);
    let mut came_from: HashMap<T, T> = HashMap::with_capacity(max_states);
    let mut g_score: HashMap<T, u32> = HashMap::with_capacity(max_states);

    let start_node = arena.alloc(AStarNode::new(start.clone(), 0, heuristic_fn(&start)));

    open_set.push(start_node.clone());
    g_score.insert(start.clone(), 0);

    while let Some(current_node) = open_set.pop() {
        if goal_fn(&current_node.state) {
            let mut total_path = vec![current_node.state.clone()];
            let mut current = current_node.state.clone();
            while let Some(next) = came_from.get(&current) {
                total_path.push(next.clone());
                current = next.clone();
            }
            total_path.reverse();
            return Some((total_path, current_node.g_cost));
        }

        let current_state = current_node.state.clone();

        for (neighbor, move_cost) in neighbors_fn(&current_state) {
            let tentative_g_score = g_score[&current_state] + move_cost;
            if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                came_from.insert(neighbor.clone(), current_state.clone());
                g_score.insert(neighbor.clone(), tentative_g_score);

                let f_cost = tentative_g_score + heuristic_fn(&neighbor);
                let neighbor_node =
                    arena.alloc(AStarNode::new(neighbor.clone(), tentative_g_score, f_cost));

                open_set.push(neighbor_node.clone());
            }
        }
    }

    None
}
