use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn is_user_writing(last_used_at: &Arc<AtomicU64>, timeout_ms: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64;
    let last_used = last_used_at.load(Ordering::Relaxed);

    now.saturating_sub(last_used) < timeout_ms
}
