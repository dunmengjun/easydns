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
use futures_util::FutureExt;
use futures_util::future::select_all;

static UPSTREAM_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:0").await.unwrap()
        })
    })
});

static SERVER_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:2053").await.unwrap()
        })
    })
});

static ANSWER_REG_TABLE: Lazy<DashMap<u16, Sender<DNSAnswer>>> = Lazy::new(|| {
    DashMap::new()
});

static UPSTREAM_DNS_SERVERS: Lazy<Vec<&str>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(3);
    vec.push("114.114.114.114:53");
    vec.push("8.8.8.8:53");
    vec.push("1.1.1.1:53");
    vec
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
                    match ANSWER_REG_TABLE.remove(answer.get_id()) {
                        None => {}
                        Some((_, sender)) => {
                            if let Err(e) = sender.send(answer) {
                                ANSWER_REG_TABLE.remove(e.get_id());
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
        let (buffer, src) = recv_query().await?;
        tokio::spawn(async move {
            match handle_task(src, buffer).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("error occur here main{:?}", e)
                }
            }
        });
    }
}

async fn recv_query() -> Result<(PacketBuffer, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = SERVER_SOCKET.recv_from(buffer.as_mut_slice()).await?;
    Ok((buffer, src))
}

async fn recv_answer() -> Result<(DNSAnswer, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = UPSTREAM_SOCKET.recv_from(buffer.as_mut_slice()).await?;
    Ok((DNSAnswer::from(buffer), src))
}

async fn handle_task(src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
    let query = DNSQuery::from(buffer);
    println!("dns query: {:?}", query);
    if let Some(answer) = get_answer(&query) {
        SERVER_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
    } else {
        //对上游dns服务器的优选
        let mut future_vec =
            Vec::with_capacity(UPSTREAM_DNS_SERVERS.len());
        for address in UPSTREAM_DNS_SERVERS.iter() {
            future_vec.push(send_and_recv(address, &query).boxed());
        }
        let (pass_time, answer) = select_all(future_vec).await.0?;
        println!("dns answer time:{}", pass_time);
        println!("dns answer: {:?}", answer);
        SERVER_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
        store_answer(answer);
    }
    Ok(())
}

async fn send_and_recv(address: &str, query: &DNSQuery) -> Result<(u128, DNSAnswer)> {
    let start = Instant::now();

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let next_id = next_id();
    ANSWER_REG_TABLE.insert(next_id, sender);
    UPSTREAM_SOCKET.send_to(query.to_u8_with_id(next_id).as_slice(), address).await?;
    let mut answer = receiver.await?;
    ANSWER_REG_TABLE.remove(answer.get_id());
    answer.set_id(query.get_id().clone());

    let pass_time = start.elapsed().as_micros();
    Ok((pass_time, answer))
}

