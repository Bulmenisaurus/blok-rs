use std::collections::HashMap;

use crate::board::BoardState;

#[derive(Clone, Debug)]
pub struct MonteCarloNode {
    pub parent_idx: Option<usize>,

    pub n_plays: usize,
    pub n_wins: usize,

    pub own_idx: usize,
    pub children: HashMap<u32, Option<usize>>,
}

impl MonteCarloNode {
    pub fn new(idx: usize, parent_idx: Option<usize>, unexpanded_plays: Vec<u32>) -> Self {
        let mut children = HashMap::new();
        for play in unexpanded_plays {
            children.insert(play, None);
        }

        Self {
            parent_idx,
            n_plays: 0,
            n_wins: 0,
            own_idx: idx,
            children,
        }
    }

    pub fn child_node(&self, play: u32) -> usize {
        self.children
            .get(&play)
            .expect("Child node not found")
            .expect("Child not expanded")
    }

    pub fn expand(
        &mut self,
        play: u32,
        unexpanded_plays: Vec<u32>,
        new_idx: usize,
    ) -> Result<MonteCarloNode, &str> {
        if !self.children.contains_key(&play) {
            return Err("Play not found");
        }

        let child_node = MonteCarloNode::new(new_idx, Some(self.own_idx), unexpanded_plays);

        self.children.insert(play, Some(new_idx));

        Ok(child_node)
    }

    pub fn all_plays(&self) -> Vec<u32> {
        self.children.iter().map(|(&play, _)| play).collect()
    }

    pub fn unexpanded_plays(&self) -> Vec<u32> {
        self.children
            .iter()
            .filter_map(|(play, idx)| {
                let node = idx;
                if node.is_none() { Some(*play) } else { None }
            })
            .collect()
    }

    pub fn is_fully_expanded(&self) -> bool {
        self.children.iter().all(|(_, idx)| idx.is_some())
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn get_ucb1(&self, bias_param: f64, all_nodes: &[MonteCarloNode]) -> f64 {
        let parent = self.parent_idx.expect("UCB1 not defined for root node");
        let parent = &all_nodes[parent];

        self.n_wins as f64 / self.n_plays as f64
            + f64::sqrt(bias_param * f64::ln(parent.n_plays as f64) / self.n_plays as f64)
    }

    //TODO: could we just keep track of parent_n_plays?
    pub fn get_puct(
        &self,
        board: &BoardState,
        parent_node: &MonteCarloNode,
        bias_param: f64,
    ) -> f64 {
        let score = self.n_wins as f64 / self.n_plays as f64;

        let nn_scale = 1500.;
        let nn_probability = f64::tanh(board.sample_nn() as f64 / nn_scale);

        let parent_node = &parent_node;

        let parent_n_plays = parent_node.n_plays;
        let self_n_plays = self.n_plays;

        score
            + bias_param * nn_probability * f64::sqrt(parent_n_plays as f64)
                / (1. + self_n_plays as f64)
    }
}
