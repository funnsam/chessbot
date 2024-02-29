use std::collections::HashMap;

const T_TABLE_ELMS: usize = super::config::T_TABLE_SIZE / core::mem::size_of::<TransTableEntry>();

pub struct TransTableEntry {
    pub depth: usize,
    pub eval: i32,
    pub age: usize,
}

pub struct TransTable(pub HashMap<u64, TransTableEntry>);

impl TransTable {
    pub fn new() -> Self {
        Self(HashMap::with_capacity(T_TABLE_ELMS))
    }

    pub fn insert(&mut self, k: u64, v: TransTableEntry) {
        if let Some(old) = self.0.get_mut(&k) {
            *old = v;
            return;
        }

        if self.0.len() == T_TABLE_ELMS {
            let mut min_score = usize::MAX;
            let mut rm_k = 0;

            for (k, v) in self.0.iter() {
                let score = v.age << 3 + (super::config::MAX_SEARCH_DEPTH - v.depth);
                if score < min_score {
                    min_score = score;
                    rm_k = *k;
                }
            }

            self.0.remove(&rm_k);
        }

        self.0.insert(k, v);
    }

    #[inline(always)]
    pub fn get(&self, k: &u64) -> Option<&TransTableEntry> {
        self.0.get(k)
    }

    #[inline(always)]
    pub fn usage(&self) -> f32 {
        self.0.len() as f32 / T_TABLE_ELMS as f32
    }
}
