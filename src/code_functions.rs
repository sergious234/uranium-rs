use crate::variables::constants::{DEFAULT_NTHREADS, NTHREADS};

#[allow(non_snake_case)]
#[allow(unused)]
/// Returns the actual max threads allowed.
pub fn N_THREADS() -> usize {
    match NTHREADS.read() {
        Ok(e) => *e,
        Err(_) => DEFAULT_NTHREADS,
    }
}
