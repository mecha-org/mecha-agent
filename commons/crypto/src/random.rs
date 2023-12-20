use rand::{distributions::Alphanumeric, thread_rng, Rng};

/*
 * that allows you to generate random strings based on a given length.
 */
pub fn generate_random_alphanumeric(length: usize) -> String {
    let code: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    code
}
