mod cmd;
mod pool;
mod resp;
mod server;

use server::RedisServer;

fn main() {
    let server = RedisServer::new("127.0.0.1:6379", 4);
    server.serve();
}
