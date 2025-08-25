use rand::prelude::IndexedRandom;
use rand::rng;

use crate::board::{BoardState, GameResult, Player};
use crate::mcts::MonteCarloNode;
use crate::movegen::generate_moves;

pub struct MonteCarlo {
    ucb1_explore_param: f64,
    pub nodes: Vec<MonteCarloNode>,
}

impl Default for MonteCarlo {
    fn default() -> Self {
        Self::new()
    }
}

impl MonteCarlo {
    pub fn new() -> Self {
        Self {
            //TODO: what actually was it
            ucb1_explore_param: 2.,
            nodes: Vec::new(),
        }
    }

    // Clear the search to prepare for a new search
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn run_search(&mut self, state: &BoardState, difficulty: &str) {
        self.make_root_node(state);
        let iterations = match difficulty {
            "test" => 1_000,
            "eval" => 5_000,
            "easy" => 10_000,
            "medium" => 20_000,
            "hard" => 60_000,
            _ => 60_000,
        };

        println!("Running search with {} iterations", iterations);

        for _ in 0..iterations {
            let tree_state: &mut BoardState = &mut state.clone();

            let node_idx = self.select(tree_state);
            let node = &self.nodes[node_idx];

            let winner = tree_state.game_result();

            if !node.is_leaf() && winner == GameResult::InProgress {
                let new_node_idx = self.expand(node_idx, tree_state);
                // the player to move on the expanded state, before the simulation (used to update the correct n_wins during backpropagation)
                let player = tree_state.player;
                let winner = self.simulate(tree_state);

                self.backpropagate(new_node_idx, winner, player);
            } else {
                self.backpropagate(node_idx, winner, tree_state.player);
            }
        }
    }

    fn make_root_node(&mut self, state: &BoardState) {
        let unexpanded_moves = generate_moves(state);
        let new_idx = self.nodes.len();

        if new_idx != 0 {
            panic!("Root node not at 0");
        }
        let node = MonteCarloNode::new(new_idx, None, unexpanded_moves);
        self.nodes.push(node);
    }

    pub fn best_play(&mut self) -> Result<u32, &str> {
        let node = &self.nodes[0];

        if !node.is_fully_expanded() {
            println!("WARNING: Root node is not fully expanded");
        }

        let all_plays = node.all_plays();

        let best_play = all_plays
            .iter()
            .max_by_key(|a| &self.nodes[node.child_node(**a)].n_plays);

        match best_play {
            Some(play) => Ok(*play),
            None => Err("No best play found. Was best_play called on a leaf node?"),
        }
    }

    /// Phase 1, Selection: Select until not fully expanded OR leaf
    fn select(&mut self, state: &mut BoardState) -> usize {
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
            node = &self.nodes[child_idx];

            // update the board state to include this move
            state.do_move(best_play);
        }

        node.own_idx
    }

    /// Phase 2, Expansion: Expand a random unexpanded child node
    fn expand(&mut self, node_idx: usize, current_state: &mut BoardState) -> usize {
        let new_idx = self.nodes.len();

        let node: &mut MonteCarloNode = &mut self.nodes[node_idx];
        let plays = node.unexpanded_plays();

        // Pick a random move from the unexpanded plays

        let mut rng = rng();
        let &random_move = plays.choose(&mut rng).expect("No moves to choose from");

        // update the state
        current_state.do_move(random_move);

        let child_unexpanded_plays = generate_moves(current_state);

        let child_node = node
            .expand(random_move, child_unexpanded_plays, new_idx)
            .unwrap();

        self.nodes.push(child_node);

        new_idx
    }

    /// Phase 3, Simulation: Play game to terminal state, return winner
    fn simulate(&self, current_state: &mut BoardState) -> GameResult {
        let mut rng = rng();
        let mut state = current_state.clone();

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
    fn backpropagate(&mut self, node_idx: usize, winner: GameResult, player_to_move: Player) {
        let mut current_node: &mut MonteCarloNode = &mut self.nodes[node_idx];
        let mut player = player_to_move;
        loop {
            current_node.n_plays += 1;

            // need to inver it as the evaluation is from the perspective of the parent
            let player_to_win = player.other();

            match (player_to_win, winner) {
                (Player::White, GameResult::PlayerAWon)
                | (Player::Black, GameResult::PlayerBWon) => current_node.n_wins += 1,
                _ => {}
            }

            let parent_node_idx = current_node.parent_idx;
            if parent_node_idx.is_none() {
                return;
            }

            current_node = &mut self.nodes[parent_node_idx.unwrap()];
            player = player.other();
        }
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> (usize, usize) {
        let root = &self.nodes[0];

        (root.n_wins, root.n_plays)
    }
}
