use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tracing::{info, trace};

/*
 * that allows you to generate random strings based on a given length.
 */
pub fn generate_random_alphanumeric(length: usize) -> String {
    trace!(
        task = "generate_random_alphanumeric",
        "init length - {}",
        length
    );
    let code: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    info!(task = "generate_random_alphanumeric", "code generated");
    code
}
