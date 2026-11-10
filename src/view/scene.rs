// SPDX-License-Identifier: GPL-3.0-or-later
//! The game screen drawn into art framebuffers: the island, the HUD, the dice, the hire
//! animations, the tutorial's callouts and a dragged mini die. A port of GameView's drawing.
//!
//! The island and the interface are two images, each at its own scale (see `layout`), put
//! together a screen row at a time as the frame is encoded (`Frame::to_png`). Where both
//! share a scale, as on a phone, the interface is drawn straight over the island and there
//! is one image, as there always was.
//!
//! Also here: which thing a tap means (`Scene::hit`), since that depends on what was drawn.

use crate::art::{self, Image, Layer, Mask};
use crate::rules::art as rart;
use crate::rules::board::RoadShape;
use crate::rules::board_layers;
use crate::rules::build_plan::Kind;
use crate::rules::dice::DiceValue;
use crate::rules::game::{self, Game};
use crate::rules::hire_anim;
use crate::rules::map_def::MapDef;
use crate::rules::pieces::{BoardPieces, Piece};
use crate::rules::pt::Pt;
use crate::rules::spend_anim;
use crate::rules::tutorial::{Step, Tutorial};
use crate::rules::tutorial_layout::{self, Placement, Rect, Side};
use crate::rules::view_model::{self, GlowTier, RollLabel};
use crate::rules::voyage::{self, Stage};

use super::assets::{Assets, Tinted, WATER_STEP_MS, WATER_STEPS};
use super::layout::{
    self, BadgePlace, DIE, Edge, Layout, MINI_DIE, MINI_GAP, MINI_PAD, MiniTab, Spot, TrackRow,
    UNDO_SIZE,
};

// The UI kit's palette (tools/make_ui.py), ARGB
pub const INK: u32 = 0xFF1E140A;
pub const WOOD_DARK: u32 = 0xFF33261A;
pub const WOOD: u32 = 0xFF5A3C14;
pub const WOOD_LIGHT: u32 = 0xFF86622E;
pub const IVORY: u32 = 0xFFFFF4D6;
/// The tile under each legend glyph, the dice faces' colour.
pub const LEGEND_TILE: u32 = 0xFFE0B870;
pub const AMBER: u32 = 0xFFE6A817;
/// Numbers on unbuilt spots.
pub const QUIET: u32 = 0xFFD6C7A0;
pub const TRAY_FACE: u32 = 0xFF261B12;
pub const MUTED: u32 = 0xFFD2BE94;
pub const GOLD: u32 = LEGEND_TILE;
const MENU_FACE: u32 = 0xFF4A331C;
const MENU_FACE_DOWN: u32 = 0xFF3A2916;
const MENU_LIGHT: u32 = 0xFF6E4E2A;
const MENU_DARK: u32 = 0xFF2E2012;
const MENU_BAR_W: i32 = 11;
const MENU_BAR_H: i32 = 2;
const MENU_BAR_PITCH: i32 = 4;
/// The shadow of everything not built yet: black at 40%.
const FOOTPRINT: u32 = 0x66000000;
const OFFER_SPRITE_ALPHA: u8 = 150;
const CALLOUT_CORNER: i32 = 6;
const CALLOUT_PLATE_H: i32 = 11;
const CALLOUT_PLATE_TOP: i32 = 1;
// Ark Pixel's line box: capitals and digits are rows 4 to 10, an accent starts at row 1
const FONT_CAP_ROW: i32 = 4;
/// The display font's first row of ink in its cell.
const DISPLAY_CAP_ROW: i32 = 0;
const FONT_CAPS: i32 = 7;
const FONT_TOP_ROW: i32 = 1;
// Micro 5's digits are rows 3 to 7 of its line box
const NUMERAL_ROW: i32 = 3;
const NUMERAL_H: i32 = 5;
const RECIPE_H: i32 = layout::RECIPE_H;
const RECIPE_ROW: i32 = RECIPE_H + 1;
const RECIPE_INSET_Y: i32 = 3;
const BUOY_DROP: i32 = 4;
/// A race's sign: its height, and the space inside it either side of its label.
const SIGN_H: i32 = 13;
const SIGN_PAD: i32 = 3;
const BADGE_GLYPH: i32 = 7;
const BADGE_GAP_IN: i32 = 2;
const BADGE_GAP: i32 = 5;
const BADGE_DIM_ALPHA: u8 = 140;
const TRADER_NUMBER_DROP: i32 = 3;
const ROLL_CORNER: i32 = 6;
/// The landscape undo button while there is nothing to take back.
const UNDO_IDLE_ALPHA: u8 = 70;
const ROLL_LABEL: u32 = 0xFF1E140A;
const ROLL_LABEL_DISABLED: u32 = 0xFF4A463E;
/// The dragged die is drawn this far above the finger, so the fingertip does not hide it.
pub const DRAG_RAISE: i32 = 12;
const HIRE_GLOW_REACH: i32 = 2;
/// The part of the board the footprint and offer layers cover: the island and every sprite
/// that can stand on it.
const LAYER_X: i32 = 96;
const LAYER_Y: i32 = 96;
const LAYER_W: i32 = 312;
const LAYER_H: i32 = 312;
/// Offer colours from the weakest claim on a shared pixel to the strongest.
const OFFER_COLORS: [u32; 6] = [
    view_model::HIGHLIGHT_CHAIN,
    view_model::HIGHLIGHT_KNIGHT_CHAIN,
    view_model::HIGHLIGHT_WILDCARD_CHAIN,
    view_model::HIGHLIGHT,
    view_model::HIGHLIGHT_KNIGHT,
    view_model::HIGHLIGHT_WILDCARD,
];
/// Which family each offer colour belongs to: 0 the greens, 1 the blues.
const OFFER_FAMILY: [usize; 6] = [0, 1, 1, 0, 1, 1];
/// The village's cost as the key lists it: brick first (a cost is a set; only the picture differs).
const LEGEND_VILLAGE: [DiceValue; 4] = [
    DiceValue::Brick,
    DiceValue::Lumber,
    DiceValue::Wool,
    DiceValue::Grain,
];

/// The words the screen draws, already formatted for the current locale.
#[derive(Clone, Debug, Default)]
pub struct Strings {
    pub score: String,
    pub bonuses: String,
    /// The landscape HUD's labels over the costs and the turn track.
    pub costs: String,
    pub turns: String,
    pub bank: String,
    pub first_roll: String,
    pub second_roll: String,
    pub third_roll: String,
    pub play_again: String,
    pub finish: String,
    pub done: String,
    pub next_turn: String,
    pub got_it: String,
    pub skip: String,
    /// The tutorial's text for each step, the ship's for this game's kind of island.
    pub tutorial: Vec<(Step, String)>,
    /// A played game is on the board, not the one in progress: the roll button reads this
    /// and takes the player back.
    pub viewing: Option<String>,
}

impl Strings {
    pub fn roll(&self, label: RollLabel) -> &str {
        match label {
            RollLabel::FirstRoll => &self.first_roll,
            RollLabel::SecondRoll => &self.second_roll,
            RollLabel::ThirdRoll => &self.third_roll,
            RollLabel::PlayAgain => &self.play_again,
            RollLabel::Done => &self.done,
            RollLabel::Finish => &self.finish,
            RollLabel::NextTurn => &self.next_turn,
        }
    }

    fn tutorial(&self, step: Step) -> &str {
        self.tutorial
            .iter()
            .find(|(s, _)| *s == step)
            .map_or("", |(_, t)| t.as_str())
    }
}

/// A button shown pressed: while a finger holds it, or for a moment after a key or a tap on a
/// platform with no press event to hold it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Menu,
    Undo,
    Roll,
    GotIt,
    Skip,
}

/// A mini die a finger is carrying (trader resource classic mode): its bank slot and where
/// the finger is, in art pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carry {
    pub slot: usize,
    pub x: i32,
    pub y: i32,
}

/// The screen's own state that the drawing reads and the input changes.
pub struct Ui {
    /// True while a question is up: the tutorial's callout waits for it to go.
    pub tutorial_held: bool,
    pub carry: Option<Carry>,
    pub pressed: Option<(Button, u64)>,
    /// The callout as last drawn, for its buttons' taps.
    pub callout: Option<Placement>,
}

impl Default for Ui {
    fn default() -> Self {
        Ui::new()
    }
}

impl Ui {
    pub fn new() -> Ui {
        Ui {
            tutorial_held: false,
            carry: None,
            pressed: None,
            callout: None,
        }
    }

    fn is_pressed(&self, b: Button, now: u64) -> bool {
        self.pressed.is_some_and(|(p, until)| p == b && now < until)
    }
}

/// What a tap at an art pixel means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    GotIt,
    Skip,
    Menu,
    Undo,
    Die(usize),
    Roll,
    /// A banked mini die (only a drag does anything with it).
    MiniDie(usize),
    Piece(Piece),
    Nothing,
}

/// One hire's animation (hire_anim): the trader, when it began, and what it turned out to be.
#[derive(Clone, Copy, Debug)]
struct Hire {
    tier: usize,
    slot: usize,
    start: u64,
    epoch: u32,
    completes: bool,
    /// None until decided at the next frame, then whether the resource flies.
    flies: Option<bool>,
    /// The place on the tab the resource lands in, once known.
    place: Option<i32>,
}

/// The ship's sail on the board: the stage drawn last frame, and the sail under way.
#[derive(Clone, Copy, Debug, Default)]
struct Sail {
    shown: Option<Stage>,
    from: Pt,
    start: u64,
    ms: i32,
}

#[derive(Default)]
struct OfferLayers {
    stand_fill: [Option<Mask>; 6],
    stand_rim: [Option<Mask>; 6],
    road_fill: [Option<Mask>; 6],
    road_rim: [Option<Mask>; 6],
}

/// What one render gives back: the picture, as its two regions, and what else it found.
pub struct Frame {
    /// The island's image: the whole screen at the island's scale. Where the interface
    /// shares that scale it is drawn over this, and `ui` is None.
    pub island: Image,
    /// The interface at its own scale, clear wherever the island shows through.
    pub ui: Option<Image>,
    /// Something is moving: render again next frame.
    pub animating: bool,
    /// The sea is moving: render again at this time (ms), when the water takes its next step.
    pub water_due: Option<u64>,
    /// The tutorial's progress changed: save it.
    pub tutorial_dirty: bool,
}

