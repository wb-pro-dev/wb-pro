mod config;
mod repl;
mod server;
mod store;
mod subscribers;
mod utils;
mod worterbuch;

use crate::{config::Config, repl::repl, worterbuch::Worterbuch};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    signal::unix::{signal, SignalKind},
    spawn,
    sync::RwLock,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config = Config::new()?;

    log::debug!("Separator: {}", config.separator);
    log::debug!("Wildcard: {}", config.wildcard);
    log::debug!("Multi-Wildcard: {}", config.multi_wildcard);

    let worterbuch = Worterbuch::with_config(config.clone());
    let worterbuch = Arc::new(RwLock::new(worterbuch));

    #[cfg(feature = "graphql")]
    spawn(server::warp::start(worterbuch.clone(), config));

    spawn(server::tcp::start(worterbuch.clone(), config));

    spawn(repl(worterbuch));

    let mut signal = signal(SignalKind::terminate())?;
    signal.recv().await;

    Ok(())
}
