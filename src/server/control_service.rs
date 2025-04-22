use anyhow::Result;
use log::{debug, error, trace};

use crate::Request;

pub(crate) async fn handle_control_request_in_worker(result: Result<usize>, buf: &[u8]) -> bool {
    match result {
        Ok(0) => {
            trace!("Control service closed by client, going to shutdown workers");
            true
        }
        Ok(n) => {
            let req: Request = match serde_json::from_slice(&buf[..n]) {
                Ok(req) => req,
                Err(err) => {
                    error!("Failed to deserilize control request: {}", err);
                    return true;
                }
            };

            debug!("Received a control request: {:?}", req);

            false
        }
        Err(err) => {
            error!("Failed to read control stream: {}", err);
            true
        }
    }
}
