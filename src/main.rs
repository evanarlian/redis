mod cmd;
mod pool;
mod resp;
mod server;

use server::RedisServer;

use rand::{self, Rng};

fn main() {
    println!("random number {}", rand::thread_rng().gen_range(0..100));
    let server = RedisServer::new("127.0.0.1:6379", 4);
    server.serve();
}
