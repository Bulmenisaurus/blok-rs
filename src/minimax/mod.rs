use crate::{board::BoardState, minimax::searcher::Searcher};

mod searcher;
mod transposition_table;

pub fn search(state: &BoardState, timeout_ms: usize) -> u32 {
    let mut searcher = Searcher::new();
    searcher.search_root(state, timeout_ms)
}

pub fn search_nodes(state: &BoardState, nodes: u32) -> u32 {
    let mut searcher = Searcher::new();
    searcher.search_root_nodes(state, nodes)
}
