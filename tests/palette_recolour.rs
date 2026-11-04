// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from PaletteRecolourTest: the dark look recolours the button art, the light look
//! leaves it alone. Also the team colour ramp it shares its palette with.

use dayapp::rules::palette::Palette;
use dayapp::rules::team_color;

#[test]
fn light_leaves_the_art_alone() {
    let l = Palette::LIGHT;
    assert_eq!(0xFF7A5A2C, l.recolour_field(0xFF7A5A2C, false));
    assert_eq!(0xFFA0783C, l.recolour_thumb(0xFFA0783C));
}

#[test]
fn dark_field_maps_each_art_colour() {
    let d = Palette::DARK;
    assert_eq!(0x00000000, d.recolour_field(0x00000000, false));
    assert_eq!(d.field_shadow, d.recolour_field(0xFF1E140A, false));
    assert_eq!(d.field_face, d.recolour_field(0xFF7A5A2C, true));
    assert_eq!(d.field_inner, d.recolour_field(0xFF5A3C14, false));
    assert_eq!(d.field_light, d.recolour_field(0xFF5A3C14, true));
}

#[test]
fn dark_thumb_keeps_its_bevel_tones() {
    let d = Palette::DARK;
    assert_eq!(d.thumb_face, d.recolour_thumb(0xFFA0783C));
    assert_ne!(d.recolour_thumb(0xFFA0783C), d.recolour_thumb(0xFFB48C4C));
    assert_eq!(d.thumb_outline, d.recolour_thumb(0xFF1E140A));
}

#[test]
fn the_team_colour_reference_shade_becomes_the_players_colour() {
    let ramp = team_color::ramp(&team_color::MAGENTA, 0xFFD32F2F);
    assert_eq!(0xD32F2F, ramp[0]);
    // Darker palette entries go toward black, lighter ones toward white
    assert!(ramp[1] & 0xFF0000 < ramp[0] & 0xFF0000);
    assert!(ramp[18] & 0xFF > ramp[0] & 0xFF);
    let mut pixels = [0xFFF49AC1, 0x80F49AC3, 0xFF123456, 0x00F49AC1];
    team_color::recolor(&mut pixels, &team_color::MAGENTA, 0xFFD32F2F);
    assert_eq!([0xFFD32F2F, 0x80D32F2F, 0xFF123456, 0x00F49AC1], pixels);
    let mut road = [0xFFF49AC1, 0xFF123456, 0x80123456];
    team_color::unpainted(&mut road, &team_color::MAGENTA);
    assert_eq!([0, 0xFF123456, 0], road);
}
