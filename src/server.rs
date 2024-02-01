use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use crate::pool::ThreadPool;

use super::commands;
use super::pool;
use super::resp;
use super::resp::dtypes::RespValue;

pub type Database = Arc<RwLock<HashMap<String, String>>>;

pub struct RedisServer {
    pool: ThreadPool,
    listener: TcpListener,
    db: Database,
}
impl RedisServer {
    pub fn new(addr: &str) -> RedisServer {
        let pool = pool::ThreadPool::build(4);
        let listener = TcpListener::bind(addr).unwrap();
        RedisServer {
            pool,
            listener,
            db: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn serve(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("accepted new connection");
                    self.pool.submit({
                        let db = Arc::clone(&self.db);
                        || {
                            RedisServer::handle(db, stream);
                        }
                    });
                }

                Err(e) => {
                    println!("listener error: {}", e);
                }
            }
        }
    }
    // handle is standalone function, not a method!
    fn handle(db: Database, mut stream: TcpStream) {
        loop {
            // TODO super bad code i think, need some cool buffer tricks
            let mut buffer = [0u8; 100];
            match stream.read(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    let payload = &buffer[..bytes_read];
                    let mut it = match resp::array::parse_client_bytes(payload) {
                        Ok(array) => array.into_iter(),
                        Err(e) => {
                            stream.write_all(e.to_output().as_bytes()).unwrap();
                            continue;
                        }
                    };
                    let cmd = match commands::Cmd::from_bulk_strings(&mut it) {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            stream.write_all(e.to_output().as_bytes()).unwrap();
                            continue;
                        }
                    };
                    // TODO REFAC and check if the arc clone logic is correct??
                    let resp_out = match cmd.respond(Arc::clone(&db)) {
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
}

fn main() {}
