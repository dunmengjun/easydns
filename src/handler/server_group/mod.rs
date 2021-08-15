mod fast_server_sender;
mod prefer_server_sender;
mod combine_server_sender;
mod query_executor;

use crate::protocol::{DNSQuery, DNSAnswer};
use crate::system::Result;
use async_trait::async_trait;
use crate::handler::server_group::fast_server_sender::FastServerSender;
use crate::handler::server_group::prefer_server_sender::PreferServerSender;
use crate::handler::server_group::combine_server_sender::CombineServerSender;
use crate::handler::server_group::query_executor::QueryExecutor;

#[async_trait]
pub trait ServerSender: Sync + Send {
    async fn send(&self, query: &DNSQuery) -> Result<DNSAnswer>;
}

pub struct ServerGroup {
    servers: Vec<String>,
    server_sender: Box<dyn ServerSender>,
}

impl ServerGroup {
    pub async fn from(servers: Vec<String>, strategy: usize, duration: u64) -> Result<Self> {
        let query_executor = QueryExecutor::create().await?;
        let server_sender: Box<dyn ServerSender> = match strategy {
            0 => Box::new(FastServerSender::from(query_executor, servers.clone(), duration)),
            1 => Box::new(PreferServerSender::from(query_executor, servers.clone())),
            2 => Box::new(CombineServerSender::from(query_executor, servers.clone())),
            _ => panic!("不支持的server strategy类型！"),
        };
        Ok(ServerGroup {
            servers,
            server_sender,
        })
    }

    pub async fn send_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        self.server_sender.send(query).await
    }
}
