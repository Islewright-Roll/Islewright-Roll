// SPDX-License-Identifier: GPL-3.0-or-later
//! Where everything on the game screen goes, in art pixels: pure arithmetic over the canvas
//! size, the system insets, the island and the dice side, so it can be tested without a
//! window.
//!
//! Portrait is the Android layout, number for number (GameView.layout and the bars it drew):
//! a top bar with the score, the costs and the menu button over the turn track, the island
//! in the middle, and a bottom bar with the dice and the roll button.
//!
//! A phone stays upright: the app locks it to portrait.
//!
//! Wide canvases (a tablet on its side, a desktop window) show the island upright in open sea
//! beside one panel of wood on the side that was the bottom of the screen. The panel hugs what
//! it holds, as the portrait bars do: from the top the score and the menu, the costs key at
//! 16 px and the turn track winding back and forth in three rows; from the bottom, under the
//! thumbs, the roll and undo buttons, the dice and the bank. The island takes all the width the
//! panel leaves, at the largest scale that keeps sea round its land.
//!
//! The screen has two regions, each at its own whole-number scale: the interface (the bars or
//! columns, the HUD, the dice, the callouts) and the island under it (see `IslandView`). The
//! interface takes the largest scale its arrangement fits at (see `Layout::new`), and the
//! island the largest scale at which its land still fits beside the bars or the panel, so the
//! island also takes whatever room the interface leaves. The rects
//! below are in interface pixels; the island's are in its own.

use crate::art::{DESIGN_W, Fit, Image, MIN_H};
use crate::rules::art as rart;
use crate::rules::board;
use crate::rules::map_def::MapDef;
use crate::rules::pt::Pt;
use crate::rules::tutorial_layout::Rect;
use crate::rules::voyage::{self, Strip};

use super::DiceSide;

// ---------------------------------------------------------------- the Android layout

pub const TOP_H: i32 = 62;
pub const BOTTOM_H: i32 = 66;
pub const DICE_GAP: i32 = 6;
pub const HUD_INSET: i32 = 6;
pub const BOX_GAP: i32 = 3;
pub const BOX_Y: i32 = 6;
pub const BOX_H: i32 = 34;
pub const SCORE_W: i32 = 40;
pub const COSTS_W: i32 = 135;
pub const MENU_W: i32 = DESIGN_W as i32 - 2 * HUD_INSET - 2 * BOX_GAP - SCORE_W - COSTS_W;
pub const STRIP_Y: i32 = 43;
/// A race's bonuses hang on a sign from the top bar's bottom edge, this many rows of rope below it.
pub const SIGN_DROP: i32 = 3;
/// A die is a square of this many art pixels.
pub const DIE: i32 = 28;
pub const ROLL_W: i32 = 96;
pub const ROLL_H: i32 = 20;
pub const UNDO_SIZE: i32 = 16;
pub const MINI_DIE: i32 = 16;
pub const MINI_GAP: i32 = 3;
pub const MINI_PER_ROW: i32 = 6;
pub const MINI_PAD: i32 = 3;
pub const MINI_MARGIN: i32 = 2;
/// The band of the turn track: the water's height and a pixel of frame above and below.
pub const BAND_H: i32 = rart::TRACK_H + 2;

// ---------------------------------------------------------------- landscape

/// The panel's column stands this far in from the screen's side, top and bottom: FLOAT of sea round the panel's wood, then PANEL_PAD of wood round what it holds.
pub const COL_INSET: i32 = 6;
pub const FLOAT: i32 = 2;
pub const PANEL_PAD: i32 = COL_INSET - FLOAT;
/// The costs key in landscape: each recipe a 16 px glyph on its tile, then a 16 px icon per
/// resource, one recipe a row.
pub const KEY_ICON: i32 = 16;
pub const KEY_ROW_H: i32 = KEY_ICON + 2;
pub const KEY_ROW: i32 = KEY_ROW_H + 1;
pub const KEY_CELL: i32 = KEY_ICON + 1;
pub const KEY_INSET: i32 = 3;
/// A recipe's width for n resources: its tile, a pixel, the dark strip of cells, a pixel.
pub const fn key_recipe_w(n: i32) -> i32 {
    KEY_ROW_H + 1 + (1 + n * KEY_CELL) + 1
}
/// What the panel holds: as wide as the castle's recipe (five resources) in its tray.
pub const PANEL_W: i32 = key_recipe_w(5) + 2 * KEY_INSET;
pub const KEY_H: i32 = KEY_INSET + 4 * KEY_ROW - 1 + KEY_INSET;
/// The panel's column: what it holds and the inset either side.
pub const COL_W: i32 = PANEL_W + 2 * COL_INSET;
/// A panel's label on the wood above it: the font's caps and two rows below them.
pub const LABEL_H: i32 = 9;
/// The race bonuses stacked in the landscape score tray: a glyph's 7 rows and the gap.
pub const BADGE_ROW: i32 = 12;
/// The menu button, in the panel's top right corner.
pub const LAND_MENU_W: i32 = 30;
/// The turn track's three rows, and the rows of water between them where it turns.
pub const TRACK_ROWS: i32 = 3;
pub const TRACK_ROW_GAP: i32 = 4;
pub const TRACK_BLOCK_H: i32 = TRACK_ROWS * BAND_H + (TRACK_ROWS - 1) * TRACK_ROW_GAP;
/// The wood between the groups in the panel: at least GROUP_GAP, and up to GAP_MAX where the
/// panel has rows to spare.
pub const GROUP_GAP: i32 = 5;
pub const GAP_MAX: i32 = 12;
/// The open sea the island keeps round its land in landscape, where it can.
pub const SEA_ROOM: i32 = 12;
pub const LAND_MINI_PER_ROW: i32 = 5;
/// The panel's lower groups: the bank, its mini dice on a tray of two rows, the dice in two
/// rows of three, the roll button with the undo button at its end.
const MINI_TRAY_H: i32 = 2 * MINI_DIE + MINI_GAP + 2 * MINI_PAD + 2;
const DICE_BLOCK_H: i32 = 2 * DIE + DICE_GAP;
const ROLL_BLOCK_H: i32 = ROLL_H;
/// The landscape roll button, leaving room for the undo button beside it.
const LAND_ROLL_W: i32 = PANEL_W - UNDO_SIZE - 4;
/// The panel's groups, top to bottom: the score, the labelled costs, the labelled track, then
/// the lower groups.
const PANEL_GROUPS: [i32; 6] = [
    BOX_H,
    LABEL_H + KEY_H,
    LABEL_H + TRACK_BLOCK_H,
    LABEL_H + MINI_TRAY_H,
    DICE_BLOCK_H,
    ROLL_BLOCK_H,
];
/// The clear sea kept round the island's land in landscape.
pub const LAND_MARGIN: i32 = 4;

/// Whether the HUD shows race bonuses on this island: a race with a bonus to earn.
fn has_badges(map: &MapDef) -> bool {
    map.target_score > 0 && (map.trader_bonus_at > 0 || map.long_road.is_some())
}

/// The least height of a column holding these groups in one panel.
fn column_h(heights: &[i32]) -> i32 {
    let n = heights.len() as i32;
    heights.iter().sum::<i32>() + GROUP_GAP * (n - 1) + 2 * COL_INSET
}

// The sprites the land extent is measured with (their PNGs' sizes; a test checks them)
pub const VILLAGE_W: i32 = 20;
pub const VILLAGE_H: i32 = 19;
pub const CASTLE_W: i32 = 28;
pub const CASTLE_H: i32 = 30;
pub const TRADER_W: i32 = 28;
pub const TRADER_H: i32 = 19;
/// Half a hex's width and height from its centre (board.rs: 72 by 72, flat topped).
const HEX_HALF: i32 = 36;

