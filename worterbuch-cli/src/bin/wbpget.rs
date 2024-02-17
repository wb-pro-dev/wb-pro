/*
 *  Worterbuch cli client for retrieving entries
 *
 *  Copyright (C) 2024 Michael Bachmann
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;
use tokio_graceful_shutdown::{SubsystemHandle, Toplevel};
use worterbuch_cli::{next_item, print_change_event, print_message, provide_keys};
use worterbuch_client::config::Config;
use worterbuch_client::{connect, AuthToken};

#[derive(Parser)]
#[command(author, version, about = "Get values for patterns from a Wörterbuch.", long_about = None)]
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
    /// Output data in JSON and expect input data to be JSON.
    #[arg(short, long)]
    json: bool,
    /// Patterns to be fetched from Wörterbuch in the form "PATTERN1 PATTERN2 PATTERN3 ...". When omitted, patterns will be read from stdin. When reading patterns from stdin, one pattern is expected per line.
    patterns: Option<Vec<String>>,
    /// Auth token to be used to authenticate with the server
    #[arg(long)]
    auth: Option<AuthToken>,
    /// Print only the received key/value pairs
    #[arg(short, long)]
    raw: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    Toplevel::new()
        .start("wbpget", run)
        .catch_signals()
        .handle_shutdown_requests(Duration::from_millis(1000))
        .await?;

    Ok(())
}

async fn run(subsys: SubsystemHandle) -> Result<()> {
    let mut config = Config::new();
    let args: Args = Args::parse();

    config.auth_token = args.auth.or(config.auth_token);

    config.proto = if args.ssl {
        "wss".to_owned()
    } else {
        "tcp".to_owned()
    };
    config.host_addr = args.addr.unwrap_or(config.host_addr);
    config.port = args.port.unwrap_or(config.port);
    let json = args.json;
    let raw = args.raw;
    let patterns = args.patterns;

    let (disco_tx, mut disco_rx) = mpsc::channel(1);
    let on_disconnect = async move {
        disco_tx.send(()).await.ok();
    };

    let wb = connect(config, on_disconnect).await?;
    let mut responses = wb.all_messages().await?;

    let mut trans_id = 0;
    let mut acked = 0;

    let mut rx = provide_keys(patterns, subsys.clone());
    let mut done = false;

    loop {
        if done && acked >= trans_id {
            break;
        }
        select! {
            _ = subsys.on_shutdown_requested() => break,
            _ = disco_rx.recv() => {
                log::warn!("Connection to server lost.");
                subsys.request_global_shutdown();
            }
            msg = responses.recv() => if let Some(msg) = msg {
                if let Some(tid) = msg.transaction_id() {
                    if tid > acked {
                        acked = tid;
                    }
                }
                if raw {
                    print_change_event(&msg, json);
                } else {
                    print_message(&msg, json, false);
                }
            },
            recv = next_item(&mut rx, done) => match recv {
                Some(key ) => trans_id = wb.pget_async(key).await?,
                None => done = true,
            },
        }
    }

    Ok(())
}
