//! A layer over WebSocket enables carrying byte stream, for both native and WebAssembly.
//!
//! Providing methods able to wrap any WebSocket message stream implementation,
//! and impl [`AsyncRead`], [`AsyncBufRead`] and [`AsyncWrite`].
//!
//! # Usage
//!
//! run `cargo add stream-ws` to bring it into your crate.
//!
//! Examples in `examples/`.
//!
//! ## Tungstenite
//!
//! With feature `tungstenite`.
//!
//! For `WebSocketStream` in either crate `tokio-tungstenite` or `async-tungstenite`,
//! use `let stream = stream_ws::tungstenite::WsByteStream::new(inner)`.
//!
//! ## Gloo (for WebAssembly)
//!
//! With feature `gloo`.
//!
//! use `let stream = stream_ws::gloo::WsByteStream::new(inner)`.
//!
//! ## Wrapping underlying stream of other WebSocket implementation
//!
//! Your WebSocket implementation should have a struct `S` satisfying trait bound
//! `Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin` where `Msg` and `E`
//! are message and error type of the implementation.
//!
//! Create a struct `Handler` and impl [`WsMessageHandle`], which is easy, and then
//! call `Handler::wrap_stream(underlying_stream)` to get a [`WsByteStream`].
//!
//! # Crate features
//!
//! - `tokio`: impl `tokio`'s [`AsyncRead`], [`AsyncBufRead`] and [`AsyncWrite`] variants
//! - `tungstenite`: handlers for message and error type from crate `tungstenite`
//! - `gloo`: handlers for message and error type from crate `gloo`

#[cfg(feature = "gloo")]
#[cfg_attr(docsrs, doc(cfg(feature = "gloo")))]
pub mod gloo;
#[cfg(feature = "tungstenite")]
#[cfg_attr(docsrs, doc(cfg(feature = "tungstenite")))]
pub mod tungstenite;

use futures::{ready, AsyncBufRead, AsyncRead, AsyncWrite, Sink, Stream};
use pin_project::pin_project;
use std::{io, marker::PhantomData, pin::Pin, task::Poll};
#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
use tokio::io::{
    AsyncBufRead as TokioAsyncBufRead, AsyncRead as TokioAsyncRead, AsyncWrite as TokioAsyncWrite,
};

#[derive(Debug)]
pub enum WsMessageKind {
    /// Messages considered as payload carriers
    Bytes(Vec<u8>),
    Close,
    /// Messages that will be ignored
    Other,
}

#[derive(Debug)]
pub enum WsErrorKind {
    Io(io::Error),
    /// Normally closed. Won't be considered as an error.
    Closed,
    AlreadyClosed,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

/// Classify messages and errors of WebSocket.
pub trait WsMessageHandle<Msg, E> {
    fn message_into_kind(msg: Msg) -> WsMessageKind;
    fn error_into_kind(e: E) -> WsErrorKind;
    /// These bytes will be carried by the `Msg` and sent by the underlying stream.
    fn message_from_bytes<T: Into<Vec<u8>>>(bytes: T) -> Msg;

    fn wrap_stream<S>(inner: S) -> WsByteStream<S, Msg, E, Self>
    where
        S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    {
        WsByteStream::new(inner)
    }
}

/// A wrapper implements [`AsyncRead`], [`AsyncBufRead`] and [`AsyncWrite`],
/// around a established websocket connect which
/// implemented `Stream` and `Sink` traits.
///
/// Bytes are transported on binary messages over WebSocket.
/// Other messages except close messages will be ignored.
///
/// # Assumption:
///
/// The underlying stream should automatically handle ping and pong messages.
#[pin_project]
pub struct WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    #[pin]
    inner: S,
    state: State,
    _marker: PhantomData<H>,
}

#[derive(Debug)]
struct State {
    read: ReadState,
    write: WriteState,
}

#[derive(Debug)]
enum ReadState {
    /// Buf is empty and ready to read from WebSocket.
    Pending,
    /// Bytes from a message not all being read, are stored in `buf`. The amount of bytes read is `amt_read`.
    Ready { buf: Vec<u8>, amt_read: usize },
    /// Connection was shut down correctly.
    Terminated,
}

