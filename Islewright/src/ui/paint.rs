// SPDX-License-Identifier: GPL-3.0-or-later
//! The kit's art and the drawing every widget shares: the brass button frames, the parchment
//! and dark wood panels, the recessed boxes of check boxes and radio buttons, the focus ring and
//! the small icons, all drawn at one art pixel per image pixel (the port of PixelFrameDrawable,
//! PixelCheckBox and PixelRadioButton). Pure drawing onto `art::Image`, so it is unit tested.

use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use crate::art::Image;
use crate::rules::palette::Palette;
use crate::view::assets::{FONT_BODY, FONT_DISPLAY, FONT_DISPLAY_TXT, FONT_TEXT_TXT};
use crate::view::font::BitmapFont;

/// The corner of the 24 px nine-slice frames.
pub const CORNER: i32 = 6;
/// The brass buttons look the same everywhere, so their label colors are constants (PixelUi).
pub const BUTTON_TEXT: u32 = 0xFFFFF4D6;
pub const BUTTON_TEXT_DISABLED: u32 = 0xFFC0BCB0;
/// The focus ring: a dark line outside a cream one, so it shows on parchment and on wood alike.
const RING_OUTER: u32 = 0xFF1E140A;
const RING_INNER: u32 = 0xFFFFF4D6;
/// The highlight behind a focused row that has no frame of its own (a check box, a radio).
const ROW_FOCUS: u32 = 0x40A0783C;

thread_local! {
    static CURRENT: Cell<Palette> = const { Cell::new(Palette::LIGHT) };
    static CHANGES: Cell<Option<day::prelude::Signal<u64>>> = const { Cell::new(None) };
}

/// Counts the look's changes. Every widget reads it as it draws, so a screen changes look in
/// place, as PixelDialog.restyleOpen had it, keeping its scroll position.
pub fn palette_changes() -> day::prelude::Signal<u64> {
    CHANGES.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = day::prelude::Signal::global(0);
            c.set(Some(s));
            s
        }
    })
}

/// The look screens are drawn in right now.
pub fn palette() -> Palette {
    CURRENT.with(Cell::get)
}

/// Makes `p` the look, and repaints what is on screen when that changes it.
pub fn set_palette(p: Palette) {
    if CURRENT.with(|c| c.replace(p)) != p {
        palette_changes().update(|n| *n += 1);
    }
}

macro_rules! art {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resource/assets/art/",
            $name,
            ".png"
        ))
    };
}

fn load(bytes: &[u8]) -> Image {
    // The art is part of the build, so a PNG that does not decode is a broken build
    Image::from_png(bytes).expect("bundled art decodes")
}

/// The fonts and frames, decoded once.
pub struct Kit {
    pub body: BitmapFont,
    pub display: BitmapFont,
    pub button: Image,
    pub pressed: Image,
    pub disabled: Image,
    pub panel: Image,
    pub panel_dark: Image,
}

impl Kit {
    fn new() -> Kit {
        Kit {
            body: BitmapFont::parse(FONT_BODY, FONT_TEXT_TXT).expect("bundled font"),
            display: BitmapFont::parse(FONT_DISPLAY, FONT_DISPLAY_TXT).expect("bundled font"),
            button: load(art!("ui_button_normal")),
            pressed: load(art!("ui_button_pressed")),
            disabled: load(art!("ui_button_disabled")),
            panel: load(art!("ui_panel")),
            panel_dark: load(art!("ui_panel_dark")),
        }
    }

    /// The font for a face.
    pub fn font(&self, face: super::text::Face) -> &BitmapFont {
        match face {
            super::text::Face::Body => &self.body,
            super::text::Face::Display => &self.display,
        }
    }

    /// The panel art of the current look.
    pub fn panel_art(&self) -> &Image {
        if palette().dark {
            &self.panel_dark
        } else {
            &self.panel
        }
    }
}

thread_local! {
    static KIT: OnceCell<Rc<Kit>> = const { OnceCell::new() };
}

/// The kit's art, decoded on first use.
pub fn kit() -> Rc<Kit> {
    KIT.with(|k| k.get_or_init(|| Rc::new(Kit::new())).clone())
}

/// The small pictures the dialogs and pages show beside text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Wool,
    Grain,
    Brick,
    Ore,
    Lumber,
    Gold,
    Any,
    Trader,
    Road,
    HighlightBank,
    HighlightChain,
    HighlightNow,
}