// ---------------------------------------------------------------- the island's region

/// The island's region of the screen: an image of its own at its own whole-number scale,
/// which covers the whole screen (the sea runs on under the bars, and the screen edges never
/// show anything but sea), with the board drawn into it upright.
///
/// This is the one place that says how the board's pixels reach the screen, so the rest of
/// the screen asks it and never works it out itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandView {
    /// Screen pixels per island pixel, across and down.
    pub scale: u32,
    /// The island's image: the screen at the island's scale, rounded up so the last
    /// column and row of screen pixels are covered too.
    pub frame_w: i32,
    pub frame_h: i32,
    /// Where the board's top left is in the island's image.
    pub board_x: i32,
    pub board_y: i32,
}

/// The island pixels wholly inside an area given in interface pixels, at island scale `s`
/// with the interface at `ui`: both start at the screen's top left.
fn area_at(area: Rect, ui: u32, s: u32) -> Rect {
    let (ui, s) = (ui as i32, s as i32);
    let x0 = (area.x * ui + s - 1).div_euclid(s);
    let y0 = (area.y * ui + s - 1).div_euclid(s);
    let x1 = ((area.x + area.w) * ui).div_euclid(s);
    let y1 = ((area.y + area.h) * ui).div_euclid(s);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

impl IslandView {
    /// The island's region for the interface's fit: the largest scale, from the interface's
    /// own up, at which all of the land (`land`, in board pixels) shows inside `area` (the
    /// island's part of the screen, in interface pixels). `show` is the land with what may be
    /// cropped (the moored ship): it is centred there where it fits, and otherwise moved as
    /// little as keeps the land whole, so the scale never waits on the ship.
    ///
    /// In portrait the island never goes below the interface's scale, so on a phone, where
    /// nothing larger fits, both share one scale; then `at_ui_scale` (where the portrait layout
    /// puts the board) places it, which may crop the land on a short screen. Above it, the
    /// whole land must fit, so growing never hides any of the island.
    ///
    /// In landscape (`at_ui_scale` is None) the land must always fit, because the upright
    /// island is taller than the columns need to be, so there it may go below the interface's
    /// scale, as far as 1.
    fn fit(fit: &Fit, area: Rect, land: Rect, show: Rect, at_ui_scale: Option<Pt>) -> IslandView {
        let ui = fit.scale;
        // The largest scale at which the land fits with this much sea round it
        let largest = |room: i32| {
            let (need_w, need_h) = (land.w + 2 * room, land.h + 2 * room);
            let mut scale = ui;
            if at_ui_scale.is_none() {
                while scale > 1 {
                    let a = area_at(area, ui, scale);
                    if a.w >= need_w && a.h >= need_h {
                        break;
                    }
                    scale -= 1;
                }
            }
            // Each step up shrinks the area in island pixels, so the first that fails ends it
            while scale < 4 * ui {
                let a = area_at(area, ui, scale + 1);
                if a.w < need_w || a.h < need_h {
                    break;
                }
                scale += 1;
            }
            scale
        };
        let mut scale = largest(0);
        // In landscape the land keeps SEA_ROOM of open sea on every side where that costs it
        // no more than takes it down to the interface's own scale
        if at_ui_scale.is_none() {
            let roomy = largest(SEA_ROOM);
            if roomy >= ui.min(scale) {
                scale = roomy;
            }
        }
        let (sw, sh) = (fit.width * ui, fit.height * ui);
        let frame_w = sw.div_ceil(scale) as i32;
        let frame_h = sh.div_ceil(scale) as i32;
        let (board_x, board_y) = match at_ui_scale {
            Some(at) if scale == ui => (at.x, at.y),
            // In landscape the land sits in the middle, the same sea above as below; a ship
            // moored past it may run off the screen's edge
            None => {
                let a = area_at(area, ui, scale);
                (
                    a.x + (a.w - land.w) / 2 - land.x,
                    a.y + (a.h - land.h) / 2 - land.y,
                )
            }
            _ => {
                let a = area_at(area, ui, scale);
                // Where the land sits in `show`
                let inner = Rect::new(land.x - show.x, land.y - show.y, land.w, land.h);
                // The land centred, then moved toward a side `show` runs past, as far as the
                // room on the other side allows, so more of the ship shows
                let place = |start: i32, room: i32, size: i32, at: i32, len: i32| {
                    let land = start + (room - len) / 2;
                    let (before, after) = (land - start, start + room - land - len);
                    let (past_before, past_after) = (at - before, size - at - len - after);
                    let shift = past_before.clamp(0, after) - past_after.clamp(0, before);
                    land - at + shift
                };
                let x = place(a.x, a.w, show.w, inner.x, inner.w);
                let y = place(a.y, a.h, show.h, inner.y, inner.h);
                (x - show.x, y - show.y)
            }
        };
        IslandView {
            scale,
            frame_w,
            frame_h,
            board_x,
            board_y,
        }
    }

    /// The board pixel under a pixel of the island's image.
    pub fn to_board(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.board_x, y - self.board_y)
    }

    /// The pixel of the island's image a board pixel shows on.
    pub fn to_frame(&self, x: i32, y: i32) -> (i32, i32) {
        (x + self.board_x, y + self.board_y)
    }

    /// The rectangle of the island's image a rectangle of the board covers.
    pub fn rect_to_frame(&self, r: Rect) -> Rect {
        Rect::new(r.x + self.board_x, r.y + self.board_y, r.w, r.h)
    }

    /// The board column at the image's left edge: what the ship's far stop is measured from.
    pub fn visible_left(&self) -> i32 {
        -self.board_x
    }
}

/// A point on the screen, as a tap or a drag gives it: the screen pixel (from the top left of
/// the image the screen is shown as), and the interface pixel it falls in. The island's
/// pixel under it comes from `Layout::spot_to_board`, since the island has its own scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    pub sx: i32,
    pub sy: i32,
}

// ---------------------------------------------------------------- the sea beyond the board

/// The outer band of the board image that is open sea on every island: wider than the
/// island's coast leaves at no edge (the land and its shallows start about 130 px in).
pub const SEA_BAND: i32 = 96;

/// The board pixel (along one axis) that a layer pixel u outside the board shows: the board's
/// own outer band of sea, reflected at the board's edge and then back and forth, so the water
/// runs on without a seam however far the frame reaches. Inside the board it is u itself.
pub fn mirror(u: i32, size: i32, band: i32) -> i32 {
    let fold = |d: i32| {
        let t = d.rem_euclid(2 * band);
        if t < band { t } else { 2 * band - 1 - t }
    };
    if u < 0 {
        fold(-u - 1)
    } else if u >= size {
        size - 1 - fold(u - size)
    } else {
        u
    }
}

/// A layer of lw x lh with the board at (bx, by) and the sea reflected round it to the edges.
pub fn sea_layer(board: &Image, lw: i32, lh: i32, bx: i32, by: i32) -> Image {
    let (bw, bh) = (board.width as i32, board.height as i32);
    let mut out = Image::new(lw.max(1) as u32, lh.max(1) as u32);
    let cols: Vec<usize> = (0..lw)
        .map(|x| mirror(x - bx, bw, SEA_BAND.min(bw)) as usize)
        .collect();
    for y in 0..lh {
        let sy = mirror(y - by, bh, SEA_BAND.min(bh)) as usize;
        let src = &board.rgba[sy * bw as usize * 4..(sy + 1) * bw as usize * 4];
        let dst = &mut out.rgba[(y * lw) as usize * 4..((y + 1) * lw) as usize * 4];
        for (x, &sx) in cols.iter().enumerate() {
            dst[x * 4..x * 4 + 4].copy_from_slice(&src[sx * 4..sx * 4 + 4]);
        }
    }
    out
}

