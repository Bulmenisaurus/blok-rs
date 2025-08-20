use std::collections::HashMap;

use rand::rng;
use rand::seq::SliceRandom;

use crate::board::{BoardState, GameResult};
use crate::mcts::MonteCarloNode;
use crate::movegen::generate_moves;

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

    pub fn run_search(&mut self, state: BoardState) {
        let iterations = 1000;

        for i in 0..iterations {
            let node = self.select(state);
            let winner: GameResult = node.state.winner();
            if (node.is_leaf() == false && winner == GameResult::None) {
                node = self.expand(node);
                winner = self.simulate(node);
            }

            self.backpropagate(node, winner);
        }
    }

    fn make_node(&mut self, state: BoardState) {
        if !self.nodes.contains_key(&state.hash()) {
            let unexpanded_moves = generate_moves(&state);
            let node = MonteCarloNode::new(None, None, state, unexpanded_moves);
            self.nodes.insert(state.hash(), node);
        }
    }

    pub fn best_play(&mut self, state: BoardState) -> Result<u32, &str> {
        self.make_node(state);
        if !self.nodes.get(&state.hash()).unwrap().is_fully_expanded() {
            return Err("Node is not fully expanded");
        }

        let node = self.nodes.get(&state.hash()).unwrap();
        let all_plays = node.all_plays();
        let mut best_play: Option<u32> = None;
        let mut max_plays: usize = 0;

        for play in all_plays {
            let child_node = node.child_node(play);
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
    fn select<'a>(&'a mut self, state: &BoardState) -> &'a mut MonteCarloNode {
        // Get the node for the given state, panic if not found
        let mut node = self
            .nodes
            .get_mut(&state.hash())
            .expect("Node not found in select");

        while node.is_fully_expanded() && !node.is_leaf() {
            let plays = node.all_plays();
            let mut best_play: Option<u32> = None;
            let mut best_ucb1 = f64::NEG_INFINITY;

            for &play in &plays {
                let child_node = node.child_node(play);
                let child_ucb1 = child_node.get_ucb1(self.ucb1_explore_param);
                if child_ucb1 > best_ucb1 || best_play.is_none() {
                    best_play = Some(play);
                    best_ucb1 = child_ucb1;
                }
            }

            let best_play =
                best_play.expect("No best play found. Was select called on a leaf node?");
            // Move to the child node for the best play
            // We need to get a mutable reference to the child node.
            // This requires some care to avoid double mutable borrows.
            // We'll use the hash of the child state to get the child node from self.nodes.
            let child_hash = node.child_node(best_play).state.hash();
            node = self
                .nodes
                .get_mut(&child_hash)
                .expect("Child node not found in select");
        }
        node
    }

    /// Phase 2, Expansion: Expand a random unexpanded child node
    fn expand(&mut self, node: &mut MonteCarloNode) -> &mut MonteCarloNode {
        // Get the list of unexpanded plays (moves)
        let plays = node.unexpanded_plays();
        assert!(!plays.is_empty(), "No unexpanded plays to expand");

        // Pick a random move from the unexpanded plays

        let mut rng = rng();
        let &random_move = plays.choose(&mut rng).expect("No moves to choose from");

        // Clone the state and apply the move
        let mut child_state = node.state.clone();
        child_state.do_move(random_move);

        // Get all legal moves from the new state
        let child_unexpanded_plays = generate_moves(&child_state);

        // Expand the node with the new move, state, and unexpanded plays
        let child_node = node.expand(random_move, child_state.clone(), child_unexpanded_plays);

        // Insert the new child node into the nodes map
        self.nodes.insert(child_state.hash(), child_node);

        // Return a mutable reference to the newly inserted child node
        self.nodes
            .get_mut(&child_state.hash())
            .expect("Child node not found after insertion")
    }

    /// Phase 4, Backpropagation: Update ancestor statistics
    fn backpropagate(&mut self, mut node_hash: u64, winner: GameOutcome) {
        // Traverse up the tree, updating statistics
        while let Some(node) = self.nodes.get_mut(&node_hash) {
            node.n_plays += 1;
            // Parent's choice
            // Assuming node.state has a field 'to_move' or similar
            // and that other_player and winner are comparable
            if other_player(node.state.to_move) == winner {
                node.n_wins += 1;
            }
            // Move to parent
            if let Some(parent_hash) = node.parent {
                node_hash = parent_hash;
            } else {
                break;
            }
        }
    }

    fn get_stats(&self, state: BoardState) -> (usize, usize) {
        let node = self.nodes.get(&state.hash()).unwrap();
        (node.n_plays, node.n_wins)
    }
}
