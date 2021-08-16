use tokio::net::UdpSocket;
use std::net::SocketAddr;
use crate::system::Result;
use crate::protocol::DNSAnswer;
use crate::cursor::{Cursor};

pub struct ClientSocket {
    socket: UdpSocket,
}

impl ClientSocket {
    pub async fn new(port: u16) -> Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
        Ok(ClientSocket {
            socket
        })
    }
    pub async fn recv(&self) -> Result<(Cursor<u8>, SocketAddr)> {
        let mut buf = [0u8; 256];
        let (_, src) = self.socket
            .recv_from(&mut buf)
            .await?;
        Ok((Cursor::form(buf.into()), src))
    }

    pub async fn back_to(&self, client: SocketAddr, answer: DNSAnswer) -> Result<()> {
        self.socket.send_to(answer.to_u8_vec().as_slice(), client).await?;
        Ok(())
    }
}