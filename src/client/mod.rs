mod client_inner;

pub(crate) use client_inner::*;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures::future::join_all;
use log::{debug, error, info, trace};
use tokio::net::UdpSocket;
use tokio::pin;
use tokio::sync::{watch, Mutex};
use tokio::time::sleep;

use crate::args::Args;
use crate::client::ClientInner;
use crate::{Mode, Request};

pub struct Client {
    mode: Mode,
    thread_num: usize,
    remote_addr: String,
    remote_port: u16,
    bind_addr: Option<String>,
    duration: u64,

    inner: Arc<Mutex<ClientInner>>,
}

impl Client {
    pub async fn new(args: &Args) -> Result<Self> {
        if !args.client {
            return Err(anyhow!("not client mode"));
        }

        let remote_addr = args.addr.clone().ok_or(anyhow!("empty addr"))?;

        let inner = Arc::new(Mutex::new(
            ClientInner::new(&remote_addr, args.control_port)
                .await
                .context("client inner new")?,
        ));

        Ok(Client {
            mode: args.mode(),
            thread_num: args.thread_num,
            remote_addr,
            remote_port: args.port,
            bind_addr: args.bind_addr.clone(),
            duration: args.duration,

            inner,
        })
    }
}

impl Client {
    pub async fn run(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let start_req = Request::new_start(self.mode, self.thread_num, self.remote_port);
        inner
            .csend(&start_req)
            .await
            .context("send start request")?;
        inner.crecv().await.context("recv start response")?;

        // start stress test

        match self.mode {
            Mode::Udp => self.run_udp().await.context("run udp")?,
            _ => return Err(anyhow!("not support mode")),
        }

        Ok(())
    }

    async fn run_udp(&self) -> Result<()> {
        let mut join_handles = Vec::with_capacity(self.thread_num);
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        for i in 0..self.thread_num {
            let addr = format!("{}:{}", self.remote_addr, self.remote_port);
            let bind_addr = self.bind_addr.clone();
            let worker_shutdown_rx = shutdown_rx.clone();
            join_handles.push(tokio::spawn(async move {
                debug!("start worker {}", i);
                udp_worker(i, addr, bind_addr, worker_shutdown_rx).await;
            }));
        }

        let complete = sleep(Duration::from_secs(self.duration));

        let all_workers_future = join_all(join_handles);
        pin!(all_workers_future);

        complete.await;
        shutdown_tx.send(()).context("send shutdown")?;
        all_workers_future.await;

        Ok(())
    }
}

async fn udp_worker(
    worker_id: usize,
    addr: String,
    bind_addr: Option<String>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let worker = async move {
        let buf = vec![0u8; 512];
        let mut total = 0;
        let mut succeeded = 0;
        let mut failed = 0;
        let mut bytes_wrote = 0;
        loop {
            let bind_addr = bind_addr.clone().unwrap_or("0.0.0.0:0".to_string());
            let addr = addr.clone();
            let socket = UdpSocket::bind(&bind_addr)
                .await
                .context("bind udp socket")?;

            tokio::select! {
                biased;

                _ = shutdown_rx.changed() => {
                    info!(
                        "worker {}: total={}, succeeded={}, failed={}, bytes_wrote={}",
                        worker_id, total, succeeded, failed, bytes_wrote
                    );
                    break;
                }

                result = socket.send_to(&buf, &addr) => {
                    total += 1;
                    match result {
                        Ok(n) => {
                            succeeded += 1;
                            bytes_wrote += n;
                        }
                        Err(err) => {
                            failed += 1;
                            trace!("worker {}: udp send_to error: {:?}", worker_id, err);
                        }
                    }
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    if let Err(err) = worker.await {
        error!(
            "worker {}: udp worker exited with error: {}",
            worker_id, err
        );
    }
}
