use std::collections::HashMap;

use crate::board::BoardState;

#[derive(Clone)]
struct ChildInfo {
    play: u32,
    node: Option<MonteCarloNode>,
}

#[derive(Clone)]
pub struct MonteCarloNode {
    play: Option<u32>,
    state: BoardState,
    n_plays: usize,
    n_wins: usize,
    // Hash of parent
    parent: Option<String>,
    children: HashMap<u32, ChildInfo>,
}

impl MonteCarloNode {
    pub fn new(
        parent: Option<String>,
        play: Option<u32>,
        state: BoardState,
        unexpanded_playes: Vec<u32>,
    ) -> Self {
        let mut children = HashMap::new();
        for play in unexpanded_playes {
            children.insert(play, ChildInfo { play, node: None });
        }

        Self {
            play,
            state,
            n_plays: 0,
            n_wins: 0,
            parent,
            children,
        }
    }

    pub fn child_node(&self, play: u32) -> Result<MonteCarloNode, &str> {
        let child_info = self.children.get(&play);

        if child_info.is_none() {
            return Err("Child node not found");
        }

        let child_info = child_info.unwrap();

        if child_info.node.is_none() {
            return Err("Child not expanded");
        }

        Ok(child_info.node.clone().unwrap())
    }

    pub fn expand(
        &mut self,
        play: u32,
        child_state: BoardState,
        unexpanded_plays: Vec<u32>,
    ) -> Result<MonteCarloNode, &str> {
        if !self.children.contains_key(&play) {
            return Err("Play not found");
        }

        let child_node = MonteCarloNode::new(
            Some(self.state.hash()),
            Some(play),
            child_state,
            unexpanded_plays,
        );

        self.children.insert(
            play,
            ChildInfo {
                play,
                node: Some(child_node.clone()),
            },
        );

        Ok(child_node)
    }
}
