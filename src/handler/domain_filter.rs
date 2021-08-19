use async_trait::async_trait;
use crate::filter::Filter;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::system::Result;
use crate::protocol_new::{DnsAnswer, SoaAnswer, DnsQuery};

#[derive(Clone)]
pub struct DomainFilter {
    filter: Arc<Filter>,
}

impl DomainFilter {
    pub fn new(filter: Arc<Filter>) -> Self {
        DomainFilter {
            filter
        }
    }
}

#[async_trait]
impl Handler for DomainFilter {
    async fn handle(&self, clain: Clain, query: DnsQuery) -> Result<DnsAnswer> {
        let domain = query.get_name().clone();
        if self.filter.contain(&domain) {
            //返回soa
            return Ok(DnsAnswer::from(SoaAnswer::default_soa(
                query.get_id().clone(), domain)));
        }
        clain.next(query).await
    }
}