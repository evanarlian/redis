use std::{collections::HashMap, time::SystemTime};

use rand::Rng;

#[derive(Clone)]
pub struct RedisValue {
    pub content: String,
    pub expiry: Option<SystemTime>,
}

#[derive(Clone)]
struct Wrapper {
    r: RedisValue,
    i: usize,
}

// https://interviewing.io/questions/insert-delete-get-random-o-1
pub struct RandomMap {
    map: HashMap<String, Wrapper>,
    vec: Vec<String>,
}
impl RandomMap {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            map: HashMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        assert_eq!(
            self.map.len(),
            self.vec.len(),
            "RandomMap map and vec lengths are different"
        );
        self.map.len()
    }
    fn reorganize_map_vec(&mut self, del_i: usize) {
        // because of o(1) random access, need to maintain both map and vec
        // 0. special case element on the last cannot be swapped with itself
        let last_idx = self.vec.len() - 1;
        if del_i == last_idx {
            self.vec.pop().unwrap();
            return;
        }
        // 1. swap vec[del_i] with the last element
        self.vec.swap(del_i, last_idx);
        // 2. pop last elem, guaranteed success if everything is right
        self.vec.pop().unwrap();
        // 3. change the map[vec[del_i]] to point to originally deleted location
        self.map.get_mut(&self.vec[del_i]).unwrap().i = del_i;
    }
    pub fn evict(&mut self, key: &str) -> Option<RedisValue> {
        // return true if evicted
        // extract out the deleted index inside braces to satisfy borrow checker
        let maybe_del_i = {
            // if let below explained: all these must match if want to evict:
            // * key exists
            // * expiry exists
            // * now is in the past
            if let Some(Wrapper {
                r:
                    RedisValue {
                        content: _,
                        expiry: Some(expiry),
                    },
                i: del_i,
            }) = self.map.get(key)
            {
                // timeline:
                // (expiry)   (now)  ->  if expiry is in the past (elapsed exist), delete
                // (now)   (expiry)  ->  if expiry is in the future (elapsed error)
                if expiry.elapsed().is_ok() {
                    Some(*del_i) // evict!
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(del_i) = maybe_del_i {
            let evicted = self.map.remove(key).unwrap();
            self.reorganize_map_vec(del_i);
            Some(evicted.r)
        } else {
            None
        }
    }
    pub fn del(&mut self, key: &str) -> Option<RedisValue> {
        // THE D of CRUD
        // delete will always remove the value, and returning previous value if exists (useful for GETDEL)
        if self.evict(key).is_some() {
            // there is no point in checking again, since you have just been evicting
            return None;
        }
        if let Some(Wrapper { r: del_r, i: del_i }) = self.map.remove(key) {
            self.reorganize_map_vec(del_i);
            Some(del_r)
        } else {
            // key does not exist, do nothing
            None
        }
    }
    fn _get(&mut self, key: &str) -> Option<Wrapper> {
        // THE R of CRUD
        // gets the redis value after expired key eviction
        self.evict(key);
        self.map.get(key).cloned()
    }
    pub fn set(&mut self, key: String, new_r: RedisValue) -> Option<RedisValue> {
        // THE CU of CRUD
        // note that eviction has been handled by self._get (hence we do not use HashMap::insert directly)
        // returns the previous value of the key, useful for cmd GETSET
        if let Some(Wrapper { r: old_r, i: old_i }) = self._get(&key) {
            // replace the just that one key of the map
            let new_w = Wrapper { r: new_r, i: old_i };
            self.map.insert(key, new_w);
            Some(old_r)
        } else {
            // insert both map and vec
            let new_w = Wrapper {
                r: new_r,
                i: self.len(),
            };
            self.map.insert(key.clone(), new_w);
            self.vec.push(key);
            None
        }
    }
    pub fn get(&mut self, key: &str) -> Option<RedisValue> {
        // the sole purpose of this get is to remove internal Wrapper
        self._get(key).map(|w| w.r)
    }
    pub fn random_evict(&mut self) -> Option<RedisValue> {
        // also called redis active eviction
        if self.len() == 0 {
            return None;
        }
        let random_idx = rand::thread_rng().gen_range(0..self.len());
        let random_key = self.vec[random_idx].clone();
        self.evict(&random_key)
    }
}
