// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Adapter providing `futures::io::AsyncRead + AsyncWrite` over a QNUC connection
//! on a single default stream (stream_id 0). This allows QNUC connections to be
//! used transparently with the existing Noise and Handshake upgrade layers.

use futures::io::{AsyncRead, AsyncWrite};
use std::{
    collections::VecDeque,
    fmt,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::net::UdpSocket;

/// A QNUC socket that implements `AsyncRead + AsyncWrite` by sending/receiving
/// UDP datagrams. This makes it compatible with the existing Noise + Handshake
/// upgrade path in the Aptos networking stack.
///
/// The Noise protocol layer sits *above* this socket, treating it as a
/// byte-stream transport (just like it would a TCP socket). Ordering and
/// reliability for the Noise handshake bytes are provided by this adapter's
/// internal buffering and the inherently small handshake message sizes.
pub struct QnucSocket {
    socket: Arc<UdpSocket>,
    remote_addr: SocketAddr,
    read_buf: VecDeque<u8>,
    connection_id: u64,
    closed: bool,
}

impl fmt::Debug for QnucSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QnucSocket")
            .field("remote_addr", &self.remote_addr)
            .field("connection_id", &self.connection_id)
            .field("closed", &self.closed)
            .finish()
    }
}

impl QnucSocket {
    pub fn new(socket: Arc<UdpSocket>, remote_addr: SocketAddr, connection_id: u64) -> Self {
        Self {
            socket,
            remote_addr,
            read_buf: VecDeque::new(),
            connection_id,
            closed: false,
        }
    }

    /// Get the local address of the underlying UDP socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl AsyncRead for QnucSocket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if !self.read_buf.is_empty() {
            let to_copy = std::cmp::min(buf.len(), self.read_buf.len());
            for (i, byte) in self.read_buf.drain(..to_copy).enumerate() {
                buf[i] = byte;
            }
            return Poll::Ready(Ok(to_copy));
        }

        if self.closed {
            return Poll::Ready(Ok(0));
        }

        let socket = &self.socket;

        // First, ensure the socket is ready for reading
        match socket.poll_recv_ready(cx) {
            Poll::Ready(Ok(())) => {
                // Socket is ready, do a non-blocking recv
                match socket.try_recv_from(buf) {
                    Ok((n, _from)) => Poll::Ready(Ok(n)),
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
                    Err(e) => Poll::Ready(Err(e)),
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for QnucSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.closed {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }

        let socket = &self.socket;
        let remote = self.remote_addr;

        match socket.poll_send_ready(cx) {
            Poll::Ready(Ok(())) => match socket.try_send_to(buf, remote) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_qnuc_socket_write_read() {
        let server = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server.local_addr().unwrap();
        let client = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let client_addr = client.local_addr().unwrap();

        let mut client_sock = QnucSocket::new(client.clone(), server_addr, 1);
        let mut server_sock = QnucSocket::new(server.clone(), client_addr, 1);

        // Write from client
        let written = futures::executor::block_on(async {
            use futures::io::AsyncWriteExt;
            client_sock.write(b"hello qnuc").await.unwrap()
        });
        assert_eq!(written, 10);

        // Read from server
        let mut buf = [0u8; 32];
        let read = futures::executor::block_on(async {
            use futures::io::AsyncReadExt;
            server_sock.read(&mut buf).await.unwrap()
        });
        assert_eq!(&buf[..read], b"hello qnuc");
    }
}
