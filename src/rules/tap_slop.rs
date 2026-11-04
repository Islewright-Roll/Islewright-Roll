// SPDX-License-Identifier: GPL-3.0-or-later
//! Whether a touch was a tap or a swipe. A finger that lands on one spot and lifts on another
//! is crossing the screen, not choosing the second spot, so it must never build.

/// The system's touch slop is the travel the platform itself forgives in a tap. The board's
/// targets are small, so a tap wobbles more than a list item's; twice the system figure keeps
/// those taps and still refuses a swipe.
pub const SLOP_FACTOR: f32 = 2.0;

/// True when the touch ended no further from where it began than slop_px (screen pixels).
pub fn is_tap(down_x: f32, down_y: f32, up_x: f32, up_y: f32, slop_px: f32) -> bool {
    let dx = up_x - down_x;
    let dy = up_y - down_y;
    dx * dx + dy * dy <= slop_px * slop_px
}
