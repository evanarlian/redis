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
                let array = resp::array::parse_client_bytes(payload)?;
                // let cmd = commands::Command::from_bulk_string(&)?;
                let response = match &array[0].0.to_lowercase()[..] {
                    "ping" => "+PONG\r\n".to_owned(),
                    "echo" => format!("+{}\r\n", array[1].0),
                    _ => unreachable!(),
                };
                stream.write_all(response.as_bytes()).unwrap();
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