// ---------------------------------------------------------------- the island's extent

/// The part of the board that shows in landscape, in board pixels: the land (`land_core`) and
/// the ship moored at the pier, with LAND_MARGIN of sea round it. Where the screen is short the
/// ship may be cropped, never the land.
pub fn land_extent(map: &'static MapDef) -> Rect {
    let core = land_core(map);
    let moored = voyage::ship_origin(voyage::Stage::Moored, voyage::DOCK_MOORING, 0);
    let x0 = core.x.min(moored.x + rart::SHIP_BOX_X - LAND_MARGIN);
    let y0 = core.y.min(moored.y + rart::SHIP_BOX_Y - LAND_MARGIN);
    let x1 = (core.x + core.w).max(moored.x + rart::SHIP_BOX_X + rart::SHIP_BOX_W + LAND_MARGIN);
    let y1 = (core.y + core.h).max(moored.y + rart::SHIP_BOX_Y + rart::SHIP_BOX_H + LAND_MARGIN);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// The part of the board that must always show in landscape, in board pixels: every land hex,
/// every building and trader that can stand on it with its label, and the row under the lowest
/// building, with LAND_MARGIN of sea round it. This decides the island's scale.
pub fn land_core(map: &'static MapDef) -> Rect {
    let b = board::Board::for_map(map);
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let mut add = |x: i32, y: i32, w: i32, h: i32| {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x + w);
        y1 = y1.max(y + h);
    };
    for c in &map.hex {
        add(c[0] - HEX_HALF, c[1] - HEX_HALF, 2 * HEX_HALF, 2 * HEX_HALF);
    }
    for v in &b.village {
        add(
            v.x - rart::VILLAGE_AX,
            v.y - rart::VILLAGE_AY,
            VILLAGE_W,
            VILLAGE_H,
        );
    }
    for c in &b.city {
        add(
            c.x - rart::CASTLE_AX,
            c.y - rart::CASTLE_AY,
            CASTLE_W,
            CASTLE_H,
        );
    }
    for t in crate::rules::trader_layout::anchors(map) {
        // The sprite, and the row of its number and banked resource under it
        add(
            t.x - rart::TRADER_AX,
            t.y - rart::TRADER_AY,
            TRADER_W,
            TRADER_H.max(rart::TRADER_LABEL_DROP + rart::TRADER_AY + 12),
        );
    }
    // Down to the row under the lowest building, whatever the hexes and sprites reach
    let centre_x = map.hex.iter().map(|c| c[0]).min().unwrap_or(0);
    add(centre_x, voyage::ISLAND_FOOT - 1, 1, 1);
    Rect::new(
        x0 - LAND_MARGIN,
        y0 - LAND_MARGIN,
        x1 - x0 + 2 * LAND_MARGIN,
        y1 - y0 + 2 * LAND_MARGIN,
    )
}

// ---------------------------------------------------------------- the turn track

/// One band of the turn track: its frame, and the cells of the strip it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackRow {
    /// The frame round the water, outline included.
    pub band: Rect,
    /// The first cell this row shows, and how many.
    pub first: i32,
    pub cells: i32,
    /// The art column of the strip's left edge (cell positions are relative to it).
    pub origin_x: i32,
    /// The strip's numbers for this row. Its `turns` counts this row's cells only, and the
    /// last row's ends at the pier.
    pub strip: Strip,
    pub pier: bool,
    /// The ship sails this row right to left: the track runs back and forth down the HUD.
    pub reverse: bool,
}

impl TrackRow {
    /// The water's top row.
    pub fn water_y(&self) -> i32 {
        self.band.y + 1
    }

    /// The left edge of a sprite `w` wide whose strip position is `x` (relative to the
    /// strip's left edge): as it is, or mirrored across the band in a row sailed backwards.
    pub fn place(&self, x: i32, w: i32) -> i32 {
        let at = self.origin_x + x;
        if self.reverse {
            2 * self.band.x + self.band.w - at - w
        } else {
            at
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub rows: Vec<TrackRow>,
    pub turns: i32,
    pub flags: bool,
}

impl Track {
    /// The row and its own cell for a ship cell (0 to turns, turns being in harbour).
    pub fn ship_row(&self, cell: i32) -> (usize, i32) {
        for (i, r) in self.rows.iter().enumerate() {
            if cell < r.first + r.cells || i == self.rows.len() - 1 {
                return (i, cell - r.first);
            }
        }
        (0, cell)
    }

    /// The frames of every band together (the tutorial points at it).
    pub fn rect(&self) -> Rect {
        let first = self.rows[0].band;
        let last = self.rows[self.rows.len() - 1].band;
        Rect::new(first.x, first.y, first.w, last.y + last.h - first.y)
    }

    /// The portrait strip: one row across the HUD.
    fn one_row(map: &MapDef, hud_x: i32, edge: i32) -> Track {
        let strip = Strip::for_map(map, DESIGN_W as i32);
        Track {
            rows: vec![TrackRow {
                band: Rect::new(
                    hud_x + HUD_INSET,
                    edge,
                    DESIGN_W as i32 - 2 * HUD_INSET,
                    BAND_H,
                ),
                first: 0,
                cells: strip.turns,
                origin_x: hud_x + strip.left,
                strip,
                pier: true,
                reverse: false,
            }],
            turns: strip.turns,
            flags: strip.flags,
        }
    }

    /// The landscape strip: the same cells in `n` rows within a column `col_w` wide, the
    /// first rows each holding as many and the last the rest with the pier, at one pitch so
    /// the cells line up.
    fn rows(map: &MapDef, col_x: i32, col_w: i32, inset: i32, top: i32, n: i32) -> Track {
        let whole = Strip::for_map(map, DESIGN_W as i32);
        let (turns, flags) = (whole.turns, whole.flags);
        let upper = (turns + n - 1) / n;
        let lower = turns - upper * (n - 1);
        let band_x = col_x + inset;
        let band_w = col_w - 2 * inset;
        // The last row's strip, narrowed until an upper row also fits its band
        let mut room = col_w;
        let strip = loop {
            let s = Strip::new(lower, flags, room);
            let upper_w =
                s.pad + s.pitch * (upper - 1) + rart::TRACK_FLAG_W.max(rart::TRACK_SHIP_W);
            if s.left + upper_w < band_w + inset || room <= 40 {
                break s;
            }
            room -= 2;
        };
        let origin_x = col_x + (col_w - strip.width) / 2;
        let row = |i: i32, first: i32, cells: i32, s: Strip, pier: bool| TrackRow {
            band: Rect::new(band_x, top + i * (BAND_H + TRACK_ROW_GAP), band_w, BAND_H),
            first,
            cells,
            origin_x,
            strip: s,
            pier,
            reverse: (n - 1 - i) % 2 == 1,
        };
        // An upper row's own strip has two cells more than it shows, so none of its cells
        // is ever the last turn's (which the strip puts alongside the pier)
        let upper_strip = Strip {
            turns: upper + 2,
            ..strip
        };
        let mut rows: Vec<TrackRow> = (0..n - 1)
            .map(|i| row(i, i * upper, upper, upper_strip, false))
            .collect();
        rows.push(row(n - 1, upper * (n - 1), lower, strip, true));
        Track { rows, turns, flags }
    }

    /// The landscape track: one stretch of water running back and forth down the HUD in
    /// TRACK_ROWS rows, the last ending at the pier.
    fn back_and_forth(map: &MapDef, col_x: i32, top: i32) -> Track {
        Track::rows(map, col_x, COL_W, COL_INSET, top, TRACK_ROWS)
    }
}

// ---------------------------------------------------------------- the layout

/// Device pixels of the canvas that system bars (or a notch) sit over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

/// A stretch of the bars' dark wood, and the side of it that meets the island, where it has
/// a line of lighter wood and a line of ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    pub rect: Rect,
    pub edge: Edge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    /// A panel standing on the sea: ink and lighter wood all round, its corners cut.
    Around,
}

/// Where the race bonuses go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgePlace {
    /// On a wooden sign whose right edge is `right` and top `top`, hung from the bar above it.
    Sign { right: i32, top: i32 },
    /// In the score's tray, one above the other BADGE_ROW apart, their right edge at `right`
    /// and the first glyph's top at `top`.
    Panel { right: i32, top: i32 },
}

