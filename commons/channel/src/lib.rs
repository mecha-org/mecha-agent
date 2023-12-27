use std::time::Duration;

use anyhow::{bail, Result};
use tokio::{
    select,
    time::{self, timeout_at, Instant},
};
static CHANNEL_RECV_TIMEOUT: u64 = 5000;
pub async fn recv_with_timeout<T>(rx: tokio::sync::oneshot::Receiver<Result<T>>) -> Result<T> {
    let timeout_duration = Duration::from_millis(CHANNEL_RECV_TIMEOUT);
    let timeout = Instant::now() + timeout_duration;
    let mut interval = time::interval(timeout_duration);
    tokio::select! {
        result = timeout_at(timeout, rx) => match result {
            Ok(msg) => match msg {
                Ok(msg) => return msg,
                Err(err) => bail!(err),
            },
            Err(err) => bail!(err),
        },
        _ = interval.tick() => {
            bail!("timeout")
        },
        // _ = tokio::time::sleep(timeout_duration) => bail!("timeout"),
    };
}
