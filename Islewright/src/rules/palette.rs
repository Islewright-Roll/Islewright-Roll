// SPDX-License-Identifier: GPL-3.0-or-later
//! Every colour the pixel kit paints with, in one place, for one look. The dark values are
//! the "dark kit" palette in tools/pieces.txt, with its contrast ratios. Colours are ARGB.
//!
//! The brass buttons are the same in both looks, so their label colours are not here.

use super::high_scores::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// True for the dark look. Used only by the drawables whose art differs, never by screens.
    pub dark: bool,
    /// Body text, titles, the score rows.
    pub ink: u32,
    /// Descriptions and other quiet text.
    pub secondary: u32,
    pub link: u32,
    /// The line under a title and the dividers between rows.
    pub rule: u32,
    /// Text typed into a recessed field, and its hint.
    pub field_text: u32,
    pub field_hint: u32,
    /// The mark in a radio box or a check box.
    pub mark: u32,
    /// A recessed field, radio box or check box: face, outline, bottom and right inner light
    /// (the dark look recolours the pressed button art with these).
    pub field_face: u32,
    pub field_shadow: u32,
    pub field_light: u32,
    /// The inner shade along the top and left of a dark recess, between its outline and its face.
    pub field_inner: u32,
    /// The slider: groove outline, groove face, groove top edge, thumb outline, face, top edge.
    pub track_outline: u32,
    pub track_face: u32,
    pub track_top: u32,
    pub thumb_outline: u32,
    pub thumb_face: u32,
    pub thumb_top: u32,
    /// The slider thumb's other tones in the dark look: lip, inner edge, shade, pressed face.
    pub thumb_lip: u32,
    pub thumb_edge: u32,
    pub thumb_shade: u32,
    pub thumb_pressed: u32,
    pub scroll_thumb: u32,
    /// The outline around the colour preview.
    pub frame: u32,
}

impl Palette {
    /// Parchment and brass: the look the game shipped with.
    pub const LIGHT: Palette = Palette {
        dark: false,
        ink: 0xFF2A1A0E,
        secondary: 0xFF5A3C14,
        link: 0xFF8C3A10,
        rule: 0xFFC8A860,
        field_text: 0xFFFFF4D6,
        field_hint: 0xFFC0BCB0,
        mark: 0xFFFFF4D6,
        field_face: 0xFF7A5A2C,
        field_shadow: 0xFF1E140A,
        field_light: 0xFFA0783C,
        field_inner: 0xFF5A3C14,
        track_outline: 0xFF1E140A,
        track_face: 0xFF7A5A2C,
        track_top: 0xFF5A3C14,
        thumb_outline: 0xFF1E140A,
        thumb_face: 0xFFA0783C,
        thumb_top: 0xFFE0B870,
        thumb_lip: 0xFFB48C4C,
        thumb_edge: 0xFF86622E,
        thumb_shade: 0xFF5A3C14,
        thumb_pressed: 0xFF7A5A2C,
        scroll_thumb: 0xFF5A3C14,
        frame: 0xFF1E140A,
    };

    /// Darker wood: tools/pieces.txt, "dark kit".
    pub const DARK: Palette = Palette {
        dark: true,
        ink: 0xFFD2BE94,
        secondary: 0xFFB8A47C,
        link: 0xFF7DB8D8,
        rule: 0xFFB8914F,
        field_text: 0xFFD2BE94,
        field_hint: 0xFFB8A47C,
        mark: 0xFFE0B870,
        field_face: 0xFF1E150C,
        field_shadow: 0xFF120B05,
        field_light: 0xFF5A4630,
        field_inner: 0xFF181008,
        track_outline: 0xFF120B05,
        track_face: 0xFF1E150C,
        track_top: 0xFF120B05,
        thumb_outline: 0xFF120B05,
        thumb_face: 0xFFB8914F,
        thumb_top: 0xFFE0B870,
        thumb_lip: 0xFFC9A45E,
        thumb_edge: 0xFF8F6F3A,
        thumb_shade: 0xFF6E5530,
        thumb_pressed: 0xFF9A7A40,
        scroll_thumb: 0xFFB8A47C,
        frame: 0xFFB8914F,
    };

    /// One colour of the pressed button art (the recessed field, radio box and check box) as
    /// this palette paints it. The art has four colours: transparent, outline, face and one
    /// shade used for the top and left bevel and the bottom and right inner line, told apart
    /// by lower_right. The light palette returns the colour untouched.
    pub fn recolour_field(&self, argb: u32, lower_right: bool) -> u32 {
        if !self.dark || (argb >> 24) == 0 {
            return argb;
        }
        match argb & 0xFFFFFF {
            0x1E140A => self.field_shadow,
            0x7A5A2C => self.field_face,
            0x5A3C14 => {
                if lower_right {
                    self.field_light
                } else {
                    self.field_inner
                }
            }
            _ => argb,
        }
    }

    /// One colour of the button art (the slider thumb, any state) as this palette paints it.
    pub fn recolour_thumb(&self, argb: u32) -> u32 {
        if !self.dark || (argb >> 24) == 0 {
            return argb;
        }
        match argb & 0xFFFFFF {
            0x1E140A => self.thumb_outline,
            0xE0B870 | 0xFFF4D6 => self.thumb_top,
            0xA0783C => self.thumb_face,
            0x5A3C14 => self.thumb_shade,
            0xB48C4C => self.thumb_lip,
            0x86622E => self.thumb_edge,
            0x7A5A2C => self.thumb_pressed,
            0x8C8880 => 0xFF6A655A,
            0x6E6A60 => 0xFF4A463E,
            0x7C7870 => 0xFF5A564C,
            0x5E5A52 => 0xFF3E3A33,
            _ => argb,
        }
    }

    /// Whether a Theme setting, with the OS in night mode or not, means the dark look.
    pub fn is_dark(theme: Theme, os_night: bool) -> bool {
        match theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => os_night,
        }
    }

    pub fn for_mode(theme: Theme, os_night: bool) -> &'static Palette {
        if Palette::is_dark(theme, os_night) {
            &Palette::DARK
        } else {
            &Palette::LIGHT
        }
    }
}
