use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

mod commands;
mod pool;
mod resp;

use resp::dtypes::{RespValue, SimpleError};

fn handle(mut stream: TcpStream) {
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
                let resp_out = cmd.respond();
                stream.write_all(resp_out.to_output().as_bytes()).unwrap();
            }
            _ => break,
        }
    }
}

fn main() {
    let pool = pool::ThreadPool::build(4);

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                pool.submit(|| {
                    handle(stream);
                });
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
