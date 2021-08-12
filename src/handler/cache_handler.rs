use async_trait::async_trait;
use crate::cache::CachePool;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::Result;

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
        let cloned_cache_pool = self.cache_pool.clone();
        let async_func = move || {
            tokio::spawn(async move {
                match temp_chain.next(cloned_query).await {
                    Ok(answer) => {
                        cloned_cache_pool.store_answer(&answer);
                    }
                    Err(e) => {
                        error!("async get answer from server error: {:?}", e)
                    }
                }
            });
        };
        if let Some(answer) = self.cache_pool.get_answer(&query, async_func) {
            return Ok(answer);
        } else {
            let result = clain.next(query).await?;
            self.cache_pool.store_answer(&result);
            Ok(result)
        }
    }
}