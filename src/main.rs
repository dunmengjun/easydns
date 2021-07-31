mod buffer;
mod protocol;

use std::net::{UdpSocket};
use crate::buffer::PacketBuffer;
use std::collections::HashMap;
use crate::protocol::{DNSQuery, DNSAnswer};

//dig @127.0.0.1 -p 2053 www.baidu.com
fn main() {
    let socket = UdpSocket::bind(("0.0.0.0", 2053)).unwrap();
    let mut query_map = HashMap::new();
    loop {
        let mut buffer = PacketBuffer::new();
        let (_, src) = socket.recv_from(buffer.as_mut_slice()).unwrap();
        if src.ip().to_string() == "114.114.114.114" {
            let answer = DNSAnswer::from(buffer);
            println!("dns answer: {:?}", answer);
            if let Some(address) = query_map.get(answer.get_id()) {
                //不管有没有send成功，都从map里删除ip代表这个请求处理成功
                socket.send_to(answer.to_u8_vec().as_slice(), address).unwrap_or_else(|_e| 0);
                query_map.remove(answer.get_id());
            }
        } else {
            let query = DNSQuery::from(buffer);
            println!("dns query: {:?}", query);
            if let Ok(_) = socket.send_to(query.to_u8_vec().as_slice(), ("114.114.114.114", 53)) {
                query_map.insert(query.get_id().clone(), src);
            }
        }
    }
}
