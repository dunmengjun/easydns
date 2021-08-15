use async_trait::async_trait;
use tokio_icmp::Pinger;
use std::sync::Arc;
use crate::protocol::{DNSQuery, DNSAnswer};
use crate::handler::{Clain, Handler};
use crate::system::Result;
use futures_util::future::select_all;

#[derive(Clone)]
pub struct IpChoiceMaker {
    pinger: Arc<Option<Pinger>>,
}

impl IpChoiceMaker {
    pub fn new(pinger: Arc<Option<Pinger>>) -> Self {
        IpChoiceMaker {
            pinger
        }
    }
}

#[async_trait]
impl Handler for IpChoiceMaker {
    async fn handle(&self, clain: &mut Clain, query: DNSQuery) -> Result<DNSAnswer> {
        let mut answer = clain.next(query).await?;
        let ip_vec = answer.get_ip_vec();
        if ip_vec.is_empty() {
            return Ok(answer);
        }
        if let Some(pinger) = self.pinger.as_ref() {
            if ip_vec.len() == 1 {
                answer.retain_ip(ip_vec[0]);
                return Ok(answer);
            }
            let mut ping_future_vec = Vec::new();
            for ip in &ip_vec {
                let future = pinger.chain(ip.clone()).send();
                ping_future_vec.push(future);
            }
            let index = select_all(ping_future_vec).await.1;
            answer.retain_ip(ip_vec[index]);
        } else {
            answer.retain_ip(ip_vec[0]);
        }
        Ok(answer)
    }
}