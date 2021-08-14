use dashmap::DashMap;
use std::hash::Hash;
use std::collections::hash_map::RandomState;
use std::sync::Mutex;

pub trait GetOrdKey {
    type Output: Ord + Clone;
    fn get_order_key(&self) -> Self::Output;
}

pub struct LimitedMap<K, V> {
    records: DashMap<K, V>,
    limit: usize,
    lock_key: Mutex<usize>,
}

impl<K, V> LimitedMap<K, V>
    where K: Eq + Hash + Clone, V: Clone + GetOrdKey {
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
        let guard = self.lock_key.lock().unwrap();
        if self.records.len() >= self.limit {
            let vec = &mut Vec::new();
            self.records.iter().for_each(|e| {
                vec.push((e.key().clone(), e.value().get_order_key()))
            });
            vec.sort_unstable_by_key(|(_, sort_key)| sort_key.clone());
            let keys: Vec<&K> = vec[0..self.limit / 10].iter()
                .map(|(k, _)| k).collect();
            self.records.retain(|r, _| {
                !keys.contains(&r)
            })
        }
        drop(guard);
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

#[cfg(test)]
mod tests {
    use crate::cache::limit_map::{LimitedMap, GetOrdKey};

    impl GetOrdKey for i32 {
        type Output = i32;

        fn get_order_key(&self) -> Self::Output {
            self.clone()
        }
    }

    #[test]
    fn should_insert_into_map_when_call_insert_given_empty_map() {
        let map = LimitedMap::from(1);

        map.insert(1, 1);

        let result = map.records.get(&1)
            .map(|r| r.value().clone());
        assert_eq!(Some(1), result)
    }

    #[test]
    fn should_insert_into_map_and_remove_10_persist_when_call_insert_given_full_map() {
        let map = LimitedMap::from(100);
        (0..100).for_each(|r| {
            map.records.insert(r, r);
        });

        map.insert(1000, 1000);

        let result = map.records.get(&1000)
            .map(|r| r.value().clone());
        let over_result = map.records.get(&9).map(|r| r.value().clone());
        assert_eq!(Some(1000), result);
        assert_eq!(91, map.records.len());
        assert_eq!(None, over_result)
    }

    #[test]
    fn should_remove_from_map_when_call_remove_given_has_value_in_map() {
        let map = LimitedMap::from(1);
        map.records.insert(1, 1);

        map.remove(&1);

        let result = map.records.get(&1)
            .map(|r| r.value().clone());
        assert_eq!(None, result)
    }

    #[test]
    fn should_return_true_when_call_is_empty_given_empty_map() {
        let map = LimitedMap::<i32, i32>::from(1);

        let result = map.is_empty();

        assert!(result)
    }

    #[test]
    fn should_return_false_when_call_is_empty_given_has_value_in_map() {
        let map = LimitedMap::from(1);
        map.records.insert(1, 1);

        let result = map.is_empty();

        assert!(!result)
    }

    #[test]
    fn should_return_value_when_call_get_given_has_value() {
        let map = LimitedMap::from(1);
        map.records.insert(1, 1);

        let result = map.get(&1);

        assert_eq!(Some(1), result)
    }

    #[test]
    fn should_return_none_when_call_get_given_no_value() {
        let map = LimitedMap::from(1);
        map.records.insert(2, 1);

        let result = map.get(&1);

        assert_eq!(None, result)
    }
}