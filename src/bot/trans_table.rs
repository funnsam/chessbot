use chess::CacheTable;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct TransTableEntry {
    pub depth: usize,
    pub eval: i32,
    pub age: usize,
}

pub struct TransTable(pub CacheTable<TransTableEntry>);

impl TransTable {
    pub fn new() -> Self {
        Self(CacheTable::new(super::config::T_TABLE_SIZE, TransTableEntry {
            depth: 0,
            eval: 0,
            age: 0
        }))
    }

    pub fn insert(&mut self, k: u64, v: TransTableEntry) {
        self.0.add(k, v);
    }

    #[inline(always)]
    pub fn get(&self, k: u64) -> Option<TransTableEntry> {
        self.0.get(k)
    }
}
