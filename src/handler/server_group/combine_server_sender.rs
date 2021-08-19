use crate::system::Result;
use async_trait::async_trait;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::handler::server_group::ServerSender;
use crate::protocol_new::{DnsAnswer, Ipv4Answer, FailureAnswer, DnsQuery};

pub struct CombineServerSender {
    executor: QueryExecutor,
    servers: Vec<String>,
}

#[async_trait]
impl ServerSender for CombineServerSender {
    async fn send(&self, query: DnsQuery) -> Result<DnsAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.executor.exec(address.as_str(), query.clone()));
        }
        let mut ipv4_answer = Ipv4Answer::empty_answer(
            query.get_id().clone(), query.get_name().clone());
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
                query.get_id().clone(), query.get_name().clone()).into());
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