impl Icon {
    /// The icon the rules text names, by its art file name.
    pub fn by_name(name: &str) -> Option<Icon> {
        Some(match name {
            "icon_wool_12" => Icon::Wool,
            "icon_grain_12" => Icon::Grain,
            "icon_brick_12" => Icon::Brick,
            "icon_ore_12" => Icon::Ore,
            "icon_lumber_12" => Icon::Lumber,
            "icon_gold_12" => Icon::Gold,
            "icon_any_12" => Icon::Any,
            "glyph_trader_12" => Icon::Trader,
            "glyph_road_12" => Icon::Road,
            "rules_highlight_bank" => Icon::HighlightBank,
            "rules_highlight_chain" => Icon::HighlightChain,
            "rules_highlight_now" => Icon::HighlightNow,
            _ => return None,
        })
    }

    /// The art, decoded.
    pub fn image(self) -> Image {
        load(match self {
            Icon::Wool => art!("icon_wool_12"),
            Icon::Grain => art!("icon_grain_12"),
            Icon::Brick => art!("icon_brick_12"),
            Icon::Ore => art!("icon_ore_12"),
            Icon::Lumber => art!("icon_lumber_12"),
            Icon::Gold => art!("icon_gold_12"),
            Icon::Any => art!("icon_any_12"),
            Icon::Trader => art!("glyph_trader_12"),
            Icon::Road => art!("glyph_road_12"),
            Icon::HighlightBank => art!("rules_highlight_bank"),
            Icon::HighlightChain => art!("rules_highlight_chain"),
            Icon::HighlightNow => art!("rules_highlight_now"),
        })
    }
}

/// How a widget is showing right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Look {
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
}

/// A nine-slice frame of 24 px art at any size of at least two corners.
pub fn frame(img: &mut Image, art: &Image, x: i32, y: i32, w: i32, h: i32) {
    let corner = CORNER.min(w / 2).min(h / 2);
    img.nine_slice(art, x, y, w, h, corner);
}

/// The brass button frame in its state: pressed and disabled have their own art.
pub fn button(img: &mut Image, kit: &Kit, x: i32, y: i32, w: i32, h: i32, look: Look) {
    let art = if look.disabled {
        &kit.disabled
    } else if look.pressed {
        &kit.pressed
    } else {
        &kit.button
    };
    frame(img, art, x, y, w, h);
}

/// The focus ring just inside a widget's edge: two lines over its art.
pub fn focus_ring(img: &mut Image, x: i32, y: i32, w: i32, h: i32) {
    img.frame(x, y, w, h, RING_OUTER);
    img.frame(x + 1, y + 1, w - 2, h - 2, RING_INNER);
}

/// A faint band behind a focused row, inside its ring.
pub fn row_focus(img: &mut Image, x: i32, y: i32, w: i32, h: i32) {
    img.fill_argb(x, y, w, h, ROW_FOCUS);
}

/// The size of a check box's or radio button's recessed box.
pub const BOX: i32 = 12;

/// The recessed box of a check box or a radio button: the pressed button art, at 12 px.
pub fn recess(img: &mut Image, kit: &Kit, x: i32, y: i32) {
    field(img, kit, x, y, BOX, BOX);
}

/// A copy of `art` with each pixel's color swapped by `swap`, given the pixel and where it is.
fn recolored(art: &Image, swap: impl Fn(u32, i32, i32) -> u32) -> Image {
    let mut out = art.clone();
    let (w, h) = (art.width as i32, art.height as i32);
    for y in 0..h {
        for x in 0..w {
            let c = crate::art::rgba(swap(art.argb_at(x, y), x, y));
            let at = ((y * w + x) * 4) as usize;
            out.rgba[at..at + 4].copy_from_slice(&c);
        }
    }
    out
}

/// A recessed frame for a text field or a box: the pressed button art, recolored in the dark
/// look. The dark shades tell the upper left bevel from the lower right inner line.
pub fn field(img: &mut Image, kit: &Kit, x: i32, y: i32, w: i32, h: i32) {
    let p = palette();
    if !p.dark {
        frame(img, &kit.pressed, x, y, w, h);
        return;
    }
    let size = kit.pressed.width as i32;
    let art = recolored(&kit.pressed, |c, ax, ay| {
        p.recolour_field(c, ax + ay >= size)
    });
    frame(img, &art, x, y, w, h);
}

/// The size of a slider's thumb and how tall its row is.
pub const THUMB_W: i32 = 12;
pub const SLIDER_H: i32 = 16;

