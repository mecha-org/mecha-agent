use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tracing::{info, trace};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/*
 * that allows you to generate random strings based on a given length.
 */
pub fn generate_random_alphanumeric(length: usize) -> String {
    trace!(
        func = "generate_random_alphanumeric",
        package = PACKAGE_NAME,
        "length - {}",
        length
    );
    let code: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    info!(
        func = "generate_random_alphanumeric",
        package = PACKAGE_NAME,
        "code - {}",
        code
    );
    code
}
