// SPDX-License-Identifier: GPL-3.0-or-later
//! The game screen's art and fonts, compiled into the binary. `day::resource()` has no files
//! on the web build, so the bytes come in through `include_bytes!` instead, and every target
//! decodes the same PNGs the same way.

use std::collections::HashMap;
use std::rc::Rc;

use crate::art::{Image, Mask};
use crate::rules::board::RoadShape;
use crate::rules::dice::DiceValue;
use crate::rules::team_color;

use super::font::BitmapFont;

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
macro_rules! font {
    ($name:literal, png) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resource/assets/font/",
            $name,
            ".png"
        ))
    };
    ($name:literal, txt) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resource/assets/font/",
            $name,
            ".txt"
        ))
    };
}

pub const FONT_BODY: &[u8] = font!("font_body", png);
pub const FONT_BODY_TXT: &str = font!("font_body", txt);
pub const FONT_DISPLAY: &[u8] = font!("font_display", png);
pub const FONT_DISPLAY_TXT: &str = font!("font_display", txt);
pub const FONT_NUMERALS: &[u8] = font!("font_numerals", png);
pub const FONT_NUMERALS_TXT: &str = font!("font_numerals", txt);

/// The board art of an island, by its map id.
pub fn board_png(map_id: &str) -> &'static [u8] {
    match map_id {
        "dunholm" => art!("board_dunholm"),
        _ => art!("board_tarnholm"),
    }
}

fn load(bytes: &[u8]) -> Image {
    // The art is part of the build, so a PNG that does not decode is a broken build
    Image::from_png(bytes).expect("bundled art decodes")
}

/// A sprite with its team colour taken out (team_color::unpainted): a road's cobbles without
/// its kerb.
fn unpainted(sprite: &Image) -> Image {
    let mut px = sprite.to_argb();
    team_color::unpainted(&mut px, &team_color::MAGENTA);
    Image::from_argb(sprite.width, sprite.height, &px)
}

/// A sprite recoloured for a player colour (team_color::recolor).
pub fn recolored(sprite: &Image, color: u32) -> Image {
    let mut px = sprite.to_argb();
    team_color::recolor(&mut px, &team_color::MAGENTA, color);
    Image::from_argb(sprite.width, sprite.height, &px)
}

pub struct Assets {
    pub font_display: BitmapFont,
    pub font_body: BitmapFont,
    pub font_numerals: BitmapFont,
    /// By RoadShape as usize: the sprite, its mask, and its cobbles without the kerb.
    pub road: [Image; 3],
    pub road_mask: [Mask; 3],
    pub road_surface: [Image; 3],
    pub village: Image,
    pub village_mask: Mask,
    pub castle: Image,
    pub castle_mask: Mask,
    pub trader: Image,
    pub trader_mask: Mask,
    pub king_ship: Image,
    pub dock: Image,
    pub track_water: Image,
    pub track_buoy: Image,
    pub track_flag: Image,
    pub track_flag_empty: Image,
    pub track_ship: Image,
    pub track_pier: Image,
    pub undo_glyph: Image,
    pub panel_dark: Image,
    /// The dark panel's own colours, for the callout's pointer: its outermost line, its gold
    /// line, and what fills it.
    pub panel_ink: u32,
    pub panel_gold: u32,
    pub panel_fill: u32,
    pub glyph_road: Image,
    pub glyph_trader: Image,
    pub glyph_village: Image,
    pub glyph_castle: Image,
    pub badge_road: Image,
    pub badge_trader: Image,
    icon12: HashMap<DiceValue, Image>,
    icon16: HashMap<DiceValue, Image>,
    /// Per 12 px icon, the empty rows under its art (see cargo_y in the scene).
    cargo_drop: HashMap<DiceValue, i32>,
    pub die_face: Image,
    pub die_face_held: Image,
    pub die_slot_empty: Image,
    pub button_normal: Image,
    pub button_pressed: Image,
    pub button_disabled: Image,
    boards: HashMap<String, Rc<Image>>,
}

