// SPDX-License-Identifier: GPL-3.0-or-later
//! Battle for Wesnoth team colouring, ported from src/color_range.cpp recolor_palette(). A
//! sprite carries a fixed magenta palette (the horseman's trim, the house roofs, the road
//! kerbs) whose first entry is the reference shade. Each palette colour maps onto a ramp
//! through the player's colour: at or below the reference's brightness it runs from the
//! player colour down to black, above it from the player colour up to white.
//!
//! These work on plain ARGB pixel buffers; caching the recoloured sprites per colour is the
//! UI's business.

pub const MAGENTA: [u32; 19] = [
    0xF49AC1, 0x3F0016, 0x55002A, 0x690039, 0x7B0045, 0x8C0051, 0x9E005D, 0xB10069, 0xC30074,
    0xD6007F, 0xEC008C, 0xEE3D96, 0xEF5BA1, 0xF172AC, 0xF287B6, 0xF6ADCD, 0xF8C1D9, 0xFAD5E5,
    0xFDE9F1,
];

/// Edge pixels are partly transparent, and a premultiplied bitmap gives them back a shade or
/// two off their palette colour. Match them this loosely.
pub const SOFT_TOLERANCE: u32 = 8;

use super::board_layers::round_f32;

/// The pixels (ARGB) recoloured for the player's colour: every palette pixel goes onto the
/// ramp, keeping its alpha.
pub fn recolor(pixels: &mut [u32], palette: &[u32], player_color: u32) {
    let ramp = ramp(palette, player_color | 0xFF000000);
    for px in pixels.iter_mut() {
        let alpha = *px >> 24;
        if alpha == 0 {
            continue;
        }
        let rgb = *px & 0xFFFFFF;
        if let Some(m) = find(palette, rgb, if alpha == 255 { 0 } else { SOFT_TOLERANCE }) {
            *px = (alpha << 24) | ramp[m];
        }
    }
}

/// The sprite without its team colour: only the fully opaque pixels that are not in the
/// palette, everything else clear. For a road that is the cobbled surface with the kerb taken
/// off. Independent of the player colour, so the caller keeps the result.
pub fn unpainted(pixels: &mut [u32], palette: &[u32]) {
    for px in pixels.iter_mut() {
        if (*px >> 24) != 255 || find(palette, *px & 0xFFFFFF, 0).is_some() {
            *px = 0;
        }
    }
}

/// Each palette entry's colour for this player colour.
pub fn ramp(palette: &[u32], player_color: u32) -> Vec<u32> {
    let ref_avg = average(palette[0]);
    palette
        .iter()
        .map(|&entry| {
            let avg = average(entry);
            let mut out = 0;
            for shift in [16, 8, 0] {
                let mid = ((player_color >> shift) & 0xFF) as f32;
                let value = if avg <= ref_avg {
                    round_f32(mid * (avg as f32 / ref_avg as f32))
                } else {
                    round_f32(255.0 + (mid - 255.0) * ((255 - avg) as f32 / (255 - ref_avg) as f32))
                };
                out |= (value.clamp(0, 255) as u32) << shift;
            }
            out
        })
        .collect()
}

fn average(rgb: u32) -> u32 {
    (((rgb >> 16) & 0xFF) + ((rgb >> 8) & 0xFF) + (rgb & 0xFF)) / 3
}

/// The palette entry this colour is, exactly or (with a tolerance) the nearest one within it.
pub fn find(palette: &[u32], rgb: u32, tolerance: u32) -> Option<usize> {
    let mut best = None;
    let mut best_distance = tolerance + 1;
    for (i, &entry) in palette.iter().enumerate() {
        if entry == rgb {
            return Some(i);
        }
        if tolerance > 0 {
            let d = [16, 8, 0]
                .iter()
                .map(|&s| {
                    (((entry >> s) & 0xFF) as i32 - ((rgb >> s) & 0xFF) as i32).unsigned_abs()
                })
                .max()
                .unwrap_or(0);
            if d < best_distance {
                best_distance = d;
                best = Some(i);
            }
        }
    }
    best
}
