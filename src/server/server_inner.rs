use anyhow::{Context, Result};
use log::error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::channel;

use crate::server::UdpStressTest;
use crate::{Request, Response};

pub(crate) struct ServerInner {
    control_listener: TcpListener,
    buffer: Vec<u8>,
}

impl ServerInner {
    pub(crate) async fn new(addr: &str, port: u16) -> Result<Self> {
        let control_listener = TcpListener::bind(format!("{}:{}", addr, port))
            .await
            .context("control server bind")?;

        let buffer = vec![0; 1024];

        Ok(Self {
            control_listener,
            buffer,
        })
    }
}

impl ServerInner {
    pub(crate) async fn start_stress_test(&mut self, bind_addr: &str) -> Result<()> {
        let (mut control_stream, _) = self.control_listener.accept().await.context("accept")?;

        let bytes_read = control_stream
            .read(self.buffer.as_mut())
            .await
            .context("read from control service")?;
        let req: Request =
            serde_json::from_slice(&self.buffer[..bytes_read]).context("json to request")?;
        let Request::Start(req) = &req;

        let (control_stream, mut control_stream1) =
            try_clone_stream(control_stream).context("clone control stream")?;

        let mut process = UdpStressTest::new(control_stream, bind_addr, req.port, req.thread_num)
            .await
            .context("new streass test process")?;

        let (server_started_tx, mut server_started_rx) = channel(1);

        tokio::spawn(async move {
            if let Err(err) = process.run(server_started_tx).await {
                error!("Stress test process failed: {:?}", err);
            }
        });

        server_started_rx
            .recv()
            .await
            .context("recv server started")?;

        control_stream1
            .write_all(
                serde_json::to_string(&Response::new_ok(()))
                    .context("response to json")?
                    .as_bytes(),
            )
            .await
            .context("send ok response")?;

        Ok(())
    }
}

fn try_clone_stream(stream: TcpStream) -> Result<(TcpStream, TcpStream)> {
    let std_stream = stream.into_std().context("convert to std")?;
    let std_stream1 = std_stream.try_clone().context("try clone")?;

    Ok((
        TcpStream::from_std(std_stream).context("convert to tokio")?,
        TcpStream::from_std(std_stream1).context("convert to tokio")?,
    ))
}
