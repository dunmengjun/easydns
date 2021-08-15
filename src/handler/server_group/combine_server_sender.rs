use crate::protocol::{DNSQuery, DNSAnswer};
use crate::system::Result;
use async_trait::async_trait;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::handler::server_group::ServerSender;

pub struct CombineServerSender {
    executor: QueryExecutor,
    servers: Vec<String>,
}

#[async_trait]
impl ServerSender for CombineServerSender {
    async fn send(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.executor.exec(address.as_str(), query));
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
        if answer.is_empty_answers() {
            return Err("all servers return error".into());
        }
        Ok(answer)
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