impl Frame {
    fn layers<'a>(&'a self, l: &Layout) -> Vec<Layer<'a>> {
        let mut layers = vec![Layer {
            image: &self.island,
            scale: l.isle.scale,
        }];
        if let Some(ui) = &self.ui {
            layers.push(Layer {
                image: ui,
                scale: l.fit.scale,
            });
        }
        layers
    }

    /// The screen as a PNG of one pixel per device pixel, each region's pixels whole blocks
    /// of its own scale. With one region this is exactly the old single framebuffer's PNG.
    pub fn to_png(&self, l: &Layout) -> Vec<u8> {
        if self.ui.is_none() {
            return self.island.to_png_scaled(l.isle.scale);
        }
        let (w, h) = l.screen_size();
        art::compose_png(w, h, &self.layers(l))
    }

    /// The screen as an image, for tests and snapshots.
    pub fn composed(&self, l: &Layout) -> Image {
        let (w, h) = l.screen_size();
        art::compose(w, h, &self.layers(l))
    }
}

pub struct Scene {
    pub assets: Assets,
    map: &'static MapDef,
    pub pieces: BoardPieces,
    tinted: Option<Tinted>,
    footprints: Option<(Vec<bool>, Mask, Mask)>,
    glow: Option<(GlowKey, Image)>,
    offer_key: Option<(Vec<bool>, Vec<u32>)>,
    offers: OfferLayers,
    /// The sea layer at each step of the water's loop, as each is first needed; the still sea
    /// is step 0, the loop's first step being the still board.
    sea: Option<(SeaKey, Vec<Option<Image>>)>,
    /// Whether the sea moves (the Animate water setting); the view sets it before each render.
    pub animate_water: bool,
    sail: Sail,
    hires: Vec<Hire>,
    spends: Vec<(game::SpendEvent, u64)>,
    rings: Vec<(game::RingEvent, u64)>,
}

/// What the glow layer was built from: what is built, the player's colour, the offers.
type GlowKey = (Vec<bool>, u32, Vec<u32>);
/// What the sea layers were built for: the layer's size, the board's place in it, the island.
type SeaKey = (i32, i32, i32, i32, String);

/// Which positions are built, one entry each: the footprints and offers depend on it.
fn built_key(game: &Game) -> Vec<bool> {
    let s = &game.playsheet;
    s.traders
        .iter()
        .chain(&s.roads)
        .chain(&s.villages)
        .chain(&s.cities)
        .copied()
        .collect()
}

fn color_index(argb: u32) -> Option<usize> {
    OFFER_COLORS.iter().position(|&c| c == argb)
}

fn in_family(c: Option<usize>, family: usize) -> bool {
    c.is_some_and(|c| OFFER_FAMILY[c] == family)
}

/// The costs tray's recipes in reading order: what each builds and what it costs.
const RECIPES: [(Kind, &[DiceValue]); 4] = [
    (Kind::Road, game::RESOURCES_ROAD),
    (Kind::City, game::RESOURCES_CITY),
    (Kind::Village, &LEGEND_VILLAGE),
    (Kind::Knight, game::RESOURCES_KNIGHT),
];

/// Where the costs tray draws the recipe of `kind`: two rows of two or, in landscape, one
/// column of four at 16 px.
fn recipe_rect(l: &Layout, kind: Kind) -> Rect {
    let i = RECIPES.iter().position(|(k, _)| *k == kind).unwrap_or(0);
    if l.key_large {
        return Rect::new(
            l.costs.x + layout::KEY_INSET,
            l.costs.y + layout::KEY_INSET + i as i32 * l.give.key_row,
            layout::key_recipe_w(RECIPES[i].1.len() as i32),
            layout::KEY_ROW_H,
        );
    }
    let sp = &l.spread;
    let left = l.costs.x + sp.recipe_inset;
    let top = l.costs.y + RECIPE_INSET_Y;
    let (x, y) = if i % 2 == 0 {
        (left, top + (i as i32 / 2) * RECIPE_ROW)
    } else {
        (
            left + sp.recipe_w(recipes_before(i) as i32) + sp.recipe_gap,
            top + RECIPE_ROW * (i as i32 / 2),
        )
    };
    Rect::new(x, y, sp.recipe_w(RECIPES[i].1.len() as i32), RECIPE_H)
}

/// How many resources the recipe left of the nth one (in two rows of two) shows.
fn recipes_before(n: usize) -> usize {
    if n == 1 {
        game::RESOURCES_ROAD.len()
    } else {
        LEGEND_VILLAGE.len()
    }
}

/// How far down an icon sits on a mini die so its ink is centred on the face.
fn mini_icon_drop(v: DiceValue) -> i32 {
    match v {
        DiceValue::Brick => -2,
        DiceValue::Any => 1,
        _ => 0,
    }
}

impl Scene {
    pub fn new(map: &'static MapDef) -> Scene {
        Scene {
            assets: Assets::load(),
            map,
            pieces: BoardPieces::for_map(map),
            tinted: None,
            footprints: None,
            glow: None,
            offer_key: None,
            offers: OfferLayers::default(),
            sea: None,
            animate_water: false,
            sail: Sail::default(),
            hires: Vec::new(),
            spends: Vec::new(),
            rings: Vec::new(),
        }
    }

    /// Follows the game onto another island: every cached layer is sized for one map.
    fn follow_map(&mut self, map: &'static MapDef) {
        if std::ptr::eq(map, self.map) {
            return;
        }
        self.map = map;
        self.pieces = BoardPieces::for_map(map);
        self.footprints = None;
        self.glow = None;
        self.offer_key = None;
        self.offers = OfferLayers::default();
        self.sea = None;
        self.sail = Sail::default();
        self.hires.clear();
        self.spends.clear();
        self.rings.clear();
    }

    // ------------------------------------------------------------ pieces

    fn mask(&self, game: &Game, p: &Piece) -> &Mask {
        let a = &self.assets;
        match p.kind {
            Kind::Road => &a.road_mask[game.board.road_shape[p.index] as usize],
            Kind::Village => &a.village_mask,
            Kind::City => &a.castle_mask,
            Kind::Knight => &a.trader_mask,
        }
    }

    /// The board position of the top left of a piece's sprite.
    fn origin(game: &Game, p: &Piece) -> Pt {
        match p.kind {
            Kind::Road => game.board.road_origin[p.index],
            Kind::Village => Pt::new(p.anchor.x - rart::VILLAGE_AX, p.anchor.y - rart::VILLAGE_AY),
            Kind::City => Pt::new(p.anchor.x - rart::CASTLE_AX, p.anchor.y - rart::CASTLE_AY),
            Kind::Knight => Pt::new(p.anchor.x - rart::TRADER_AX, p.anchor.y - rart::TRADER_AY),
        }
    }

