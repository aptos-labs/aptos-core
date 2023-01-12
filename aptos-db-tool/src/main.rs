mod app;
use anyhow::Result;
use aptos_logger::{error, Level, Logger};
use clap::Parser;
use commands::backup::utils::GlobalRestoreOptions;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    Logger::new().level(Level::Info).init();
    let tool = app::Tool::from_args();

    let global_opt: GlobalRestoreOptions = tool.global.clone().try_into()?;

    tool.process(global_opt).await.map_err(|e| {
        error!("main_impl() failed: {}", e);
        e
    })
}
