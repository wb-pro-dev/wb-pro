use anyhow::Result;
use clap::Arg;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{spawn, time::sleep};
#[cfg(feature = "graphql")]
use worterbuch_cli::gql::GqlConnection;
#[cfg(feature = "tcp")]
use worterbuch_cli::tcp::TcpConnection;
#[cfg(feature = "ws")]
use worterbuch_cli::ws::WsConnection;
use worterbuch_cli::{utils::app, Connection};

#[cfg(feature = "tcp")]
async fn connect(proto: &str, host: &str, port: u16) -> Result<TcpConnection> {
    worterbuch_cli::tcp::connect(proto, host, port).await
}

#[cfg(feature = "ws")]
async fn connect(proto: &str, host: &str, port: u16) -> Result<WsConnection> {
    worterbuch_cli::ws::connect(proto, host, port).await
}

#[cfg(feature = "graphql")]
async fn connect(proto: &str, host: &str, port: u16) -> Result<GqlConnection> {
    worterbuch_cli::gql::connect(proto, host, port).await
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let (matches, proto, host_addr, port, _json) = app(
        "wbimp",
        "Import key/value pairs from JSON files into Wörterbuch.",
        false,
        vec![Arg::with_name("PATHS")
            .multiple(true)
            .help(
                r#"Paths to the JSON files to be imported. Note that this refers to the file system of the server, the files will NOT be uploaded from the client."#,
            )
            .takes_value(true)
            .required(true)],
    )?;

    let paths = matches
        .get_many::<String>("PATHS")
        .expect("paths are required");

    let mut con = connect(&proto, &host_addr, port).await?;

    let mut trans_id = 0;
    let acked = Arc::new(Mutex::new(0));
    let acked_recv = acked.clone();

    let mut responses = con.responses();

    spawn(async move {
        while let Ok(msg) = responses.recv().await {
            let tid = msg.transaction_id();
            let mut acked = acked_recv.lock().expect("mutex is poisoned");
            if tid > *acked {
                *acked = tid;
            }
        }
    });

    for path in paths {
        trans_id = con.import(path)?;
    }

    loop {
        let acked = *acked.lock().expect("mutex is poisoned");
        if acked < trans_id {
            sleep(Duration::from_millis(100)).await;
        } else {
            break;
        }
    }

    Ok(())
}
