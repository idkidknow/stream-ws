use std::{
    io::{self, Read},
    sync::Arc,
};
use stream_ws::{tungstenite::WsMessageHandler, WsMessageHandle};
use tokio::{
    io::{split, stdout, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, Notify},
};
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> io::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or("ws://127.0.0.1:8080/".to_owned());
    let (ws, _) = connect_async(url).await.unwrap();
    // let stream = stream_ws::tungstenite::WsByteStream::new(ws);
    let stream = WsMessageHandler::wrap_stream(ws);
    let (read, write) = split(stream);
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();
    let (stdin_tx, stdin_rx) = mpsc::channel(1);
    std::thread::spawn(|| stdin_thread(stdin_tx));
    tokio::spawn(send(write, notify, stdin_rx));
    receive(read, notify2).await?;
    Ok(())
}

async fn receive(mut read: impl AsyncRead + Unpin, notify: Arc<Notify>) -> io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let len = read.read(&mut buf).await?;
        if len == 0 {
            break;
        }
        stdout().write_all(&buf[..len]).await?;
        stdout().flush().await?;
    }
    let _ = notify.notify_one();
    Ok(())
}

async fn send(
    mut write: impl AsyncWrite + Unpin,
    closed: Arc<Notify>,
    mut rx: mpsc::Receiver<Vec<u8>>,
) -> io::Result<()> {
    loop {
        let res = tokio::select! {
            res = rx.recv() => res,
            _ = closed.notified() => break,
        };
        if let Some(bytes) = res {
            write.write_all(&bytes).await?;
            write.flush().await?;
        }
    }
    write.shutdown().await?;
    Ok(())
}

fn stdin_thread(tx: mpsc::Sender<Vec<u8>>) -> io::Result<()> {
    let mut buf = [0u8; 1024];
    let mut stdin = std::io::stdin();
    loop {
        let len = stdin.read(&mut buf)?;
        if len == 0 {
            break;
        }
        let _ = tx.blocking_send(buf[..len].to_owned());
    }
    Ok(())
}
