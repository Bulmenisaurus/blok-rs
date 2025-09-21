use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy, Debug)]
pub struct TranspositionTableEntry {
    pub score: i32,
    pub depth: usize,
    pub flag: TTFlag,
    pub best_move: u32,
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
        let existing = self.entries.get(&hash);
        if let Some(existing) = existing {
            if existing.depth > entry.depth {
                return;
            }
        }
        self.entries.insert(hash, entry);
    }

    pub fn get(&self, hash: u64) -> Option<&TranspositionTableEntry> {
        self.entries.get(&hash)
    }
}
