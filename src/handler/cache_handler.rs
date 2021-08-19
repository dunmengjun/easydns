use async_trait::async_trait;
use crate::cache::CachePool;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::system::{Result};
use futures_util::FutureExt;
use crate::protocol_new::{DnsAnswer, DnsQuery};

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
    async fn handle(&self, clain: Clain, query: DnsQuery) -> Result<DnsAnswer> {
        let id = query.get_id().clone();
        self.cache_pool
            .get(query.get_name().clone(), clain.next(query).boxed()).await
            .map(|mut r| {
                r.set_id(id);
                r
            })
    }
}