/// How the banked mini dice stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniTab {
    /// On a tab of the bottom bar's wood rising over the sea, the bar's top row given.
    Bar { bar_top: i32 },
    /// In a tray in the landscape panel.
    Tray,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The interface's fit: its scale, and where its pixels sit among the device's.
    pub fit: Fit,
    /// The interface's art canvas: the screen's size in art pixels.
    pub width: i32,
    pub height: i32,
    /// None in portrait; the dice side in landscape.
    pub landscape: Option<DiceSide>,
    /// The costs key at 16 px, its four recipes in one column (landscape), not two rows of
    /// two at 12 px.
    pub key_large: bool,
    pub pad_top: i32,
    pub pad_bottom: i32,
    pub pad_left: i32,
    pub pad_right: i32,
    /// The island's region: its scale, its image and the board's place in it.
    pub isle: IslandView,
    /// The part of the frame a tap reaches the island in.
    pub island: Rect,
    pub bars: Vec<Bar>,
    /// Portrait: the 216 wide HUD's left edge. Landscape: the panel's column.
    pub hud_x: i32,
    /// The tray behind the score: the score's own, or in landscape a panel it shares with
    /// the race bonuses.
    pub score_tray: Rect,
    pub score: Rect,
    pub costs: Rect,
    /// Where the costs' and the track's labels start (the top of their caps), in landscape.
    pub costs_label: Option<Pt>,
    pub turns_label: Option<Pt>,
    /// The bank's tray and its label, in landscape: always there, empty or not.
    pub bank: Option<Rect>,
    pub bank_label: Option<Pt>,
    pub menu: Rect,
    /// The menu button and a couple of pixels round it.
    pub menu_hit: Rect,
    pub badges: BadgePlace,
    pub track: Track,
    pub dice: [Rect; 6],
    /// The roll button as laid out; a long label widens it about its centre.
    pub roll: Rect,
    pub undo: Rect,
    /// The top left of the first mini die's place, the next ones to its right, a second row
    /// above it on the bar's tab or below it in the tray.
    pub mini_x: i32,
    pub mini_y: i32,
    pub mini_per_row: i32,
    pub mini_tab: MiniTab,
}

fn ceil_div(a: u32, b: u32) -> i32 {
    a.div_ceil(b.max(1)) as i32
}

/// The portrait scale: the largest at which the design width and minimum height fit in what
/// the system bars leave (ArtCanvas.resize).
pub fn portrait_scale(w: u32, h: u32, insets: Insets) -> u32 {
    (w / DESIGN_W)
        .min(h.saturating_sub(insets.top + insets.bottom) / MIN_H)
        .max(1)
}

/// The least art size the landscape layout needs for an island: the two columns and the
/// upright island between them, and the taller column.
pub fn landscape_min(map: &'static MapDef) -> (i32, i32) {
    let land = land_core(map);
    let w = COL_W + land.w;
    // Only the panel: where the upright island is too tall, it goes down a scale on its own
    // (`IslandView::fit`), so the dice stay large enough to tap.
    let h = column_h(&PANEL_GROUPS);
    (w, h)
}

/// The landscape scale, or None where the layout does not fit even at 1.
pub fn landscape_scale(w: u32, h: u32, insets: Insets, map: &'static MapDef) -> Option<u32> {
    let (min_w, min_h) = landscape_min(map);
    let aw = w.saturating_sub(insets.left + insets.right);
    let ah = h.saturating_sub(insets.top + insets.bottom);
    // The interface stays within a scale of the island, whose height decides its own: larger
    // and the columns would crowd out the game they frame.
    let island = (ah / land_core(map).h as u32).max(1);
    let scale = (aw / min_w as u32).min(ah / min_h as u32).min(island + 1);
    (scale >= 1).then_some(scale)
}

fn fit_at(w: u32, h: u32, scale: u32) -> Fit {
    let (width, height) = ((w / scale).max(1), (h / scale).max(1));
    Fit {
        scale,
        width,
        height,
        offset_x: w.saturating_sub(width * scale) / 2,
        offset_y: h.saturating_sub(height * scale) / 2,
    }
}

impl Layout {
    /// The layout for a canvas of w x h device pixels. Wider than tall goes landscape where
    /// the landscape layout's interface fits at least as large as portrait's would; anything
    /// else (a tall window, a nearly square one) keeps the Android layout. Either way the
    /// interface takes the largest scale its own layout fits at, and the island then grows
    /// into whatever room that leaves (`IslandView::fit`).
    ///
    /// The interface does not shrink to let the island grow more: that would only trade one
    /// region's size for the other's.
    pub fn new(w: u32, h: u32, insets: Insets, map: &'static MapDef, side: DiceSide) -> Layout {
        let w = w.max(1);
        let h = h.max(1);
        let portrait = portrait_scale(w, h, insets);
        match landscape_scale(w, h, insets, map).filter(|&s| w > h && s >= portrait) {
            Some(scale) => Layout::landscape(fit_at(w, h, scale), insets, map, side),
            None => Layout::portrait(
                // The art starts at the top of the window, as the Android app's did, so the rows
                // left over go below the bottom bar and every pixel above lands where it did there
                Fit {
                    offset_y: 0,
                    ..fit_at(w, h, portrait)
                },
                insets,
                map,
            ),
        }
    }

