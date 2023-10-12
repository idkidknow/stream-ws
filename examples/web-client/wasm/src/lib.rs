use std::sync::Arc;

use futures::{AsyncWriteExt, AsyncReadExt, io::{ReadHalf, WriteHalf}, lock::Mutex};
use gloo::net::websocket::futures::WebSocket;
use wasm_bindgen::prelude::*;
use stream_ws::{gloo::WsMessageHandler, WsMessageHandle};

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

type InnerStream = stream_ws::gloo::WsByteStream<WebSocket>;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Stream(Arc<Mutex<ReadHalf<InnerStream>>>, Arc<Mutex<WriteHalf<InnerStream>>>);

#[wasm_bindgen]
pub fn ws_client() -> Result<Stream, js_sys::Error> {
    let ws = WebSocket::open("ws://127.0.0.1:8080").map_err(to_js_error)?;
    let stream = WsMessageHandler::wrap_stream(ws);
    let (read, write) = stream.split();
    Ok(Stream(Arc::new(Mutex::new(read)), Arc::new(Mutex::new(write))))
}

#[wasm_bindgen]
pub async fn send_msg(stream: &Stream, msg: &str) -> Result<(), js_sys::Error> {
    let write = &stream.1;
    let mut write = write.lock().await;
    let bytes = msg.as_bytes();
    write.write_all(bytes).await.map_err(to_js_error)?;
    write.flush().await.map_err(to_js_error)?;
    drop(write);
    Ok(())
}

#[wasm_bindgen]
pub async fn receive_msg(stream: &Stream, max_len: usize) -> Result<String, js_sys::Error> {
    let read = &stream.0;
    let mut read = read.lock().await;
    let mut buf = vec![0u8; max_len];
    let len = read.read(&mut buf).await.map_err(to_js_error)?;
    drop(read);
    buf.truncate(len);
    let str = String::from_utf8(buf).map_err(to_js_error)?;
    Ok(str)
}

fn to_js_error<E: Into<Box<dyn std::error::Error>>>(e: E) -> js_sys::Error {
    let e: Box<_> = e.into();
    js_sys::Error::new(&e.to_string())
}
