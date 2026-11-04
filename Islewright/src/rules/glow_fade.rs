// SPDX-License-Identifier: GPL-3.0-or-later
//! On an island whose traders are hired in order, the next one glows in full and the ones
//! after it fade: half, then a fifth, then nothing, so the eye goes to the trader that can
//! actually be hired.

pub const STRENGTH: [f32; 3] = [1.0, 0.5, 0.2];

/// Which step of the fade this trader is on: 0 for the next one to hire, 1 for the one after
/// it, and so on, counting the traders not yet hired that come before it in the order. None
/// for a trader already hired (and for a tier that does not exist). `hired` is
/// Playsheet::traders: 1 based by tier, entry 0 unused.
pub fn step(hired: &[bool], tier: usize) -> Option<usize> {
    if tier < 1 || tier >= hired.len() || hired[tier] {
        return None;
    }
    Some((1..tier).filter(|&earlier| !hired[earlier]).count())
}

/// The glow's strength at a step, 0 to 1; none past the last step or for no step.
pub fn strength(step: Option<usize>) -> f32 {
    step.and_then(|s| STRENGTH.get(s).copied()).unwrap_or(0.0)
}
