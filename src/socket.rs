use std::sync::Arc;
use std::net::UdpSocket;
use rand::{Rng};

pub struct UdpSocketPool {
    sockets: Vec<Arc<UdpSocket>>,
}

impl UdpSocketPool {
    pub fn new() -> Self {
        let mut sockets = Vec::new();
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 11345)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 11346)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 11347)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 11348)).unwrap()));
        UdpSocketPool {
            sockets
        }
    }

    pub fn get_socket(&self) -> Arc<UdpSocket> {
        self.sockets[rand::thread_rng().gen_range(0..4)].clone()
    }
}