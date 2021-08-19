use tokio::net::UdpSocket;
use std::net::SocketAddr;
use crate::system::{Result, QueryBuf, default_value};
use crate::protocol::DnsAnswer;

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
    pub async fn recv(&self) -> Result<(QueryBuf, SocketAddr)> {
        let mut buf: QueryBuf = default_value();
        let (_, src) = self.socket
            .recv_from(&mut buf)
            .await?;
        Ok((buf, src))
    }

    pub async fn back_to(&self, client: SocketAddr, answer: DnsAnswer) -> Result<()> {
        self.socket.send_to(answer.to_bytes().as_slice(), client).await?;
        Ok(())
    }
}