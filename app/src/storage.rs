//! The last completed level index, and nothing else. Per-level progress is
//! deliberately not persisted.

pub use backend::{load_last_completed, save_last_completed};

#[cfg(target_arch = "wasm32")]
mod backend {
    const KEY: &str = "water-sort.last-completed";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load_last_completed() -> Option<usize> {
        storage()?.get_item(KEY).ok()??.parse().ok()
    }

    pub fn save_last_completed(index: usize) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(KEY, &index.to_string());
        }
    }
}

/// Native is a development target, so progress only lives as long as the
/// process.
#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::sync::Mutex;

    static LAST_COMPLETED: Mutex<Option<usize>> = Mutex::new(None);

    pub fn load_last_completed() -> Option<usize> {
        *LAST_COMPLETED.lock().ok()?
    }

    pub fn save_last_completed(index: usize) {
        if let Ok(mut last) = LAST_COMPLETED.lock() {
            *last = Some(index);
        }
    }
}
