use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

mod commands;
mod pool;
mod resp;

fn handle(mut stream: TcpStream) -> Result<(), &'static str> {
    loop {
        let mut buffer = [0u8; 100];
        match stream.read(&mut buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                let payload = &buffer[..bytes_read];
                let array = resp::Array::from_client_bytes(payload)?;
                let bulkstring = resp::BulkString::from_array(&array)?;
                let cmd = commands::Command::from_bulk_string(&bulkstring)?;
                stream.write_all(cmd.respond().as_bytes()).unwrap();
            }
            _ => break,
        }
    }
    Ok(())
}

fn main() {
    let pool = pool::ThreadPool::build(4);

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                pool.submit(|| {
                    handle(stream).unwrap();
                });
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
