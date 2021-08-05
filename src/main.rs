mod buffer;
mod protocol;
mod cache;
mod system;

use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use crate::system::Result;
use tokio::net::UdpSocket;
use tokio::time::Instant;
use std::net::SocketAddr;
use once_cell::sync::{Lazy};
use dashmap::DashMap;
use tokio::task::yield_now;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

static UPSTREAM_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:0").await.unwrap()
        })
    })
});

static ACCEPT_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:2053").await.unwrap()
        })
    })
});

static ACCEPT_MAP: Lazy<DashMap<String, DNSAnswer>> = Lazy::new(|| {
    DashMap::new()
});

//dig @127.0.0.1 -p 2053 www.baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl_c event");
        cache::run_abort_action().await.unwrap();
        std::process::exit(0);
    });
    tokio::spawn(async move {
        loop {
            match recv_answer().await {
                Ok((answer, src)) => {
                    ACCEPT_MAP.insert(
                        gen_key(src.to_string(), answer.get_id(), answer.get_domain()),
                        answer);
                }
                Err(e) => {
                    eprintln!("error occur here accept {:?}", e)
                }
            }
        }
    });
    loop {
        let (query, src) = recv_query().await?;
        println!("dns query: {:?}", query);
        tokio::spawn(async move {
            match handle_task(src, query).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("error occur here main{:?}", e)
                }
            }
        });
    }
}

fn gen_key(mut src: String, id: u16, domain: &Vec<u8>) -> String {
    src.push('|');
    let result = String::from_utf8(domain.clone()).unwrap();
    src.push_str(&result);
    src.push('|');
    let x = id.to_be_bytes();
    src.push(x[0] as char);
    src.push(x[1] as char);
    src
}

async fn recv_query() -> Result<(DNSQuery, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = ACCEPT_SOCKET.recv_from(buffer.as_mut_slice()).await?;
    Ok((DNSQuery::from(buffer), src))
}

async fn recv_answer() -> Result<(DNSAnswer, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = UPSTREAM_SOCKET.recv_from(buffer.as_mut_slice()).await?;
    Ok((DNSAnswer::from(buffer), src))
}

async fn handle_task(src: SocketAddr, query: DNSQuery) -> Result<()> {
    if let Some(answer) = get_answer(&query) {
        ACCEPT_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
    } else {
        println!("map: {:?}", ACCEPT_MAP);
        let r: (u128, DNSAnswer) = tokio::select! {
            r1 = send_and_recv("8.8.8.8:53", &query) => { r1? },
            r2 = send_and_recv("114.114.114.114:53", &query) => { r2? },
        };
        println!("dns answer time:{}", r.0);
        println!("dns answer: {:?}", r.1);
        ACCEPT_SOCKET.send_to(r.1.to_u8_vec().as_slice(), src).await?;
        store_answer(r.1);
    }
    Ok(())
}

async fn send_and_recv(address: &str, query: &DNSQuery) -> Result<(u128, DNSAnswer)> {
    let start = Instant::now();
    UPSTREAM_SOCKET.send_to(query.to_u8_vec().as_slice(), address).await?;
    let key = gen_key(String::from(address), query.get_id().clone(), query.get_domain());
    loop {
        match ACCEPT_MAP.remove(&key) {
            None => { yield_now().await; }
            Some(answer) => {
                let pass_time = start.elapsed().as_micros();
                return Ok((pass_time, answer.1));
            }
        }
    }
}

