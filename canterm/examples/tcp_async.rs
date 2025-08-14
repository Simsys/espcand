use std::time::Duration;

use smol::{
    future::FutureExt,
    io::{self, AsyncReadExt, AsyncWriteExt}, 
    net::TcpStream,
    Timer, 
    Unblock,
};


async fn connect() -> io::Result<TcpStream> {
    TcpStream::connect("192.168.178.170:1234").await
}

fn main() -> ! {
    smol::block_on(async {
        let mut buf = [0_u8; 1024];
        let mut stdout = Unblock::new(std::io::stdout());
        let mut reconnect = false;

        let mut stream = connect().await.unwrap();
        println!("Connected");

        loop {
            if reconnect {
                if let Ok(new_stream) = connect().await {
                    println!("Reconnected");
                    stream = new_stream;
                }
                reconnect = false;
            }

            FutureExt::or(
                async {
                    let n = match stream.read(&mut buf).await {
                        Ok(n) => n,
                        Err(_) => 0,
                    };
                    stdout.write_all(&buf[..n]).await.unwrap();
                },
                async {
                    Timer::after(Duration::from_secs(1)).await;
                    reconnect = true;
                }
            ).await;
        }
    })
}

