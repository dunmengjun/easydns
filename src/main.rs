mod buffer;
mod protocol;
mod cache;
mod scheduler;
mod timer;

use std::net::{UdpSocket, SocketAddr};
use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use crossbeam_channel::bounded;
use std::thread;
use std::sync::Arc;

#[macro_use]
extern crate lazy_static;

//dig @127.0.0.1 -p 2053 www.baidu.com
fn main() {
    let (s, r) =
        bounded::<(Arc<UdpSocket>, SocketAddr, DNSQuery)>(1000);
    let mut base_port = 11153;
    for _ in 0..4 {
        let receiver = r.clone();
        thread::spawn(move || {
            let socket = UdpSocket::bind(("0.0.0.0", base_port)).unwrap();
            loop {
                let (client, src, query) = receiver.recv().unwrap();
                if let Some(answer) = get_answer(&query) {
                    client.send_to(answer.to_u8_vec().as_slice(), src).unwrap();
                } else {
                    socket.send_to(query.to_u8_vec().as_slice(), ("114.114.114.114", 53)).unwrap();

                    let mut buffer = PacketBuffer::new();
                    socket.recv(buffer.as_mut_slice()).unwrap();

                    let answer = DNSAnswer::from(buffer);
                    println!("dns answer: {:?}", answer);
                    client.send_to(answer.to_u8_vec().as_slice(), src).unwrap();
                    store_answer(answer);
                }
            }
        });
        base_port += 1;
    }

    let arc_socket = Arc::new(UdpSocket::bind(("0.0.0.0", 2053)).unwrap());
    let socket = arc_socket.clone();
    loop {
        let mut buffer = PacketBuffer::new();
        let (_, src) = socket.recv_from(buffer.as_mut_slice()).unwrap();
        let query = DNSQuery::from(buffer);
        println!("dns query: {:?}", query);
        s.send((arc_socket.clone(), src, query)).unwrap();
    }
}
