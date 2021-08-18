use crate::protocol::{DNSQuery};
use crate::system::Result;
use async_trait::async_trait;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::handler::server_group::ServerSender;
use crate::protocol_new::{DnsAnswer, Ipv4Answer, FailureAnswer};

pub struct CombineServerSender {
    executor: QueryExecutor,
    servers: Vec<String>,
}

#[async_trait]
impl ServerSender for CombineServerSender {
    async fn send(&self, query: &DNSQuery) -> Result<DnsAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.executor.exec(address.as_str(), query));
        }
        let mut ipv4_answer = Ipv4Answer::empty_answer(
            query.get_id().clone(), query.get_readable_domain());
        for future in future_vec {
            match future.await {
                Ok(r) => {
                    ipv4_answer.combine(r);
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
        if ipv4_answer.is_empty() {
            return Ok(FailureAnswer::new(
                query.get_id().clone(), query.get_readable_domain()).into());
        } else {
            Ok(ipv4_answer.into())
        }
    }
}

impl CombineServerSender {
    pub fn from(executor: QueryExecutor, servers: Vec<String>) -> Self {
        CombineServerSender {
            executor,
            servers,
        }
    }
}