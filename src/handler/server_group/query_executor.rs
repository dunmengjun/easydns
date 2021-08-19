use tokio::net::UdpSocket;
use dashmap::DashMap;
use tokio::sync::oneshot::Sender;
use crate::system::{Result, next_id, AnswerBuf, default_value};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use std::time::Duration;
use crate::protocol_new::{DnsAnswer, FailureAnswer, DnsQuery};

pub struct QueryExecutor {
    socket: Arc<UdpSocket>,
    reg_table: Arc<DashMap<u16, Sender<DnsAnswer>>>,
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

    pub async fn exec(&self, address: &str, mut query: DnsQuery) -> Result<DnsAnswer> {
        let (sender, receiver) = oneshot::channel();
        let client_query_id = query.get_id();
        let next_id = next_id();
        self.reg_table.insert(next_id, sender);
        query.set_id(next_id);
        let query_vec: Vec<u8> = (&query).into();
        self.socket
            .send_to(query_vec.as_slice(), address)
            .await?;
        let mut answer = match timeout(Duration::from_secs(3), receiver).await {
            Ok(result) => {
                result?
            }
            Err(_) => {
                FailureAnswer::new(client_query_id, query.get_name().clone()).into()
            }
        };
        self.reg_table.remove(&next_id);
        answer.set_id(client_query_id);
        Ok(answer)
    }

    async fn recv(&self) -> Result<()> {
        let mut buf: AnswerBuf = default_value();
        self.socket.recv_from(&mut buf).await?;
        let answer = DnsAnswer::from(buf);
        match self.reg_table.remove(&answer.get_id()) {
            Some((_, sender)) => {
                if let Err(e) = sender.send(answer) {
                    self.reg_table.remove(&e.get_id());
                }
            }
            None => {}
        }
        Ok(())
    }
}