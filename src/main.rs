use axum::{
    Router,
    extract::{
        Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(60 * 30);

#[derive(Debug, Deserialize)]
struct TunnelQuery {
    host: String,
    port: u16,
    secret: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let secret = std::env::var("TUNNEL_SECRET").expect("TUNNEL_SECRET must be set");
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:5152".to_string());

    let app = Router::new()
        .route("/tunnel", get(tunnel_handler))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(secret);

    let addr: SocketAddr = bind_addr.parse().expect("invalid BIND_ADDR");
    println!("[INFO] tunnel node starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn tunnel_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<TunnelQuery>,
    axum::extract::State(expected_secret): axum::extract::State<String>,
) -> impl IntoResponse {
    if query.secret != expected_secret {
        return (StatusCode::UNAUTHORIZED, "invalid secret").into_response();
    }

    ws.on_upgrade(move |socket| handle_tunnel(socket, query))
}

async fn handle_tunnel(socket: WebSocket, query: TunnelQuery) {
    println!(
        "[INFO] establishing tunnel to {}:{}",
        query.host, query.port
    );

    let Ok(Ok(stream)) = timeout(
        CONNECT_TIMEOUT,
        TcpStream::connect((query.host.as_str(), query.port)),
    )
    .await
    else {
        println!("[WARN] tcp connect failed to {}:{}", query.host, query.port);
        return;
    };

    let (mut tcp_read, mut tcp_write) = stream.into_split();
    let (mut ws_write, mut ws_read) = socket.split();

    let tcp_to_ws = tokio::spawn(async move {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match timeout(IO_TIMEOUT, tcp_read.read(&mut buf)).await {
                Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break,
                Ok(Ok(n)) => {
                    if ws_write
                        .send(Message::Binary(buf[..n].to_vec().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    let ws_to_tcp = tokio::spawn(async move {
        loop {
            match timeout(IO_TIMEOUT, ws_read.next()).await {
                Ok(Some(Ok(Message::Binary(data)))) => {
                    if tcp_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(_))) | Err(_) => break,
            }
        }
    });

    tokio::select! {
        _ = tcp_to_ws => {},
        _ = ws_to_tcp => {},
    }

    println!("[INFO] tunnel closed for {}:{}", query.host, query.port);
}
