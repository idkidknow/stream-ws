# stream-ws

[![docs.rs](https://img.shields.io/docsrs/stream-ws)](https://docs.rs/crate/stream-ws/latest)
[![repo](https://img.shields.io/badge/repo-stream--ws-blue?logo=github)](https://github.com/idkidknow/stream-ws)
[![crates-io](https://img.shields.io/badge/crates--io-stream--ws-blue)](https://crates.io/crates/stream-ws)

A layer over WebSocket enables carrying byte stream, for both native and WebAssembly.

Providing methods able to wrap any WebSocket message stream implementation,
and impl [`AsyncRead`](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html),
[`AsyncBufRead`](https://docs.rs/futures-io/latest/futures_io/trait.AsyncBufRead.html)
and [`AsyncWrite`](https://docs.rs/futures-io/latest/futures_io/trait.AsyncWrite.html).

## Usage

run `cargo add stream-ws` to bring it into your crate.

Examples in [`examples/`](https://github.com/idkidknow/stream-ws/tree/main/examples).

### Tungstenite

With feature `tungstenite`.

For `WebSocketStream` in either crate [`tokio-tungstenite`](https://crates.io/crates/tokio-tungstenite)
or [`async-tungstenite`](https://crates.io/crates/async-tungstenite),
use

```rust
let stream = stream_ws::tungstenite::WsByteStream::new(inner)
```

### Gloo (for WebAssembly)

With feature `gloo`.

For [`WebSocket`](https://docs.rs/gloo-net/latest/gloo_net/websocket/futures/struct.WebSocket.html)
in crate [`gloo`](https://crates.io/crates/gloo)
use

```rust
let stream = stream_ws::gloo::WsByteStream::new(inner)
```

### Wrapping underlying stream of other WebSocket implementation

Your WebSocket implementation should have a struct `S` satisfying trait bound

```rust
Stream<Item = Result<Msg, E>> + Sink<Msg, Error = E> + Unpin
```

where `Msg` and `E`
are message and error type of the implementation.

Create a struct `Handler` and impl
[`WsMessageHandle`](https://docs.rs/stream-ws/latest/stream_ws/trait.WsMessageHandle.html),
which is easy, and then
call `Handler::wrap_stream(underlying_stream)`
 to get a [`WsByteStream`](https://docs.rs/stream-ws/latest/stream_ws/struct.WsByteStream.html).

## Crate features

- `tokio`: impl `tokio`'s `AsyncRead`, `AsyncBufRead` and `AsyncWrite` variants
- `tungstenite`: handlers for message and error type from crate `tungstenite`
- `gloo`: handlers for message and error type from crate `gloo`

License: MIT OR Apache-2.0