    fn portrait(fit: Fit, insets: Insets, map: &'static MapDef) -> Layout {
        let (width, height) = (fit.width as i32, fit.height as i32);
        let pad_top = ceil_div(insets.top, fit.scale);
        let pad_bottom = ceil_div(insets.bottom, fit.scale);
        let hud_x = (width - DESIGN_W as i32) / 2;
        let bar_bottom = pad_top + TOP_H;
        let bottom_top = height - pad_bottom - BOTTOM_H;
        let at = voyage::board_origin(width, bar_bottom, bottom_top);

        let row = 6 * DIE + 5 * DICE_GAP;
        let dice_x = (width - row) / 2;
        let dice_y = bottom_top + 8;
        let dice = std::array::from_fn(|i| {
            Rect::new(dice_x + i as i32 * (DIE + DICE_GAP), dice_y, DIE, DIE)
        });
        let roll_y = dice_y + DIE + 6;
        let menu_x = hud_x + DESIGN_W as i32 - HUD_INSET - MENU_W;
        let box_top = pad_top + BOX_Y;
        let track = Track::one_row(map, hud_x, pad_top + STRIP_Y);
        let island = Rect::new(0, bar_bottom, width, bottom_top - bar_bottom);
        Layout {
            fit,
            width,
            height,
            landscape: None,
            key_large: false,
            pad_top,
            pad_bottom,
            pad_left: 0,
            pad_right: 0,
            isle: IslandView::fit(&fit, island, land_core(map), land_extent(map), Some(at)),
            island,
            bars: vec![
                Bar {
                    rect: Rect::new(0, 0, width, bar_bottom),
                    edge: Edge::Bottom,
                },
                Bar {
                    rect: Rect::new(0, bottom_top, width, height - bottom_top),
                    edge: Edge::Top,
                },
            ],
            hud_x,
            score_tray: Rect::new(hud_x + HUD_INSET, box_top, SCORE_W, BOX_H),
            score: Rect::new(hud_x + HUD_INSET, box_top, SCORE_W, BOX_H),
            costs: Rect::new(
                hud_x + HUD_INSET + SCORE_W + BOX_GAP,
                box_top,
                COSTS_W,
                BOX_H,
            ),
            costs_label: None,
            turns_label: None,
            bank: None,
            bank_label: None,
            menu: Rect::new(menu_x, box_top, MENU_W, BOX_H),
            menu_hit: Rect::new(menu_x - 2, pad_top, MENU_W + 4, BOX_Y + BOX_H + 2),
            badges: BadgePlace::Sign {
                right: hud_x + DESIGN_W as i32 - HUD_INSET,
                top: bar_bottom + SIGN_DROP,
            },
            track,
            dice,
            roll: Rect::new((width - ROLL_W) / 2, roll_y, ROLL_W, ROLL_H),
            undo: Rect::new(
                hud_x + DESIGN_W as i32 - UNDO_SIZE - 2,
                roll_y + (ROLL_H - UNDO_SIZE) / 2,
                UNDO_SIZE,
                UNDO_SIZE,
            ),
            mini_x: hud_x + MINI_MARGIN + MINI_PAD,
            mini_y: bottom_top - MINI_PAD - MINI_DIE,
            mini_per_row: MINI_PER_ROW,
            mini_tab: MiniTab::Bar {
                bar_top: bottom_top,
            },
        }
    }

    /// A tablet on its side or a desktop window: the upright island between two columns of
    /// wood, the HUD on one side and the dice on the side that was the bottom of the screen,
    /// the menu in the screen's top right corner. The HUD holds the score (with a race's
    /// bonuses beside it), the costs key at 16 px under its label, and the turn track under
    /// its label running back and forth in three rows. Each column spreads its groups down
    /// its height, so the wood between them stays even.
    fn landscape(fit: Fit, insets: Insets, map: &'static MapDef, side: DiceSide) -> Layout {
        let (width, height) = (fit.width as i32, fit.height as i32);
        let pad_top = ceil_div(insets.top, fit.scale);
        let pad_bottom = ceil_div(insets.bottom, fit.scale);
        let pad_left = ceil_div(insets.left, fit.scale);
        let pad_right = ceil_div(insets.right, fit.scale);
        // One panel down the dice's side, the island and its sea in all the rest
        let col_x = match side {
            DiceSide::Right => width - pad_right - COL_W,
            DiceSide::Left => pad_left,
        };
        let hud_x = col_x;
        let island = match side {
            DiceSide::Right => Rect::new(
                pad_left,
                pad_top,
                col_x - pad_left,
                height - pad_top - pad_bottom,
            ),
            DiceSide::Left => Rect::new(
                col_x + COL_W,
                pad_top,
                width - pad_right - col_x - COL_W,
                height - pad_top - pad_bottom,
            ),
        };
        // The land centred there with sea round it, at the largest scale it fits
        let isle = IslandView::fit(&fit, island, land_core(map), land_extent(map), None);

        // The panel stands on the sea, the way the portrait bars hug what they hold: from the
        // top the score with the menu in the corner, the costs and the turns; from the bottom,
        // under the thumbs, the roll, the dice and the bank. What height is left over parts
        // the two.
        let top = pad_top + FLOAT;
        let floor = height - pad_bottom - FLOAT;
        let panel_x = col_x + COL_INSET;
        let menu = Rect::new(
            col_x + COL_W - COL_INSET - LAND_MENU_W,
            top + PANEL_PAD,
            LAND_MENU_W,
            BOX_H,
        );
        let score_tray = Rect::new(
            panel_x,
            top + PANEL_PAD,
            PANEL_W - LAND_MENU_W - BOX_GAP,
            BOX_H,
        );
        // The score's words and number on the tray's left, a race's bonuses on its right
        let score_w = if has_badges(map) {
            SCORE_W + 4
        } else {
            score_tray.w
        };
        let score = Rect::new(score_tray.x, score_tray.y, score_w, BOX_H);
        let stacked = [map.long_road.is_some(), map.trader_bonus_at > 0]
            .iter()
            .filter(|&&b| b)
            .count() as i32;
        let badges = BadgePlace::Panel {
            right: score_tray.x + score_tray.w - 5,
            top: score_tray.y + (BOX_H - 7 - (stacked - 1).max(0) * BADGE_ROW) / 2,
        };
        // Rows the panel has beyond what it needs go into its gaps alike, up to GAP_MAX each,
        // and any more between the turns and the bank
        let room = floor - top - 2 * PANEL_PAD;
        let need = column_h(&PANEL_GROUPS) - 2 * COL_INSET;
        let n = PANEL_GROUPS.len() as i32 - 1;
        let gap = (GROUP_GAP + (room - need).max(0) / n).min(GAP_MAX);
        let costs_label = Pt::new(panel_x + 1, score_tray.y + BOX_H + gap);
        let costs = Rect::new(panel_x, costs_label.y + LABEL_H, PANEL_W, KEY_H);
        let turns_label = Pt::new(panel_x + 1, costs.y + KEY_H + gap);
        let track = Track::back_and_forth(map, col_x, turns_label.y + LABEL_H);

        let roll_y = floor - PANEL_PAD - ROLL_BLOCK_H;
        let dice_y = roll_y - gap - DICE_BLOCK_H;
        let bank_y = dice_y - gap - MINI_TRAY_H;
        let bank_label = Pt::new(panel_x + 1, bank_y - LABEL_H);
        let block = 3 * DIE + 2 * DICE_GAP;
        let dx = col_x + (COL_W - block) / 2;
        let dice = std::array::from_fn(|i| {
            let (c, row) = (i as i32 % 3, i as i32 / 3);
            Rect::new(
                dx + c * (DIE + DICE_GAP),
                dice_y + row * (DIE + DICE_GAP),
                DIE,
                DIE,
            )
        });
        let minis = LAND_MINI_PER_ROW * (MINI_DIE + MINI_GAP) - MINI_GAP;
        let bank = Rect::new(panel_x, bank_y, PANEL_W, MINI_TRAY_H);
        let bars = vec![Bar {
            rect: Rect::new(col_x + FLOAT, top, COL_W - 2 * FLOAT, floor - top),
            edge: Edge::Around,
        }];
        Layout {
            fit,
            width,
            height,
            landscape: Some(side),
            key_large: true,
            pad_top,
            pad_bottom,
            pad_left,
            pad_right,
            isle,
            island,
            bars,
            hud_x,
            score_tray,
            score,
            costs,
            costs_label: Some(costs_label),
            turns_label: Some(turns_label),
            bank: Some(bank),
            bank_label: Some(bank_label),
            menu,
            menu_hit: Rect::new(menu.x - 2, menu.y - 2, menu.w + 4, menu.h + 4),
            badges,
            track,
            dice,
            roll: Rect::new(panel_x, roll_y, LAND_ROLL_W, ROLL_H),
            undo: Rect::new(
                panel_x + PANEL_W - UNDO_SIZE,
                roll_y + (ROLL_H - UNDO_SIZE) / 2,
                UNDO_SIZE,
                UNDO_SIZE,
            ),
            mini_x: bank.x + (bank.w - minis) / 2,
            mini_y: bank.y + 1 + MINI_PAD,
            mini_per_row: LAND_MINI_PER_ROW,
            mini_tab: MiniTab::Tray,
        }
    }

