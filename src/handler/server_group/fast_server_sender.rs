use std::sync::{Arc, Mutex};
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::Result;
use tokio::time::Duration;
use futures_util::FutureExt;
use async_trait::async_trait;
use futures_util::future::select_all;
use tokio::time::interval;
use crate::handler::server_group::query_executor::QueryExecutor;
use crate::handler::server_group::ServerSender;

pub struct FastServerSender {
    executor: Arc<QueryExecutor>,
    servers: Arc<Vec<String>>,
    fast_server: Arc<Mutex<String>>,
}

#[async_trait]
impl ServerSender for FastServerSender {
    async fn send(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let address = self.fast_server.lock().unwrap().clone();
        self.executor.exec(address.as_str(), query).await
    }
}

impl FastServerSender {
    pub fn from(
        query_executor: QueryExecutor,
        servers: Vec<String>,
        duration_secs: u64,
    ) -> Self {
        let executor = Arc::new(query_executor);
        let cloned_executor = executor.clone();
        let arc_servers = Arc::new(servers);
        let cloned_servers = arc_servers.clone();
        let fast_server = Arc::new(Mutex::new(String::new()));
        let cloned_fast_server = fast_server.clone();
        let sender = FastServerSender {
            executor: cloned_executor,
            servers: cloned_servers,
            fast_server: cloned_fast_server,
        };

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(duration_secs));
            loop {
                interval.tick().await;
                let test_query = DNSQuery::from_domain("www.baidu.com");
                if let Err(e) = sender.preferred_dns_server(test_query).await {
                    error!("interval task upstream servers choose has error: {:?}", e)
                }
            }
        });

        FastServerSender {
            executor,
            servers: arc_servers,
            fast_server,
        }
    }

    async fn preferred_dns_server(&self, query: DNSQuery) -> Result<()> {
        let (_, index) = self.get_answer_from_fast_server(&query).await?;
        *self.fast_server.lock().unwrap() = self.servers[index].clone();
        Ok(())
    }

    async fn get_answer_from_fast_server(&self, query: &DNSQuery) -> Result<(DNSAnswer, usize)> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.executor.exec(address.as_str(), &query).boxed());
        }
        let (result, index, _) = select_all(future_vec).await;
        let answer = result?;
        Ok((answer, index))
    }
}