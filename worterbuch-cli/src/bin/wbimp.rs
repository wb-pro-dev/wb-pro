use anyhow::Result;
use clap::Parser;
use std::{
    process,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{spawn, time::sleep};
use worterbuch_cli::print_message;
use worterbuch_client::config::Config;
use worterbuch_client::connect;

#[derive(Parser)]
#[command(author, version, about = "Import key/value pairs from JSON files into Wörterbuch.", long_about = None)]
struct Args {
    /// Connect to the Wörterbuch server using SSL encryption.
    #[arg(short, long)]
    ssl: bool,
    /// The address of the Wörterbuch server. When omitted, the value of the env var WORTERBUCH_HOST_ADDRESS will be used. If that is not set, 127.0.0.1 will be used.
    #[arg(short, long)]
    addr: Option<String>,
    /// The port of the Wörterbuch server. When omitted, the value of the env var WORTERBUCH_PORT will be used. If that is not set, 4242 will be used.
    #[arg(short, long)]
    port: Option<u16>,
    /// Paths to the JSON files to be imported. Note that this refers to the file system of the server, the files will NOT be uploaded from the client.
    paths: Vec<String>,
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let config = Config::new()?;
    let args: Args = Args::parse();

    let proto = if args.ssl {
        "wss".to_owned()
    } else {
        "ws".to_owned()
    };
    let host_addr = args.addr.unwrap_or(config.host_addr);
    let port = args.port.unwrap_or(config.port);
    let paths = args.paths;

    let on_disconnect = async move {
        log::warn!("Connection to server lost.");
        process::exit(1);
    };

    let mut con = connect(&proto, &host_addr, port, vec![], vec![], on_disconnect).await?;

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
            print_message(&msg, false);
        }
    });

    for path in paths {
        trans_id = con.import(path.to_owned())?;
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
