use async_trait::async_trait;
use std::sync::Arc;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::handler::{Handler, Clain};
use crate::system::Result;
use crate::handler::server_group::ServerGroup;

#[derive(Clone)]
pub struct LegalChecker {
    server_group: Arc<ServerGroup>,
}

impl LegalChecker {
    pub fn new(server_group: Arc<ServerGroup>) -> Self {
        LegalChecker {
            server_group
        }
    }
}

#[async_trait]
impl Handler for LegalChecker {
    async fn handle(&self, clain: Clain, query: DNSQuery) -> Result<DNSAnswer> {
        if !query.is_supported() {
            debug!("The dns query is not supported , will not mit the cache!");
            let answer = self.server_group.send_query(&query).await?;
            debug!("dns answer: {:?}", answer);
            return Ok(answer);
        } else {
            clain.next(query).await
        }
    }
}