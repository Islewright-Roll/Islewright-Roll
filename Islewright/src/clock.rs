// SPDX-License-Identifier: GPL-3.0-or-later
//! Milliseconds on the wall clock, on every target. The game's roll lockout, the delayed
//! sounds and the dates on top scores all read it.
//!
//! The web build has no working `std::time::SystemTime` (wasm32-unknown-unknown panics), so
//! there the page's own `Date.now()` answers, through Day's web toolkit.

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        day_toolkit_dom::now_epoch_ms()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
