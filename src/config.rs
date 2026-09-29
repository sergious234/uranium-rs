// region:    --- Modules
use std::sync::RwLock;
// endregion: --- Modules

/// Configuration for downloaders.
#[derive(Debug, Clone, Copy)]
pub struct DownloaderConfig {
    max_concurrent: usize,
}

impl Default for DownloaderConfig {
    /// Returns a DownloaderConfig with max_concurrent set to DEFAULT_NTHREADS.
    fn default() -> Self {
        Self {
            max_concurrent: DEFAULT_NTHREADS,
        }
    }
}

impl DownloaderConfig {
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }

    #[must_use]
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max.max(1);
        self
    }

    #[must_use]
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

// region:    --- Globals (deprecated, kept for compat)

/// In case `NTHREADS` can't be read this value is returned.
pub const DEFAULT_NTHREADS: usize = 8;

/// Global concurrent limit — **deprecated**, prefer `DownloaderConfig`.
/// Kept so `crate::common::config::num_threads()` and `crate::set_threads`
/// continue to work without breaking existing callers.
pub static NTHREADS: RwLock<usize> = RwLock::new(DEFAULT_NTHREADS);

/// Returns the current global thread limit, falling back to `DEFAULT_NTHREADS`.
pub fn num_threads() -> usize {
    NTHREADS
        .read()
        .map(|g| *g)
        .unwrap_or(DEFAULT_NTHREADS)
}

/// Sets the global thread limit. Returns `None` if the lock is poisoned.
pub fn set_global_threads(t: usize) -> Option<()> {
    let mut g = NTHREADS.write().ok()?;
    *g = t.max(1);
    Some(())
}

// endregion: --- Globals
