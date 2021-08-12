use tokio::net::UdpSocket;
use dashmap::DashMap;
use tokio::sync::oneshot::Sender;
use crate::protocol::{DNSAnswer, DNSQuery};
use std::sync::Mutex;
use crate::config::Config;
use crate::system::{Result, next_id};
use crate::buffer::PacketBuffer;
use tokio::sync::oneshot;
use futures_util::FutureExt;
use futures_util::future::select_all;

pub struct ServerGroup {
    socket: UdpSocket,
    reg_table: DashMap<u16, Sender<DNSAnswer>>,
    servers: Vec<String>,
    fast_server: Mutex<String>,
    pub server_choose_strategy: usize,
    pub server_choose_duration_h: usize,
}

impl ServerGroup {
    pub async fn from(config: &Config) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let reg_table = DashMap::new();
        let servers = config.servers.clone();
        let fast_server = Mutex::new(servers[0].clone());
        Ok(ServerGroup {
            socket,
            reg_table,
            servers,
            fast_server,
            server_choose_strategy: config.server_choose_strategy,
            server_choose_duration_h: config.server_choose_duration_h,
        })
    }
    pub async fn recv(&self) -> Result<()> {
        let mut buffer = PacketBuffer::new();
        self.socket.recv_from(buffer.as_mut_slice()).await?;
        let answer = DNSAnswer::from(buffer);
        match self.reg_table.remove(answer.get_id()) {
            None => {}
            Some((_, sender)) => {
                if let Err(e) = sender.send(answer) {
                    self.reg_table.remove(e.get_id());
                }
            }
        }
        Ok(())
    }

    async fn exec_query(&self, address: &str, query: &DNSQuery) -> Result<DNSAnswer> {
        let (sender, receiver) = oneshot::channel();
        let next_id = next_id();
        self.reg_table.insert(next_id, sender);
        self.socket
            .send_to(query.to_u8_with_id(next_id).as_slice(), address)
            .await?;
        let mut answer = receiver.await?;
        self.reg_table.remove(answer.get_id());
        answer.set_id(query.get_id().clone());
        Ok(answer)
    }

    async fn fast_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let address = self.fast_server.lock().unwrap().clone();
        self.exec_query(address.as_str(), query).await
    }

    async fn prefer_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let (answer, _) = self.get_answer_from_fast_server(query).await?;
        Ok(answer)
    }

    async fn combine_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.exec_query(address.as_str(), query));
        }
        let mut answer = DNSAnswer::from_query(query);
        for future in future_vec {
            match future.await {
                Ok(r) => {
                    answer.combine(r);
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
        if answer.is_empty() {
            return Err("all servers return error".into());
        }
        Ok(answer)
    }

    pub async fn send_query(&self, query: &DNSQuery) -> Result<DNSAnswer> {
        match self.server_choose_strategy {
            0 => {
                self.fast_query(query).await
            }
            1 => {
                self.prefer_query(query).await
            }
            2 => {
                self.combine_query(query).await
            }
            e => {
                panic!("Unsupported server choose strategy: {}", e);
            }
        }
    }

    async fn get_answer_from_fast_server(&self, query: &DNSQuery) -> Result<(DNSAnswer, usize)> {
        let servers = &self.servers;
        let mut future_vec = Vec::with_capacity(servers.len());
        for address in servers.iter() {
            future_vec.push(self.exec_query(address.as_str(), &query).boxed());
        }
        let (result, index, _) = select_all(future_vec).await;
        let answer = result?;
        Ok((answer, index))
    }

    pub async fn preferred_dns_server(&self, query: DNSQuery) -> Result<()> {
        let (_, index) = self.get_answer_from_fast_server(&query).await?;
        *self.fast_server.lock().unwrap() = self.servers[index].clone();
        Ok(())
    }
}