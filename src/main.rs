#![feature(future_readiness_fns)]
#![feature(const_generics)]
#![allow(incomplete_features)]

mod buffer;
mod protocol;
mod cache;
mod system;

use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use crate::system::Result;
use crate::system::register_abort_action;
use tokio::net::UdpSocket;
use tokio::time::Instant;
use std::future::Future;
use std::sync::Arc;
use std::net::SocketAddr;
use std::error::Error;

//dig @127.0.0.1 -p 2053 www.baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    // register_abort_action([
    //     cache::get_abort_action(),
    // ]);
    let r = Arc::new(UdpSocket::bind("0.0.0.0:2053").await?);
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    loop {
        let mut buffer = PacketBuffer::new();
        let (_, src) = r.recv_from(buffer.as_mut_slice()).await?;
        let query = DNSQuery::from(buffer);
        println!("dns query: {:?}", query);
        let s = Arc::clone(&r);
        let n = Arc::clone(&socket);
        tokio::spawn(async move {
            match handle_task(s, n, src, query).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("error occur here {:?}", e)
                }
            }
        });
    }
}

async fn handle_task(s: Arc<UdpSocket>, n: Arc<UdpSocket>, src: SocketAddr, query: DNSQuery) -> Result<()> {
    if let Some(answer) = get_answer(&query) {
        s.send_to(answer.to_u8_vec().as_slice(), src).await?;
    } else {
        let (pass_time, answer) =
            send_and_recv(&n, "114.114.114.114:53", &query).await?;
        println!("114.114.114.114:{}", pass_time);
        println!("dns answer: {:?}", answer);
        s.send_to(answer.to_u8_vec().as_slice(), src).await?;
        store_answer(answer);
    }
    Ok(())
}

async fn send_and_recv(socket: &UdpSocket, address: &str, query: &DNSQuery)
                       -> Result<(u128, DNSAnswer)> {
    let start = Instant::now();
    socket.send_to(query.to_u8_vec().as_slice(), address).await?;
    let mut buffer = PacketBuffer::new();
    socket.recv(buffer.as_mut_slice()).await?;
    let pass_time = start.elapsed().as_micros();
    Ok((pass_time, DNSAnswer::from(buffer)))
}

