# stream-ws

A layer over WebSocket enables carrying byte stream, for both native and WebAssembly.

Providing methods able to wrap any WebSocket message stream implementation,
and impl [`AsyncRead`], [`AsyncBufRead`] and [`AsyncWrite`].

## Usage

run `cargo add stream-ws` to bring it into your crate.

Examples in `examples/`.

### Tungstenite

With feature `tungstenite`.

For `WebSocketStream` in either crate `tokio-tungstenite` or `async-tungstenite`,
use `let stream = stream_ws::tungstenite::WsByteStream::new(inner)`.

### Gloo (for WebAssembly)

With feature `gloo`.

use `let stream = stream_ws::gloo::WsByteStream::new(inner)`.

### Wrapping underlying stream of other WebSocket implementation

Your WebSocket implementation should have a struct `S` satisfying trait bound
`Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin` where `Msg` and `E`
are message and error type of the implementation.

Create a struct `Handler` and impl [`WsMessageHandle`], which is easy, and then
call `Handler::wrap_stream(underlying_stream)` to get a [`WsByteStream`].

## Crate features

- `tokio`: impl `tokio`'s [`AsyncRead`], [`AsyncBufRead`] and [`AsyncWrite`] variants
- `tungstenite`: handlers for message and error type from crate `tungstenite`
- `gloo`: handlers for message and error type from crate `gloo`