#[derive(Debug)]
enum WriteState {
    Ready,
    Closed,
}

impl<S, Msg, E, H> WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            state: State {
                read: ReadState::Pending,
                write: WriteState::Ready,
            },
            _marker: PhantomData,
        }
    }

    /// This function tries reading next message to fill the buf (set the state to `ReadState::Ready`).
    ///
    /// Returns `None` if the stream has been fused. (The state is set to `ReadState::Terminated`)
    fn fill_buf_with_next_msg(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<io::Result<()>>> {
        let mut this = self.project();
        loop {
            // Try read from inner stream.
            let res = ready!(this.inner.as_mut().poll_next(cx));
            let Some(res) = res else {
                // `res` is None. Stream has been fused.
                this.state.read = ReadState::Terminated;
                return Poll::Ready(None);
            };
            match res {
                Ok(msg) => {
                    let msg = H::message_into_kind(msg);
                    match msg {
                        WsMessageKind::Bytes(msg) => {
                            this.state.read = ReadState::Ready {
                                buf: msg,
                                amt_read: 0,
                            };
                            return Poll::Ready(Some(Ok(())));
                        }
                        WsMessageKind::Close => {
                            this.state.read = ReadState::Terminated;
                            return Poll::Ready(None);
                        }
                        WsMessageKind::Other => {
                            // Simply ignore it.
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let e = H::error_into_kind(e);
                    match e {
                        WsErrorKind::Io(e) => {
                            return Poll::Ready(Some(Err(e)));
                        }
                        WsErrorKind::Closed => {
                            this.state.read = ReadState::Terminated;
                            return Poll::Ready(None);
                        }
                        WsErrorKind::AlreadyClosed => {
                            this.state.read = ReadState::Terminated;
                            let e = io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                            return Poll::Ready(Some(Err(e)));
                        }
                        WsErrorKind::Other(e) => {
                            return Poll::Ready(Some(Err(io::Error::new(io::ErrorKind::Other, e))));
                        }
                    }
                }
            }
        }
    }
}

impl<S, Msg, E, H> AsyncRead for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        dst: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let this = self.as_mut().project();
            match this.state.read {
                ReadState::Pending => {
                    let res = ready!(self.as_mut().fill_buf_with_next_msg(cx));
                    match res {
                        Some(Ok(())) => continue, // The state is assumed to be `Ready`
                        Some(Err(e)) => return Poll::Ready(Err(e)),
                        None => continue, // The state is assumed to be `Terminated`
                    }
                }
                ReadState::Ready {
                    ref buf,
                    ref mut amt_read,
                } => {
                    let buf = &buf[*amt_read..];
                    let len = std::cmp::min(dst.len(), buf.len());
                    dst[..len].copy_from_slice(&buf[..len]);
                    if len == buf.len() {
                        this.state.read = ReadState::Pending;
                    } else {
                        *amt_read += len;
                    }
                    return Poll::Ready(Ok(len));
                }
                ReadState::Terminated => {
                    return Poll::Ready(Ok(0));
                }
            }
        }
    }
}

impl<S, Msg, E, H> AsyncBufRead for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_fill_buf(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<&[u8]>> {
        loop {
            let this = self.as_mut().project();
            match this.state.read {
                ReadState::Pending => {
                    let res = ready!(self.as_mut().fill_buf_with_next_msg(cx));
                    match res {
                        Some(Ok(())) => continue, // The state is assumed to be `Ready`
                        Some(Err(e)) => return Poll::Ready(Err(e)),
                        None => continue, // The state is assumed to be `Terminated`
                    }
                }
                ReadState::Ready { .. } => {
                    // Borrow and match again to meet the lifetime requirement.
                    let this = self.project();
                    let ReadState::Ready { ref buf, amt_read } = this.state.read else {
                        unreachable!()
                    };
                    return Poll::Ready(Ok(&buf[amt_read..]));
                }
                ReadState::Terminated => {
                    return Poll::Ready(Ok(&[]));
                }
            }
        }
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        let ReadState::Ready {
            ref buf,
            ref mut amt_read,
        } = self.state.read
        else {
            return;
        };
        *amt_read = std::cmp::min(buf.len(), *amt_read + amt);
        if *amt_read == buf.len() {
            self.state.read = ReadState::Pending;
        }
    }
}

