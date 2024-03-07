use core::mem::*;
use std::sync::atomic::*;
use super::config::T_TABLE_SIZE;

const MASK: usize = T_TABLE_SIZE - 1;

pub struct TransTableEntry {
    pub depth: usize,
    pub eval: i32,
    pub age: usize,
}

impl TransTableEntry {
    #[inline(always)]
    fn to_u8s<'a>(&'a self) -> &'a [u8] {
        unsafe { transmute::<_, &'a [u8; size_of::<Self>()]>(self) }
    }
}

pub struct HashTableEntry {
    pub hash: AtomicU64,
    pub age_depth: AtomicU64,
    pub eval_checksum: AtomicU64,
}

impl Clone for HashTableEntry {
    fn clone(&self) -> Self {
        Self {
            hash: AtomicU64::new(self.hash.load(Ordering::Relaxed)),
            age_depth: AtomicU64::new(self.age_depth.load(Ordering::Relaxed)),
            eval_checksum: AtomicU64::new(self.eval_checksum.load(Ordering::Relaxed)),
        }
    }
}

pub struct TransTable {
    inner: Box<[HashTableEntry; T_TABLE_SIZE]>,
}

impl TransTable {
    pub fn new() -> Self {
        let inner = Box::<[MaybeUninit<HashTableEntry>; T_TABLE_SIZE]>::new_uninit();
        let mut inner = unsafe { inner.assume_init() };

        for i in inner.iter_mut() {
            *i = MaybeUninit::new(HashTableEntry {
                hash: AtomicU64::new(0),
                age_depth: AtomicU64::new(0),
                eval_checksum: AtomicU64::new(0),
            });
        }

        let inner = unsafe { transmute(inner) };

        Self {
            inner,
        }
    }

    pub fn insert(&self, k: u64, v: TransTableEntry) {
        let checksum = murmur3::murmur3_32(&mut v.to_u8s(), 0).unwrap();

        let age_depth = ((v.age as u64) << 32) | ((v.depth as u64) & 0xffff_ffff);
        let eval_checksum = ((v.eval as u64) << 32) | (checksum as u64);

        let idx = k as usize & MASK;
        self.inner[idx].hash.store(k ^ age_depth, Ordering::Relaxed);
        self.inner[idx].age_depth.store(age_depth, Ordering::Relaxed);
        self.inner[idx].eval_checksum.store(eval_checksum, Ordering::Relaxed);
    }

    pub fn get(&self, k: u64) -> Option<TransTableEntry> {
        let idx = k as usize & MASK;
        let hash = self.inner[idx].hash.load(Ordering::Relaxed);
        let age_depth = self.inner[idx].age_depth.load(Ordering::Relaxed);

        if (hash ^ age_depth) != k {
            return None;
        }

        let eval_checksum = self.inner[idx].eval_checksum.load(Ordering::Relaxed);
        let checksum = eval_checksum as u32;
        let entry = TransTableEntry {
            depth: age_depth as usize & 0xffff_ffff,
            age: (age_depth >> 32) as usize,
            eval: (eval_checksum >> 32) as i32,
        };

        if checksum == murmur3::murmur3_32(&mut entry.to_u8s(), 0).unwrap() {
            Some(entry)
        } else {
            None
        }
    }
}