    fn sprite<'a>(t: &'a Tinted, p: &Piece) -> &'a Image {
        match p.kind {
            Kind::Village => &t.village,
            Kind::City => &t.castle,
            _ => &t.trader,
        }
    }

    fn offer_argb(game: &Game, p: &Piece) -> u32 {
        game.offer(p.kind, p.index).map_or(0, |o| o.argb())
    }

    /// Calls `f` with each layer pixel (x, y) of a piece's silhouette.
    fn stamp(&self, game: &Game, p: &Piece, mut f: impl FnMut(usize, i32, i32)) {
        let mask = self.mask(game, p);
        let o = Self::origin(game, p);
        let (ox, oy) = (o.x - LAYER_X, o.y - LAYER_Y);
        for my in 0..mask.height {
            for mx in 0..mask.width {
                let (x, y) = (ox + mx, oy + my);
                if mask.bits[(my * mask.width + mx) as usize]
                    && x >= 0
                    && y >= 0
                    && x < LAYER_W
                    && y < LAYER_H
                {
                    f((y * LAYER_W + x) as usize, x, y);
                }
            }
        }
    }

    fn all_pieces(&self) -> Vec<Piece> {
        self.pieces.all().copied().collect()
    }

    // ------------------------------------------------------------ layers

    /// The footprints of everything not built: roads in one layer, the rest in another, no
    /// pixel in both. Rebuilt only when something is built.
    fn refresh_footprints(&mut self, game: &Game, key: &[bool]) {
        if self.footprints.as_ref().is_some_and(|(k, _, _)| k == key) {
            return;
        }
        let mut roads = vec![false; (LAYER_W * LAYER_H) as usize];
        let mut stands = roads.clone();
        for p in self.all_pieces() {
            if !game.is_built(p.kind, p.index) {
                let target = if p.kind == Kind::Road {
                    &mut roads
                } else {
                    &mut stands
                };
                self.stamp(game, &p, |i, _, _| target[i] = true);
            }
        }
        let road = Mask {
            width: LAYER_W,
            height: LAYER_H,
            bits: board_layers::cut_out(&roads, &stands),
        };
        let stand = Mask {
            width: LAYER_W,
            height: LAYER_H,
            bits: stands,
        };
        self.footprints = Some((key.to_vec(), road, stand));
    }

    /// The glow round every spot that could be built next and is not on offer.
    fn refresh_glow(&mut self, game: &Game, key: &[bool], offers: &[u32]) {
        let color = game.player_color | 0xFF000000;
        let wanted = (key.to_vec(), color, offers.to_vec());
        if self.glow.as_ref().is_some_and(|(k, _)| *k == wanted) {
            return;
        }
        let n = (LAYER_W * LAYER_H) as usize;
        let mut dist = vec![-1i32; n];
        let mut tier = vec![0i32; n];
        let mut road_only = vec![false; n];
        for (i, p) in self.all_pieces().iter().enumerate() {
            let t = if offers[i] == 0 && !game.is_built(p.kind, p.index) {
                game.glow_tier(p.kind, p.index)
            } else {
                GlowTier::None
            };
            if t == GlowTier::None {
                continue;
            }
            let is_road = p.kind == Kind::Road;
            self.stamp(game, p, |q, _, _| {
                road_only[q] = is_road && (dist[q] != 0 || road_only[q]);
                dist[q] = 0;
                tier[q] = tier[q].max(t as i32);
            });
        }
        let rgb = color & 0xFFFFFF;
        let road_rgb = board_layers::desaturate(rgb, view_model::GLOW_ROAD_SATURATION) & 0xFFFFFF;
        let ring_rgb = board_layers::halo_rgb(rgb, view_model::GLOW_HALO_LIGHT_ABOVE);
        board_layers::grow(
            &mut dist,
            &mut tier,
            &mut road_only,
            LAYER_W as usize,
            LAYER_H as usize,
            2,
        );
        let mut out = vec![0u32; n];
        for p in 0..n {
            let d = dist[p];
            if d != 1 && d != 2 {
                continue;
            }
            let t = match tier[p] {
                1 => GlowTier::Faint,
                2 => GlowTier::Weak,
                3 => GlowTier::Waiting,
                _ => GlowTier::Full,
            };
            if t == GlowTier::Waiting
                && !view_model::waiting_dash_shows(p as i32 % LAYER_W, p as i32 / LAYER_W)
            {
                continue;
            }
            let strength = view_model::glow_strength(t);
            out[p] = board_layers::glow_argb(
                d,
                if road_only[p] { road_rgb } else { rgb },
                ring_rgb,
                view_model::GLOW_ALPHA,
                view_model::GLOW_HALO_ALPHA,
                strength,
            );
        }
        self.glow = Some((
            wanted,
            Image::from_argb(LAYER_W as u32, LAYER_H as u32, &out),
        ));
    }

    /// The offer layers: a fill and a rim per colour, for buildings and for roads, the roads'
    /// cut to the new ground they would add (see GameView.refreshOffers).
    fn refresh_offers(&mut self, game: &Game, key: &[bool], offers: &[u32]) {
        let wanted = (key.to_vec(), offers.to_vec());
        if self.offer_key.as_ref() == Some(&wanted) {
            return;
        }
        self.offer_key = Some(wanted);
        let pieces = self.all_pieces();
        let n = (LAYER_W * LAYER_H) as usize;
        let mut layers = OfferLayers::default();
        for (c, &color) in OFFER_COLORS.iter().enumerate() {
            let mut fill: Option<Vec<bool>> = None;
            for (i, p) in pieces.iter().enumerate() {
                if p.kind == Kind::Road || offers[i] != color {
                    continue;
                }
                let bits = fill.get_or_insert_with(|| vec![false; n]);
                self.stamp(game, p, |q, _, _| bits[q] = true);
            }
            if let Some(bits) = fill {
                let m = Mask {
                    width: LAYER_W,
                    height: LAYER_H,
                    bits,
                };
                layers.stand_rim[c] = Some(m.rim());
                layers.stand_fill[c] = Some(m);
            }
        }

        let mut owner: Vec<Option<usize>> = vec![None; n];
        let mut any_road = false;
        for (i, p) in pieces.iter().enumerate() {
            if p.kind == Kind::Road
                && let Some(c) = color_index(offers[i])
            {
                self.stamp(game, p, |q, _, _| owner[q] = owner[q].max(Some(c)));
                any_road = true;
            }
        }
        if any_road {
            let mut held = vec![false; n];
            for p in &pieces {
                if game.is_built(p.kind, p.index) || p.kind != Kind::Road {
                    self.stamp(game, p, |q, _, _| held[q] = true);
                }
            }
            let mut fill: [Option<Vec<bool>>; 6] = Default::default();
            let mut rim: [Option<Vec<bool>>; 6] = Default::default();
            for y in 0..LAYER_H {
                for x in 0..LAYER_W {
                    let p = (y * LAYER_W + x) as usize;
                    let Some(c) = owner[p] else { continue };
                    if held[p] {
                        continue;
                    }
                    fill[c].get_or_insert_with(|| vec![false; n])[p] = true;
                    let family = OFFER_FAMILY[c];
                    let edge = x == 0
                        || y == 0
                        || x == LAYER_W - 1
                        || y == LAYER_H - 1
                        || !in_family(owner[p - 1], family)
                        || !in_family(owner[p + 1], family)
                        || !in_family(owner[p - LAYER_W as usize], family)
                        || !in_family(owner[p + LAYER_W as usize], family);
                    let r = rim[c].get_or_insert_with(|| vec![false; n]);
                    if edge {
                        r[p] = true;
                    }
                }
            }
            for c in 0..6 {
                let mask = |bits: Vec<bool>| Mask {
                    width: LAYER_W,
                    height: LAYER_H,
                    bits,
                };
                layers.road_fill[c] = fill[c].take().map(mask);
                layers.road_rim[c] = rim[c].take().map(mask);
            }
        }
        self.offers = layers;
    }

    fn draw_offer(
        layer: &mut Image,
        fill: &Option<Mask>,
        rim: &Option<Mask>,
        argb: u32,
        bx: i32,
        by: i32,
    ) {
        let Some(fill) = fill else { return };
        let rim_alpha = ((argb >> 24) * 2).min(255);
        layer.silhouette(fill, bx + LAYER_X, by + LAYER_Y, argb);
        if let Some(rim) = rim {
            layer.silhouette(
                rim,
                bx + LAYER_X,
                by + LAYER_Y,
                (argb & 0xFFFFFF) | rim_alpha << 24,
            );
        }
    }

    // ------------------------------------------------------------ the frame

    /// Draws the whole screen for this game and layout at time `now` (ms).
    pub fn render(
        &mut self,
        game: &mut Game,
        l: &Layout,
        ui: &mut Ui,
        tutorial: &mut Tutorial,
        s: &Strings,
        now: u64,
    ) -> Frame {
        self.follow_map(game.map);
        for e in game.take_hire_events() {
            self.hires.push(Hire {
                tier: e.tier,
                slot: e.slot,
                start: now.saturating_add_signed(e.delay_ms as i64),
                epoch: e.epoch,
                completes: e.completes,
                flies: None,
                place: None,
            });
        }
        for e in game.take_spend_events() {
            self.spends
                .push((e, now.saturating_add_signed(e.delay_ms as i64)));
        }
        for e in game.take_ring_events() {
            self.rings.retain(|(r, _)| r.die != e.die);
            self.rings.push((e, now));
        }
        let game: &Game = game;
        let color = game.player_color | 0xFF000000;
        if self.tinted.as_ref().is_none_or(|t| t.color != color) {
            self.tinted = Some(Tinted::new(&self.assets, color));
        }
        self.update_hires(game, l, now);
        let epoch = game.state_epoch;
        self.spends.retain(|(e, start)| {
            e.epoch == epoch && (now as i64 - *start as i64) < spend_anim::total() as i64
        });
        self.rings.retain(|(e, start)| {
            e.epoch == epoch && now.saturating_sub(*start) < spend_anim::ring_total() as u64
        });
        let mut animating =
            !self.hires.is_empty() || !self.spends.is_empty() || !self.rings.is_empty();

        // The island, upright, in its own layer over the sea; the sea at its step of the
        // water's loop where it moves, else the still board (the loop's first step)
        let step = if self.animate_water {
            ((now / WATER_STEP_MS) % WATER_STEPS as u64) as usize
        } else {
            0
        };
        let v = &l.isle;
        let sea_key = (
            v.frame_w,
            v.frame_h,
            v.board_x,
            v.board_y,
            game.map.id.clone(),
        );
        if self.sea.as_ref().is_none_or(|(k, _)| *k != sea_key) {
            self.sea = Some((sea_key, vec![None; WATER_STEPS]));
        }
        let built = self
            .sea
            .as_ref()
            .is_some_and(|(_, seas)| seas[step].is_some());
        if !built {
            let sea = if self.animate_water {
                let steps = self.assets.water(&game.map.id);
                layout::sea_layer(&steps[step], v.frame_w, v.frame_h, v.board_x, v.board_y)
            } else {
                let board = self.assets.board(&game.map.id);
                layout::sea_layer(&board, v.frame_w, v.frame_h, v.board_x, v.board_y)
            };
            self.sea.as_mut().expect("sea layers").1[step] = Some(sea);
        }
        let mut layer = self.sea.as_ref().expect("sea layers").1[step]
            .clone()
            .expect("sea layer");
        animating |= self.draw_island(&mut layer, game, l, now);
        let mut island = layer;
        self.draw_island_marks(&mut island, game, l);
        self.draw_hires_on_island(&mut island, game, l, now);

        // The interface over the island where both have one scale; else on a clear image of
        // its own at its own scale, which the encoding lays over the island's
        let mut own = (v.scale != l.fit.scale).then(|| Image::new(l.width as u32, l.height as u32));
        let frame = own.as_mut().unwrap_or(&mut island);
        for bar in &l.bars {
            draw_bar(frame, bar.rect, bar.edge);
        }
        self.draw_hud(frame, game, l, ui, s, now);
        animating |= self.draw_controls(frame, game, l, ui, s, now);
        self.draw_hires(frame, game, l, now);
        self.draw_spends(frame, game, l, now);
        let tutorial_dirty = self.draw_tutorial(frame, game, l, ui, tutorial, s, now);
        self.draw_carry(frame, game, l, ui);
        animating |= ui.pressed.is_some_and(|(_, until)| now < until);
        Frame {
            island,
            ui: own,
            animating,
            water_due: self
                .animate_water
                .then(|| (now / WATER_STEP_MS + 1) * WATER_STEP_MS),
            tutorial_dirty,
        }
    }

    /// The island in board order (GameView.drawIsland), without its text and icons. Returns
    /// whether the ship is under sail.
    fn draw_island(&mut self, layer: &mut Image, game: &Game, l: &Layout, now: u64) -> bool {
        let (bx, by) = (l.isle.board_x, l.isle.board_y);
        let key = built_key(game);
        let pieces = self.all_pieces();
        let offers: Vec<u32> = pieces.iter().map(|p| Self::offer_argb(game, p)).collect();
        self.refresh_footprints(game, &key);
        self.refresh_glow(game, &key, &offers);
        self.refresh_offers(game, &key, &offers);
        let tinted = self.tinted.as_ref().expect("tinted");
        let a = &self.assets;
        let (road_fp, stand_fp) = {
            let f = self.footprints.as_ref().expect("footprints");
            (&f.1, &f.2)
        };
        layer.silhouette(road_fp, bx + LAYER_X, by + LAYER_Y, FOOTPRINT);
        layer.sprite(
            &self.glow.as_ref().expect("glow").1,
            bx + LAYER_X,
            by + LAYER_Y,
            255,
        );

        // Roads, then the cobbles of every built road again over all the kerbs, so a kerb
        // survives only on the outside of the network
        for pass in 0..2 {
            for p in &self.pieces.roads {
                if game.playsheet.roads[p.index] {
                    let o = game.board.road_origin[p.index];
                    let shape = game.board.road_shape[p.index] as usize;
                    let sprite = if pass == 0 {
                        &tinted.road[shape]
                    } else {
                        &a.road_surface[shape]
                    };
                    layer.sprite(sprite, bx + o.x, by + o.y, 255);
                }
            }
        }
        layer.sprite(
            &a.dock,
            bx + voyage::DOCK_SPRITE.x,
            by + voyage::DOCK_SPRITE.y,
            255,
        );

        // The King's ship, sailing from stop to stop
        let done = game.is_game_done();
        let stage = voyage::stage_for(game.map, game.turns_taken, done, game.playsheet.score());
        let mut at = voyage::ship_origin(stage, voyage::DOCK_MOORING, l.isle.visible_left());
        if self.sail.shown != Some(stage) {
            self.sail.ms = voyage::sail_ms(self.sail.shown, stage);
            self.sail.from = self.sail.shown.map_or(at, |from| {
                voyage::sail_start(from, voyage::DOCK_MOORING, l.isle.visible_left())
            });
            self.sail.start = now;
            self.sail.shown = Some(stage);
        }
        let mut sailing = false;
        if self.sail.ms > 0 {
            let elapsed = now.saturating_sub(self.sail.start) as i32;
            if elapsed < self.sail.ms {
                at = voyage::sail_at(self.sail.from, at, elapsed, self.sail.ms);
                sailing = true;
            } else {
                self.sail.ms = 0;
            }
        }
        if stage != Stage::None {
            // Its flag is team magenta, flying the player's colour as the strip's ship does
            layer.sprite(&tinted.king_ship, bx + at.x, by + at.y, 255);
        }

        for (c, &color) in OFFER_COLORS.iter().enumerate() {
            let (fill, rim) = (&self.offers.road_fill[c], &self.offers.road_rim[c]);
            Self::draw_offer(layer, fill, rim, color, bx, by);
        }
        layer.silhouette(stand_fp, bx + LAYER_X, by + LAYER_Y, FOOTPRINT);
        for p in &self.pieces.standing {
            if game.is_built(p.kind, p.index) {
                let o = Self::origin(game, p);
                let alpha = self.cart_alpha(p, now);
                layer.sprite(Self::sprite(tinted, p), bx + o.x, by + o.y, alpha);
            }
        }
        for p in &self.pieces.standing {
            if game.offer(p.kind, p.index).is_some() {
                let o = Self::origin(game, p);
                layer.sprite(
                    Self::sprite(tinted, p),
                    bx + o.x,
                    by + o.y,
                    OFFER_SPRITE_ALPHA,
                );
            }
        }
        for (c, &color) in OFFER_COLORS.iter().enumerate() {
            let (fill, rim) = (&self.offers.stand_fill[c], &self.offers.stand_rim[c]);
            Self::draw_offer(layer, fill, rim, color, bx, by);
        }
        sailing
    }

    /// Text on the island's image, drawn centred on where the board puts its ink (`ink`, in
    /// board pixels).
    #[allow(clippy::too_many_arguments)]
    fn board_text(
        island: &mut Image,
        l: &Layout,
        font: &super::font::BitmapFont,
        text: &str,
        ink: Rect,
        argb: u32,
        outline: u32,
    ) {
        let f = l.isle.rect_to_frame(ink);
        let w = font.ink_width(text);
        let x = f.x + (f.w - w) / 2;
        let y = f.y + (f.h - FONT_CAPS) / 2 - FONT_CAP_ROW;
        font.draw_outlined(island, text, x, y, argb, outline);
    }

    /// The top left in the island's image of a 12 px icon whose cell is at this board point.
    fn board_icon_at(l: &Layout, x: i32, y: i32) -> Pt {
        let f = l.isle.rect_to_frame(Rect::new(x, y, 12, 12));
        Pt::new(f.x, f.y)
    }

    /// Where a trader's cargo rides, in board pixels: the cell on its bed, lowered by the
    /// icon's own clear rows so every cargo stands on the bed.
    fn cargo_cell(&self, anchor: Pt, v: DiceValue) -> Pt {
        Pt::new(
            anchor.x - rart::TRADER_AX + rart::TRADER_CARGO_X,
            anchor.y - rart::TRADER_AY + rart::TRADER_CARGO_Y + self.assets.cargo_drop(v),
        )
    }

    /// The labels of empty spots, the long road's bonus, the cargo of unhired traders and the
    /// shared resources between their carts: everything on the island that must read upright.
    fn draw_island_marks(&self, frame: &mut Image, game: &Game, l: &Layout) {
        let a = &self.assets;
        let font = &a.font_body;
        let map = game.map;
        for p in &self.pieces.standing {
            if game.is_built(p.kind, p.index) {
                continue;
            }
            match p.kind {
                Kind::Knight => {
                    let slot = map.trader_bank[p.index - 1];
                    if slot > 0 && self.pieces.shared_at[slot].is_none() {
                        let v = map.bank_resource[slot];
                        let cell = self.cargo_cell(p.anchor, v);
                        let at = Self::board_icon_at(l, cell.x, cell.y);
                        frame.sprite(a.icon12(v), at.x, at.y, 255);
                    }
                    if map.ordered_traders {
                        let tier = p.index.to_string();
                        let w = font.ink_width(&tier);
                        let ink = Rect::new(
                            p.anchor.x - w / 2,
                            p.anchor.y + TRADER_NUMBER_DROP + FONT_CAP_ROW,
                            w,
                            FONT_CAPS,
                        );
                        Self::board_text(frame, l, font, &tier, ink, QUIET, INK);
                    }
                }
                _ => {
                    let village = p.kind == Kind::Village;
                    let points = if village {
                        map.village_points[p.index]
                    } else {
                        map.city_points[p.index]
                    }
                    .to_string();
                    let rise = if village {
                        rart::VILLAGE_LABEL_RISE
                    } else {
                        rart::CASTLE_LABEL_RISE
                    };
                    let w = font.ink_width(&points);
                    let ink = Rect::new(p.anchor.x - w / 2, p.anchor.y - rise - 3, w, FONT_CAPS);
                    Self::board_text(frame, l, font, &points, ink, QUIET, INK);
                }
            }
        }
        // The long road's bonus beside it: left of a slanting road, above a level one
        if let Some(road) = map.long_road {
            let bonus = format!("+{}", map.long_road_bonus);
            let middle = game.board.road_anchor[road];
            let w = font.ink_width(&bonus);
            let level = game.board.road_shape[road] == RoadShape::Horizontal;
            let x = if level {
                middle.x - w / 2
            } else {
                middle.x - 2 * crate::rules::board::ROAD_CAP - 1 - w
            };
            let y = if level {
                middle.y - 8 - 2 * crate::rules::board::ROAD_CAP
            } else {
                middle.y - 7
            };
            let color = if game.playsheet.roads[road] {
                AMBER
            } else {
                QUIET
            };
            Self::board_text(
                frame,
                l,
                font,
                &bonus,
                Rect::new(x, y + FONT_CAP_ROW, w, FONT_CAPS),
                color,
                INK,
            );
        }
        for (slot, at) in self.pieces.resting_shared(game) {
            let f = Self::board_icon_at(l, at.x, at.y);
            frame.sprite(a.icon12(map.bank_resource[slot]), f.x, f.y, 255);
        }
    }

    // ------------------------------------------------------------ the HUD

    fn draw_hud(&self, frame: &mut Image, game: &Game, l: &Layout, ui: &Ui, s: &Strings, now: u64) {
        self.draw_score(frame, game, l, s);
        self.draw_costs(frame, l);
        let (font, cap_row) = if l.give.label_display {
            (&self.assets.font_display, DISPLAY_CAP_ROW)
        } else {
            (&self.assets.font_body, FONT_CAP_ROW)
        };
        let labels = [
            (l.costs_label, &s.costs),
            (l.turns_label, &s.turns),
            (l.bank_label, &s.bank),
        ];
        for (at, text) in labels {
            if let Some(at) = at {
                font.draw(frame, text, at.x, at.y - cap_row, MUTED);
            }
        }
        let down = ui.is_pressed(Button::Menu, now);
        draw_menu(frame, l.menu, down);
        self.draw_voyage(frame, game, l);
        if game.is_race() {
            self.draw_bonuses(frame, game, l, s);
        }
    }

    fn draw_score(&self, frame: &mut Image, game: &Game, l: &Layout, s: &Strings) {
        let a = &self.assets;
        let t = l.score_tray;
        tray(frame, t.x, t.y, t.w, t.h);
        let r = l.score;
        centred(
            &a.font_body,
            frame,
            &s.score,
            r.x,
            r.w,
            r.y + 3 - FONT_CAP_ROW,
            MUTED,
        );
        // The number alone, centred in the tray below its label: a turn's gain is on the turn strip
        let below = 3 + 7 + 1;
        centred(
            &a.font_display,
            frame,
            &game.playsheet.score().to_string(),
            r.x,
            r.w,
            r.y + below + (r.h - 1 - below - 8) / 2,
            IVORY,
        );
    }

    fn draw_costs(&self, frame: &mut Image, l: &Layout) {
        let t = self.tinted.as_ref().expect("tinted");
        let a = &self.assets;
        let r = l.costs;
        tray(frame, r.x, r.y, r.w, r.h);
        for (kind, cost) in RECIPES {
            let at = recipe_rect(l, kind);
            if l.key_large {
                let glyph = match kind {
                    Kind::Road => &t.glyph16_road,
                    Kind::City => &t.glyph16_castle,
                    Kind::Village => &t.glyph16_village,
                    Kind::Knight => &a.glyph16_trader,
                };
                self.draw_recipe16(frame, at, glyph, cost);
                continue;
            }
            let glyph = match kind {
                Kind::Road => &t.glyph_road,
                Kind::City => &t.glyph_castle,
                Kind::Village => &t.glyph_village,
                Kind::Knight => &a.glyph_trader,
            };
            self.draw_recipe(frame, at, &l.spread, glyph, cost);
        }
    }

    /// A recipe of the landscape key: the 16 px glyph on its tile, then a dark strip with a
    /// 16 px icon in each cell.
    fn draw_recipe16(&self, frame: &mut Image, at: Rect, glyph: &Image, cost: &[DiceValue]) {
        let h = layout::KEY_ROW_H;
        frame.plate(at.x, at.y, at.w, h, WOOD);
        frame.plate(at.x, at.y, h, h, LEGEND_TILE);
        frame.sprite(glyph, at.x + 1, at.y + 1, 255);
        let strip_x = at.x + h + 1;
        let strip_w = 1 + cost.len() as i32 * layout::KEY_CELL;
        frame.fill_argb(strip_x, at.y + 1, strip_w, h - 2, INK);
        for (i, &v) in cost.iter().enumerate() {
            let x = strip_x + 1 + layout::KEY_CELL * i as i32;
            if let Some(icon) = self.assets.icon16(v) {
                frame.sprite(icon, x, at.y + 1, 255);
            }
        }
    }

    /// A recipe of the portrait key: the glyph on its tile, then a dark strip with an icon in
    /// each cell, the strip showing between them as the top bar's spread allows.
    fn draw_recipe(
        &self,
        frame: &mut Image,
        at: Rect,
        sp: &layout::Spread,
        glyph: &Image,
        cost: &[DiceValue],
    ) {
        let (x, y) = (at.x, at.y);
        frame.plate(x, y, at.w, RECIPE_H, WOOD);
        frame.plate(x, y, RECIPE_H, RECIPE_H, LEGEND_TILE);
        frame.sprite(glyph, x + 1, y + 1, 255);
        let strip_x = x + RECIPE_H + 1;
        let n = cost.len() as i32;
        frame.fill_argb(strip_x, y + 1, sp.icon_strip_w(n), RECIPE_H - 2, INK);
        for (i, &v) in cost.iter().enumerate() {
            frame.sprite(
                self.assets.icon12(v),
                strip_x + sp.icon_gap() + sp.icon_pitch * i as i32,
                // The brick's ink sits low in its cell (it is drawn to stand on a cart bed)
                y + if v == DiceValue::Brick { 0 } else { 1 },
                255,
            );
        }
    }

    fn badge_width(&self, bonus: i32) -> i32 {
        BADGE_GLYPH + BADGE_GAP_IN + self.assets.font_body.ink_width(&format!("+{bonus}"))
    }

    /// A race's two bonuses: on a wooden sign hung by two ropes from the bar, with their label on
    /// the wood and the badges in a dark strip sunk into it, or in a small tray in the columns.
    /// Each is dim until earned and amber after.
    fn draw_bonuses(&self, frame: &mut Image, game: &Game, l: &Layout, s: &Strings) {
        let map = game.map;
        let a = &self.assets;
        let trader = map.trader_bonus_at > 0;
        let road = map.long_road.is_some();
        let inner = if trader {
            self.badge_width(map.trader_bonus)
        } else {
            0
        } + if road {
            self.badge_width(map.long_road_bonus)
        } else {
            0
        } + if trader && road { BADGE_GAP } else { 0 };
        let (mut at, glyph_y) = match l.badges {
            BadgePlace::Sign { right, top } => {
                let label_w = a.font_body.ink_width(&s.bonuses);
                let strip_w = inner + 4;
                let w = 1 + SIGN_PAD + label_w + SIGN_PAD + strip_w + 2 + 1;
                let x = right - w;
                frame.fill_argb(x + 4, top - layout::SIGN_DROP, 1, layout::SIGN_DROP, MUTED);
                frame.fill_argb(
                    x + w - 5,
                    top - layout::SIGN_DROP,
                    1,
                    layout::SIGN_DROP,
                    MUTED,
                );
                frame.plate(x, top, w, SIGN_H, INK);
                frame.plate(x + 1, top + 1, w - 2, SIGN_H - 2, WOOD);
                frame.fill_argb(x + 2, top + 1, w - 4, 1, WOOD_LIGHT);
                a.font_body.draw(
                    frame,
                    &s.bonuses,
                    x + 1 + SIGN_PAD,
                    top + 3 - FONT_CAP_ROW,
                    MUTED,
                );
                let strip_x = x + w - 3 - strip_w;
                frame.fill_argb(strip_x, top + 2, strip_w, SIGN_H - 4, INK);
                (strip_x + 2, top + 3)
            }
            BadgePlace::Panel { right, top } => {
                // One above the other, each against the tray's right
                let mut y = top;
                if let Some(r) = map.long_road {
                    let glyph = &self.tinted.as_ref().expect("tinted").badge_road;
                    let (bonus, earned) = (map.long_road_bonus, game.playsheet.roads[r]);
                    self.draw_badge(
                        frame,
                        right - self.badge_width(bonus),
                        y,
                        glyph,
                        bonus,
                        earned,
                    );
                    y += layout::BADGE_ROW;
                }
                if trader {
                    let earned = game.playsheet.trader_count() >= map.trader_bonus_at;
                    let bonus = map.trader_bonus;
                    let x = right - self.badge_width(bonus);
                    self.draw_badge(frame, x, y, &a.badge_trader, bonus, earned);
                }
                return;
            }
        };
        if let Some(r) = map.long_road {
            let glyph = &self.tinted.as_ref().expect("tinted").badge_road;
            let earned = game.playsheet.roads[r];
            self.draw_badge(frame, at, glyph_y, glyph, map.long_road_bonus, earned);
            at += self.badge_width(map.long_road_bonus) + BADGE_GAP;
        }
        if trader {
            let earned = game.playsheet.trader_count() >= map.trader_bonus_at;
            self.draw_badge(
                frame,
                at,
                glyph_y,
                &a.badge_trader,
                map.trader_bonus,
                earned,
            );
        }
    }

    /// One bonus badge: its 7 px glyph with its top at `y`, and what the bonus is worth.
    #[allow(clippy::too_many_arguments)]
    fn draw_badge(
        &self,
        frame: &mut Image,
        x: i32,
        y: i32,
        glyph: &Image,
        bonus: i32,
        earned: bool,
    ) {
        let font = &self.assets.font_body;
        let alpha = if earned { 255 } else { BADGE_DIM_ALPHA };
        frame.sprite(glyph, x, y, alpha);
        let color = if earned { AMBER } else { WOOD_LIGHT };
        let text = format!("+{bonus}");
        font.draw(
            frame,
            &text,
            x + BADGE_GLYPH + BADGE_GAP_IN,
            y - FONT_CAP_ROW,
            color,
        );
    }

    /// The King's voyage: a band of sea per track row, with the flags of finished turns, the
    /// buoys of turns to come, the ship and the pier (GameView.drawVoyage).
    fn draw_voyage(&self, frame: &mut Image, game: &Game, l: &Layout) {
        let a = &self.assets;
        let map = game.map;
        let done = game.is_game_done();
        let track = &l.track;
        let (first, flags, cell) = if map.turn_cap == 0 {
            (
                voyage::window_start(game.turns_taken, done),
                voyage::window_flags(game.turns_taken, done),
                voyage::window_ship_cell(game.turns_taken, done),
            )
        } else {
            let reached = game.target_score > 0 && game.playsheet.score() >= game.target_score;
            (
                0,
                if track.flags { game.turns_taken } else { 0 },
                voyage::cell(map.turn_cap, game.turns_taken, done, reached),
            )
        };
        let tinted = self.tinted.as_ref().expect("tinted");
        let buoy = &tinted.buoy;
        for row in &track.rows {
            let b = row.band;
            let y = row.water_y();
            frame.frame(b.x, b.y, b.w, b.h, INK);
            frame.tile(&a.track_water, b.x + 1, y, b.w - 2);
            let right = b.x + b.w - 1;
            let bottom = y + rart::TRACK_H;
            for (cx, cy) in [(b.x, b.y), (right, b.y), (b.x, bottom), (right, bottom)] {
                frame.fill_argb(cx, cy, 1, 1, WOOD_DARK);
            }
            for (cx, cy) in [
                (b.x + 1, y),
                (right - 1, y),
                (b.x + 1, bottom - 1),
                (right - 1, bottom - 1),
            ] {
                frame.fill_argb(cx, cy, 1, 1, INK);
            }
        }
        for pair in track.rows.windows(2) {
            self.draw_bend(frame, &pair[0], &pair[1]);
        }
        for row in &track.rows {
            let y = row.water_y();
            for j in 0..row.cells {
                let i = row.first + j;
                if i < flags {
                    let x = row.place(row.strip.cell_x(j), rart::TRACK_FLAG_W);
                    let score = game.playsheet.turn_score_of(first + i + 1);
                    self.draw_flag(frame, x, y, score);
                } else if i > cell && (track.flags || i % 2 == 1) {
                    let bx = row.place(row.strip.buoy_x(j), rart::TRACK_BUOY_W);
                    frame.sprite(buoy, bx, y + BUOY_DROP, 255);
                }
            }
        }
        let (r, j) = track.ship_row(cell);
        let row = &track.rows[r];
        let ship = if row.reverse {
            &tinted.track_ship_back
        } else {
            &tinted.track_ship
        };
        frame.sprite(
            ship,
            row.place(row.strip.ship_x(j), rart::TRACK_SHIP_W),
            row.water_y(),
            255,
        );
        let last = &track.rows[track.rows.len() - 1];
        frame.sprite(
            // Over the ship lying at it, so its tower is whole
            &tinted.track_pier,
            last.place(last.strip.pier_x(), rart::TRACK_PIER_W),
            last.water_y(),
            255,
        );
    }

    /// Where the track turns from one row down into the next: at the end the upper row sails
    /// towards, the water runs on down through both bands' frames and the gap between them,
    /// a channel as wide as the water is tall, walled in ink on either side.
    fn draw_bend(&self, frame: &mut Image, upper: &TrackRow, lower: &TrackRow) {
        let water = &self.assets.track_water;
        let (tw, th) = (water.width as i32, water.height as i32);
        let b = upper.band;
        let w = rart::TRACK_H;
        let (outer, inner) = if upper.reverse {
            (b.x, b.x + w + 1)
        } else {
            let outer = b.x + b.w - 1;
            (outer, outer - w - 1)
        };
        let first = inner.min(outer) + 1;
        // From the upper band's last water row to the lower band's first
        let y0 = upper.water_y() + rart::TRACK_H - 1;
        let y1 = lower.water_y();
        for y in y0..=y1 {
            // The water's pattern carried on down from the upper band's, from its inner rows
            // only: its first and last are the band's darker edges, which in the channel
            // would read as a line across the turn
            let sy = 1 + (y - upper.water_y() - 1).rem_euclid(th - 2);
            for x in first..first + w {
                let sx = (x - (b.x + 1)).rem_euclid(tw);
                frame.sprite_part(water, sx, sy, 1, 1, x, y);
            }
        }
        // The channel's walls: the outer one straight down, the inner one through the gap
        frame.fill_argb(outer, y0, 1, y1 - y0 + 1, INK);
        let gap_top = b.y + b.h - 1;
        frame.fill_argb(inner, gap_top, 1, lower.band.y - gap_top + 1, INK);
    }

    fn draw_flag(&self, frame: &mut Image, x: i32, y: i32, turn_score: i32) {
        let a = &self.assets;
        if turn_score <= 0 {
            frame.sprite(&a.track_flag_empty, x, y, 255);
            return;
        }
        frame.sprite(&a.track_flag, x, y, 255);
        let mut text = turn_score.to_string();
        if a.font_numerals.ink_width(&text) > rart::TRACK_FLAG_FIELD_W - 2 {
            text = "+".into();
        }
        let ink = a.font_numerals.ink_width(&text);
        let top =
            y + rart::TRACK_FLAG_FIELD_Y + (rart::TRACK_FLAG_FIELD_H - NUMERAL_H) / 2 - NUMERAL_ROW;
        let left = x + rart::TRACK_FLAG_FIELD_X + (rart::TRACK_FLAG_FIELD_W - ink + 1) / 2;
        a.font_numerals.draw(frame, &text, left, top, INK);
    }

    // ------------------------------------------------------------ dice and buttons

    /// The roll button as drawn: widened for a long label, centred where the layout put it.
    pub fn roll_bounds(&self, game: &Game, l: &Layout, s: &Strings) -> Rect {
        let text = s
            .viewing
            .as_deref()
            .unwrap_or_else(|| s.roll(game.roll_label()));
        let w = l
            .roll
            .w
            .max(self.assets.font_display.ink_width(text) + 2 * ROLL_CORNER + 4);
        Rect::new(l.roll.x - (w - l.roll.w) / 2, l.roll.y, w, l.roll.h)
    }

    /// The dice, the roll button, the mini dice and the undo button. Returns whether the roll
    /// button is still locked after the game ended (it re-enables visibly).
    fn draw_controls(
        &self,
        frame: &mut Image,
        game: &Game,
        l: &Layout,
        ui: &Ui,
        s: &Strings,
        now: u64,
    ) -> bool {
        let a = &self.assets;
        let t = self.tinted.as_ref().expect("tinted");
        for (i, r) in l.dice.iter().enumerate() {
            let die = &game.dice[i];
            let ring = self.rings.iter().find(|(e, _)| e.die == i);
            let value = match ring {
                Some((e, start)) if spend_anim::ring_shows_old_face((now - start) as i32) => e.was,
                _ => die.value,
            };
            let icon = if die.is_usable() {
                a.icon16(value)
            } else {
                None
            };
            let (x, y) = (r.x, r.y);
            match icon {
                Some(icon) => {
                    let face = if game.die_stays(i) {
                        &t.die_face_held
                    } else {
                        &a.die_face
                    };
                    frame.sprite(face, x, y, 255);
                    frame.sprite(icon, x + (DIE - 16) / 2, y + (DIE - 16) / 2, 255);
                }
                None => frame.sprite(&a.die_slot_empty, x, y, 255),
            }
            if let Some((_, start)) = ring {
                let elapsed = (now - start) as i32;
                let (w, h) = (r.w + 2, r.h + 2);
                let n = (spend_anim::ring_drawn(elapsed) * spend_anim::ring_len(w, h) as f32)
                    .round() as usize;
                let alpha = spend_anim::ring_alpha(elapsed).clamp(0, 255) as u32;
                let argb = (view_model::HIGHLIGHT_KNIGHT & 0xFFFFFF) | alpha << 24;
                for (px, py) in spend_anim::ring_pixels(w, h, n) {
                    frame.fill_argb(r.x - 1 + px, r.y - 1 + py, 1, 1, argb);
                }
            }
        }
        let locked = game.is_roll_locked() && s.viewing.is_none();
        let down = ui.is_pressed(Button::Roll, now) && !locked;
        let text = s
            .viewing
            .as_deref()
            .unwrap_or_else(|| s.roll(game.roll_label()));
        let b = self.roll_bounds(game, l, s);
        let skin = if locked {
            &a.button_disabled
        } else if down {
            &a.button_pressed
        } else {
            &a.button_normal
        };
        frame.nine_slice(skin, b.x, b.y, b.w, b.h, ROLL_CORNER);
        let font = &a.font_display;
        let tw = font.ink_width(text);
        font.draw(
            frame,
            text,
            b.x + (b.w - tw) / 2,
            b.y + (b.h - font.height()) / 2 + down as i32,
            if locked {
                ROLL_LABEL_DISABLED
            } else {
                ROLL_LABEL
            },
        );

        self.draw_mini_dice(frame, game, l, ui);
        if game.can_undo() {
            let u = l.undo;
            if ui.is_pressed(Button::Undo, now) {
                frame.plate(u.x, u.y, UNDO_SIZE, UNDO_SIZE, WOOD);
            }
            frame.sprite(&a.undo_glyph, u.x, u.y, 255);
        } else if l.landscape.is_some() {
            // Its place beside the roll button kept, faint until there is a build to take back
            frame.sprite(&a.undo_glyph, l.undo.x, l.undo.y, UNDO_IDLE_ALPHA);
        }
        locked
    }

    /// The banked resources nobody has used yet, each a mini die, in bank slot order on a tab
    /// of the bar's wood (portrait) or a tray in the panel (landscape).
    fn draw_mini_dice(&self, frame: &mut Image, game: &Game, l: &Layout, ui: &Ui) {
        if let Some(b) = l.bank {
            tray(frame, b.x, b.y, b.w, b.h);
        }
        let slots = game.banked_slots();
        if slots.is_empty() {
            return;
        }
        let count = slots.len() as i32;
        let per = l.mini_per_row;
        let pitch = MINI_DIE + MINI_GAP;
        let rows = (count + per - 1) / per;
        for row in 0..rows {
            let in_row = per.min(count - row * per);
            let w = in_row * pitch - MINI_GAP + 2 * MINI_PAD;
            let top = l.mini_y - row * pitch - MINI_PAD;
            let left = l.mini_x - MINI_PAD - 1;
            match l.mini_tab {
                MiniTab::Bar { bar_top } => {
                    let h = bar_top - top + 2;
                    frame.fill_argb(left + 1, top - 1, w, 1, INK);
                    frame.fill_argb(left, top, w + 2, h - 1, INK);
                    frame.fill_argb(left + 2, top, w - 2, 1, WOOD_DARK);
                    frame.fill_argb(left + 1, top + 1, w, h - 2, WOOD_DARK);
                }
                // The tray is drawn whole, above
                MiniTab::Tray => {}
            }
        }
        for (n, &slot) in slots.iter().enumerate() {
            if self.flying(slot) || ui.carry.is_some_and(|c| c.slot == slot) {
                continue;
            }
            self.mini_die_face(
                frame,
                l.slot_x(n as i32, count),
                l.slot_y(n as i32),
                game.bank_resource(slot),
            );
        }
    }

    /// A mini die, its top left at (x, y).
    fn mini_die_face(&self, f: &mut Image, x: i32, y: i32, v: DiceValue) {
        f.plate(x, y, MINI_DIE, MINI_DIE, INK);
        f.plate(x + 1, y + 1, MINI_DIE - 2, MINI_DIE - 2, LEGEND_TILE);
        f.sprite(self.assets.icon12(v), x + 2, y + 2 + mini_icon_drop(v), 255);
    }

    // ------------------------------------------------------------ hiring

    fn flying(&self, slot: usize) -> bool {
        self.hires
            .iter()
            .any(|h| h.slot == slot && h.flies == Some(true))
    }

    fn flying_count(&self) -> i32 {
        self.hires.iter().filter(|h| h.flies == Some(true)).count() as i32
    }

    fn place_of(game: &Game, slot: usize) -> i32 {
        (1..slot)
            .filter(|&s| game.playsheet.can_use_knight_resource(s))
            .count() as i32
    }

    /// Settles each hire before a frame: whether its resource flies, and drops those that are
    /// over or stale (the game replaced, the trader gone, the resource spent or moved).
    fn update_hires(&mut self, game: &Game, _l: &Layout, now: u64) {
        let sheet = &game.playsheet;
        self.hires.retain_mut(|h| {
            let mut keep = h.epoch == game.state_epoch && sheet.is_trader_built(h.tier);
            let available = sheet.can_use_knight_resource(h.slot);
            if keep && h.flies.is_none() {
                h.flies = Some(h.completes && available);
            }
            if keep && h.flies == Some(true) {
                let place = Self::place_of(game, h.slot);
                keep = available && h.place.is_none_or(|p| p == place);
                h.place = Some(place);
            }
            let elapsed = now as i64 - h.start as i64;
            if keep && elapsed >= hire_anim::total(h.flies == Some(true)) as i64 {
                keep = false;
            }
            keep
        });
    }

    /// The opacity a standing cart is drawn at: full, or whatever its hire's fade reached.
    fn cart_alpha(&self, p: &Piece, now: u64) -> u8 {
        if p.kind == Kind::Knight
            && let Some(h) = self.hires.iter().find(|h| h.tier == p.index)
        {
            return hire_anim::cart_alpha((now as i64 - h.start as i64) as i32).clamp(0, 255) as u8;
        }
        255
    }

    /// Where a hired trader's resource starts from, in the island's image: on its cart's bed,
    /// or for a shared slot between the carts.
    fn hire_from(&self, game: &Game, l: &Layout, h: &Hire) -> Pt {
        let v = game.map.bank_resource[h.slot];
        if let Some(at) = self.pieces.shared_at[h.slot] {
            return Self::board_icon_at(l, at.x, at.y);
        }
        let anchor = trader_anchor(&self.pieces, h.tier);
        let cell = self.cargo_cell(anchor, v);
        Self::board_icon_at(l, cell.x, cell.y)
    }

    fn glow_tint(game: &Game) -> u32 {
        board_layers::lighten(game.player_color | 0xFF000000, hire_anim::GLOW_LIGHTEN) & 0xFFFFFF
    }

    /// What of each hire happens on its cart, drawn on the island at the island's scale: the
    /// resource fading as the cart fades in, or lifting off before its mini die's plate
    /// appears behind it. From the plate on it is the interface's (`draw_hires`).
    fn draw_hires_on_island(&self, island: &mut Image, game: &Game, l: &Layout, now: u64) {
        let a = &self.assets;
        for h in &self.hires {
            let elapsed = (now as i64 - h.start as i64) as i32;
            let v = game.map.bank_resource[h.slot];
            let icon = a.icon12(v);
            let from = self.hire_from(game, l, h);
            let shared = self.pieces.shared_at[h.slot].is_some();
            if h.flies != Some(true) {
                let alpha = if shared && !h.completes {
                    0
                } else {
                    hire_anim::icon_alpha(elapsed, false)
                };
                if alpha > 0 {
                    island.sprite(icon, from.x, from.y, alpha.clamp(0, 255) as u8);
                }
                continue;
            }
            if hire_anim::plate_shown(elapsed) {
                continue;
            }
            let at = hire_anim::icon_at(from, from, elapsed);
            let strength = hire_anim::glow(elapsed);
            if strength > 0.0 {
                let tint = Self::glow_tint(game);
                draw_hire_glow(island, Some(icon), at.x, at.y, tint, strength);
            }
            island.sprite(icon, at.x, at.y, 255);
        }
    }

    /// The flights of banked resources, on their mini die's plate from the cart to the tab,
    /// drawn in the interface at its scale.
    fn draw_hires(&self, frame: &mut Image, game: &Game, l: &Layout, now: u64) {
        for h in &self.hires {
            let elapsed = (now as i64 - h.start as i64) as i32;
            if h.flies != Some(true) || !hire_anim::plate_shown(elapsed) {
                continue;
            }
            let v = game.map.bank_resource[h.slot];
            let on_cart = self.hire_from(game, l, h);
            let from = l.island_to_ui(on_cart.x, on_cart.y);
            let place = h.place.unwrap_or(0);
            let drop = mini_icon_drop(v);
            let count = game.banked_slots().len() as i32;
            let to = Pt::new(l.slot_x(place, count) + 2, l.slot_y(place) + 2 + drop);
            let at = hire_anim::icon_at(from, to, elapsed);
            let strength = hire_anim::glow(elapsed);
            if strength > 0.0 {
                let tint = Self::glow_tint(game);
                draw_hire_glow(frame, None, at.x - 2, at.y - 2 - drop, tint, strength);
            }
            self.mini_die_face(frame, at.x - 2, at.y - 2 - drop, v);
        }
    }

    /// Each banked resource a build spent, its mini die flying from where it sat on the tab
    /// to the middle of the piece it paid for and fading there, in the bank's blue glow.
    fn draw_spends(&self, frame: &mut Image, game: &Game, l: &Layout, now: u64) {
        for (e, start) in &self.spends {
            let elapsed = (now as i64 - *start as i64) as i32;
            let alpha = spend_anim::alpha(elapsed);
            if alpha <= 0 {
                continue;
            }
            let Some(p) = self
                .pieces
                .all()
                .find(|p| p.kind == e.kind && p.index == e.index)
            else {
                continue;
            };
            let target = self.piece_rect(game, l, p);
            // Where it sat among those banked then: the ones still there and those this build
            // spent with it
            let with = self.spends.iter().filter(|(_, s)| s == start).count();
            let count = (game.banked_slots().len() + with) as i32;
            let from = Pt::new(l.slot_x(e.place, count), l.slot_y(e.place));
            let to = Pt::new(
                target.x + (target.w - MINI_DIE) / 2,
                target.y + (target.h - MINI_DIE) / 2,
            );
            let at = spend_anim::die_at(from, to, elapsed);
            let strength = spend_anim::glow(elapsed);
            if strength > 0.0 {
                let tint = view_model::HIGHLIGHT_KNIGHT & 0xFFFFFF;
                draw_hire_glow(frame, None, at.x, at.y, tint, strength);
            }
            let mut tile = Image::new(MINI_DIE as u32, MINI_DIE as u32);
            self.mini_die_face(&mut tile, 0, 0, e.value);
            frame.sprite(&tile, at.x, at.y, alpha.clamp(0, 255) as u8);
        }
    }

    /// Where a piece is drawn, in the interface's pixels.
    fn piece_rect(&self, game: &Game, l: &Layout, p: &Piece) -> Rect {
        let o = Self::origin(game, p);
        let (w, h) = match p.kind {
            Kind::Road => {
                let sprite = &self.assets.road[game.board.road_shape[p.index] as usize];
                (sprite.width as i32, sprite.height as i32)
            }
            Kind::Village => (layout::VILLAGE_W, layout::VILLAGE_H),
            Kind::City => (layout::CASTLE_W, layout::CASTLE_H),
            Kind::Knight => (layout::TRADER_W, layout::TRADER_H),
        };
        l.board_rect_to_ui(Rect::new(o.x, o.y, w, h))
    }

    // ------------------------------------------------------------ the tutorial

    /// The rectangle a step's callout describes, or None when it is not on screen.
    fn tutorial_target(&self, step: Step, game: &Game, l: &Layout, s: &Strings) -> Option<Rect> {
        match step {
            Step::Roll | Step::End => Some(self.roll_bounds(game, l, s)),
            Step::Keep => {
                let first = l.dice[0];
                let last = l.dice[5];
                Some(Rect::new(
                    first.x,
                    first.y,
                    last.x + last.w - first.x,
                    last.y + last.h - first.y,
                ))
            }
            Step::Costs => Some(recipe_rect(l, Kind::Road)),
            Step::CostVillage => Some(recipe_rect(l, Kind::Village)),
            Step::CostCastle => Some(recipe_rect(l, Kind::City)),
            Step::CostTrader => Some(recipe_rect(l, Kind::Knight)),
            Step::Saved => Some(l.menu),
            Step::Build => {
                let p = self.pieces.offered_spot(game)?;
                Some(self.piece_rect(game, l, &p))
            }
            Step::Undo => Some(l.undo),
            Step::Ship => Some(l.track.rect()),
            Step::Gold => game
                .dice
                .iter()
                .position(|d| d.is_usable() && d.value == DiceValue::Gold)
                .map(|i| l.dice[i]),
            Step::Bank | Step::BankClassic => {
                let count = game.banked_slots().len() as i32 - self.flying_count();
                if count <= 0 {
                    return None;
                }
                // Every mini die there, wherever the tab or tray stands them
                let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                for n in 0..count {
                    let (x, y) = (l.slot_x(n, count), l.slot_y(n));
                    (x0, y0) = (x0.min(x), y0.min(y));
                    (x1, y1) = (x1.max(x + MINI_DIE), y1.max(y + MINI_DIE));
                }
                Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
            }
        }
    }

    /// Asks the tutorial which step is due and draws its callout. Returns whether the
    /// tutorial's saved progress changed.
    #[allow(clippy::too_many_arguments)]
    fn draw_tutorial(
        &self,
        frame: &mut Image,
        game: &Game,
        l: &Layout,
        ui: &mut Ui,
        tutorial: &mut Tutorial,
        s: &Strings,
        now: u64,
    ) -> bool {
        ui.callout = None;
        if game.tutorial_paused {
            tutorial.forget();
            return false;
        }
        let step = tutorial.update(&game.tutorial_snapshot(self.flying_count()));
        let dirty = tutorial.take_dirty();
        let Some(step) = step else { return dirty };
        if ui.tutorial_held {
            return dirty;
        }
        let Some(target) = self.tutorial_target(step, game, l, s) else {
            return dirty;
        };
        let font = &self.assets.font_body;
        let measure = |t: &str| font.ink_width(t);
        let canvas = tutorial_layout::Canvas {
            w: l.fit.width as i32,
            top_limit: l.pad_top,
            bottom_limit: l.fit.height as i32 - l.pad_bottom,
        };
        let Some(p) = tutorial_layout::place(
            target,
            s.tutorial(step),
            &s.got_it,
            &s.skip,
            canvas,
            &measure,
        ) else {
            return dirty;
        };
        let o = tutorial_layout::OUTLINE;
        frame.bevel_frame(
            target.x - o,
            target.y - o,
            target.w + 2 * o,
            target.h + 2 * o,
            game.player_color | 0xFF000000,
        );
        let a = &self.assets;
        let panel = p.panel;
        frame.nine_slice(
            &a.panel_dark,
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            CALLOUT_CORNER,
        );
        self.draw_pointer(frame, &p);
        let mut line_top = p.text_y - FONT_TOP_ROW;
        for line in &p.lines {
            font.draw(frame, line, p.text_x, line_top, IVORY);
            line_top += tutorial_layout::LINE_PITCH;
        }
        let g = p.got_it;
        let got_down = ui.is_pressed(Button::GotIt, now);
        frame.plate(g.x, g.y + CALLOUT_PLATE_TOP, g.w, CALLOUT_PLATE_H, INK);
        frame.plate(
            g.x + 1,
            g.y + CALLOUT_PLATE_TOP + 1,
            g.w - 2,
            CALLOUT_PLATE_H - 2,
            if got_down { WOOD_DARK } else { WOOD },
        );
        let label_top =
            |y: i32| y + CALLOUT_PLATE_TOP + (CALLOUT_PLATE_H - FONT_CAPS) / 2 - FONT_CAP_ROW;
        font.draw(
            frame,
            &s.got_it,
            p.got_it_label_x,
            label_top(g.y) + got_down as i32,
            IVORY,
        );
        let k = p.skip;
        let skip_color = if ui.is_pressed(Button::Skip, now) {
            IVORY
        } else {
            QUIET
        };
        let skip_top = label_top(k.y);
        font.draw(frame, &s.skip, p.skip_label_x, skip_top, skip_color);
        frame.fill_argb(
            p.skip_label_x,
            skip_top + FONT_CAP_ROW + FONT_CAPS + 2,
            font.ink_width(&s.skip),
            1,
            skip_color,
        );
        ui.callout = Some(p);
        dirty
    }

    /// The callout's pointer: a triangle on the panel's edge facing the target, in the
    /// panel's own outline, gold line and fill.
    fn draw_pointer(&self, frame: &mut Image, p: &Placement) {
        let a = &self.assets;
        let half = tutorial_layout::HALF_BASE;
        for u in -4..tutorial_layout::POINTER {
            let hw = if u < 0 { half - 1 } else { half - u };
            for v in -hw..=hw {
                let color = if u < 0 {
                    if v.abs() == hw && u >= -3 {
                        a.panel_gold
                    } else {
                        a.panel_fill
                    }
                } else if v.abs() == hw {
                    a.panel_ink
                } else if v.abs() == hw - 1 {
                    a.panel_gold
                } else {
                    a.panel_fill
                };
                let (x, y) = match p.side {
                    Side::Above => (p.base_x + v, p.base_y + u),
                    Side::Below => (p.base_x + v, p.base_y - u),
                    Side::Left => (p.base_x + u, p.base_y + v),
                    Side::Right => (p.base_x - u, p.base_y + v),
                };
                frame.fill_argb(x, y, 1, 1, color);
            }
        }
    }

    // ------------------------------------------------------------ dragging a mini die

    /// The die a carried mini die would land on at (x, y): one showing, not spent, not locked.
    pub fn drop_die_at(
        &self,
        game: &Game,
        l: &Layout,
        x: i32,
        y: i32,
        slot: usize,
    ) -> Option<usize> {
        (0..l.dice.len()).find(|&i| l.dice[i].contains(x, y) && game.can_overwrite_die(i, slot))
    }

    fn draw_carry(&self, frame: &mut Image, game: &Game, l: &Layout, ui: &mut Ui) {
        let Some(c) = ui.carry else { return };
        if !game.playsheet.can_use_knight_resource(c.slot) {
            // The game moved under the drag (an undo, a restart): nothing to carry
            ui.carry = None;
            return;
        }
        if let Some(i) = self.drop_die_at(game, l, c.x, c.y, c.slot) {
            let d = l.dice[i];
            frame.frame(
                d.x - 1,
                d.y - 1,
                d.w + 2,
                d.h + 2,
                game.player_color | 0xFF000000,
            );
        }
        self.mini_die_face(
            frame,
            c.x - MINI_DIE / 2,
            c.y - MINI_DIE / 2 - DRAG_RAISE,
            game.bank_resource(c.slot),
        );
    }

    // ------------------------------------------------------------ taps

    /// The bank slot of the mini die drawn at (x, y), with a pixel of slack round each.
    pub fn mini_slot_at(&self, game: &Game, l: &Layout, x: i32, y: i32) -> Option<usize> {
        let slots = game.banked_slots();
        let count = slots.len() as i32;
        slots.into_iter().enumerate().find_map(|(n, slot)| {
            let (left, top) = (l.slot_x(n as i32, count), l.slot_y(n as i32));
            (x >= left - 1 && x < left + MINI_DIE + 1 && y >= top - 1 && y < top + MINI_DIE + 1)
                .then_some(slot)
        })
    }

    /// Whether a mini die may be picked up now (classic mode, after a roll, not over, landed).
    pub fn may_carry(&self, game: &Game, slot: usize) -> bool {
        game.trader_classic() && game.rolls >= 1 && !game.is_game_done() && !self.flying(slot)
    }

    /// What a tap at a spot means, in GameView.touchAt's order: the callout's buttons, the
    /// menu, the undo button, a mini die, the dice and the roll button, then the island. All
    /// but the island are the interface's, tested at its pixel; the island is tested at its
    /// own pixel, which on a larger island is a finer grid than the interface's.
    pub fn hit(&self, game: &Game, l: &Layout, ui: &Ui, spot: Spot) -> Hit {
        let (x, y) = (spot.x, spot.y);
        if let Some(p) = &ui.callout {
            if p.got_it.contains(x, y) {
                return Hit::GotIt;
            }
            if p.skip.contains(x, y) {
                return Hit::Skip;
            }
        }
        if l.menu_hit.contains(x, y) {
            return Hit::Menu;
        }
        let u = l.undo;
        if game.can_undo() && Rect::new(u.x - 2, u.y - 2, u.w + 4, u.h + 4).contains(x, y) {
            return Hit::Undo;
        }
        // Only one that can be picked up takes the touch from what is under it
        if let Some(slot) = self
            .mini_slot_at(game, l, x, y)
            .filter(|&slot| self.may_carry(game, slot))
        {
            return Hit::MiniDie(slot);
        }
        if let Some(i) = l.dice.iter().position(|d| d.contains(x, y)) {
            return Hit::Die(i);
        }
        if l.roll.contains(x, y) {
            return Hit::Roll;
        }
        if l.island.contains(x, y) {
            let (bx, by) = l.spot_to_board(spot);
            if let Some(p) = self.pieces.piece_at(game, bx, by) {
                return Hit::Piece(p);
            }
        }
        Hit::Nothing
    }
}

