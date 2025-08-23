use std::collections::HashMap;

use rand::prelude::IndexedRandom;
use rand::rng;
use rand::seq::SliceRandom;

use crate::board::{BoardState, GameResult, Player};
use crate::mcts::MonteCarloNode;
use crate::movegen::generate_moves;

pub struct MonteCarlo {
    // game: BoardState,
    ucb1_explore_param: f64,
    nodes: Vec<MonteCarloNode>,
}

impl MonteCarlo {
    pub fn new() -> Self {
        Self {
            // game,
            //TODO: what actually was it
            ucb1_explore_param: 2.,
            nodes: Vec::new(),
        }
    }

    // Clear the search to prepare for a new search
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn run_search(&mut self, state: BoardState) {
        self.make_root_node(state);
        let iterations = 15_000;

        for _ in 0..iterations {
            let node_idx = self.select();
            let node = &self.nodes[node_idx];
            let winner: GameResult = node.state.game_result();

            if node.is_leaf() == false && winner == GameResult::InProgress {
                let new_node_idx = self.expand(node_idx);
                let winner = self.simulate(new_node_idx);

                self.backpropagate(new_node_idx, winner);
            } else {
                self.backpropagate(node_idx, winner);
            }
        }
    }

    fn make_root_node(&mut self, state: BoardState) {
        let unexpanded_moves = generate_moves(&state);
        let new_idx = self.nodes.len();

        if new_idx != 0 {
            panic!("Root node not at 0");
        }
        let node = MonteCarloNode::new(new_idx, None, None, state, unexpanded_moves);
        self.nodes.push(node);
    }

    pub fn best_play(&mut self) -> Result<u32, &str> {
        let node = &self.nodes[0];

        if !node.is_fully_expanded() {
            println!("WARNING: Root node is not fully expanded");
        }

        let all_plays = node.all_plays();
        let mut best_play: Option<u32> = None;
        let mut max_plays: usize = 0;

        for play in all_plays {
            let child_node = &self.nodes[node.child_node(play)];
            // skip unexpanded nodes (probably would've been caught by the condition above)
            if child_node.n_plays == 0 {
                continue;
            }
            if child_node.n_plays > max_plays || best_play.is_none() {
                best_play = Some(play);
                max_plays = child_node.n_plays;
            }
        }

        match best_play {
            Some(play) => Ok(play),
            None => Err("No best play found. Was best_play called on a leaf node?"),
        }
    }

    /// Phase 1, Selection: Select until not fully expanded OR leaf
    fn select(&mut self) -> usize {
        let mut node = &self.nodes[0];

        while node.is_fully_expanded() && !node.is_leaf() {
            let plays = node.all_plays();
            let mut best_play: Option<u32> = None;
            let mut best_ucb1 = f64::NEG_INFINITY;

            for &play in &plays {
                let child_node = &self.nodes[node.child_node(play)];

                let child_ucb1 = child_node.get_ucb1(self.ucb1_explore_param, &self.nodes);
                if child_ucb1 > best_ucb1 || best_play.is_none() {
                    best_play = Some(play);
                    best_ucb1 = child_ucb1;
                }
            }

            let best_play =
                best_play.expect("No best play found. Was select called on a leaf node?");

            let child_idx = node.child_node(best_play);
            node = &self.nodes[child_idx]
        }

        node.own_idx
    }

    /// Phase 2, Expansion: Expand a random unexpanded child node
    fn expand(&mut self, node_idx: usize) -> usize {
        let new_idx = self.nodes.len();

        let node: &mut MonteCarloNode = &mut self.nodes[node_idx];
        let plays = node.unexpanded_plays();

        // Pick a random move from the unexpanded plays

        let mut rng = rng();
        let &random_move = plays.choose(&mut rng).expect("No moves to choose from");

        let mut child_state = node.state.clone();
        child_state.do_move(random_move);
        let child_unexpanded_plays = generate_moves(&child_state);

        let child_node = node
            .expand(random_move, child_state, child_unexpanded_plays, new_idx)
            .unwrap();

        self.nodes.push(child_node);
        return new_idx;
    }

    /// Phase 3, Simulation: Play game to terminal state, return winner
    fn simulate(&self, node_idx: usize) -> GameResult {
        let node = &self.nodes[node_idx];
        let mut rng = rng();
        let mut state = node.state.clone();

        loop {
            let winner = state.game_result();
            if winner != GameResult::InProgress {
                return winner;
            }
            let plays = generate_moves(&state);
            let play = plays.choose(&mut rng).unwrap();
            state.do_move(*play);
        }
    }

    /// Phase 4, Backpropagation: Update ancestor statistics
    fn backpropagate(&mut self, node_idx: usize, winner: GameResult) {
        let mut current_node: &mut MonteCarloNode = &mut self.nodes[node_idx];
        loop {
            current_node.n_plays += 1;

            let player_to_win = current_node.state.player.other();

            match (player_to_win, winner) {
                (Player::White, GameResult::PlayerAWon)
                | (Player::Black, GameResult::PlayerBWon) => current_node.n_wins += 1,
                _ => {}
            }

            let parent_node_idx = current_node.parent_idx;
            if parent_node_idx.is_none() {
                return;
            }

            current_node = &mut self.nodes[parent_node_idx.unwrap()]
        }
    }

    pub fn get_stats(&self) -> (usize, usize) {
        let root = &self.nodes[0];

        (root.n_wins, root.n_plays)
    }
}
