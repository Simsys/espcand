use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> ! {
    let mut buf = [0_u8; 1024];
    let mut stream = TcpStream::connect("192.168.178.170:1234").unwrap();

    loop {
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        print!("{}", str::from_utf8(&buf[..n]).unwrap());
    }

} // the stream is closed here