fn trader_anchor(pieces: &BoardPieces, tier: usize) -> Pt {
    pieces
        .standing
        .iter()
        .find(|p| p.kind == Kind::Knight && p.index == tier)
        .map_or(Pt::new(0, 0), |p| p.anchor)
}

/// The glow of a resource being banked, round what it hugs: the icon's solid pixels, or the
/// mini die's plate when there is no icon.
fn draw_hire_glow(
    frame: &mut Image,
    icon: Option<&Image>,
    x: i32,
    y: i32,
    rgb: u32,
    strength: f32,
) {
    let side = MINI_DIE + 2 * HIRE_GLOW_REACH;
    let mut mask = vec![false; (side * side) as usize];
    let size = icon.map_or(MINI_DIE, |i| i.width as i32);
    for py in 0..size {
        for px in 0..size {
            let solid = match icon {
                None => hire_anim::plate_has(px, py, MINI_DIE),
                Some(i) => i.argb_at(px, py) >> 24 >= 128,
            };
            mask[((py + HIRE_GLOW_REACH) * side + px + HIRE_GLOW_REACH) as usize] = solid;
        }
    }
    let out = board_layers::outline(
        &mask,
        side as usize,
        side as usize,
        rgb,
        board_layers::halo_rgb(rgb, view_model::GLOW_HALO_LIGHT_ABOVE),
        view_model::GLOW_ALPHA,
        view_model::GLOW_HALO_ALPHA,
        strength,
    );
    let glow = Image::from_argb(side as u32, side as u32, &out);
    frame.sprite(&glow, x - HIRE_GLOW_REACH, y - HIRE_GLOW_REACH, 255);
}

