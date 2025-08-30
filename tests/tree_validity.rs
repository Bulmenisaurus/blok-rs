use blok_rs::board::{BoardState, StartPosition};
use blok_rs::mcts::MonteCarloNode;
use blok_rs::mcts::monte_carlo::MonteCarlo;
use blok_rs::nn::NNUE;

#[test]
pub fn is_valid_tree() {
    let game = BoardState::new(StartPosition::Corner, NNUE);
    let mut mcts = MonteCarlo::new(NNUE, false);
    mcts.run_search(&game, "test");

    // assert!(is_valid_node(&mcts.nodes, &mcts.nodes[2]));
    for node in &mcts.nodes {
        assert!(is_valid_node(&mcts.nodes, node));
    }
}

// check that the number of visits to the node is equal to the sum of the visits to the children
pub fn is_valid_node(all_nodes: &Vec<MonteCarloNode>, node: &MonteCarloNode) -> bool {
    let visits = node.n_plays;
    let mut children_visits = 0;
    for child_idx in node.children.values() {
        if let Some(child_idx) = child_idx {
            children_visits += all_nodes[*child_idx].n_plays;
        }
    }

    // All nodes except the root node should have one more visit than the sum of the visits to the children (because of the visits to any child was it iself)
    if node.own_idx == 0 {
        visits == children_visits
    } else {
        visits == children_visits + 1
    }
}
