use dashmap::DashMap;
use std::hash::Hash;
use std::collections::hash_map::RandomState;
use std::sync::Mutex;

pub trait GetOrdKey {
    type Output: Ord;
    fn get_order_key(&self) -> Self::Output;
}

pub struct LimitedMap<K, V> {
    records: DashMap<K, V>,
    limit: usize,
    lock_key: Mutex<usize>,
}

impl<K, V> LimitedMap<K, V>
    where K: Eq + Hash, V: Clone + GetOrdKey {
    pub fn new() -> Self {
        LimitedMap {
            records: DashMap::new(),
            limit: 0,
            lock_key: Mutex::new(0),
        }
    }

    pub fn from(limit: usize) -> Self {
        LimitedMap {
            records: DashMap::with_capacity(limit),
            limit,
            lock_key: Mutex::new(0),
        }
    }
    pub fn get(&self, key: &K) -> Option<V> {
        self.records.get(key).map(|r| r.value().clone())
    }

    pub fn insert(&self, key: K, value: V) {
        //如果超过了限制的大小，则删除掉十分之一最小的记录
        if self.records.len() >= self.limit {
            let guard = self.lock_key.lock().unwrap();
            let vec = &mut Vec::new();
            self.records.iter().for_each(|e| {
                vec.push(e)
            });
            vec.sort_unstable_by_key(|e| e.get_order_key());
            vec[0..self.limit / 10].iter().for_each(|e| {
                self.records.remove(e.key());
            });
            drop(guard)
        }
        self.records.insert(key, value);
    }

    pub fn remove(&self, key: &K) {
        self.records.remove(key);
    }

    pub fn iter(&self) -> dashmap::iter::Iter<K, V, RandomState, DashMap<K, V, RandomState>> {
        self.records.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}