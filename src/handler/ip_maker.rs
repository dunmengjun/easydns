use async_trait::async_trait;
use tokio_icmp::Pinger;
use std::sync::Arc;
use crate::handler::{Clain, Handler};
use crate::system::Result;
use futures_util::future::select_all;
use crate::protocol::{DnsAnswer, Ipv4Answer, DnsQuery};
use std::net::IpAddr;

#[derive(Clone)]
pub struct IpChoiceMaker {
    pinger: Arc<Pinger>,
}

impl IpChoiceMaker {
    pub fn new(pinger: Arc<Pinger>) -> Self {
        IpChoiceMaker {
            pinger
        }
    }
}

#[async_trait]
impl Handler for IpChoiceMaker {
    async fn handle(&self, clain: Clain, query: DnsQuery) -> Result<DnsAnswer> {
        let mut answer = clain.next(query).await?;
        if let Some(ipv4_answer) = answer.as_mut_any().downcast_mut::<Ipv4Answer>() {
            let ip_vec = ipv4_answer.get_all_ips();
            if ip_vec.len() == 1 {
                return Ok(answer);
            }
            let mut ping_future_vec = Vec::new();
            ip_vec.iter().for_each(|r| {
                let ip = *r.clone();
                let future = self.pinger.chain(IpAddr::V4(ip)).send();
                ping_future_vec.push(future);
            });
            let index = select_all(ping_future_vec).await.1;
            let ip = ip_vec[index].clone();
            ipv4_answer.retain_ip(&ip);
        }
        Ok(answer)
    }
}

#[derive(Clone)]
pub struct IpFirstMaker;

#[async_trait]
impl Handler for IpFirstMaker {
    async fn handle(&self, clain: Clain, query: DnsQuery) -> Result<DnsAnswer> {
        let mut answer = clain.next(query).await?;
        if let Some(ipv4_answer) = answer.as_mut_any().downcast_mut::<Ipv4Answer>() {
            let vec = ipv4_answer.get_all_ips();
            let addr = vec[0].clone();
            ipv4_answer.retain_ip(&addr);
        }
        Ok(answer)
    }
}