use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let mut buffer = [0u8; 100];
                let bytes_read = stream.read(&mut buffer).unwrap();
                println!("{:?}", std::str::from_utf8(&buffer[..bytes_read]));
                stream.write_all("+PONG\r\n".as_bytes()).unwrap();
                stream.write_all("+PONG\r\n".as_bytes()).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
