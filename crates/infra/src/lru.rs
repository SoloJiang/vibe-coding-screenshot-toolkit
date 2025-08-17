use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

pub struct LruCache<K, V> {
    cap: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn put(&mut self, k: K, v: V) {
        if self.map.contains_key(&k) {
            self.order.retain(|x| x != &k);
        }
        self.map.insert(k.clone(), v);
        self.order.push_front(k.clone());
        if self.order.len() > self.cap {
            if let Some(old) = self.order.pop_back() {
                self.map.remove(&old);
            }
        }
    }

    pub fn get(&mut self, k: &K) -> Option<&V> {
        if self.map.contains_key(k) {
            let kcl = k.clone();
            self.order.retain(|x| x != k);
            self.order.push_front(kcl);
        }
        self.map.get(k)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru() {
        let mut c = LruCache::new(2);
        c.put(1, "a");
        c.put(2, "b");
        assert_eq!(c.len(), 2);
        c.get(&1);
        c.put(3, "c");
        assert!(c.get(&2).is_none());
        assert!(c.get(&1).is_some());
        assert!(c.get(&3).is_some());
    }
}
