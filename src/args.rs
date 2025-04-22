use clap::Parser;

use crate::Mode;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    /// Server.
    pub server: bool,
    #[arg(short, long)]
    /// Client.
    pub client: bool,
    #[arg(short, long, default_value_t = 8080)]
    /// Port for stress test.
    /// This field is set by client, and will be sent to server by control
    /// service.
    pub port: u16,
    #[arg(long, default_value_t = 8081)]
    /// Control port.
    pub control_port: u16,
    #[arg(long)]
    /// Bind address, optional.
    pub bind_addr: Option<String>,
    #[arg(short, long)]
    /// Server address, must be set for client.
    pub addr: Option<String>,
    #[arg(short, long, default_value_t = 10)]
    /// Stress test duration in seconds.
    pub duration: u64,
    #[arg(short, long, default_value_t = String::from("tcp"))]
    /// Mode, possible values: "tcp", "udp".
    pub mode: String,
    #[arg(short, long, default_value_t = 1)]
    /// The number of threads, specifically, tokio threads (coroutines).
    pub thread_num: usize,
}

impl Args {
    pub fn mode(&self) -> Mode {
        match self.mode.as_str() {
            "tcp" => Mode::Tcp,
            "udp" => Mode::Udp,
            _ => Mode::Tcp,
        }
    }
}
