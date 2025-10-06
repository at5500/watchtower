//! Simple WebSocket echo server for testing Watchtower WebSocket transport

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("websocket_server=info")
        .init();

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;

    info!("WebSocket server listening on: {}", addr);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        info!("New connection from: {}", peer_addr);
        tokio::spawn(handle_connection(stream, peer_addr));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, peer_addr: SocketAddr) {
    match tokio_tungstenite::accept_async(stream).await {
        Ok(ws_stream) => {
            info!("WebSocket connection established with: {}", peer_addr);

            if let Err(e) = handle_websocket(ws_stream, peer_addr).await {
                error!("Error handling WebSocket: {} ({})", e, peer_addr);
            }
        }
        Err(e) => {
            error!("WebSocket handshake failed: {} ({})", e, peer_addr);
        }
    }
}

async fn handle_websocket(
    ws_stream: WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut write, mut read) = ws_stream.split();

    // Echo messages back and broadcast to all (simple implementation)
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(msg) => match msg {
                Message::Text(text) => {
                    info!("Received text from {}: {}", peer_addr, text);

                    // Echo back
                    if let Err(e) = write.send(Message::Text(text.clone())).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
                Message::Binary(data) => {
                    info!("Received binary from {}: {} bytes", peer_addr, data.len());

                    // Echo back
                    if let Err(e) = write.send(Message::Binary(data.clone())).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
                Message::Ping(payload) => {
                    info!("Received ping from {}", peer_addr);
                    if let Err(e) = write.send(Message::Pong(payload)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Message::Pong(_) => {
                    info!("Received pong from {}", peer_addr);
                }
                Message::Close(frame) => {
                    info!("Received close from {}: {:?}", peer_addr, frame);
                    break;
                }
                Message::Frame(_) => {
                    warn!("Received raw frame from {} (ignoring)", peer_addr);
                }
            },
            Err(e) => {
                error!("Error reading message from {}: {}", peer_addr, e);
                break;
            }
        }
    }

    info!("Connection closed: {}", peer_addr);
    Ok(())
}
