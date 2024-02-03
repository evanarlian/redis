use std::sync::{Arc, RwLock};

pub mod database;

pub type Database = Arc<RwLock<database::RandomMap>>;
