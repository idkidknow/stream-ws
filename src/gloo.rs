use crate::{WsErrorKind, WsMessageHandle, WsMessageKind};
use gloo_net::websocket::{Message, WebSocketError as Error};

pub type WsByteStream<S> = crate::WsByteStream<S, Message, Error, WsMessageHandler>;

pub struct WsMessageHandler;

impl WsMessageHandle<Message, Error> for WsMessageHandler {
    fn message_into_kind(msg: Message) -> WsMessageKind {
        match msg {
            Message::Bytes(msg) => WsMessageKind::Bytes(msg),
            _ => WsMessageKind::Other,
        }
    }

    fn error_into_kind(e: Error) -> WsErrorKind {
        match e {
            Error::ConnectionClose(event) if event.was_clean => WsErrorKind::Closed,
            e => WsErrorKind::Other(Box::new(e)),
        }
    }

    fn message_from_bytes<T: Into<Vec<u8>>>(bytes: T) -> Message {
        Message::Bytes(bytes.into())
    }
}
