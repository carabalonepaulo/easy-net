# easy-net

A small and opinionated ENet wrapper for Rust.

`easy-net` exists for one reason: provide a simple and direct networking API for games and real-time applications without requiring async runtimes, background threads, or large abstractions.

The library is intentionally minimal.

## Features

* Reliable UDP using ENet
* No async runtime
* No background threads
* Poll-based API
* Simple event queue
* Stable client IDs
* Lightweight and straightforward integration with game loops and ECS frameworks

## Design

`easy-net` is built around a very simple model:

```rust
server.poll(Duration::from_millis(1));

while let Some(event) = server.next_event() {
    match event {
        ServerEvent::ClientConnect(id) => {}
        ServerEvent::ClientDisconnected(id) => {}
        ServerEvent::PacketReceived(id, data) => {}
    }
}
```

The library intentionally avoids:

* Tokio
* Internal threads
* Async/await
* Callback systems
* Complex transport/protocol abstractions

The goal is to stay small, predictable, and easy to integrate into existing game loops.

## Installation

```toml
[dependencies]
easy-net = "0.1"
```

## Server Example

```rust
use easy_net::{Server, ServerEvent};
use std::time::Duration;

fn main() {
    let mut server = Server::new(5057, 64).unwrap();

    loop {
        server.poll(Duration::from_millis(1));

        while let Some(event) = server.next_event() {
            match event {
                ServerEvent::ClientConnect(id) => {
                    println!("client connected: {}", id);
                }
                ServerEvent::ClientDisconnected(id) => {
                    println!("client disconnected: {}", id);
                }
                ServerEvent::PacketReceived(id, data) => {
                    println!("received {} bytes from {}", data.len(), id);

                    server.send_to(id, b"pong");
                }
            }
        }
    }
}
```

## Client Example

```rust
use easy_net::{Client, ClientEvent};
use std::time::Duration;

fn main() {
    let mut client = Client::new("127.0.0.1", 5057).unwrap();

    loop {
        client.poll(Duration::from_millis(1));

        while let Some(event) = client.next_event() {
            match event {
                ClientEvent::Connected => {
                    println!("connected");
                    client.send(b"ping");
                }
                ClientEvent::Disconnected => {
                    println!("disconnected");
                }
                ClientEvent::PacketReceived(data) => {
                    println!("received {} bytes", data.len());
                }
            }
        }
    }
}
```

## Notes

* `easy-net` currently uses a single reliable ENet channel.
* The API is intentionally small and focused.
* The library assumes exclusive access to client/server instances.
* ENet is initialized globally on first use.

## License

MIT
