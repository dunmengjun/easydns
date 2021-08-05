mod buffer;
mod protocol;
mod cache;
mod system;

use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use crate::system::{Result, next_id};
use tokio::net::UdpSocket;
use tokio::time::Instant;
use std::net::SocketAddr;
use once_cell::sync::{Lazy};
use dashmap::DashMap;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tokio::sync::oneshot::Sender;

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

static ACCEPT_MAP: Lazy<DashMap<u16, Sender<DNSAnswer>>> = Lazy::new(|| {
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
                Ok((answer, _)) => {
                    match ACCEPT_MAP.remove(answer.get_id()) {
                        None => {}
                        Some((_, sender)) => {
                            if let Err(e) = sender.send(answer) {
                                ACCEPT_MAP.remove(e.get_id());
                            }
                        }
                    }
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

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let next_id = next_id();
    ACCEPT_MAP.insert(next_id, sender);
    UPSTREAM_SOCKET.send_to(query.to_u8_with_id(next_id).as_slice(), address).await?;
    let mut answer = receiver.await?;
    ACCEPT_MAP.remove(answer.get_id());
    answer.set_id(query.get_id().clone());

    let pass_time = start.elapsed().as_micros();
    Ok((pass_time, answer))
}

