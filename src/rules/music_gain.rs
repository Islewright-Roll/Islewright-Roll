// SPDX-License-Identifier: GPL-3.0-or-later
//! How loud the Music volume setting is. The setting is a whole number from 0 to 100; the
//! gain handed to the player is that fraction squared. Loudness is heard roughly
//! logarithmically, so a straight fraction would feel as if the slider did nothing over its
//! top half and everything in the bottom; squaring spreads the steps more evenly to the ear.
//!
//! The gain applies on top of the device's media volume, which the hardware keys and the
//! system own: the app never reads or changes the media stream's volume.

/// The player's gain, 0 to 1, for a setting; out of range settings are pulled into range.
pub fn gain_for(setting: i32) -> f32 {
    let fraction = setting.clamp(0, 100) as f32 / 100.0;
    fraction * fraction
}
