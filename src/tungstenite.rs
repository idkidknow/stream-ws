use crate::{WsErrorKind, WsMessageHandle, WsMessageKind};
use tungstenite::{Error, Message};

pub type WsByteStream<S> = crate::WsByteStream<S, Message, Error, WsMessageHandler>;

pub struct WsMessageHandler;

impl WsMessageHandle<Message, Error> for WsMessageHandler {
    fn message_into_kind(msg: Message) -> WsMessageKind {
        match msg {
            Message::Binary(msg) => WsMessageKind::Bytes(msg),
            Message::Close(_) => WsMessageKind::Close,
            _ => WsMessageKind::Other,
        }
    }

    fn error_into_kind(e: Error) -> WsErrorKind {
        match e {
            Error::ConnectionClosed => WsErrorKind::Closed,
            Error::AlreadyClosed => WsErrorKind::AlreadyClosed,
            Error::Io(e) => WsErrorKind::Io(e),
            e => WsErrorKind::Other(Box::new(e)),
        }
    }

    fn message_from_bytes<T: Into<Vec<u8>>>(bytes: T) -> Message {
        Message::Binary(bytes.into())
    }
}