impl Assets {
    pub fn load() -> Assets {
        let road = [
            load(art!("road_h")),
            load(art!("road_slash")),
            load(art!("road_backslash")),
        ];
        let road_mask = [Mask::of(&road[0]), Mask::of(&road[1]), Mask::of(&road[2])];
        let road_surface = [
            unpainted(&road[0]),
            unpainted(&road[1]),
            unpainted(&road[2]),
        ];
        let village = load(art!("piece_village"));
        let castle = load(art!("piece_castle"));
        let trader = load(art!("piece_trader"));
        let panel_dark = load(art!("ui_panel_dark"));
        let mid = panel_dark.height as i32 / 2;
        let icons = |sixteen: bool| -> HashMap<DiceValue, Image> {
            let pairs: [(DiceValue, &[u8], &[u8]); 7] = [
                (DiceValue::Wool, art!("icon_wool_12"), art!("icon_wool_16")),
                (
                    DiceValue::Grain,
                    art!("icon_grain_12"),
                    art!("icon_grain_16"),
                ),
                (
                    DiceValue::Brick,
                    art!("icon_brick_12"),
                    art!("icon_brick_16"),
                ),
                (DiceValue::Ore, art!("icon_ore_12"), art!("icon_ore_16")),
                (
                    DiceValue::Lumber,
                    art!("icon_lumber_12"),
                    art!("icon_lumber_16"),
                ),
                (DiceValue::Gold, art!("icon_gold_12"), art!("icon_gold_16")),
                (DiceValue::Any, art!("icon_any_12"), art!("icon_any_16")),
            ];
            pairs
                .into_iter()
                .map(|(v, a, b)| (v, load(if sixteen { b } else { a })))
                .collect()
        };
        let icon12 = icons(false);
        let cargo_drop = icon12
            .iter()
            .map(|(v, img)| (*v, img.clear_rows_below()))
            .collect();
        Assets {
            font_display: BitmapFont::parse(FONT_DISPLAY, FONT_DISPLAY_TXT).expect("display font"),
            font_body: BitmapFont::parse(FONT_BODY, FONT_BODY_TXT).expect("body font"),
            font_numerals: BitmapFont::parse(FONT_NUMERALS, FONT_NUMERALS_TXT)
                .expect("numerals font"),
            road,
            road_mask,
            road_surface,
            village_mask: Mask::of(&village),
            village,
            castle_mask: Mask::of(&castle),
            castle,
            trader_mask: Mask::of(&trader),
            trader,
            king_ship: load(art!("piece_kingship")),
            dock: load(art!("dock")),
            track_water: load(art!("track_water")),
            track_buoy: load(art!("track_buoy")),
            track_flag: load(art!("track_flag")),
            track_flag_empty: load(art!("track_flag_empty")),
            track_ship: load(art!("track_ship")),
            track_pier: load(art!("track_pier")),
            undo_glyph: load(art!("undo_glyph")),
            panel_ink: panel_dark.argb_at(0, mid),
            panel_gold: panel_dark.argb_at(2, mid),
            panel_fill: panel_dark.argb_at(panel_dark.width as i32 / 2, mid),
            panel_dark,
            glyph_road: load(art!("glyph_road_12")),
            glyph_trader: load(art!("glyph_trader_12")),
            glyph_village: load(art!("glyph_village_12")),
            glyph_castle: load(art!("glyph_castle_12")),
            badge_road: load(art!("badge_road_7")),
            badge_trader: load(art!("badge_trader_7")),
            icon12,
            icon16: icons(true),
            cargo_drop,
            die_face: load(art!("die_face")),
            die_face_held: load(art!("die_face_held")),
            die_slot_empty: load(art!("die_slot_empty")),
            button_normal: load(art!("ui_button_normal")),
            button_pressed: load(art!("ui_button_pressed")),
            button_disabled: load(art!("ui_button_disabled")),
            boards: HashMap::new(),
        }
    }

    /// The 12 px icon of a resource; anything without one of its own shows the "?".
    pub fn icon12(&self, v: DiceValue) -> &Image {
        self.icon12.get(&v).unwrap_or(&self.icon12[&DiceValue::Any])
    }

    pub fn icon16(&self, v: DiceValue) -> Option<&Image> {
        self.icon16.get(&v)
    }

    pub fn cargo_drop(&self, v: DiceValue) -> i32 {
        self.cargo_drop.get(&v).copied().unwrap_or(0)
    }

    pub fn road_index(shape: RoadShape) -> usize {
        shape as usize
    }

    /// An island's board, decoded the first time it is asked for.
    pub fn board(&mut self, map_id: &str) -> Rc<Image> {
        self.boards
            .entry(map_id.to_owned())
            .or_insert_with(|| Rc::new(load(board_png(map_id))))
            .clone()
    }
}

/// The sprites that carry the player's colour, recoloured for one colour and kept until it
/// changes (TeamColor's cache on Android).
pub struct Tinted {
    pub color: u32,
    pub road: [Image; 3],
    pub village: Image,
    pub castle: Image,
    pub trader: Image,
    pub die_face_held: Image,
    pub badge_road: Image,
    pub glyph_road: Image,
    pub buoy: Image,
}

impl Tinted {
    pub fn new(a: &Assets, color: u32) -> Tinted {
        Tinted {
            color,
            road: [
                recolored(&a.road[0], color),
                recolored(&a.road[1], color),
                recolored(&a.road[2], color),
            ],
            village: recolored(&a.village, color),
            castle: recolored(&a.castle, color),
            trader: recolored(&a.trader, color),
            die_face_held: recolored(&a.die_face_held, color),
            badge_road: recolored(&a.badge_road, color),
            glyph_road: recolored(&a.glyph_road, color),
            buoy: recolored(&a.track_buoy, color),
        }
    }
}
