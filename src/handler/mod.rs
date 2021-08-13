use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::time::Duration;
use tokio::time::interval;
use tokio_icmp::Pinger;

use server::ServerGroup;

use crate::buffer::PacketBuffer;
use crate::cache::CachePool;
use crate::config::Config;
use crate::filter::Filter;
use crate::handler::cache_handler::CacheHandler;
use crate::handler::domain_filter::DomainFilter;
use crate::handler::ip_maker::IpChoiceMaker;
use crate::handler::legal_checker::LegalChecker;
use crate::handler::query_sender::QuerySender;
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::Result;

mod legal_checker;
mod cache_handler;
mod query_sender;
mod ip_maker;
mod domain_filter;
mod server;

pub struct HandlerContext {
    server_group: Arc<ServerGroup>,
    main_socket: UdpSocket,
    pinger: Arc<Option<Pinger>>,
    cache_pool: Arc<CachePool>,
    filter: Arc<Filter>,
}

impl HandlerContext {
    pub async fn from(config: Config) -> Result<Self> {
        let pinger = if config.ip_choose_strategy == 0 {
            None
        } else {
            Some(tokio_icmp::Pinger::new().await?)
        };
        let main_socket = UdpSocket::bind(("0.0.0.0", config.port)).await?;
        let server_group = Arc::new(ServerGroup::from(&config).await?);
        let cache_pool = Arc::new(CachePool::from(&config).await?);
        let filter = Arc::new(Filter::from(&config).await);
        Ok(HandlerContext {
            server_group,
            main_socket,
            pinger: Arc::new(pinger),
            cache_pool,
            filter,
        })
    }

    async fn back_to_client(&self, client: SocketAddr, answer: DNSAnswer) -> Result<()> {
        self.main_socket
            .send_to(answer.to_u8_vec().as_slice(), client)
            .await?;
        Ok(())
    }

    pub async fn recv_query(&self) -> Result<(PacketBuffer, SocketAddr)> {
        let mut buffer = PacketBuffer::new();
        let (_, src) = self
            .main_socket
            .recv_from(buffer.as_mut_slice())
            .await?;
        Ok((buffer, src))
    }

    pub async fn handle_task(&self, src: SocketAddr, buffer: PacketBuffer) -> Result<()> {
        let mut query_clain = Clain::new();
        query_clain.add(DomainFilter::new(self.filter.clone()));
        query_clain.add(LegalChecker::new(self.server_group.clone()));
        query_clain.add(CacheHandler::new(self.cache_pool.clone()));
        query_clain.add(IpChoiceMaker::new(self.pinger.clone()));
        query_clain.add(QuerySender::new(self.server_group.clone()));
        let answer = query_clain.next(DNSQuery::from(buffer)).await?;
        self.back_to_client(src, answer).await
    }
}

pub fn setup_exit_process_task(context: &HandlerContext) {
    //创建任务去监听ctrl_c event
    let cloned_cache_pool = context.cache_pool.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl_c event");
        cloned_cache_pool.exit_process_action().await.unwrap();
        std::process::exit(0);
    });
}

pub fn setup_answer_accept_task(context: &HandlerContext) {
    //创建任务去recv从上游dns服务器返回的answer
    let cloned_server_group = context.server_group.clone();
    tokio::spawn(async move {
        loop {
            match cloned_server_group.recv().await {
                Ok(()) => {}
                Err(e) => error!("error occur here accept {:?}", e),
            }
        }
    });
}

pub fn setup_choose_fast_server_task(context: &HandlerContext) {
    let server_group = context.server_group.clone();
    if server_group.server_choose_strategy != 0 {
        return;
    }
    //创建定时任务去定时的优选上游dns servers,半天触发一次
    let duration_secs = server_group.server_choose_duration_h * 60 * 60;
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(duration_secs as u64));
        loop {
            interval.tick().await;
            let test_query = DNSQuery::from_domain("www.baidu.com");
            if let Err(e) = server_group.preferred_dns_server(test_query).await {
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

    async fn next(&mut self, query: DNSQuery) -> Result<DNSAnswer> {
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
    async fn handle(&self, clain: &mut Clain, query: DNSQuery) -> Result<DNSAnswer>;
}

trait HandlerCloner {
    fn clone_handler(&self) -> Box<dyn Handler>;
}

impl<T> HandlerCloner for T where T: 'static + Clone + Handler {
    fn clone_handler(&self) -> Box<dyn Handler> {
        Box::new(self.clone())
    }
}
