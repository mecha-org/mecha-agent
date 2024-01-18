use std::time::Duration;

use anyhow::{bail, Result};
use tokio::time::{timeout_at, Instant};
use tracing::error;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
static CHANNEL_RECV_TIMEOUT: u64 = 2000;
pub async fn recv_with_timeout<T>(rx: tokio::sync::oneshot::Receiver<Result<T>>) -> Result<T> {
    let fn_name = "recv_with_timeout";
    let timeout_duration = Duration::from_millis(CHANNEL_RECV_TIMEOUT);
    let timeout = Instant::now() + timeout_duration;
    tokio::select! {
        result = timeout_at(timeout, rx) => match result {
            Ok(msg) => match msg {
                Ok(msg) => return msg,
                Err(err) => {
                    error!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "error while receiving message from channel - {}", err);
                        bail!(err);
                    },
                },
            Err(err) => {
                error!(
                    func = "recv_with_timeout",
                    package = PACKAGE_NAME,
                    "error while receiving message from channel - {}", err
                );
                bail!(err)},
        },
        _ = tokio::time::sleep(timeout_duration) => {
            error!(
                func = "recv_with_timeout",
                package = PACKAGE_NAME,
                "timeout while receiving message from channel"
            );
            bail!("timeout while receiving message from channel");
        }
    };
}
pub async fn recv_with_custom_timeout<T>(
    duration: u64,
    rx: tokio::sync::oneshot::Receiver<Result<T>>,
) -> Result<T> {
    let fn_name = "recv_with_custom_timeout";
    let timeout_duration = Duration::from_millis(duration);
    let timeout = Instant::now() + timeout_duration;
    tokio::select! {
        result = timeout_at(timeout, rx) => match result {
            Ok(msg) => match msg {
                Ok(msg) => return msg,
                Err(err) => {
                    error!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "error while receiving message from channel - {}", err);
                        bail!(err);
                    },
                },
            Err(err) => {
                error!(
                    func = "recv_with_custom_timeout",
                    package = PACKAGE_NAME,
                    "error while receiving message from channel - {}", err
                );
                bail!(err)},
        },
        _ = tokio::time::sleep(timeout_duration) => {
            error!(
                func = "recv_with_custom_timeout",
                package = PACKAGE_NAME,
                "timeout while receiving message from channel"
            );
            bail!("timeout while receiving message from channel");
        }
    };
}
