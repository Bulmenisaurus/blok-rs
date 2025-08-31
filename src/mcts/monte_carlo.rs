use rand::prelude::IndexedRandom;
use rand::rng;

use crate::board::{BoardState, GameResult, Player};
use crate::mcts::MonteCarloNode;
use crate::movegen::generate_moves;
use crate::nn::Network;

pub struct MonteCarlo {
    ucb1_explore_param: f64,
    pub nodes: Vec<MonteCarloNode>,
    network: Network,
    debug: bool,
}

impl MonteCarlo {
    pub fn new(network: Network, debug: bool) -> Self {
        Self {
            ucb1_explore_param: 0.0,
            nodes: Vec::new(),
            network,
            debug,
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
            "hard" => 100_000,
            _ => 60_000,
        };

        for _ in 0..iterations {
            let tree_state: &mut BoardState = &mut state.clone();

            let node_idx = self.select(tree_state);
            let node = &self.nodes[node_idx];

            let winner = tree_state.game_result();

            if !node.is_leaf() && winner == GameResult::InProgress {
                let new_node_idx = self.expand(node_idx, tree_state);
                // the player to move on the expanded state, before the simulation (used to update the correct n_wins during backpropagation)
                let player = tree_state.player;
                let probability = self.simulate(tree_state);

                self.backpropagate(new_node_idx, probability, player);
            } else {
                let effective_probability = match winner {
                    GameResult::PlayerAWon => 1.0,
                    GameResult::PlayerBWon => -1.0,
                    GameResult::Draw => 0.0,
                    GameResult::InProgress => unreachable!(),
                };
                self.backpropagate(node_idx, effective_probability, tree_state.player);
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

                let child_ucb1 = child_node.get_ucb1();
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

    /// Phase 3, Simulation. Instead of a playout as we would do in the simplest MCTS, now we use a neural network to evaluate the position
    fn simulate(&self, current_state: &mut BoardState) -> f64 {
        //TODO: this evaluation is negated, I think because in the training data I mistakenly negated evals?
        let eval = -if current_state.player == Player::White {
            self.network.evaluate(
                &current_state.player_a_accumulator,
                &current_state.player_b_accumulator,
            )
        } else {
            self.network.evaluate(
                &current_state.player_b_accumulator,
                &current_state.player_a_accumulator,
            )
        };

        //TODO: this is like totally bsed no clue what the actual eval scale is...
        //TODO: in testing, both 1500 and 2500 worked well
        let scale = 1500.0;
        let squished = f64::tanh(eval as f64 / scale);

        if current_state.player == Player::White {
            squished
        } else {
            -squished
        }
    }

    /// Phase 4, Backpropagation: Update ancestor statistics
    fn backpropagate(&mut self, node_idx: usize, score: f64, player_to_move: Player) {
        let mut current_node: &mut MonteCarloNode = &mut self.nodes[node_idx];
        let mut player = player_to_move;
        loop {
            current_node.n_plays += 1;

            // need to inver it as the evaluation is from the perspective of the parent
            let player_to_win = player.other();

            let player_factor = if player_to_win == Player::White {
                1.0
            } else {
                -1.0
            };
            current_node.score += score * player_factor;

            let parent_node_idx = current_node.parent_idx;
            if parent_node_idx.is_none() {
                return;
            }

            current_node = &mut self.nodes[parent_node_idx.unwrap()];
            player = player.other();
        }
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> (usize, f64) {
        let root = &self.nodes[0];

        (root.n_plays, root.score)
    }
}