    /// The interface's scale: screen pixels per interface pixel.
    pub fn ui_scale(&self) -> i32 {
        self.fit.scale as i32
    }

    /// The screen in screen pixels: the image the frame is shown as, one pixel per device
    /// pixel. Device pixels left over beyond it (fewer than the interface's scale) show the
    /// canvas's sea colour.
    pub fn screen_size(&self) -> (u32, u32) {
        (
            self.fit.width * self.fit.scale,
            self.fit.height * self.fit.scale,
        )
    }

    /// The spot under a point of the canvas given in device pixels (ArtCanvas.toArtX).
    pub fn to_spot(&self, device_x: f64, device_y: f64) -> Spot {
        let sx = (device_x - self.fit.offset_x as f64).floor() as i32;
        let sy = (device_y - self.fit.offset_y as f64).floor() as i32;
        self.screen_spot(sx, sy)
    }

    /// The spot at a screen pixel.
    pub fn screen_spot(&self, sx: i32, sy: i32) -> Spot {
        let k = self.ui_scale();
        Spot {
            x: sx.div_euclid(k),
            y: sy.div_euclid(k),
            sx,
            sy,
        }
    }

    /// The spot in the middle of an interface pixel.
    pub fn ui_spot(&self, x: i32, y: i32) -> Spot {
        let k = self.ui_scale();
        self.screen_spot(x * k + k / 2, y * k + k / 2)
    }

    /// The spot in the middle of the island pixel a board pixel shows on.
    pub fn board_spot(&self, x: i32, y: i32) -> Spot {
        let (ix, iy) = self.isle.to_frame(x, y);
        let k = self.isle.scale as i32;
        self.screen_spot(ix * k + k / 2, iy * k + k / 2)
    }

    /// The board pixel under a spot, at the island's own scale.
    pub fn spot_to_board(&self, spot: Spot) -> (i32, i32) {
        let k = self.isle.scale as i32;
        self.isle
            .to_board(spot.sx.div_euclid(k), spot.sy.div_euclid(k))
    }

    /// The interface pixel nearest a point of the island's image (where something that
    /// starts on the island and moves into the interface starts from).
    pub fn island_to_ui(&self, x: i32, y: i32) -> Pt {
        let (k, ui) = (self.isle.scale as i32, self.ui_scale());
        Pt::new(
            (x * k + ui / 2).div_euclid(ui),
            (y * k + ui / 2).div_euclid(ui),
        )
    }

