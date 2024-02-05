use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct RedisArgs {
    #[arg(long, default_value_t = 6379)]
    pub port: u16,
    #[arg(long, default_value = "data/")]
    pub dir: PathBuf,
    #[arg(long, default_value = "dump.rdb")]
    pub dbfilename: PathBuf,
}
// TODO port might need to be a string