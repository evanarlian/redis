mod args;
mod cmd;
mod db;
mod pool;
mod rdb;
mod resp;
mod server;

use clap::Parser;
use server::RedisServer;

fn main() {
    let args = args::RedisArgs::parse();
    println!("{:#?}", args);
    let server = RedisServer::new(&args, 4);
    server.serve();
}
