use crate::protocol::{DNSQuery, DNSAnswer};
use async_trait::async_trait;
use crate::system::Result;
use futures_util::future::select_all;
use futures_util::FutureExt;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::handler::server_group::ServerSender;

pub struct PreferServerSender {
    executor: QueryExecutor,
    servers: Vec<String>,
}

#[async_trait]
impl ServerSender for PreferServerSender {
    async fn send(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.executor.exec(address.as_str(), &query).boxed());
        }
        let (result, _, _) = select_all(future_vec).await;
        let answer = result?;
        Ok(answer)
    }
}

impl PreferServerSender {
    pub fn from(executor: QueryExecutor, servers: Vec<String>) -> Self {
        PreferServerSender {
            executor,
            servers,
        }
    }
}