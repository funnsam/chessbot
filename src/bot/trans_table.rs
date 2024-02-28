use std::collections::HashMap;

const T_TABLE_SIZE: usize = 64 * 1024 * 1024;
const T_TABLE_ELMS: usize = T_TABLE_SIZE / core::mem::size_of::<TransTableEntry>();

pub struct TransTableEntry {
    pub depth: usize,
    pub eval: i32,
}

pub struct TransTable(pub HashMap<u64, TransTableEntry>);

impl TransTable {
    pub fn new() -> Self {
        Self(HashMap::with_capacity(T_TABLE_ELMS))
    }

    pub fn insert(&mut self, k: u64, v: TransTableEntry) {
        if let Some(old) = self.0.get_mut(&k) {
            // if v.depth > old.depth {
                *old = v;
                return;
            // }
        }

        if self.0.len() == T_TABLE_ELMS {
            let a = *self.0.keys().next().unwrap();
            self.0.remove(&a);
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
