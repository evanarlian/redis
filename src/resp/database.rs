// TODO this should not be coupled with resp, yank out this file later
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use rand::rngs::ThreadRng;

pub struct RedisValue {
    pub content: String,
    pub expiry: Option<SystemTime>,
}

struct Wrapper {
    r: RedisValue,
    i: usize,
}

pub type Database = Arc<RwLock<HashMap<String, RedisValue>>>;

// https://interviewing.io/questions/insert-delete-get-random-o-1
struct RandomMap {
    map: HashMap<String, Wrapper>,
    vec: Vec<String>,
    rng: ThreadRng,
}
impl RandomMap {
    fn new() -> Self {
        Self {
            vec: Vec::new(),
            map: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
    fn len(&self) -> usize {
        assert_eq!(
            self.map.len(),
            self.vec.len(),
            "RandomMap map and vec lengths are different"
        );
        self.map.len()
    }
    fn reorganize_map_vec(&mut self, del_i: usize) {
        // because of o(1) random access, need to maintain both map and vec
        // 1. swap vec[del_i] with the last element
        let last_idx = self.vec.len() - 1;
        self.vec.swap(del_i, last_idx);
        // 2. pop last elem, guaranteed success if everything is right
        let last_key = self.vec.pop().unwrap();
        // 3. change the map[last_key] to point to originally deleted location
        self.map.get_mut(&last_key).unwrap().i = del_i;
    }
    pub fn evict(&mut self, key: &str) {
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
            self.map.remove(key);
            self.reorganize_map_vec(del_i);
        }
    }
    pub fn del(&mut self, key: &str) -> Option<RedisValue> {
        // THE D of CRUD
        // note that eviction has been handled by self.evict
        // will always remove the value, and returning previous value if exists (useful for getdel)
        self.evict(key);
        if let Some(Wrapper { r: del_r, i: del_i }) = self.map.remove(key) {
            self.reorganize_map_vec(del_i);
            Some(del_r)
        } else {
            // key does not exist, do nothing
            None
        }
    }
    fn _get(&mut self, key: &str) -> Option<Wrapper> {
        // INTERNAL FUNCTION: THE R of CRUD
        // gets the redis value after expired key eviction
        self.evict(key);
        todo!()
    }
    pub fn set(&mut self, key: String, new_r: RedisValue) -> Option<RedisValue> {
        // THE CU of CRUD
        // note that eviction has been handled by self._get
        // returns the previous value of the key, useful for cmd GETSET
        if let Some(Wrapper { r: old_r, i: old_i }) = self._get(&key) {  // TODO insert API does return too, check later
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
}
