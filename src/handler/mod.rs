use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio_icmp::Pinger;

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
use crate::handler::server_group::ServerGroup;

mod legal_checker;
mod cache_handler;
mod query_sender;
mod ip_maker;
mod domain_filter;
mod server_group;

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
        let server_group = Arc::new(ServerGroup::from(
            config.servers.clone(),
            config.server_choose_strategy.clone(),
            (config.server_choose_duration_h * 60 * 60) as u64,
        ).await?);
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
