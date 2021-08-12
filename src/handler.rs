use crate::buffer::PacketBuffer;
use crate::config::Config;
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::{next_id, Result};
use crate::{cache, filter};
use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::future::select_all;
use futures_util::FutureExt;
use std::net::SocketAddr;
use std::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::sync::oneshot::Sender;
use tokio::sync::{oneshot, OnceCell};
use tokio::time::interval;
use tokio::time::Duration;
use tokio_icmp::Pinger;

pub struct HandlerContext {
    server_socket: UdpSocket,
    main_socket: UdpSocket,
    pinger: Option<Pinger>,
    reg_table: DashMap<u16, Sender<DNSAnswer>>,
    servers: Vec<String>,
    fast_server: Mutex<String>,
    server_choose_strategy: usize,
    server_choose_duration_h: usize,
}

impl HandlerContext {
    pub async fn from(config: &Config) -> Result<Self> {
        let pinger = if config.ip_choose_strategy == 0 {
            None
        } else {
            Some(tokio_icmp::Pinger::new().await?)
        };
        let server_socket = UdpSocket::bind(("0.0.0.0", config.port)).await?;
        let upstream_socket = UdpSocket::bind("0.0.0.0:0").await?;
        let answer_reg_table = DashMap::new();
        let upstream_dns_servers = config.servers.clone();
        let fast_dns_server = Mutex::new(upstream_dns_servers[0].clone());
        Ok(HandlerContext {
            server_socket: upstream_socket,
            main_socket: server_socket,
            pinger,
            reg_table: answer_reg_table,
            servers: upstream_dns_servers,
            fast_server: fast_dns_server,
            server_choose_strategy: config.server_choose_strategy,
            server_choose_duration_h: config.server_choose_duration_h,
        })
    }

    async fn exec_query(&self, address: &str, query: &DNSQuery) -> Result<DNSAnswer> {
        let (sender, receiver) = oneshot::channel();
        let next_id = next_id();
        self.reg_table.insert(next_id, sender);
        self.server_socket
            .send_to(query.to_u8_with_id(next_id).as_slice(), address)
            .await?;
        let mut answer = receiver.await?;
        self.reg_table.remove(answer.get_id());
        answer.set_id(query.get_id().clone());
        Ok(answer)
    }

    async fn fast_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let address = self.fast_server.lock().unwrap().clone();
        self.exec_query(address.as_str(), query).await
    }

    async fn prefer_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let (answer, _) = self.get_answer_from_fast_server(query).await?;
        Ok(answer)
    }

    async fn combine_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.exec_query(address.as_str(), query));
        }
        let mut answer = DNSAnswer::from_query(query);
        for future in future_vec {
            match future.await {
                Ok(r) => {
                    answer.combine(r);
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
        if answer.is_empty() {
            return Err("all servers return error".into());
        }
        Ok(answer)
    }

    async fn send_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        match context().server_choose_strategy {
            0 => {
                self.fast_query(query).await
            }
            1 => {
                self.prefer_query(query).await
            }
            2 => {
                self.combine_query(query).await
            }
            e => {
                panic!("Unsupported server choose strategy: {}", e);
            }
        }
    }

    async fn get_answer_from_fast_server(&self, query: &DNSQuery) -> Result<(DNSAnswer, usize)> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.exec_query(address.as_str(), &query).boxed());
        }
        let (result, index, _) = select_all(future_vec).await;
        let answer = result?;
        Ok((answer, index))
    }

    async fn preferred_dns_server(&self, query: DNSQuery) -> Result<()> {
        let (_, index) = self.get_answer_from_fast_server(&query).await?;
        *self.fast_server.lock().unwrap() = self.servers[index].clone();
        Ok(())
    }

    async fn back_to_client(&self, client: SocketAddr, answer: DNSAnswer) -> Result<()> {
        self.main_socket
            .send_to(answer.to_u8_vec().as_slice(), client)
            .await?;
        Ok(())
    }

    async fn recv_and_handle_answer(&self) -> Result<()> {
        let mut buffer = PacketBuffer::new();
        self.server_socket.recv_from(buffer.as_mut_slice()).await?;
        let answer = DNSAnswer::from(buffer);
        match self.reg_table.remove(answer.get_id()) {
            None => {}
            Some((_, sender)) => {
                if let Err(e) = sender.send(answer) {
                    self.reg_table.remove(e.get_id());
                }
            }
        }
        Ok(())
    }
}

pub async fn recv_query() -> Result<(PacketBuffer, SocketAddr)> {
    let mut buffer = PacketBuffer::new();
    let (_, src) = context()
        .main_socket
        .recv_from(buffer.as_mut_slice())
        .await?;
    Ok((buffer, src))
}

