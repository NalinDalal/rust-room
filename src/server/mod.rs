use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use quinn::{Endpoint, ServerConfig, TransportConfig};
use rcgen;
use rustls::{Certificate, PrivateKey};
use tokio::sync::Mutex;
use quinn::{SendStream, RecvStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
// Wrapper to combine SendStream and RecvStream for WebSocketStream
pub struct QuicBiStream {
    send: SendStream,
    recv: RecvStream,
}

impl QuicBiStream {
    pub fn new(send: SendStream, recv: RecvStream) -> Self {
        Self { send, recv }
    }
}

impl AsyncRead for QuicBiStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicBiStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.send).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}
use tokio::task;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::room::{Client, ClientId, RoomManager};

fn load_or_generate_certs() -> (Certificate, PrivateKey) {
    let cert_path = Path::new("cert.der");
    let key_path = Path::new("key.der");
    if cert_path.exists() && key_path.exists() {
        let cert = fs::read(cert_path).expect("read cert");
        let key = fs::read(key_path).expect("read key");
        (
            Certificate(cert),
            PrivateKey(key),
        )
    } else {
        // Generate self-signed certs for demo
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_der = cert.serialize_der().unwrap();
        let key_der = cert.serialize_private_key_der();
        fs::write(cert_path, &cert_der).unwrap();
        fs::write(key_path, &key_der).unwrap();
        (
            Certificate(cert_der),
            PrivateKey(key_der),
        )
    }
}

pub async fn run_server() {
    let (cert, key) = load_or_generate_certs();
    let mut server_config = ServerConfig::with_single_cert(vec![cert], key).expect("bad certs");
    let mut transport_config = TransportConfig::default();
    transport_config.max_idle_timeout(Some(Duration::from_secs(60).try_into().unwrap()));
    server_config.transport = Arc::new(transport_config);

    let addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();
    let endpoint = Endpoint::server(server_config, addr).expect("failed to bind endpoint");
    println!("QUIC server listening on {}", addr);

    let room_manager = Arc::new(Mutex::new(RoomManager::new()));
    let mut client_counter: ClientId = 0;

    while let Some(connecting) = endpoint.accept().await {
        let room_manager = room_manager.clone();
        client_counter += 1;
        let client_id = client_counter;
        task::spawn(async move {
            match connecting.await {
                Ok(new_conn) => {
                    println!("New QUIC connection: {}", client_id);
                    if let Ok((send, recv)) = new_conn.accept_bi().await {
                        let quic_stream = QuicBiStream::new(send, recv);
                        let ws_stream = WebSocketStream::from_raw_socket(
                            quic_stream,
                            tokio_tungstenite::tungstenite::protocol::Role::Server,
                            None
                        ).await;
                        handle_ws_client(ws_stream, room_manager, client_id).await;
                    }
                }
                Err(e) => eprintln!("Connection failed: {e}"),
            }
        });
    }
}

async fn handle_ws_client<S>(ws_stream: WebSocketStream<S>, room_manager: Arc<Mutex<RoomManager>>, client_id: ClientId)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let room_id = "main".to_string();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    {
        let mut mgr = room_manager.lock().await;
        mgr.join_room(&room_id, Client { id: client_id, sender: tx });
    }
    println!("Client {client_id} joined room {room_id}");

    let (mut ws_write, mut ws_read) = ws_stream.split();
    let send_task = task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_write.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_read.next().await {
        if let Message::Text(text) = msg {
            let mgr = room_manager.lock().await;
            mgr.broadcast(&room_id, &format!("[{}] {}", client_id, text));
        }
    }
    {
        let mut mgr = room_manager.lock().await;
        mgr.leave_room(&room_id, client_id);
    }
    println!("Client {client_id} left room {room_id}");
    let _ = send_task.await;
}
