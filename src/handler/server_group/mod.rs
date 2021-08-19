mod fast_server_sender;
mod prefer_server_sender;
mod combine_server_sender;
mod query_executor;

use crate::system::Result;
use async_trait::async_trait;
use crate::handler::server_group::fast_server_sender::FastServerSender;
use crate::handler::server_group::prefer_server_sender::PreferServerSender;
use crate::handler::server_group::combine_server_sender::CombineServerSender;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::protocol::{DnsAnswer, DnsQuery};

#[async_trait]
pub trait ServerSender: Sync + Send {
    async fn send(&self, query: DnsQuery) -> Result<DnsAnswer>;
}

pub struct ServerGroup {
    server_sender: Box<dyn ServerSender>,
}

impl ServerGroup {
    pub async fn from(servers: Vec<String>, strategy: usize, duration_secs: u64) -> Result<Self> {
        let query_executor = QueryExecutor::create().await?;
        let server_sender: Box<dyn ServerSender> = match strategy {
            0 => Box::new(FastServerSender::from(query_executor, servers, duration_secs)),
            1 => Box::new(PreferServerSender::from(query_executor, servers)),
            2 => Box::new(CombineServerSender::from(query_executor, servers)),
            _ => panic!("不支持的server strategy类型！"),
        };
        Ok(ServerGroup {
            server_sender,
        })
    }

    pub async fn send_query(&self, query: DnsQuery) -> Result<DnsAnswer> {
        self.server_sender.send(query).await
    }
}
