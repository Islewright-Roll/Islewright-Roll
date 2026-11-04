// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from BoardLayersTest: the glow's colours, its outline and ring, and the two
//! footprint layers.

use dayapp::rules::board_layers as bl;

fn luma(argb: u32) -> i32 {
    bl::round_f32(
        (((argb >> 16) & 0xFF) * 299 + ((argb >> 8) & 0xFF) * 587 + (argb & 0xFF) * 114) as f32
            / 1000.0,
    )
}

fn chroma(argb: u32) -> i32 {
    let (r, g, b) = (
        ((argb >> 16) & 0xFF) as i32,
        ((argb >> 8) & 0xFF) as i32,
        (argb & 0xFF) as i32,
    );
    r.max(g).max(b) - r.min(g).min(b)
}

#[test]
fn factor_one_leaves_the_colour_alone() {
    assert_eq!(0xFF3A7BD5, bl::desaturate(0xFF3A7BD5, 1.0));
}

#[test]
fn factor_zero_is_the_grey_of_the_same_brightness() {
    let grey = bl::desaturate(0xFF3A7BD5, 0.0);
    assert!(chroma(grey) <= 1);
    assert!((luma(0xFF3A7BD5) - luma(grey)).abs() <= 1);
}

#[test]
fn desaturating_drains_colour_and_keeps_brightness_and_alpha() {
    let input = 0x80E02020;
    let out = bl::desaturate(input, 0.55);
    assert_eq!(0x80, out >> 24);
    assert!(chroma(out) < chroma(input));
    // Close to the factor: 55 percent of the colour is left
    assert!((chroma(input) as f64 * 0.55 - chroma(out) as f64).abs() <= 2.0);
    assert!((luma(input) - luma(out)).abs() <= 1);
}

#[test]
fn a_grey_has_nothing_to_drain() {
    assert_eq!(0xFF808080, bl::desaturate(0xFF808080, 0.55));
}

#[test]
fn channels_stay_in_range_for_extreme_colours() {
    assert_eq!(0xFFFFFFFF, bl::desaturate(0xFFFFFFFF, 0.3));
    assert_eq!(0xFF000000, bl::desaturate(0xFF000000, 0.3));
    // A pure green drains toward its grey without any channel wrapping past 0 or 255
    let out = bl::desaturate(0xFF00FF00, 0.55);
    assert!((out >> 8) & 0xFF > out & 0xFF);
    assert_eq!(0xFF, out >> 24);
}

#[test]
fn a_pixel_is_in_at_most_one_footprint_layer() {
    let roads = [true, true, false, true, false, true];
    let stands = [false, true, true, false, false, true];
    let road_layer = bl::cut_out(&roads, &stands);
    assert_eq!(vec![true, false, false, true, false, false], road_layer);
    for p in 0..roads.len() {
        assert!(!(road_layer[p] && stands[p]), "pixel {p}");
        // and nothing is lost: every pixel either layer would have drawn is still drawn
        assert_eq!(roads[p] || stands[p], road_layer[p] || stands[p]);
    }
}

#[test]
fn cut_out_leaves_the_inputs_alone() {
    let roads = [true, true];
    let stands = [false, true];
    bl::cut_out(&roads, &stands);
    assert_eq!([true, true], roads);
    assert_eq!([false, true], stands);
}

#[test]
fn lighten_moves_each_channel_part_way_to_white() {
    // 45 percent of the way: 0 -> 115, 255 stays, 100 -> 100 + 155 * 0.45 = 170
    assert_eq!(0xFF73AAFF, bl::lighten(0xFF0064FF, 0.45));
    assert_eq!(0x80FFFFFF, bl::lighten(0x80FFFFFF, 0.45));
}

#[test]
fn lighten_zero_leaves_it_alone_and_one_is_white() {
    assert_eq!(0xFF3A7BD5, bl::lighten(0xFF3A7BD5, 0.0));
    assert_eq!(0xFFFFFFFF, bl::lighten(0xFF3A7BD5, 1.0));
}

#[test]
fn lighten_keeps_alpha_and_makes_a_colour_brighter() {
    let input = 0xC0E02020;
    let out = bl::lighten(input, 0.45);
    assert_eq!(0xC0, out >> 24);
    assert!(luma(out) > luma(input));
    // and no more colourful than it was
    assert!(chroma(out) < chroma(input));
}

#[test]
fn halo_is_black_round_a_light_colour_and_white_round_a_dark_one() {
    assert_eq!(0x000000, bl::halo_rgb(0xFFFF80, 110));
    assert_eq!(0xFFFFFF, bl::halo_rgb(0x102040, 110));
}

/// A 3 by 3 block in the middle of a 9 by 9 canvas: ring one outline, ring two the halo.
#[test]
fn outline_is_one_pixel_round_the_mask_and_then_a_ring() {
    let w = 9;
    let mut mask = vec![false; w * w];
    for y in 3..6 {
        for x in 3..6 {
            mask[y * w + x] = true;
        }
    }
    let out = bl::outline(&mask, w, w, 0x112233, 0xFFFFFF, 160, 60, 1.0);
    // inside: nothing; straight out: the outline; one further: the ring; beyond: nothing
    assert_eq!(0, out[4 * w + 4]);
    assert_eq!(0xA0112233, out[4 * w + 2]);
    assert_eq!(0xA0112233, out[2 * w + 4]);
    assert_eq!(0x3CFFFFFF, out[4 * w + 1]);
    assert_eq!(0, out[4 * w]);
    // it grows four way, so the corner beyond the block's corner is not outline
    assert_eq!(0x3CFFFFFF, out[2 * w + 2]);
}

#[test]
fn outline_strength_scales_the_opacity_of_both() {
    let mut mask = vec![false; 25];
    mask[12] = true;
    let full = bl::outline(&mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 1.0);
    let half = bl::outline(&mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 0.5);
    assert_eq!(200, full[11] >> 24);
    assert_eq!(100, half[11] >> 24);
    assert_eq!(100, full[10] >> 24);
    assert_eq!(50, half[10] >> 24);
    assert_eq!(
        0,
        bl::outline(&mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 0.0)[11] >> 24
    );
}

/// grow is what the buildable spots' glow runs: the stronger tier owns a shared pixel.
#[test]
fn grow_gives_a_shared_pixel_to_the_stronger_tier() {
    let w = 5;
    let mut dist = vec![-1; w];
    let mut tier = vec![0; w];
    let mut road_only = vec![false; w];
    dist[0] = 0;
    tier[0] = 4;
    dist[2] = 0;
    tier[2] = 2;
    road_only[2] = true;
    bl::grow(&mut dist, &mut tier, &mut road_only, w, 1, 1);
    assert_eq!(1, dist[1]);
    assert_eq!(4, tier[1]);
    assert!(!road_only[1]);
    assert_eq!(1, dist[3]);
    assert_eq!(2, tier[3]);
    assert!(road_only[3]);
    assert_eq!(-1, dist[4]);
}
