use std::collections::HashMap;

#[derive(Clone)]
pub struct TranspositionTableEntry {
    pub score: i32,
    pub depth: usize,
}

#[derive(Clone)]
pub struct TranspositionTable {
    entries: HashMap<u64, TranspositionTableEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hash: u64, entry: TranspositionTableEntry) {
        self.entries.insert(hash, entry);
    }

    pub fn get(&self, hash: u64) -> Option<&TranspositionTableEntry> {
        self.entries.get(&hash)
    }
}
