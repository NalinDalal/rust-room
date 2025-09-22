use quinn::{ClientConfig, Endpoint};
use rustls::{Certificate, RootCertStore, ClientConfig as RustlsClientConfig};
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt};

#[tokio::main]
async fn main() {
    // Load server cert for client trust
    let cert = fs::read("cert.der").expect("read cert");
    let mut roots = RootCertStore::empty();
    roots.add(&Certificate(cert)).unwrap();
    let client_crypto = RustlsClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    
    let quinn_config = ClientConfig::new(Arc::new(client_crypto));

    // Connect to server
    let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
    let quinn_conn = endpoint.connect(server_addr, "localhost").unwrap().await.unwrap();
    println!("Connected to server");

    // Open a bi-directional stream for WebSocket
    let (send, recv) = quinn_conn.open_bi().await.unwrap();
    // Use the same QuicBiStream as the server
    struct QuicBiStream {
        send: quinn::SendStream,
        recv: quinn::RecvStream,
    }
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use std::pin::Pin;
    use std::task::{Context, Poll};
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
    let quic_stream = QuicBiStream { send, recv };
    let ws_stream = WebSocketStream::from_raw_socket(
        quic_stream,
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None
    ).await;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // Spawn a task to read messages from server
    tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            if let Message::Text(text) = msg {
                println!("[room] {text}");
            }
        }
    });

    // Send messages from stdin
    let mut stdin = tokio::io::stdin();
    let mut buf = [0u8; 1024];
    loop {
        let n = stdin.read(&mut buf).await.unwrap();
        if n == 0 { break; }
        let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        if msg == "/quit" { break; }
        ws_write.send(Message::Text(msg)).await.unwrap();
    }
    println!("Exiting client");
}
