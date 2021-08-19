use async_trait::async_trait;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::system::Result;
use crate::handler::server_group::ServerGroup;
use crate::protocol::{DnsAnswer, DnsQuery};

#[derive(Clone)]
pub struct QuerySender {
    server_group: Arc<ServerGroup>,
}

impl QuerySender {
    pub fn new(server_group: Arc<ServerGroup>) -> Self {
        QuerySender {
            server_group
        }
    }
}

#[async_trait]
impl Handler for QuerySender {
    async fn handle(&self, _: Clain, query: DnsQuery) -> Result<DnsAnswer> {
        let answer = self.server_group.send_query(query).await?;
        // info!("answer: {:?}", answer);
        // if answer.is_empty() {
        //     return Err("answer is empty".into());
        // }
        Ok(answer)
    }
}