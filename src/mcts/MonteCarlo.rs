use std::collections::HashMap;

use crate::board::BoardState;
use crate::mcts::MonteCarloNodeState;

pub struct MonteCarloState {
    game: BoardState,
    UCB1ExploreParam: f64,
    nodes: HashMap<String, MonteCarloNodeState>,
}

impl MonteCarloState {
    pub fn new(game: BoardState) -> Self {
        Self {
            game,
            //TODO: what actually was it
            UCB1ExploreParam: 2.,
            nodes: HashMap::new(),
        }
    }
}
