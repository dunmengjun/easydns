use std::sync::Arc;
use std::net::UdpSocket;

pub struct UdpSocketPool {
    sockets: Vec<Arc<UdpSocket>>,
    index: usize,
}

impl UdpSocketPool {
    pub fn new() -> Self {
        let mut sockets = Vec::new();
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap()));
        sockets.push(Arc::new(UdpSocket::bind(("0.0.0.0", 0)).unwrap()));
        UdpSocketPool {
            sockets,
            index: 0,
        }
    }

    pub fn take_socket(&mut self) -> Arc<UdpSocket> {
        let i = self.take_index();
        self.sockets[i].clone()
    }

    fn take_index(&mut self) -> usize {
        let result = self.index % self.sockets.len();
        self.index += 1;
        result
    }
}