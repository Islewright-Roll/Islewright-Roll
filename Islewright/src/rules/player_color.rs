// SPDX-License-Identifier: GPL-3.0-or-later
//! The player's chosen board colour, one opaque ARGB value in memory and one "#RRGGBB" hex
//! string in the settings document. The presets live here because the settings default
//! needs a value with nothing else loaded, and the board tint does not vary with the light
//! or dark theme.

/// Material 700 shades, chosen to stay distinct from each other and from the board artwork's
/// sand, sea and the green "can build" highlight.
pub const PRESETS: [u32; 6] = [
    0xFFD32F2F, // red
    0xFFF5BB30, // orange-yellow
    0xFF1B8E26, // green
    0xFF1718E1, // blue
    0xFF6922A2, // purple
    0xFF763410, // brown
];

pub const DEFAULT: u32 = PRESETS[0];

/// Parses a hex colour string permissively, so a power user's hand-edited document still
/// loads. Accepts an optional leading "#", surrounding white space, any case, and 3, 6 or 8
/// hex digits ("F00" expands to "FF0000"; 8 digits are AARRGGBB with the alpha discarded,
/// since the store never keeps an alpha).
pub fn parse_hex(s: &str) -> Result<u32, String> {
    let trimmed = s.trim();
    let mut hex: String = trimmed.strip_prefix('#').unwrap_or(trimmed).to_owned();
    if hex.chars().count() == 3 {
        hex = hex.chars().flat_map(|c| [c, c]).collect();
    }
    if !hex.is_ascii() || (hex.len() != 6 && hex.len() != 8) {
        return Err(format!("not a hex color: {s}"));
    }
    // Keep only the last 6 digits (RRGGBB); an 8 digit value's leading pair is alpha
    let rgb = &hex[hex.len() - 6..];
    u32::from_str_radix(rgb, 16)
        .map(|v| 0xFF000000 | v)
        .map_err(|_| format!("not a hex color: {s}"))
}

/// Always "#RRGGBB" upper case, alpha dropped.
pub fn to_hex(color: u32) -> String {
    format!("#{:06X}", color & 0xFFFFFF)
}

/// A color from its hue in degrees, and saturation and value from 0 to 1, opaque.
pub fn hsv_to_argb(hue: f64, sat: f64, value: f64) -> u32 {
    let h = hue.rem_euclid(360.0) / 60.0;
    let c = value * sat;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = value - c;
    let byte = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    0xFF000000 | (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

/// The hue in degrees, saturation and value of a color, as Color.colorToHSV has them.
pub fn argb_to_hsv(argb: u32) -> (f64, f64, f64) {
    let ch = |shift: u32| ((argb >> shift) & 0xFF) as f64 / 255.0;
    let (r, g, b) = (ch(16), ch(8), ch(0));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let hue = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    (hue, if max == 0.0 { 0.0 } else { d / max }, max)
}

pub fn with_alpha(color: u32, alpha: u32) -> u32 {
    (color & 0xFFFFFF) | ((alpha & 0xFF) << 24)
}

/// SRC_OVER composite of color at alpha over an opaque background, for previews.
pub fn over(color: u32, alpha: u32, background: u32) -> u32 {
    let a = (alpha & 0xFF) as f32 / 255.0;
    let mix = |shift: u32| {
        let c = ((color >> shift) & 0xFF) as f32;
        let b = ((background >> shift) & 0xFF) as f32;
        crate::rules::board_layers::round_f32(c * a + b * (1.0 - a)) as u32
    };
    0xFF000000 | (mix(16) << 16) | (mix(8) << 8) | mix(0)
}