/// A slider across `w`, its thumb's left edge at `thumb_x`: PixelUi's Track and thumb.
pub fn slider(img: &mut Image, kit: &Kit, y: i32, w: i32, thumb_x: i32, pressed: bool) {
    let p = palette();
    // A SeekBar set its 6 px track a device pixel short of 4 art pixels down: this is the nearest
    let top = y + 4;
    img.fill_argb(0, top, w, 6, p.track_outline);
    img.fill_argb(1, top + 1, w - 2, 4, p.track_face);
    img.fill_argb(1, top + 1, w - 2, 1, p.track_top);
    let source = if pressed { &kit.pressed } else { &kit.button };
    if !p.dark {
        frame(img, source, thumb_x, y, THUMB_W, SLIDER_H);
        return;
    }
    let art = recolored(source, |c, _, _| p.recolour_thumb(c));
    frame(img, &art, thumb_x, y, THUMB_W, SLIDER_H);
}

/// The tick inside a checked box, 7 x 6 px (PixelCheckBox.TICK).
const TICK: [&str; 6] = [
    "......X", ".....XX", "X...XX.", "XX.XX..", ".XXX...", "..X....",
];

/// A check box at (x, y), ticked or not.
pub fn check_box(img: &mut Image, kit: &Kit, x: i32, y: i32, on: bool) {
    recess(img, kit, x, y);
    if on {
        for (row, line) in TICK.iter().enumerate() {
            for (col, c) in line.chars().enumerate() {
                if c == 'X' {
                    img.fill_argb(x + 3 + col as i32, y + 3 + row as i32, 1, 1, palette().mark);
                }
            }
        }
    }
}

/// A radio button at (x, y): a light 4 px square in the box when chosen. `swatch` fills the box
/// with a color first, for the player color choices.
pub fn radio_box(img: &mut Image, kit: &Kit, x: i32, y: i32, on: bool, swatch: Option<u32>) {
    recess(img, kit, x, y);
    if let Some(c) = swatch {
        img.fill_argb(x + 2, y + 2, BOX - 4, BOX - 4, 0xFF000000 | c);
        if on {
            // A dark outlined light square, so it shows on any color
            img.fill_argb(x + 3, y + 3, 6, 6, 0xFF1E140A);
        }
    }
    if on {
        img.fill_argb(x + 4, y + 4, 4, 4, palette().mark);
    }
}

/// A bullet for a list line: a small square in ink.
pub fn bullet(img: &mut Image, x: i32, y: i32, size: i32) {
    img.fill_argb(x, y, size, size, palette().ink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_art_decodes_at_its_design_size() {
        let k = kit();
        for art in [&k.button, &k.pressed, &k.disabled, &k.panel, &k.panel_dark] {
            assert_eq!((art.width, art.height), (24, 24));
        }
        for name in ["icon_any_12", "glyph_trader_12", "rules_highlight_now"] {
            assert!(Icon::by_name(name).is_some_and(|i| i.image().width > 0));
        }
    }

    #[test]
    fn a_button_keeps_its_corners_at_any_size() {
        let k = kit();
        let mut img = Image::new(90, 22);
        button(&mut img, &k, 0, 0, 90, 22, Look::default());
        // The corners are the art's own corners
        for (x, y) in [(0, 0), (5, 5), (89, 21)] {
            let sx = if x < 45 { x } else { 24 - (90 - x) };
            let sy = if y < 11 { y } else { 24 - (22 - y) };
            assert_eq!(img.argb_at(x, y), k.button.argb_at(sx, sy), "{x},{y}");
        }
        let mut pressed = Image::new(90, 22);
        button(
            &mut pressed,
            &k,
            0,
            0,
            90,
            22,
            Look {
                pressed: true,
                ..Look::default()
            },
        );
        assert_ne!(img, pressed);
    }

    #[test]
    fn a_checked_box_shows_its_tick_and_a_chosen_radio_its_mark() {
        let k = kit();
        let mut off = Image::new(BOX as u32, BOX as u32);
        check_box(&mut off, &k, 0, 0, false);
        let mut on = Image::new(BOX as u32, BOX as u32);
        check_box(&mut on, &k, 0, 0, true);
        assert_eq!(on.argb_at(9, 3), palette().mark);
        assert_ne!(off.argb_at(9, 3), palette().mark);
        let mut radio = Image::new(BOX as u32, BOX as u32);
        radio_box(&mut radio, &k, 0, 0, true, Some(0xFF0000));
        assert_eq!(radio.argb_at(5, 5), palette().mark);
        assert_eq!(radio.argb_at(2, 2), 0xFFFF0000);
    }

    #[test]
    fn the_focus_ring_shows_on_light_and_dark() {
        let mut img = Image::new(10, 10);
        focus_ring(&mut img, 0, 0, 10, 10);
        assert_eq!(img.argb_at(0, 5), RING_OUTER);
        assert_eq!(img.argb_at(1, 5), RING_INNER);
        assert_eq!(img.argb_at(5, 5), 0);
    }
}
