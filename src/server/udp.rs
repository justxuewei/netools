use std::sync::Arc;
use std::vec;

use anyhow::{Context, Result};
use futures::future::join_all;
use log::{info, trace};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::pin;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch;

use crate::server::control_service::handle_control_request_in_worker;

pub(crate) struct UdpStressTest {
    control_stream: TcpStream,
    socket: Arc<UdpSocket>,
    thread_num: usize,
}

impl UdpStressTest {
    pub(crate) async fn new(
        control_stream: TcpStream,
        addr: &str,
        port: u16,
        thread_num: usize,
    ) -> Result<Self> {
        let addr = format!("{}:{}", addr, port);
        let socket = UdpSocket::bind(addr)
            .await
            .context("new process udp listener")?;
        let socket = Arc::new(socket);

        Ok(Self {
            control_stream,
            socket,
            thread_num,
        })
    }
}

impl UdpStressTest {
    pub(crate) async fn run(&mut self, prepare_ok_tx: Sender<()>) -> Result<()> {
        let shared_socket = self.socket.clone();
        let mut buf = vec![0u8; 1024];
        let mut join_handles = Vec::with_capacity(self.thread_num);
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        info!(
            "UDP stress test listening at {:?} with {} threads",
            shared_socket.local_addr().context("local_addr")?,
            self.thread_num
        );

        for i in 0..self.thread_num {
            let worker_shutdown_rx = shutdown_rx.clone();
            let socket = shared_socket.clone();
            let handle = tokio::spawn(async move {
                udp_worker(i, socket, worker_shutdown_rx).await;
            });
            join_handles.push(handle);
        }

        prepare_ok_tx.send(()).await.context("send prepare ok")?;

        let all_workers_future = join_all(join_handles);
        pin!(all_workers_future);

        loop {
            let select_shutdown_tx = shutdown_tx.clone();
            tokio::select! {
                biased;

                result = self.control_stream.read(&mut buf) => {
                    let result = result.context("read control stream");
                    let shutdown = handle_control_request_in_worker(result, &buf).await;
                    // only do quit if we haven't already
                    if shutdown {
                        select_shutdown_tx.send(()).context("send shutdown")?;
                        break;
                    }
                }

                _ = &mut all_workers_future => {
                    break;
                }
            }
        }

        all_workers_future.await;

        info!("UDP stress test completed");

        Ok(())
    }
}

async fn udp_worker(
    worker_id: usize,
    socket: Arc<UdpSocket>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let mut buf = vec![0u8; 1024];
    let mut total = 0;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut bytes_recv = 0;

    loop {
        tokio::select! {
            biased;

            // Terminate the worker if received a signal or the rx closed.
            _ = shutdown_rx.changed() => {
                trace!("worker {}: shutdown due to shutdown signal received", worker_id);
                break;
            }

            result = socket.recv_from(&mut buf) => {
                total += 1;
                match result {
                    Ok((n, _)) => {
                        succeeded += 1;
                        bytes_recv += n;
                    }
                    Err(err) => {
                        failed += 1;
                        trace!("worker {}: udp recv_from error: {:?}", worker_id, err);
                    }
                }
            }
        }
    }

    info!(
        "worker {}: total={}, succeeded={}, failed={}, bytes_recv={}",
        worker_id, total, succeeded, failed, bytes_recv
    );
}
