mod buffer;
mod protocol;
mod cache;
mod system;
mod config;

use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::cache::{get_answer, store_answer};
use crate::system::{Result, next_id};
use std::net::SocketAddr;
use futures_util::{FutureExt};
use futures_util::future::select_all;
use crate::config::*;
use tokio::time::{interval, Duration};

//dig @127.0.0.1 -p 2053 www.baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    //创建任务去监听ctrl_c event
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl_c event");
        cache::run_abort_action().await.unwrap();
        std::process::exit(0);
    });
    //创建任务去recv从上游dns服务器返回的answer
    tokio::spawn(async move {
        loop {
            match recv_and_handle_answer().await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("error occur here accept {:?}", e)
                }
            }
        }
    });
    //创建定时任务去定时的优选上游dns servers,半天触发一次
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(43200));
        loop {
            let test_query = DNSQuery::from_domain("www.baidu.com");
            match preferred_dns_server(test_query).await {
                Ok(server) => {
                    set_fast_dns_server(server);
                }
                Err(e) => {
                    eprintln!("interval task upstream servers choose has error: {:?}", e)
                }
            }
            interval.tick().await;
        }
    });
    //从客户端接受请求的主循环
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

async fn recv_and_handle_answer() -> Result<()> {
    let mut buffer = PacketBuffer::new();
    UPSTREAM_SOCKET.recv_from(buffer.as_mut_slice()).await?;
    let answer = DNSAnswer::from(buffer);
    match ANSWER_REG_TABLE.remove(answer.get_id()) {
        None => {}
        Some((_, sender)) => {
            if let Err(e) = sender.send(answer) {
                ANSWER_REG_TABLE.remove(e.get_id());
            }
        }
    }
    Ok(())
}

async fn handle_task(src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
    let query = DNSQuery::from(buffer);
    println!("dns query: {:?}", query);
    if let Some(answer) = get_answer(&query) {
        //在缓存里则直接send出去
        SERVER_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
    } else {
        let mut answer = send_and_recv(fast_dns_server(), &query).await?;
        println!("dns answer: {:?}", answer);
        //优选ip, 默认是ping协议
        preferred_with_ping(&mut answer).await?;
        SERVER_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
        store_answer(answer);
    }
    Ok(())
}

async fn preferred_dns_server(query: DNSQuery) -> Result<&'static str> {
    let mut future_vec =
        Vec::with_capacity(UPSTREAM_DNS_SERVERS.len());
    for address in UPSTREAM_DNS_SERVERS.iter() {
        future_vec.push(send_and_recv(address, &query).boxed());
    }
    Ok(UPSTREAM_DNS_SERVERS[select_all(future_vec).await.1])
}

async fn preferred_with_ping(answer: &mut DNSAnswer) -> Result<()> {
    let ip_vec = answer.get_ip_vec();
    if ip_vec.len() == 1 {
        answer.retain_ip(ip_vec[0]);
        return Ok(());
    }
    let mut ping_future_vec = Vec::new();
    for ip in &ip_vec {
        let future = PING_SOCKET.chain(ip.clone()).send();
        ping_future_vec.push(future);
    }
    let index = select_all(ping_future_vec).await.1;
    answer.retain_ip(ip_vec[index]);
    Ok(())
}

async fn send_and_recv(address: &str, query: &DNSQuery) -> Result<DNSAnswer> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let next_id = next_id();
    ANSWER_REG_TABLE.insert(next_id, sender);
    UPSTREAM_SOCKET.send_to(query.to_u8_with_id(next_id).as_slice(), address).await?;
    let mut answer = receiver.await?;
    ANSWER_REG_TABLE.remove(answer.get_id());
    answer.set_id(query.get_id().clone());
    Ok(answer)
}

