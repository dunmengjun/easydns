use tokio::net::UdpSocket;
use tokio_icmp::Pinger;
use dashmap::DashMap;
use tokio::sync::oneshot::Sender;
use crate::protocol::{DNSAnswer, DNSQuery};
use std::sync::Mutex;
use crate::system::{Result, next_id};
use tokio::sync::{OnceCell};
use tokio::time::Duration;
use futures_util::FutureExt;
use crate::buffer::PacketBuffer;
use std::net::SocketAddr;
use crate::{filter, cache};
use async_trait::async_trait;
use futures_util::future::select_all;
use tokio::time::interval;
use crate::config::Config;

struct HandlerContext {
    upstream_socket: UdpSocket,
    server_socket: UdpSocket,
    pinger: Pinger,
    answer_reg_table: DashMap<u16, Sender<DNSAnswer>>,
    upstream_dns_servers: Vec<&'static str>,
    fast_dns_server: Mutex<&'static str>,
}

impl HandlerContext {
    async fn from(config: &Config) -> Result<Self> {
        let server_socket = UdpSocket::bind(("0.0.0.0", config.port as u16)).await?;
        let upstream_socket = UdpSocket::bind("0.0.0.0:0").await?;
        let pinger = tokio_icmp::Pinger::new().await?;
        let answer_reg_table = DashMap::new();
        let upstream_dns_servers = config.servers.clone();
        let fast_dns_server = Mutex::new(upstream_dns_servers[0]);
        Ok(HandlerContext {
            upstream_socket,
            server_socket,
            pinger,
            answer_reg_table,
            upstream_dns_servers,
            fast_dns_server,
        })
    }
}

static HANDLER_CONTEXT: OnceCell<HandlerContext> = OnceCell::const_new();

pub async fn init_context(config: &Config) -> Result<()> {
    let config = HandlerContext::from(config).await?;
    match HANDLER_CONTEXT.set(config) {
        Ok(_) => {}
        Err(e) => {
            panic!("{}", e);
        }
    }
    Ok(())
}

fn upstream_socket() -> &'static UdpSocket {
    &HANDLER_CONTEXT.get().unwrap().upstream_socket
}

fn server_socket() -> &'static UdpSocket {
    &HANDLER_CONTEXT.get().unwrap().server_socket
}

fn ping() -> &'static Pinger {
    &HANDLER_CONTEXT.get().unwrap().pinger
}

fn reg_table() -> &'static DashMap<u16, Sender<DNSAnswer>> {
    &HANDLER_CONTEXT.get().unwrap().answer_reg_table
}

fn servers() -> &'static Vec<&'static str> {
    &HANDLER_CONTEXT.get().unwrap().upstream_dns_servers
}

fn fast_server() -> &'static str {
    &HANDLER_CONTEXT.get().unwrap().fast_dns_server.lock().unwrap()
}

fn set_fast_server(server: &'static str) {
    *HANDLER_CONTEXT.get().unwrap().fast_dns_server.lock().unwrap() = server;
}


pub fn setup_answer_accept_task() {
    //创建任务去recv从上游dns服务器返回的answer
    tokio::spawn(async move {
        loop {
            match recv_and_handle_answer().await {
                Ok(()) => {}
                Err(e) => {
                    error!("error occur here accept {:?}", e)
                }
            }
        }
    });
}


pub fn setup_choose_fast_server_task() {
    //创建定时任务去定时的优选上游dns servers,半天触发一次
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(43200));
        loop {
            interval.tick().await;
            let test_query = DNSQuery::from_domain("www.baidu.com");
            match preferred_dns_server(test_query).await {
                Ok(server) => {
                    set_fast_server(server);
                }
                Err(e) => {
                    error!("interval task upstream servers choose has error: {:?}", e)
                }
            }
        }
    });
}

pub async fn handle_task(src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
    let mut query_clain = Clain::new();
    query_clain.add(DomainFilter);
    query_clain.add(LegalChecker);
    query_clain.add(CacheHandler);
    query_clain.add(IpChoiceMaker);
    query_clain.add(QuerySender);
    let answer = query_clain.next(&DNSQuery::from(buffer)).await?;
    server_socket().send_to(answer.to_u8_vec().as_slice(), src).await?;
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
            debug!("The dns query is not supported , will not mit the cache and pre choose!");
            let answer = send_and_recv(fast_server(), query).await?;
            debug!("dns answer: {:?}", answer);
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
        send_and_recv(fast_server(), query).await
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

pub async fn recv_query() -> Result<(PacketBuffer, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = server_socket().recv_from(buffer.as_mut_slice()).await?;
    Ok((buffer, src))
}


async fn preferred_with_ping(answer: &mut DNSAnswer) -> Result<()> {
    let ip_vec = answer.get_ip_vec();
    if ip_vec.len() == 1 {
        answer.retain_ip(ip_vec[0]);
        return Ok(());
    }
    let mut ping_future_vec = Vec::new();
    for ip in &ip_vec {
        let future = ping().chain(ip.clone()).send();
        ping_future_vec.push(future);
    }
    let index = select_all(ping_future_vec).await.1;
    answer.retain_ip(ip_vec[index]);
    Ok(())
}

async fn recv_and_handle_answer() -> Result<()> {
    let mut buffer = PacketBuffer::new();
    upstream_socket().recv_from(buffer.as_mut_slice()).await?;
    let answer = DNSAnswer::from(buffer);
    match reg_table().remove(answer.get_id()) {
        None => {}
        Some((_, sender)) => {
            if let Err(e) = sender.send(answer) {
                reg_table().remove(e.get_id());
            }
        }
    }
    Ok(())
}

async fn preferred_dns_server(query: DNSQuery) -> Result<&'static str> {
    let servers = servers();
    let mut future_vec =
        Vec::with_capacity(servers.len());
    for address in servers.iter() {
        future_vec.push(send_and_recv(address, &query).boxed());
    }
    let (result, index, _) = select_all(future_vec).await;
    let _answer = result?;
    Ok(servers[index])
}

async fn send_and_recv(address: &str, query: &DNSQuery) -> Result<DNSAnswer> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let next_id = next_id();
    reg_table().insert(next_id, sender);
    upstream_socket().send_to(query.to_u8_with_id(next_id).as_slice(), address).await?;
    let mut answer = receiver.await?;
    reg_table().remove(answer.get_id());
    answer.set_id(query.get_id().clone());
    Ok(answer)
}

