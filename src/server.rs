use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
};

use crate::cmd::commands;
use crate::db::{database::RandomMap, Database};
use crate::pool;
use crate::resp;
use crate::resp::dtypes::RespValue;
// TODO ugly imports

pub struct RedisServer {
    pool: pool::ThreadPool,
    listener: TcpListener,
    db: Database,
}
impl RedisServer {
    pub fn new(addr: &str, num_workers: usize) -> RedisServer {
        let pool = pool::ThreadPool::build(num_workers);
        let listener = TcpListener::bind(addr).unwrap();
        RedisServer {
            pool,
            listener,
            db: Arc::new(RwLock::new(RandomMap::new())),
        }
    }
    pub fn serve(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("accepted new connection");
                    let db = Arc::clone(&self.db);
                    self.pool.submit(|| {
                        RedisServer::handle_connection(db, stream);
                    });
                }
                Err(e) => println!("connection failed: {}", e),
            }
        }
    }
    fn handle_connection(db: Database, mut stream: TcpStream) {
        // handle_connection is standalone function, not a method!
        // this is to prevent moving self.method to closure
        // this loop below is for handling multiple commands for the same connection, TODO test what is the difference between the incoming loop vs this loop?
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
                    // TODO REFAC and check if the arc clone logic is correct?? -- tied to loop problems
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
