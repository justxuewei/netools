use std::vec;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{Request, Response};

pub(crate) struct ClientInner {
    control_stream: TcpStream,
    buffer: Vec<u8>,
}

impl ClientInner {
    pub(crate) async fn new(addr: &str, port: u16) -> Result<Self> {
        let control_stream = TcpStream::connect(format!("{}:{}", addr, port))
            .await
            .context("connect to control service")?;

        let buffer = vec![0; 1024];

        Ok(ClientInner {
            control_stream,
            buffer,
        })
    }
}

impl ClientInner {
    /// Send request to server endpoint of control service.
    pub(crate) async fn csend(&mut self, req: &Request) -> Result<()> {
        let req_bytes = serde_json::to_string(req).context("request to json")?;
        self.control_stream
            .write_all(req_bytes.as_bytes())
            .await
            .context("write")?;
        Ok(())
    }

    /// Receive response from server endpoint of control service.
    pub(crate) async fn crecv<T>(&mut self) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let bytes_read = self
            .control_stream
            .read(self.buffer.as_mut())
            .await
            .context("read")?;
        let rsp = serde_json::from_slice::<Response<T>>(&self.buffer[..bytes_read])
            .context("json to response")?;

        match rsp {
            Response::Ok(rsp) => Ok(rsp),
            Response::Err(err) => Err(anyhow!("{}", err)),
        }
    }
}