impl<S, Msg, E, H> AsyncWrite for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut this = self.as_mut().project();
        loop {
            match this.state.write {
                WriteState::Ready => {
                    if let Err(e) = ready!(this.inner.as_mut().poll_ready(cx)) {
                        let e = H::error_into_kind(e);
                        match e {
                            WsErrorKind::Io(e) => {
                                return Poll::Ready(Err(e));
                            }
                            WsErrorKind::Other(e) => {
                                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                            }
                            WsErrorKind::Closed => {
                                this.state.write = WriteState::Closed;
                                return Poll::Ready(Ok(0));
                            }
                            WsErrorKind::AlreadyClosed => {
                                this.state.write = WriteState::Closed;
                                let e =
                                    io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                                return Poll::Ready(Err(e));
                            }
                        }
                    }
                    // Start sending
                    let Err(e) = this.inner.as_mut().start_send(H::message_from_bytes(buf)) else {
                        this.state.write = WriteState::Ready;
                        return Poll::Ready(Ok(buf.len()));
                    };
                    let e = H::error_into_kind(e);
                    match e {
                        WsErrorKind::Io(e) => {
                            return Poll::Ready(Err(e));
                        }
                        WsErrorKind::Other(e) => {
                            return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                        }
                        WsErrorKind::Closed => {
                            this.state.write = WriteState::Closed;
                            return Poll::Ready(Ok(0));
                        }
                        WsErrorKind::AlreadyClosed => {
                            this.state.write = WriteState::Closed;
                            let e = io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                WriteState::Closed => {
                    let e = io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                    return Poll::Ready(Err(e));
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        if let Err(e) = ready!(this.inner.as_mut().poll_flush(cx)) {
            let e = H::error_into_kind(e);
            match e {
                WsErrorKind::Io(e) => {
                    return Poll::Ready(Err(e));
                }
                WsErrorKind::Other(e) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                }
                WsErrorKind::Closed => {
                    this.state.write = WriteState::Closed;
                    return Poll::Ready(Ok(()));
                }
                WsErrorKind::AlreadyClosed => {
                    this.state.write = WriteState::Closed;
                    let e = io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                    return Poll::Ready(Err(e));
                }
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        this.state.write = WriteState::Closed;
        if let Err(e) = ready!(this.inner.as_mut().poll_close(cx)) {
            let e = H::error_into_kind(e);
            match e {
                WsErrorKind::Io(e) => {
                    return Poll::Ready(Err(e));
                }
                WsErrorKind::Other(e) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                }
                WsErrorKind::Closed => {
                    return Poll::Ready(Ok(()));
                }
                WsErrorKind::AlreadyClosed => {
                    let e = io::Error::new(io::ErrorKind::NotConnected, "Already closed");
                    return Poll::Ready(Err(e));
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
impl<S, Msg, E, H> TokioAsyncRead for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let slice = buf.initialize_unfilled();
        let n = ready!(AsyncRead::poll_read(self, cx, slice))?;
        buf.advance(n);
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
impl<S, Msg, E, H> TokioAsyncBufRead for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<&[u8]>> {
        AsyncBufRead::poll_fill_buf(self, cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        AsyncBufRead::consume(self, amt)
    }
}

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
impl<S, Msg, E, H> TokioAsyncWrite for WsByteStream<S, Msg, E, H>
where
    S: Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin,
    H: WsMessageHandle<Msg, E> + ?Sized,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        AsyncWrite::poll_write(self, cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        AsyncWrite::poll_flush(self, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        AsyncWrite::poll_close(self, cx)
    }
}
