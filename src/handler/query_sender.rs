use async_trait::async_trait;
use std::sync::Arc;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::handler::{Clain, Handler};
use crate::system::Result;
use crate::handler::server_group::ServerGroup;

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
    async fn handle(&self, _: &mut Clain, query: DNSQuery) -> Result<DNSAnswer> {
        self.server_group.send_query(&query).await
    }
}