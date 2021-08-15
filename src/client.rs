use tokio::net::UdpSocket;
use crate::buffer::PacketBuffer;
use std::net::SocketAddr;
use crate::system::Result;
use crate::protocol::DNSAnswer;

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
    pub async fn recv(&self) -> Result<(PacketBuffer, SocketAddr)> {
        let mut buffer = PacketBuffer::new();
        let (_, src) = self.socket
            .recv_from(buffer.as_mut_slice())
            .await?;
        Ok((buffer, src))
    }

    pub async fn back_to(&self, client: SocketAddr, answer: DNSAnswer) -> Result<()> {
        self.socket.send_to(answer.to_u8_vec().as_slice(), client).await?;
        Ok(())
    }
}