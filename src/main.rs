mod args;
mod client;
mod control;
pub use crate::control::*;
mod server;

use std::io::Write;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use client::Client;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::args::Args;
use crate::server::Server;

fn init_log() {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    builder
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Mode {
    Tcp,
    Udp,
}

async fn run_client(args: &Args) -> Result<()> {
    let client = Client::new(args).await.context("new")?;
    client.run().await.context("run")?;
    Ok(())
}

async fn run_server(args: &Args) -> Result<()> {
    let server = Server::new(args).await.context("new")?;
    server.run().await.context("run")?;
    Ok(())
}

#[tokio::main]
async fn main() {
    init_log();

    let mut args = Args::parse();

    if args.client {
        info!("Running as client");
        run_client(&args).await.context("run client").unwrap();
        return;
    }

    if !args.server {
        debug!("Neither client nor server mode specified, running as server by default");
        args.server = true;
    }

    info!("Running as server");
    run_server(&args).await.context("run server").unwrap();
}
