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

// TODO should i encalpusalte this??? user does not need to know this
struct Wrapper {
    r: RedisValue,
    i: usize,
}

pub type Database = Arc<RwLock<HashMap<String, RedisValue>>>;

// https://interviewing.io/questions/insert-delete-get-random-o-1
// this should not be coupled with resp, yank out this file later
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
    pub fn del(&mut self, key: &str) -> bool {
        // INTERNAL FUNCTION: THE D of CRUD
        // will always always remove the key
        // returns true if deletion happens because of caller intention
        // returns false if either key does not exist, or because of expired
        todo!()
    }
    pub fn evict(&mut self, key: &str) {
        // only delete on expired key
        todo!()
    }
    fn _get(&mut self, key: &str) -> Option<Wrapper> {
        // INTERNAL FUNCTION: THE R of CRUD
        // gets the redis value after expired key eviction
        self.evict(key);
        todo!()
    }
    pub fn set(&mut self, key: String, new_r: RedisValue) -> Option<RedisValue> {
        // THE CU of CRUD
        // note that eviction has been handled bu self._get
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
    // fn getdel(&mut self, key: &str) -> Option<RedisValue> {
    //     self.create(key);
    //     self.delete(key);
    //     todo!()
    // }
}
