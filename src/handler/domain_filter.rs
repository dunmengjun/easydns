use async_trait::async_trait;
use crate::filter::Filter;
use std::sync::Arc;
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::handler::{Clain, Handler};
use crate::system::Result;

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
    async fn handle(&self, clain: Clain, query: DNSQuery) -> Result<DNSAnswer> {
        let domain = query.get_readable_domain();
        if self.filter.contain(domain) {
            //返回soa
            return Ok(DNSAnswer::from_query_with_soa(&query));
        }
        clain.next(query).await
    }
}