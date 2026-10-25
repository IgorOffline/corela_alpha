use std::net::SocketAddr;

use futures_util::StreamExt;
use sockudo_ws::{Config, Message, Role, WebSocketStream, handshake};
use tokio::net::{TcpListener, TcpStream};

const DEFAULT_ADDR: &str = "127.0.0.1:9001";

enum Event<'a> {
    Hello { name: &'a str, version: &'a str },
    Select { q: i32, r: i32 },
}

fn parse(text: &str) -> Option<Event<'_>> {
    let mut words = text.split_whitespace();
    let event = match words.next()? {
        "hello" => Event::Hello {
            name: words.next()?,
            version: words.next()?,
        },
        "select" => Event::Select {
            q: words.next()?.parse().ok()?,
            r: words.next()?.parse().ok()?,
        },
        _ => return None,
    };
    words.next().is_none().then_some(event)
}

async fn handle(mut stream: TcpStream, peer: SocketAddr) -> sockudo_ws::Result<()> {
    let hs = handshake::server_handshake(&mut stream).await?;
    // The client is a fire-and-forget no_std sender that never reads, so it
    // cannot answer pings; liveness is left to TCP.
    let config = Config::builder().auto_ping(false).idle_timeout(0).build();
    let mut ws = WebSocketStream::from_raw_with_leftover(stream, Role::Server, config, hs.leftover);
    println!("server: {peer} connected ({})", hs.path);

    while let Some(msg) = ws.next().await {
        match msg? {
            Message::Text(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                match parse(&text) {
                    Some(Event::Hello { name, version }) => {
                        println!("server: {peer} hello from {name} {version}")
                    }
                    Some(Event::Select { q, r }) => println!("server: {peer} selected ({q}, {r})"),
                    None => println!("server: {peer} unknown message {text:?}"),
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    println!("server: {peer} disconnected");
    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_owned());
    let listener = TcpListener::bind(&addr).await?;
    println!("server: listening on ws://{addr}");
    loop {
        let (stream, peer) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle(stream, peer).await {
                eprintln!("server: {peer} error: {e}");
            }
        });
    }
}
