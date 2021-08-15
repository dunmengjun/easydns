#![feature(panic_info_message)]

use std::sync::Arc;

use simple_logger::SimpleLogger;

use crate::handler::*;
use crate::system::{Result};

mod buffer;
mod config;
mod filter;
mod protocol;
mod system;
mod handler;
mod cache;

#[macro_use]
extern crate log;

//dig @127.0.0.1 -p 2053 www.baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init()?;
    system::setup_panic_hook();

    let config = config::init_from_toml().await?;
    system::setup_log_level(&config)?;
    let handler = Arc::new(HandlerContext::from(config).await?);
    //从客户端接受请求的主循环
    loop {
        tokio::select! {
            result = handler.recv_query() => {
                let (buffer, src) = result?;
                let arc_handler = handler.clone();
                tokio::spawn(async move {
                    match arc_handler.handle_task(src, buffer).await {
                        Ok(_) => {}
                        Err(e) => error!("error occur here main{:?}", e),
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