/// The bars' dark wood, with a line of lighter wood and a line of ink along the side that
/// meets the island.
fn draw_bar(frame: &mut Image, r: Rect, edge: Edge) {
    if edge != Edge::Around {
        frame.fill_argb(r.x, r.y, r.w, r.h, WOOD_DARK);
    }
    match edge {
        Edge::Around => {
            frame.plate(r.x, r.y, r.w, r.h, INK);
            frame.plate(r.x + 1, r.y + 1, r.w - 2, r.h - 2, WOOD);
            frame.fill_argb(r.x + 2, r.y + 2, r.w - 4, r.h - 4, WOOD_DARK);
        }
        Edge::Bottom => {
            frame.fill_argb(r.x, r.y + r.h - 2, r.w, 1, WOOD);
            frame.fill_argb(r.x, r.y + r.h - 1, r.w, 1, INK);
        }
        Edge::Top => {
            frame.fill_argb(r.x, r.y, r.w, 1, INK);
            frame.fill_argb(r.x, r.y + 1, r.w, 1, WOOD);
        }
        Edge::Right => {
            frame.fill_argb(r.x + r.w - 2, r.y, 1, r.h, WOOD);
            frame.fill_argb(r.x + r.w - 1, r.y, 1, r.h, INK);
        }
        Edge::Left => {
            frame.fill_argb(r.x, r.y, 1, r.h, INK);
            frame.fill_argb(r.x + 1, r.y, 1, r.h, WOOD);
        }
    }
}

