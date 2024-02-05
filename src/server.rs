use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use crate::args::RedisArgs;
use crate::cmd::commands;
use crate::db::{
    database::{RandomMap, RedisValue},
    Database,
};
use crate::pool;
use crate::resp::{Array, RespValue};

pub struct RedisServer {
    pool: pool::ThreadPool,
    evictor: thread::JoinHandle<()>,
    listener: TcpListener,
    db: Database,
    config_db: Database,
}
impl RedisServer {
    pub fn new(args: &RedisArgs, num_workers: usize) -> RedisServer {
        let addr = format!("127.0.0.1:{}", args.port);
        let pool = pool::ThreadPool::build(num_workers);
        let listener = TcpListener::bind(addr).unwrap();
        let db = Arc::new(RwLock::new(RandomMap::new()));
        let config_db = RedisServer::populate_config_db(args);
        let evictor = RedisServer::random_evict_loop(Arc::clone(&db));
        RedisServer {
            pool,
            evictor,
            listener,
            db,
            config_db,
        }
    }
    pub fn serve(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("accepted new connection");
                    let db = Arc::clone(&self.db);
                    let config_db = Arc::clone(&self.config_db);
                    self.pool.submit(|| {
                        RedisServer::handle_connection(db, config_db, stream);
                    });
                }
                Err(e) => println!("connection failed: {}", e),
            }
        }
    }
    fn handle_connection(db: Database, config_db: Database, mut stream: TcpStream) {
        // handle_connection is standalone function, not a method, to prevent moving self.method to closure
        // this loop below is for handling multiple commands for the same, one connection
        loop {
            // TODO super bad code I think, need some cool buffer tricks
            let mut buffer = [0u8; 100];
            match stream.read(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    let payload = &buffer[..bytes_read];
                    let mut it = match Array::parse_client_bytes(payload) {
                        Ok(array) => array.into_iter(),
                        Err(e) => {
                            stream.write_all(e.to_output().as_bytes()).unwrap();
                            continue;
                        }
                    };
                    let cmd = match commands::Cmd::from_args(&mut it) {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            stream.write_all(e.to_output().as_bytes()).unwrap();
                            continue;
                        }
                    };
                    let resp_out = match cmd.run(Arc::clone(&db), Arc::clone(&config_db)) {
                        Ok(resp) => resp,
                        Err(e) => {
                            stream.write_all(e.to_output().as_bytes()).unwrap();
                            continue;
                        }
                    };
                    stream.write_all(resp_out.to_output().as_bytes()).unwrap();
                }
                _ => break,
            }
        }
    }
    fn random_evict_loop(db: Database) -> thread::JoinHandle<()> {
        eprintln!("starting active eviction background thread");
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(1000));
            let mut guard = db.write().unwrap();
            let evicted = guard.random_evict();
            if let Some((k, r)) = evicted {
                eprintln!("key {k} value {} was evicted", r.content)
            }
        })
    }
    fn populate_config_db(args: &RedisArgs) -> Database {
        let mut random_map = RandomMap::new();
        random_map.set(
            "port".into(),
            RedisValue {
                content: args.port.to_string(),
                expiry: None,
            },
        );
        random_map.set(
            "dir".into(),
            RedisValue {
                content: args.dir.to_str().expect("dir is not valid utf-8").into(),
                expiry: None,
            },
        );
        random_map.set(
            "dbfilename".into(),
            RedisValue {
                content: args
                    .dbfilename
                    .to_str()
                    .expect("dbfilename is not valid utf-8")
                    .into(),
                expiry: None,
            },
        );
        Arc::new(RwLock::new(random_map))
    }
}
// TODO graceful
