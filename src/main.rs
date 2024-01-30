use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

mod pool;

fn handle(mut stream: TcpStream) {
    loop {
        let mut buffer = [0u8; 100];
        if let Ok(bytes_read) = stream.read(&mut buffer) {
            println!("{:?}", std::str::from_utf8(&buffer[..bytes_read]));
            if stream.write("+PONG\r\n".as_bytes()).is_err() {
                break;
            }
        } else {
            break;
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
                pool.submit(|| handle(stream));
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
