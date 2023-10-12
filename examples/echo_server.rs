use std::io;
use stream_ws::{tungstenite::WsMessageHandler, WsMessageHandle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:8080".to_owned());
    let listener = TcpListener::bind(&addr).await?;
    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection: {}", &addr);
        tokio::spawn(async move {
            let ret = handler(stream).await;
            if let Err(e) = ret {
                eprintln!("Error: {}", e);
            }
            println!("Connection closed: {}", &addr);
        });
    }
    Ok(())
}

async fn handler(stream: TcpStream) -> io::Result<()> {
    let ws = accept_async(stream).await.unwrap();
    // let mut stream = stream_ws::tungstenite::WsByteStream::new(ws);
    let mut stream = WsMessageHandler::wrap_stream(ws);
    let mut buf = [0u8; 1024];
    loop {
        let len = stream.read(&mut buf).await?;
        if len == 0 {
            break;
        }
        stream.write(&buf[..len]).await?;
        stream.flush().await?;
    }
    stream.shutdown().await?;
    Ok(())
}
