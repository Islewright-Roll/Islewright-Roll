// SPDX-License-Identifier: GPL-3.0-or-later
//! The pure parts of the board's flat layers: how far the glow round a buildable road is
//! greyed, how a glow's outline and ring grow from a mask, and how the footprints split into
//! two layers that never share a pixel. Colours are ARGB.

/// Java's Math.round on a float: halves go up, toward positive infinity.
pub fn round_f32(v: f32) -> i32 {
    (v + 0.5).floor() as i32
}

/// A colour with its saturation scaled by factor (1 leaves it alone, 0 is its grey). Each
/// colour channel is mixed toward the colour's own luma, so the brightness stays put and only
/// the colour drains out. The alpha byte is kept.
pub fn desaturate(argb: u32, factor: f32) -> u32 {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    let luma = (r * 299 + g * 587 + b * 114) as f32 / 1000.0;
    (argb & 0xFF000000)
        | (mix(luma, r as f32, factor) << 16)
        | (mix(luma, g as f32, factor) << 8)
        | mix(luma, b as f32, factor)
}

fn mix(luma: f32, channel: f32, factor: f32) -> u32 {
    round_f32(luma + (channel - luma) * factor).clamp(0, 255) as u32
}

/// A colour mixed toward white by factor (0 leaves it alone, 1 is white). Each channel moves
/// that share of the way to 255; the alpha byte is kept. The tint of the glow round a
/// resource being banked, which is the player's colour but lighter than a buildable spot's.
pub fn lighten(argb: u32, factor: f32) -> u32 {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    (argb & 0xFF000000) | (toward(r, factor) << 16) | (toward(g, factor) << 8) | toward(b, factor)
}

fn toward(channel: u32, factor: f32) -> u32 {
    round_f32(channel as f32 + (255.0 - channel as f32) * factor).clamp(0, 255) as u32
}

/// The colour of the soft ring outside a glow's outline: black round a colour light enough to
/// stand out of a shadow (brighter than light_above, 0 to 255), white round one that is not.
pub fn halo_rgb(rgb: u32, light_above: u32) -> u32 {
    let brightness =
        (((rgb >> 16) & 0xFF) * 299 + ((rgb >> 8) & 0xFF) * 587 + (rgb & 0xFF) * 114) / 1000;
    if brightness > light_above {
        0x000000
    } else {
        0xFFFFFF
    }
}

/// Grows a mask outward, one step per pass. `dist` holds 0 on the mask's pixels and -1
/// elsewhere; after the call, a pixel that is `step` steps outside the mask (4 way) holds
/// `step`, up to `reach`. `tier` holds each mask pixel's strength and `road_only` whether it
/// is road alone; a grown pixel takes the strongest tier of the pixels it grew from and is
/// road only when all of them are. A step grows only from the one before it, never from
/// pixels it has just marked. The one outline-and-ring maker: the glow round buildable spots
/// and the glow round a resource being banked are both this.
pub fn grow(
    dist: &mut [i32],
    tier: &mut [i32],
    road_only: &mut [bool],
    w: usize,
    h: usize,
    reach: i32,
) {
    for step in 1..=reach {
        for y in 0..h {
            for x in 0..w {
                let p = y * w + x;
                if dist[p] != -1 {
                    continue;
                }
                let mut best = 0;
                let mut from_roads_only = true;
                let mut consider = |q: usize| {
                    if dist[q] == step - 1 {
                        best = best.max(tier[q]);
                        from_roads_only &= road_only[q];
                    }
                };
                if x > 0 {
                    consider(p - 1);
                }
                if x < w - 1 {
                    consider(p + 1);
                }
                if y > 0 {
                    consider(p - w);
                }
                if y < h - 1 {
                    consider(p + w);
                }
                if best > 0 {
                    // Marked now, settled after the pass, so a step only grows from the one before
                    dist[p] = -2 - step;
                    tier[p] = best;
                    road_only[p] = from_roads_only;
                }
            }
        }
        for d in dist.iter_mut() {
            if *d == -2 - step {
                *d = step;
            }
        }
    }
}

/// The colour of a pixel at distance d from the mask (1 the outline, 2 the ring), or 0 for a
/// pixel that holds no glow. strength scales the opacity (1 leaves it as is).
pub fn glow_argb(
    d: i32,
    outline_rgb: u32,
    ring_rgb: u32,
    outline_alpha: u32,
    ring_alpha: u32,
    strength: f32,
) -> u32 {
    if d != 1 && d != 2 {
        return 0;
    }
    let alpha = round_f32((if d == 1 { outline_alpha } else { ring_alpha }) as f32 * strength)
        .clamp(0, 255) as u32;
    (alpha << 24) | ((if d == 1 { outline_rgb } else { ring_rgb }) & 0xFFFFFF)
}

/// The outline and ring round one flat mask (w by h, with at least two clear pixels all round
/// it), as ARGB pixels: grow, then colour. For a single silhouette in a single tier.
#[allow(clippy::too_many_arguments)]
pub fn outline(
    mask: &[bool],
    w: usize,
    h: usize,
    outline_rgb: u32,
    ring_rgb: u32,
    outline_alpha: u32,
    ring_alpha: u32,
    strength: f32,
) -> Vec<u32> {
    let mut dist: Vec<i32> = mask.iter().map(|&m| if m { 0 } else { -1 }).collect();
    let mut tier: Vec<i32> = mask.iter().map(|&m| if m { 1 } else { 0 }).collect();
    let mut road_only = vec![false; mask.len()];
    grow(&mut dist, &mut tier, &mut road_only, w, h, 2);
    dist.iter()
        .map(|&d| {
            glow_argb(
                d,
                outline_rgb,
                ring_rgb,
                outline_alpha,
                ring_alpha,
                strength,
            )
        })
        .collect()
}

/// The road footprint layer: every pixel of the roads' mask that no standing piece's mask
/// holds. The standing footprints draw over it from their own layer, so a pixel is in at most
/// one of the two and is never darkened twice.
pub fn cut_out(roads: &[bool], standing: &[bool]) -> Vec<bool> {
    roads.iter().zip(standing).map(|(&r, &s)| r && !s).collect()
}
