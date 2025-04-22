mod control_service;
mod server_inner;
mod udp;

pub(crate) use server_inner::*;
pub(crate) use udp::UdpStressTest;

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use crate::args::Args;
use crate::server::ServerInner;

pub struct Server {
    bind_addr: String,

    inner: Arc<Mutex<ServerInner>>,
}

impl Server {
    pub async fn new(args: &Args) -> Result<Self> {
        let bind_addr = args.bind_addr.clone().unwrap_or("0.0.0.0".to_string());
        let inner = Arc::new(Mutex::new(
            ServerInner::new(bind_addr.as_str(), args.control_port)
                .await
                .context("new server inner")?,
        ));

        Ok(Server { bind_addr, inner })
    }
}

impl Server {
    pub async fn run(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        loop {
            inner
                .start_stress_test(&self.bind_addr)
                .await
                .context("accept stress test")?;
        }
    }
}
