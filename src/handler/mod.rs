use std::sync::Arc;

use async_trait::async_trait;
use tokio_icmp::Pinger;

use crate::cache::CachePool;
use crate::config::Config;
use crate::filter::Filter;
use crate::handler::cache_handler::CacheHandler;
use crate::handler::domain_filter::DomainFilter;
use crate::handler::ip_maker::{IpChoiceMaker, IpFirstMaker};
use crate::handler::legal_checker::LegalChecker;
use crate::handler::query_sender::QuerySender;
use crate::protocol::{DNSQuery};
use crate::system::{Result, QueryBuf};
use crate::handler::server_group::ServerGroup;
use std::option::Option::Some;
use crate::protocol_new::DnsAnswer;

mod legal_checker;
mod cache_handler;
mod query_sender;
mod ip_maker;
mod domain_filter;
mod server_group;

pub struct HandlerContext {
    server_group: Arc<ServerGroup>,
    pinger: Option<Arc<Pinger>>,
    cache_pool: Option<Arc<CachePool>>,
    filter: Arc<Filter>,
}

impl HandlerContext {
    pub async fn from(config: Config) -> Result<Self> {
        let pinger = if config.ip_choose_strategy == 0 {
            None
        } else {
            Some(Arc::new(Pinger::new().await?))
        };
        let server_group = Arc::new(ServerGroup::from(
            config.servers.clone(),
            config.server_choose_strategy.clone(),
            (config.server_choose_duration_h * 60 * 60) as u64,
        ).await?);
        let cache_pool = if config.cache_on {
            Some(Arc::new(CachePool::from(&config).await?))
        } else {
            None
        };
        let filter = Arc::new(Filter::from(&config).await);
        Ok(HandlerContext {
            server_group,
            pinger,
            cache_pool,
            filter,
        })
    }

    pub async fn handle_query(&self, buf: QueryBuf) -> Result<DnsAnswer> {
        let mut query_clain = Clain::new();
        query_clain.add(DomainFilter::new(self.filter.clone()));
        query_clain.add(LegalChecker::new(self.server_group.clone()));
        if let Some(pool) = self.cache_pool.clone() {
            query_clain.add(CacheHandler::new(pool));
        }
        if let Some(pinger) = self.pinger.clone() {
            query_clain.add(IpChoiceMaker::new(pinger));
        } else {
            query_clain.add(IpFirstMaker);
        }
        query_clain.add(QuerySender::new(self.server_group.clone()));
        query_clain.next(DNSQuery::from(buf)).await
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

    async fn next(mut self, query: DNSQuery) -> Result<DnsAnswer> {
        self.funcs.remove(0).handle(self, query).await
    }
}

#[async_trait]
trait Handler: Send + Sync {
    async fn handle(&self, clain: Clain, query: DNSQuery) -> Result<DnsAnswer>;
}