    /// The interface rectangle that covers a rectangle of the board wherever its island
    /// pixels fall (the tutorial's outline round a piece).
    pub fn board_rect_to_ui(&self, r: Rect) -> Rect {
        let f = self.isle.rect_to_frame(r);
        let (k, ui) = (self.isle.scale as i32, self.ui_scale());
        let x0 = (f.x * k).div_euclid(ui);
        let y0 = (f.y * k).div_euclid(ui);
        let x1 = -((-(f.x + f.w) * k).div_euclid(ui));
        let y1 = -((-(f.y + f.h) * k).div_euclid(ui));
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// The top left of the mini die in the nth place (0 based).
    pub fn slot_x(&self, n: i32) -> i32 {
        self.mini_x + (n % self.mini_per_row) * (MINI_DIE + MINI_GAP)
    }

    pub fn slot_y(&self, n: i32) -> i32 {
        let row = (n / self.mini_per_row) * (MINI_DIE + MINI_GAP);
        match self.mini_tab {
            // Rows rise from the bar's tab, and run down the tray from its top
            MiniTab::Bar { .. } => self.mini_y - row,
            MiniTab::Tray => self.mini_y + row,
        }
    }

    /// The device pixel rectangle the art is shown in.
    pub fn device_rect(&self) -> (u32, u32, u32, u32) {
        (
            self.fit.offset_x,
            self.fit.offset_y,
            self.fit.width * self.fit.scale,
            self.fit.height * self.fit.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::Image;

    fn tarnholm() -> &'static MapDef {
        MapDef::tarnholm()
    }

    #[test]
    fn phone_portrait_is_the_android_layout() {
        // A 412 x 892 phone at 2.625: 1082 x 2342 device pixels
        let l = Layout::new(1082, 2342, Insets::default(), tarnholm(), DiceSide::Right);
        assert_eq!(l.landscape, None);
        assert_eq!(l.fit.scale, 5);
        assert_eq!((l.width, l.height), (216, 468));
        assert_eq!(l.hud_x, 0);
        assert_eq!(MENU_W, 23);
        assert_eq!(l.menu, Rect::new(216 - 6 - 23, 6, 23, 34));
        assert_eq!(l.costs, Rect::new(49, 6, 135, 34));
        // The bottom bar's top, and the dice 8 rows below it, centred
        assert_eq!(l.dice[0], Rect::new(9, 468 - 66 + 8, 28, 28));
        assert_eq!(l.roll, Rect::new(60, 468 - 66 + 8 + 34, 96, 20));
        assert_eq!(l.track.rows.len(), 1);
        assert_eq!(l.track.rows[0].band, Rect::new(6, 43, 204, 15));
        // The island's board sits where the voyage module puts it
        let at = voyage::board_origin(216, 62, 468 - 66);
        assert_eq!((l.isle.board_x, l.isle.board_y), (at.x, at.y));
    }

    #[test]
    fn a_short_portrait_screen_pads_for_system_bars() {
        let insets = Insets {
            top: 60,
            bottom: 90,
            ..Insets::default()
        };
        let l = Layout::new(720, 1280, insets, tarnholm(), DiceSide::Right);
        assert_eq!(l.fit.scale, 3);
        assert_eq!((l.pad_top, l.pad_bottom), (20, 30));
        assert_eq!(l.track.rows[0].band.y, 20 + STRIP_Y);
        assert_eq!(l.dice[0].y, l.height - 30 - BOTTOM_H + 8);
    }

    #[test]
    fn a_wide_phone_shaped_window_gets_the_columns_with_the_island_upright() {
        // A phone is locked upright, so a canvas this long is a desktop window or a tablet in
        // split screen: the columns, never the portrait art turned
        for side in [DiceSide::Right, DiceSide::Left] {
            let l = Layout::new(2342, 1082, Insets::default(), tarnholm(), side);
            assert_eq!(l.landscape, Some(side));
            assert!(l.key_large);
            let (_, ny) = l.isle.to_frame(252, 100);
            let (_, sy) = l.isle.to_frame(252, 400);
            assert!(ny < sy);
        }
    }

    /// How a canvas is laid out: landscape or not, the interface's scale and the island's.
    fn scales(w: u32, h: u32, insets: Insets) -> (bool, u32, u32) {
        let l = Layout::new(w, h, insets, tarnholm(), DiceSide::Right);
        (l.landscape.is_some(), l.fit.scale, l.isle.scale)
    }

    #[test]
    fn each_region_takes_the_largest_scale_it_fits() {
        let none = Insets::default();
        // Phones: upright, everything at 5. A phone is locked to portrait, so a canvas of a
        // phone's shape on its side is a desktop window or a split screen: it gets the panel.
        assert_eq!(scales(1082, 2342, none), (false, 5, 5));
        assert_eq!(scales(2342, 1082, none), (true, 3, 4));
        assert_eq!(scales(2400, 1080, none), (true, 3, 4));
        assert_eq!(scales(2532, 1170, none), (true, 3, 4));
        assert_eq!(scales(2778, 1284, none), (true, 3, 5));
        // A tall phone with room between its bars for the whole island at 6
        assert_eq!(scales(1284, 2778, none), (false, 5, 6));
        // Tablets, on their side and upright
        assert_eq!(scales(2560, 1600, none), (true, 4, 6));
        assert_eq!(scales(2048, 1536, none), (true, 4, 6));
        assert_eq!(scales(1600, 2560, none), (false, 7, 7));
        assert_eq!(scales(1536, 2048, none), (false, 5, 6));
        // Desktop windows, a small laptop, a very wide screen and a nearly square window
        assert_eq!(scales(1440, 900, none), (true, 2, 3));
        assert_eq!(scales(1920, 1080, none), (true, 3, 4));
        assert_eq!(scales(2560, 1440, none), (true, 4, 5));
        assert_eq!(scales(3840, 2160, none), (true, 6, 8));
        assert_eq!(scales(1366, 768, none), (true, 2, 3));
        assert_eq!(scales(1280, 720, none), (true, 2, 2));
        assert_eq!(scales(1280, 680, none), (true, 1, 2));
        assert_eq!(scales(1000, 900, none), (true, 2, 3));
        assert_eq!(scales(3440, 1440, none), (true, 4, 5));
        assert_eq!(scales(900, 500, none), (true, 1, 1));
        // Tiny and degenerate canvases still lay out, at 1
        assert_eq!(scales(300, 300, none), (false, 1, 1));
        assert_eq!(scales(1, 1, none), (false, 1, 1));
        assert_eq!(scales(0, 0, none), (false, 1, 1));
        // System bars take their room from the scales too
        let bars = Insets {
            left: 120,
            right: 120,
            ..Insets::default()
        };
        assert_eq!(scales(1920, 1080, bars), (true, 3, 4));
        assert_eq!(scales(2560, 1600, bars), (true, 4, 6));
    }

    #[test]
    fn the_panel_holds_everything_inside_it() {
        for map in MapDef::all() {
            for (w, h) in [
                (2560, 1600),
                (2560, 1440),
                (1920, 1080),
                (1440, 900),
                (1366, 768),
                (1280, 720),
            ] {
                for side in [DiceSide::Right, DiceSide::Left] {
                    let l = Layout::new(w, h, Insets::default(), map, side);
                    let what = format!("{} {w}x{h} {side:?}", map.id);
                    assert_eq!(l.landscape, Some(side), "{what}");
                    assert!(l.key_large);
                    assert_eq!(l.bars.len(), 1, "{what}");
                    let panel = l.bars[0].rect;
                    let (hud, dice) = (panel, panel);
                    let inside = |r: Rect, c: Rect| {
                        r.x >= c.x && r.y >= c.y && r.x + r.w <= c.x + c.w && r.y + r.h <= c.y + c.h
                    };
                    // The panel holds it all, clear of the ink and wood round it
                    let shrink = |c: Rect| Rect::new(c.x + 2, c.y + 2, c.w - 4, c.h - 4);
                    for r in [l.score_tray, l.costs, l.track.rect()] {
                        assert!(inside(r, shrink(hud)), "{what}: {r:?} in {hud:?}");
                    }
                    for r in l.dice.iter().chain([&l.roll, &l.undo]) {
                        assert!(inside(*r, shrink(dice)), "{what}: {r:?} in {dice:?}");
                    }
                    let minis = l.bank.expect("a bank tray");
                    assert!(inside(minis, shrink(dice)), "{what}: minis {minis:?}");
                    for n in [0, 2 * LAND_MINI_PER_ROW - 1] {
                        let die = Rect::new(l.slot_x(n), l.slot_y(n), MINI_DIE, MINI_DIE);
                        assert!(inside(die, minis), "{what}: {die:?}");
                    }
                    assert!(minis.y + minis.h <= l.dice[0].y);
                    assert!(l.turns_label.expect("a turns label").y < minis.y);
                    // On the dice's side, top to bottom, the island and its sea beside it
                    let col_x = match side {
                        DiceSide::Right => l.width - l.pad_right - COL_W,
                        DiceSide::Left => l.pad_left,
                    };
                    assert_eq!(panel.x, col_x + FLOAT, "{what}");
                    assert!(!l.island.intersects(&panel), "{what}");
                    assert_eq!(l.island.w, l.width - l.pad_left - l.pad_right - COL_W);
                    assert_eq!(l.bars[0].edge, Edge::Around);
                    assert_eq!(panel.y, l.pad_top + FLOAT);
                    assert_eq!(panel.y + panel.h, l.height - l.pad_bottom - FLOAT);
                    // The menu in the panel's top right corner, clear of all else
                    assert!(inside(l.menu, shrink(panel)), "{what}");
                    assert_eq!(l.menu.x + l.menu.w, panel.x + panel.w - PANEL_PAD);
                    assert_eq!(l.menu.y, l.pad_top + COL_INSET);
                    assert!(!l.roll.intersects(&l.undo), "{what}");
                    for r in [l.score_tray, l.costs, l.track.rect(), minis] {
                        assert!(!l.menu.intersects(&r), "{what}: {r:?}");
                    }
                    // Top to bottom without overlap: the score, the costs' label and tray,
                    // the track's label and its rows
                    let costs_label = l.costs_label.expect("a costs label");
                    let turns_label = l.turns_label.expect("a turns label");
                    assert!(l.score_tray.y + l.score_tray.h <= costs_label.y, "{what}");
                    assert_eq!(costs_label.y + LABEL_H, l.costs.y);
                    assert!(l.costs.y + l.costs.h <= turns_label.y, "{what}");
                    assert_eq!(turns_label.y + LABEL_H, l.track.rows[0].band.y);
                    let t = &l.track;
                    assert_eq!(t.rows.len() as i32, TRACK_ROWS, "{what}");
                    assert_eq!(t.rows.iter().map(|r| r.cells).sum::<i32>(), t.turns);
                    for r in &t.rows {
                        let first = r.place(r.strip.cell_x(0), rart::TRACK_FLAG_W);
                        let last = r.place(r.strip.cell_x(r.cells - 1), rart::TRACK_FLAG_W);
                        for x in [first, last] {
                            assert!(x > r.band.x && x + rart::TRACK_FLAG_W < r.band.x + r.band.w);
                        }
                        if r.pier {
                            let pier = r.place(r.strip.pier_x(), rart::TRACK_PIER_W);
                            assert!(pier + rart::TRACK_PIER_W < r.band.x + r.band.w, "{what}");
                        }
                    }
                    // Back and forth: every other row sails right to left, the last to the pier
                    for (i, r) in t.rows.iter().enumerate() {
                        assert_eq!(r.reverse, i % 2 == 1);
                        assert_eq!(
                            r.band,
                            Rect {
                                y: r.band.y,
                                ..t.rows[0].band
                            }
                        );
                    }
                    assert!(!t.rows[t.rows.len() - 1].reverse);
                    // Everything inside the height left by the system bars
                    let floor = l.height - l.pad_bottom;
                    assert!(l.menu.y >= l.pad_top && t.rect().y + t.rect().h <= floor);
                    assert!(minis.y >= l.pad_top && l.undo.y + l.undo.h <= floor);
                }
            }
        }
    }

    #[test]
    fn a_larger_island_shows_all_its_land_inside_its_part_of_the_screen() {
        for (w, h) in [
            (2560, 1600),
            (1440, 900),
            (1920, 1080),
            (1366, 768),
            (900, 500),
            (1284, 2778),
        ] {
            for side in [DiceSide::Right, DiceSide::Left] {
                let l = Layout::new(w, h, Insets::default(), tarnholm(), side);
                let land = land_core(tarnholm());
                let ui = l.board_rect_to_ui(land);
                assert!(
                    ui.x >= l.island.x
                        && ui.y >= l.island.y
                        && ui.x + ui.w <= l.island.x + l.island.w
                        && ui.y + ui.h <= l.island.y + l.island.h,
                    "{w}x{h} {side:?}: {ui:?} outside {:?}",
                    l.island
                );
                // Centred across, to within the rounding of an island pixel at each end, in
                // screen pixels, unless moved aside to show more of the moored ship
                let f = l.isle.rect_to_frame(land);
                let (k, u) = (l.isle.scale as i32, l.ui_scale());
                let left = f.x * k - l.island.x * u;
                let right = (l.island.x + l.island.w) * u - (f.x + f.w) * k;
                let full = land_extent(tarnholm());
                let ship = (land.x - full.x).max(full.x + full.w - land.x - land.w);
                assert!(left >= 0 && right >= 0);
                assert!(
                    (left - right).abs() <= 2 * k + 2 * ship * k,
                    "{w}x{h} {side:?}: {left} {right}"
                );
                // The island's image covers the whole screen, so the sea reaches every edge
                let (sw, sh) = l.screen_size();
                let k = l.isle.scale;
                assert!(l.isle.frame_w as u32 * k >= sw && l.isle.frame_h as u32 * k >= sh);
                assert!((l.isle.frame_w as u32 - 1) * k < sw);
            }
        }
    }

    #[test]
    fn taps_go_back_to_the_board() {
        for side in [DiceSide::Right, DiceSide::Left] {
            let l = Layout::new(2342, 1082, Insets::default(), tarnholm(), side);
            let b = board::Board::for_map(tarnholm());
            for p in b.village.iter().chain(&b.road_anchor) {
                let spot = l.board_spot(p.x, p.y);
                assert!(l.island.contains(spot.x, spot.y), "{side:?} {p:?}");
                assert_eq!(l.spot_to_board(spot), (p.x, p.y));
            }
        }
        let l = Layout::new(1082, 2342, Insets::default(), tarnholm(), DiceSide::Right);
        assert_eq!(
            l.isle.to_board(10, 300),
            (10 - l.isle.board_x, 300 - l.isle.board_y)
        );
        // A device point goes to the art pixel it is in (the canvas is 2 device pixels wider
        // wider than the screen, which starts a pixel in, and the art starts at the top)
        let spot = l.to_spot(12.0, 501.0);
        assert_eq!((spot.x, spot.y, spot.sx, spot.sy), (2, 100, 11, 501));
    }

    #[test]
    fn the_sea_reflects_at_the_board_edge() {
        // Inside the board nothing moves
        assert_eq!(mirror(0, 504, 96), 0);
        assert_eq!(mirror(503, 504, 96), 503);
        // One step out on either side is the edge pixel itself, then back inward
        assert_eq!(mirror(-1, 504, 96), 0);
        assert_eq!(mirror(-2, 504, 96), 1);
        assert_eq!(mirror(504, 504, 96), 503);
        assert_eq!(mirror(505, 504, 96), 502);
        // At the band's inner edge it turns back out, so it never reaches the coast
        assert_eq!(mirror(-96, 504, 96), 95);
        assert_eq!(mirror(-97, 504, 96), 95);
        assert_eq!(mirror(-98, 504, 96), 94);
        for u in -2000..2000 {
            let m = mirror(u, 504, 96);
            assert!((0..504).contains(&m));
            if !(0..504).contains(&u) {
                assert!(!(96..408).contains(&m), "{u} -> {m}");
            }
            // Neighbours stay neighbours: no seam anywhere
            assert!((mirror(u + 1, 504, 96) - m).abs() <= 1);
        }
    }

    #[test]
    fn the_sea_layer_covers_any_frame() {
        let mut board = Image::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                board.fill_argb(x, y, 1, 1, 0xFF000000 | (x as u32) << 8 | y as u32);
            }
        }
        let layer = sea_layer(&board, 20, 12, 5, 2);
        // The board where it is, and its mirror image beside it
        assert_eq!(layer.argb_at(5, 2), board.argb_at(0, 0));
        assert_eq!(layer.argb_at(4, 2), board.argb_at(0, 0));
        assert_eq!(layer.argb_at(3, 3), board.argb_at(1, 1));
        assert_eq!(layer.argb_at(13, 9), board.argb_at(7, 7));
        assert_eq!(layer.argb_at(14, 9), board.argb_at(6, 7));
        assert!(layer.to_argb().iter().all(|&p| p >> 24 == 0xFF));
    }

