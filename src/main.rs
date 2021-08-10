#![feature(panic_info_message)]

mod buffer;
mod protocol;
mod cache;
mod system;
mod handler;
mod filter;
mod config;

use crate::system::{Result, setup_exit_process_task};
use crate::handler::*;
use simple_logger::SimpleLogger;

#[macro_use]
extern crate log;

//dig @127.0.0.1 -p 2053 www.baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init()?;
    system::setup_panic_hook();

    let config = config::init_from_toml().await?;
    system::setup_log_level(&config)?;
    handler::init_context(&config).await?;
    cache::init_context(&config).await?;
    filter::init_context(&config).await?;
    drop(config);

    setup_exit_process_task();
    setup_answer_accept_task();
    setup_choose_fast_server_task();

    //从客户端接受请求的主循环
    loop {
        let (buffer, src) = recv_query().await?;
        tokio::spawn(async move {
            match handle_task(src, buffer).await {
                Ok(_) => {}
                Err(e) => {
                    error!("error occur here main{:?}", e)
                }
            }
        });
    }
}



