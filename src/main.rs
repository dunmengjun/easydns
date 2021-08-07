mod buffer;
mod protocol;
mod cache;
mod system;
mod config;
mod filter;

use crate::buffer::PacketBuffer;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::system::{Result, next_id};
use std::net::SocketAddr;
use futures_util::{FutureExt};
use futures_util::future::select_all;
use crate::config::*;
use tokio::time::{interval, Duration};
use async_trait::async_trait;

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
            interval.tick().await;
            let test_query = DNSQuery::from_domain("www.baidu.com");
            match preferred_dns_server(test_query).await {
                Ok(server) => {
                    set_fast_dns_server(server);
                }
                Err(e) => {
                    eprintln!("interval task upstream servers choose has error: {:?}", e)
                }
            }
        }
    });
    //从客户端接受请求的主循环
    loop {
        let (buffer, src) = recv_query().await?;
        // handle_chain();
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

async fn handle_task(src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
    let mut query_clain = Clain::new();
    query_clain.add(DomainFilter);
    query_clain.add(LegalChecker);
    query_clain.add(CacheHandler);
    query_clain.add(IpChoiceMaker);
    query_clain.add(QuerySender);
    let answer = query_clain.next(&DNSQuery::from(buffer)).await?;
    SERVER_SOCKET.send_to(answer.to_u8_vec().as_slice(), src).await?;
    Ok(())
}


struct Clain {
    funcs: Vec<Box<dyn Handler + Send + Sync>>,
}

impl Clain {
    fn new() -> Self {
        Clain {
            funcs: Vec::new()
        }
    }

    fn add(&mut self, handler: impl Handler + Send + Sync + 'static) {
        self.funcs.push(Box::new(handler));
    }

    async fn next(&mut self, query: &DNSQuery) -> Result<DNSAnswer> {
        self.funcs.remove(0).handle(self, query).await
    }
}

#[async_trait]
trait Handler {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer>;
}

struct LegalChecker;

#[async_trait]
impl Handler for LegalChecker {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        if !query.is_supported() {
            println!("The dns query is not supported , will not mit the cache and pre choose!");
            let answer = send_and_recv(fast_dns_server(), query).await?;
            println!("dns answer: {:?}", answer);
            return Ok(answer);
        } else {
            clain.next(query).await
        }
    }
}

struct CacheHandler;

#[async_trait]
impl Handler for CacheHandler {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        if let Some(answer) = cache::get_answer(query) {
            return Ok(answer);
        } else {
            let result = clain.next(query).await?;
            cache::store_answer(&result);
            Ok(result)
        }
    }
}

struct QuerySender;

#[async_trait]
impl Handler for QuerySender {
    async fn handle(&self, _: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        send_and_recv(fast_dns_server(), query).await
    }
}

struct IpChoiceMaker;

#[async_trait]
impl Handler for IpChoiceMaker {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        let mut answer = clain.next(query).await?;
        preferred_with_ping(&mut answer).await?;
        Ok(answer)
    }
}

struct DomainFilter;

#[async_trait]
impl Handler for DomainFilter {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        let domain = query.get_readable_domain();
        if filter::contain(domain) {
            //返回soa
            return Ok(DNSAnswer::from_query_with_soa(query));
        }
        clain.next(query).await
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

async fn preferred_dns_server(query: DNSQuery) -> Result<&'static str> {
    let mut future_vec =
        Vec::with_capacity(UPSTREAM_DNS_SERVERS.len());
    for address in UPSTREAM_DNS_SERVERS.iter() {
        future_vec.push(send_and_recv(address, &query).boxed());
    }
    let (result, index, _) = select_all(future_vec).await;
    let _answer = result?;
    Ok(UPSTREAM_DNS_SERVERS[index])
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

