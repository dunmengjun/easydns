#![feature(panic_info_message)]

use std::sync::Arc;

use simple_logger::SimpleLogger;

use crate::handler::*;
use crate::system::{Result};
use crate::client::ClientSocket;

mod buffer;
mod config;
mod filter;
mod protocol;
mod system;
mod handler;
mod cache;
mod client;

#[macro_use]
extern crate log;

//dig @127.0.0.1 -p 2053 www.baidu.com
//dig @127.0.0.1 -p 2053 0-100.com
#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init()?;
    system::setup_panic_hook();

    let config = config::init_from_toml().await?;
    system::setup_log_level(&config)?;
    let client = Arc::new(ClientSocket::new(config.port).await?);
    let handler = Arc::new(HandlerContext::from(config).await?);
    //主循环
    loop {
        tokio::select! {
            result = client.recv() => {
                let (buffer, src) = result?;
                let arc_client = client.clone();
                let arc_handler = handler.clone();
                tokio::spawn(async move {
                    let answer = match arc_handler.handle_query(buffer).await {
                        Ok(answer) => answer,
                        Err(e) => {
                            error!("Handle query task error: {:?}", e);
                            return;
                        },
                    };
                    info!("answer: {:?}", answer);
                    if let Err(e) = arc_client.back_to(src, answer).await {
                        error!("Send answer back to client error: {:?}", e)
                    }
                });
            },
            //监听ctrl_c事件
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }
    Ok(())
}
