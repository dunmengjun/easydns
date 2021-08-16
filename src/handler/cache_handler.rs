use async_trait::async_trait;
use crate::cache::CachePool;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::{Result, block_on};

#[derive(Clone)]
pub struct CacheHandler {
    cache_pool: Arc<CachePool>,
}

impl CacheHandler {
    pub fn new(cache_pool: Arc<CachePool>) -> Self {
        CacheHandler {
            cache_pool
        }
    }
}

#[async_trait]
impl Handler for CacheHandler {
    async fn handle(&self, clain: &mut Clain, query: DNSQuery) -> Result<DNSAnswer> {
        let mut temp_chain = clain.clone();
        let cloned_query = query.clone();
        let result = self.cache_pool
            .get(query.get_domain(), Box::new(|| {
                block_on(async move {
                    temp_chain.next(cloned_query).await
                })
            }));
        result.map(|mut r| {
            r.set_id(query.get_id().clone());
            r
        })
    }
}