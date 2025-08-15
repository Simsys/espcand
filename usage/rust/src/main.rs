use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> ! {
    let mut rxbuf = [0u8; 1024];
    let mut stream = TcpStream::connect("192.168.178.170:1234").unwrap();

    // Clear all filters
    stream.write(b"$clearfilt\n").unwrap();
    
    // Define a filter for all datagrams, but only allow them through every 5 seconds
    stream.write(b"$pfilt,5000,***_****_****\n").unwrap();

    // Send a can data frame on the bus
    stream.write(b"$fts,12a,3,1a2b3c\n").unwrap();

    // Show all received frames
    loop {
        let n = match stream.read(&mut rxbuf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        print!("{}", str::from_utf8(&rxbuf[..n]).unwrap());
    }
}
