use std::sync::atomic::{AtomicU64, AtomicUsize};

#[derive(Debug, Default)]
pub struct OptimizationStats {
    pub processed: AtomicUsize,
    pub saved_bytes: AtomicU64,
    pub errors: AtomicUsize,
}