pub async fn handle_task(src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
    let mut query_clain = Clain::new();
    query_clain.add(DomainFilter);
    query_clain.add(LegalChecker);
    query_clain.add(CacheHandler);
    query_clain.add(IpChoiceMaker);
    query_clain.add(QuerySender);
    let answer = query_clain.next(&DNSQuery::from(buffer)).await?;
    context().back_to_client(src, answer).await
}

static HANDLER_CONTEXT: OnceCell<HandlerContext> = OnceCell::const_new();

pub async fn init_context(config: &Config) -> Result<()> {
    let context = HandlerContext::from(config).await?;
    match HANDLER_CONTEXT.set(context) {
        Ok(_) => {}
        Err(e) => {
            panic!("{}", e);
        }
    }
    Ok(())
}

pub fn context() -> &'static HandlerContext {
    HANDLER_CONTEXT.get().unwrap()
}

pub fn setup_answer_accept_task() {
    //创建任务去recv从上游dns服务器返回的answer
    tokio::spawn(async move {
        loop {
            match context().recv_and_handle_answer().await {
                Ok(()) => {}
                Err(e) => error!("error occur here accept {:?}", e),
            }
        }
    });
}

pub fn setup_choose_fast_server_task() {
    if context().server_choose_strategy != 0 {
        return;
    }
    //创建定时任务去定时的优选上游dns servers,半天触发一次
    tokio::spawn(async move {
        let duration_secs = context().server_choose_duration_h * 60 * 60;
        let mut interval = interval(Duration::from_secs(duration_secs as u64));
        loop {
            interval.tick().await;
            let test_query = DNSQuery::from_domain("www.baidu.com");
            if let Err(e) = context().preferred_dns_server(test_query).await {
                error!("interval task upstream servers choose has error: {:?}", e)
            }
        }
    });
}

struct Clain {
    pub funcs: Vec<Box<dyn Handler>>,
}

impl Clain {
    fn new() -> Self {
        Clain { funcs: Vec::new() }
    }

    fn add(&mut self, handler: impl Handler + Send + Sync + 'static) {
        self.funcs.push(Box::new(handler));
    }

    async fn next(&mut self, query: &DNSQuery) -> Result<DNSAnswer> {
        self.funcs.remove(0).handle(self, query).await
    }

    fn clone(&self) -> Self {
        let mut vec = Vec::new();
        self.funcs.iter().for_each(|e| vec.push(e.clone_handler()));
        Clain {
            funcs: vec
        }
    }
}

#[async_trait]
trait Handler: Send + Sync + HandlerCloner {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer>;
}

trait HandlerCloner {
    fn clone_handler(&self) -> Box<dyn Handler>;
}

impl<T> HandlerCloner for T where T: 'static + Clone + Handler {
    fn clone_handler(&self) -> Box<dyn Handler> {
        Box::new(self.clone())
    }
}


#[derive(Clone)]
struct LegalChecker;

#[async_trait]
impl Handler for LegalChecker {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        if !query.is_supported() {
            debug!("The dns query is not supported , will not mit the cache!");
            let answer = context().send_query(query).await?;
            debug!("dns answer: {:?}", answer);
            return Ok(answer);
        } else {
            clain.next(query).await
        }
    }
}

#[derive(Clone)]
struct CacheHandler;

#[async_trait]
impl Handler for CacheHandler {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        let async_func = |query: DNSQuery| {
            let mut temp_chain = clain.clone();
            tokio::spawn(async move {
                match temp_chain.next(&query).await {
                    Ok(answer) => {
                        cache::store_answer(&answer);
                    }
                    Err(e) => {
                        error!("async get answer from server error: {:?}", e)
                    }
                }
            });
        };
        if let Some(answer) = cache::get_answer(query, async_func) {
            return Ok(answer);
        } else {
            let result = clain.next(query).await?;
            cache::store_answer(&result);
            Ok(result)
        }
    }
}

#[derive(Clone)]
struct QuerySender;

#[async_trait]
impl Handler for QuerySender {
    async fn handle(&self, _: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        context().send_query(query).await
    }
}

#[derive(Clone)]
struct IpChoiceMaker;

#[async_trait]
impl Handler for IpChoiceMaker {
    async fn handle(&self, clain: &mut Clain, query: &DNSQuery) -> Result<DNSAnswer> {
        let mut answer = clain.next(query).await?;
        let ip_vec = answer.get_ip_vec();
        if let Some(pinger) = &context().pinger {
            if ip_vec.len() == 1 {
                answer.retain_ip(ip_vec[0]);
                return Ok(answer);
            }
            let mut ping_future_vec = Vec::new();
            for ip in &ip_vec {
                let future = pinger.chain(ip.clone()).send();
                ping_future_vec.push(future);
            }
            let index = select_all(ping_future_vec).await.1;
            answer.retain_ip(ip_vec[index]);
        } else {
            answer.retain_ip(ip_vec[0]);
        }
        Ok(answer)
    }
}

#[derive(Clone)]
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
