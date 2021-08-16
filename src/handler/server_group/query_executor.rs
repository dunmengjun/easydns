use tokio::net::UdpSocket;
use dashmap::DashMap;
use crate::protocol::{DNSAnswer, DNSQuery};
use tokio::sync::oneshot::Sender;
use crate::system::{Result, next_id, AnswerBuf, default_value};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use std::time::Duration;

pub struct QueryExecutor {
    socket: Arc<UdpSocket>,
    reg_table: Arc<DashMap<u16, Sender<DNSAnswer>>>,
}

impl QueryExecutor {
    pub async fn create() -> Result<Self> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        let cloned_socket = socket.clone();
        let reg_table = Arc::new(DashMap::new());
        let cloned_reg_table = reg_table.clone();

        let executor = QueryExecutor {
            socket: cloned_socket,
            reg_table: cloned_reg_table,
        };

        tokio::spawn(async move {
            loop {
                match executor.recv().await {
                    Ok(()) => {}
                    Err(e) => error!("error occur here accept {:?}", e),
                }
            }
        });

        Ok(QueryExecutor {
            socket,
            reg_table,
        })
    }

    pub async fn exec(&self, address: &str, query: &DNSQuery) -> Result<DNSAnswer> {
        let (sender, receiver) = oneshot::channel();
        let next_id = next_id();
        self.reg_table.insert(next_id, sender);
        self.socket
            .send_to(query.to_u8_with_id(next_id).as_slice(), address)
            .await?;
        let mut answer = match timeout(Duration::from_secs(3), receiver).await {
            Ok(result) => {
                result?
            }
            Err(_) => {
                DNSAnswer::from_query_with_failure(query)
            }
        };
        self.reg_table.remove(answer.get_id());
        answer.set_id(query.get_id().clone());
        Ok(answer)
    }

    async fn recv(&self) -> Result<()> {
        let mut buf: AnswerBuf = default_value();
        self.socket.recv_from(&mut buf).await?;
        let answer = DNSAnswer::from(buf);
        match self.reg_table.remove(answer.get_id()) {
            Some((_, sender)) => {
                if let Err(e) = sender.send(answer) {
                    self.reg_table.remove(e.get_id());
                }
            }
            None => {}
        }
        Ok(())
    }
}