/// An inset tray: an ink outline with its corners cut, and a face darker than the bar.
fn tray(frame: &mut Image, x: i32, y: i32, w: i32, h: i32) {
    frame.plate(x, y, w, h, INK);
    frame.plate(x + 1, y + 1, w - 2, h - 2, TRAY_FACE);
}

/// Text centred across a box of width w from x, its ink (not its trailing space) centred.
fn centred(
    font: &super::font::BitmapFont,
    frame: &mut Image,
    text: &str,
    x: i32,
    w: i32,
    top: i32,
    argb: u32,
) {
    font.draw(
        frame,
        text,
        x + (w - font.ink_width(text) + 1) / 2,
        top,
        argb,
    );
}

/// The menu button: an outline, a face with a lighter top and left edge and a darker bottom
/// and right edge, and three cream bars. Pressed, the face darkens and the bars drop a pixel.
fn draw_menu(frame: &mut Image, r: Rect, down: bool) {
    let (x, y, w, h) = (r.x, r.y, r.w, r.h);
    frame.plate(x, y, w, h, INK);
    frame.plate(
        x + 1,
        y + 1,
        w - 2,
        h - 2,
        if down { MENU_FACE_DOWN } else { MENU_FACE },
    );
    frame.fill_argb(x + 2, y + 1, w - 4, 1, MENU_LIGHT);
    frame.fill_argb(x + 1, y + 2, 1, h - 5, MENU_LIGHT);
    frame.fill_argb(x + 1, y + h - 3, w - 2, 2, MENU_DARK);
    frame.fill_argb(x + w - 2, y + 2, 1, h - 5, MENU_DARK);
    let bars = 2 * MENU_BAR_PITCH + MENU_BAR_H;
    let bar_y = y + (h - bars) / 2 + down as i32;
    // The bars keep to about half the face as a wider button spreads them
    let bar_w = MENU_BAR_W + (w - layout::MENU_W).max(0) / 2;
    for i in 0..3 {
        frame.fill_argb(
            x + (w - bar_w) / 2,
            bar_y + i * MENU_BAR_PITCH,
            bar_w,
            MENU_BAR_H,
            IVORY,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_landscape_key_fits_its_recipes() {
        let widest = [
            game::RESOURCES_ROAD.len(),
            game::RESOURCES_CITY.len(),
            LEGEND_VILLAGE.len(),
            game::RESOURCES_KNIGHT.len(),
        ]
        .into_iter()
        .map(|n| layout::key_recipe_w(n as i32))
        .max()
        .unwrap_or(0);
        assert_eq!(layout::PANEL_W, widest + 2 * layout::KEY_INSET);
    }
}
