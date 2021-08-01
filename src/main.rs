mod buffer;
mod protocol;
mod cache;
mod scheduler;
mod timer;
mod socket;

use std::net::{UdpSocket, SocketAddr};
use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use std::sync::Arc;
use crate::scheduler::{TaskScheduler, Task};
use crate::socket::UdpSocketPool;

#[macro_use]
extern crate lazy_static;

struct DnsQueryTask {
    o_socket: Arc<UdpSocket>,
    n_socket: Arc<UdpSocket>,
    src: SocketAddr,
    query: DNSQuery,
}

impl Task for DnsQueryTask {
    fn run(&self) {
        if let Some(answer) = get_answer(&self.query) {
            self.o_socket.send_to(answer.to_u8_vec().as_slice(), self.src).unwrap();
        } else {
            self.n_socket.send_to(self.query.to_u8_vec().as_slice(),
                                  ("114.114.114.114", 53)).unwrap();
            let mut buffer = PacketBuffer::new();
            self.n_socket.recv(buffer.as_mut_slice()).unwrap();
            let answer = DNSAnswer::from(buffer);
            println!("dns answer: {:?}", answer);
            self.o_socket.send_to(answer.to_u8_vec().as_slice(), self.src).unwrap();
            store_answer(answer);
        }
    }
}

impl DnsQueryTask {
    fn from(o_socket: Arc<UdpSocket>, n_socket: Arc<UdpSocket>, src: SocketAddr, query: DNSQuery) -> Self {
        DnsQueryTask {
            o_socket,
            n_socket,
            src,
            query,
        }
    }
}

//dig @127.0.0.1 -p 2053 www.baidu.com
fn main() {
    let arc_socket = Arc::new(UdpSocket::bind(("0.0.0.0", 2053)).unwrap());
    let socket = arc_socket.clone();
    let mut scheduler = TaskScheduler::from(4);
    let socket_pool = UdpSocketPool::new();
    loop {
        let mut buffer = PacketBuffer::new();
        let (_, src) = socket.recv_from(buffer.as_mut_slice()).unwrap();
        let query = DNSQuery::from(buffer);
        println!("dns query: {:?}", query);
        scheduler.publish(DnsQueryTask::from(
            arc_socket.clone(), socket_pool.get_socket(), src, query))
    }
}

