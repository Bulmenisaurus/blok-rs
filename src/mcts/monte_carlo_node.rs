use std::collections::HashMap;

use crate::board::BoardState;

#[derive(Clone, Copy)]
struct ChildInfo {
    play: u32,
    node: Option<usize>,
}

#[derive(Clone)]
pub struct MonteCarloNode {
    pub play: Option<u32>,
    pub parent_idx: Option<usize>,

    //? pub state: BoardState,
    pub n_plays: usize,
    pub n_wins: usize,

    pub own_idx: usize,
    children: HashMap<u32, ChildInfo>,
}

impl MonteCarloNode {
    pub fn new(
        idx: usize,
        parent_idx: Option<usize>,
        play: Option<u32>,
        //? state: BoardState,
        unexpanded_plays: Vec<u32>,
    ) -> Self {
        let mut children = HashMap::new();
        for play in unexpanded_plays {
            children.insert(play, ChildInfo { play, node: None });
        }

        Self {
            play,
            parent_idx,
            //? state,
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
            .node
            .expect("Child not expanded")
    }

    pub fn expand(
        &mut self,
        play: u32,
        //? child_state: BoardState,
        unexpanded_plays: Vec<u32>,
        new_idx: usize,
    ) -> Result<MonteCarloNode, &str> {
        if !self.children.contains_key(&play) {
            return Err("Play not found");
        }

        let child_node = MonteCarloNode::new(
            new_idx,
            Some(self.own_idx),
            Some(play),
            //? child_state,
            unexpanded_plays,
        );

        self.children.insert(
            play,
            ChildInfo {
                play,
                node: Some(new_idx),
            },
        );

        Ok(child_node)
    }

    pub fn all_plays(&self) -> Vec<u32> {
        self.children.iter().map(|k| k.1.play).collect()
    }

    pub fn unexpanded_plays(&self) -> Vec<u32> {
        self.children
            .iter()
            .filter_map(|k| {
                let node = k.1.node;
                if node.is_none() { Some(*k.0) } else { None }
            })
            .collect()
    }

    pub fn is_fully_expanded(&self) -> bool {
        self.children.iter().all(|k| k.1.node.is_some())
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn get_ucb1(&self, bias_param: f64, all_nodes: &Vec<MonteCarloNode>) -> f64 {
        let parent = self.parent_idx.expect("UCB1 not defined for root node");
        let parent = &all_nodes[parent];

        self.n_wins as f64 / self.n_plays as f64
            + f64::sqrt(bias_param * f64::ln(parent.n_plays as f64) / self.n_plays as f64)
    }
}
