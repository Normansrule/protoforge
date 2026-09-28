//! One-entry memo caches for expensive analyses.
//!
//! Requirement verification re-runs every module's `metrics()` on each
//! project edit. Most edits touch one lab's parameters, so each expensive
//! module caches its last result keyed on the exact inputs it reads.

use std::cell::RefCell;
use std::thread::LocalKey;

/// Slot type for [`memo`]: `thread_local! { static SLOT: Slot<MyType> = const { RefCell::new(None) }; }`
pub type Slot<T> = RefCell<Option<(String, T)>>;

/// Return the cached value when `key` matches the last call, otherwise compute and store it.
pub fn memo<T: Clone + 'static>(slot: &'static LocalKey<Slot<T>>, key: String, f: impl FnOnce() -> T) -> T {
    if let Some(v) = slot.with(|s| s.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, v)| v.clone())) {
        return v;
    }
    let v = f();
    slot.with(|s| *s.borrow_mut() = Some((key, v.clone())));
    v
}

/// Build a cache key from any serialisable inputs.
pub fn key<T: serde::Serialize>(inputs: &T) -> String {
    serde_json::to_string(inputs).unwrap_or_default()
}
