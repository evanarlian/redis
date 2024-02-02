use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

pub struct RedisValue {
    pub content: String,
    pub expiry: Option<SystemTime>,
}

pub type Database = Arc<RwLock<HashMap<String, RedisValue>>>;

struct RandomMap {
    vec: Vec<String>,
    map: HashMap<String, RedisValue>,
}
impl RandomMap {
    fn new() -> Self {
        Self {
            vec: Vec::new(),
            map: HashMap::new(),
        }
    }
    // crud
}
