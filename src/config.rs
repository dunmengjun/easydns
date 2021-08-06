use once_cell::sync::{Lazy};
use tokio::net::UdpSocket;
use tokio::task::block_in_place;
use tokio::runtime::Handle;
use tokio_icmp::Pinger;
use dashmap::DashMap;
use tokio::sync::oneshot::Sender;
use crate::protocol::DNSAnswer;
use std::sync::Mutex;

pub static UPSTREAM_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:0").await.unwrap()
        })
    })
});

pub static SERVER_SOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            UdpSocket::bind("0.0.0.0:2053").await.unwrap()
        })
    })
});

pub static PING_SOCKET: Lazy<Pinger> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            tokio_icmp::Pinger::new().await.unwrap()
        })
    })
});

pub static ANSWER_REG_TABLE: Lazy<DashMap<u16, Sender<DNSAnswer>>> = Lazy::new(|| {
    DashMap::new()
});

pub static UPSTREAM_DNS_SERVERS: Lazy<Vec<&str>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(3);
    vec.push("114.114.114.114:53");
    vec.push("8.8.8.8:53");
    vec.push("1.1.1.1:53");
    vec
});

pub static FAST_DNS_SERVER: Lazy<Mutex<&str>> = Lazy::new(|| {
    Mutex::new(UPSTREAM_DNS_SERVERS[0])
});

pub fn cache_on() -> bool {
    static CACHE_ON: bool = false;
    CACHE_ON
}

pub fn fast_dns_server() -> &'static str {
    FAST_DNS_SERVER.lock().unwrap().clone()
}

pub fn set_fast_dns_server(server: &'static str) {
    *FAST_DNS_SERVER.lock().unwrap() = server;
}