use serde::{Deserialize, Serialize};

use crate::Mode;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Start(StartRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartRequest {
    pub mode: Mode,
    pub thread_num: usize,
    pub port: u16,
}

impl Request {
    pub fn new_start(mode: Mode, thread_num: usize, port: u16) -> Self {
        Self::Start(StartRequest {
            mode,
            thread_num,
            port,
        })
    }
}
