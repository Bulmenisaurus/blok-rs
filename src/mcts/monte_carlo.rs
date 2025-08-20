use std::collections::HashMap;

use crate::board::BoardState;
use crate::mcts::MonteCarloNode;

pub struct MonteCarlo {
    game: BoardState,
    ucb1_explore_param: f64,
    nodes: HashMap<String, MonteCarloNode>,
}

impl MonteCarlo {
    pub fn new(game: BoardState) -> Self {
        Self {
            game,
            //TODO: what actually was it
            ucb1_explore_param: 2.,
            nodes: HashMap::new(),
        }
    }
}