    #[test]
    fn the_landscape_track_runs_back_and_forth() {
        let l = Layout::new(1920, 1080, Insets::default(), tarnholm(), DiceSide::Right);
        let t = &l.track;
        assert_eq!(t.rows.len(), 3);
        assert_eq!(t.rows.iter().map(|r| r.cells).sum::<i32>(), t.turns);
        // The ship: the first row left to right, the second back, the third and the
        // harbour left to right again
        let (a, b) = (t.rows[0].cells, t.rows[1].cells);
        assert_eq!(t.ship_row(0), (0, 0));
        assert_eq!(t.ship_row(a), (1, 0));
        assert_eq!(t.ship_row(a + b), (2, 0));
        assert_eq!(t.ship_row(t.turns), (2, t.rows[2].cells));
        let r = &t.rows[1];
        assert!(r.place(r.strip.cell_x(0), 13) > r.place(r.strip.cell_x(1), 13));
        // The tutorial's rectangle spans all three bands
        assert_eq!(t.rect().h, TRACK_BLOCK_H);
    }

    #[test]
    fn mini_dice_rows() {
        let l = Layout::new(1082, 2342, Insets::default(), tarnholm(), DiceSide::Right);
        assert_eq!(l.slot_x(1) - l.slot_x(0), MINI_DIE + MINI_GAP);
        assert_eq!(l.slot_x(6), l.slot_x(0));
        assert_eq!(l.slot_y(6), l.slot_y(0) - MINI_DIE - MINI_GAP);
        let land = Layout::new(1920, 1080, Insets::default(), tarnholm(), DiceSide::Right);
        assert_eq!(land.slot_y(5), land.slot_y(0) + MINI_DIE + MINI_GAP);
        // Two rows of mini dice stay below the column's top and above the dice
        assert!(land.slot_y(0) >= land.pad_top);
        assert!(land.slot_y(5) + MINI_DIE < land.dice[0].y);
    }

    #[test]
    fn sprite_sizes_match_the_art() {
        let a = super::super::assets::Assets::load();
        assert_eq!(
            (a.village.width as i32, a.village.height as i32),
            (VILLAGE_W, VILLAGE_H)
        );
        assert_eq!(
            (a.castle.width as i32, a.castle.height as i32),
            (CASTLE_W, CASTLE_H)
        );
        assert_eq!(
            (a.trader.width as i32, a.trader.height as i32),
            (TRADER_W, TRADER_H)
        );
        assert_eq!(a.button_normal.width as i32, 24);
    }
}
