// SPDX-License-Identifier: GPL-3.0-or-later
//! The game screen: the island, the HUD and the dice, drawn as pixel art (see `scene`) laid
//! out for the canvas's shape (see `layout`). The island and the interface each have their
//! own integer scale, so a big screen gives the island the room the interface leaves; the
//! two are put together into one image of one image pixel per device pixel, so nothing
//! blurs.
//!
//! The shell owns the game, the settings, sound and haptics; it hands them in through a
//! `ScreenHost`, and this module calls back when the game changed, the menu was asked for or
//! a haptic is due.

pub mod assets;
pub mod font;
pub mod layout;
pub mod scene;

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use day::prelude::*;

use crate::res;
use crate::rules::build_plan::Kind;
use crate::rules::dice::DiceValue;
use crate::rules::game::Game;
use crate::rules::map_def::MapDef;
use crate::rules::sfx::Sfx;
use crate::rules::sound_sink;
use crate::rules::tutorial::{Step, Tutorial};
use crate::rules::view_model::{BuildPrompt, RollPress};
use crate::ui;

use layout::{Insets, Layout, Spot};
use scene::{Button, Carry, Hit, Scene, Strings, Ui};

/// Which side of a landscape screen the dice go on; the HUD takes the other. The island stays
/// upright between them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DiceSide {
    Left,
    #[default]
    Right,
}

/// A haptic the screen asks for; the shell maps it to the platform's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Haptic {
    /// A die held or let go, a button pressed, a mini die picked up.
    Tap,
    /// Something was built (or a banked resource spent on a die).
    Build,
    /// Dice were thrown.
    Roll,
}

pub struct ScreenHost {
    pub game: Rc<RefCell<Game>>,
    /// Bumped by the shell after it replaces or changes the game from outside (load, new
    /// game, island switch, settings).
    pub refresh: Signal<u64>,
    pub dice_side: Signal<DiceSide>,
    /// The menu button was tapped; the shell opens its menu.
    pub on_menu: Rc<dyn Fn()>,
    /// The game changed (the shell saves it). Called after every action that moves
    /// game.revision(), and when the tutorial's progress changed (see `tutorial_progress`).
    pub on_changed: Rc<dyn Fn()>,
    /// Haptic feedback for a tap, a build, a roll; the shell maps it to day-part-haptics.
    pub haptic: Rc<dyn Fn(Haptic)>,
}

// ---------------------------------------------------------------- the tutorial's progress

thread_local! {
    /// The in-context tutorial, shared by both islands. The shell configures it from its
    /// settings and saves what `tutorial_progress` says after on_changed.
    static TUTORIAL: RefCell<Tutorial> = RefCell::new(Tutorial::new(true, 0));
    /// Set by the shell while something of its own is over the game (the welcome, a menu),
    /// so no callout shows under it.
    static HELD: Cell<bool> = const { Cell::new(false) };
}

/// Takes the tutorial's settings over (Settings, a backup import, the first launch). Bump
/// `ScreenHost::refresh` afterwards to redraw.
pub fn configure_tutorial(active: bool, seen: u32) {
    TUTORIAL.with(|t| t.borrow_mut().configure(active, seen));
}

/// What to save of the tutorial: whether it is on, and the steps seen (Tutorial::seen_mask).
pub fn tutorial_progress() -> (bool, u32) {
    TUTORIAL.with(|t| {
        let t = t.borrow();
        (t.is_active(), t.seen_mask())
    })
}

/// Holds the tutorial's callouts back while the shell shows something over the game. Bump
/// `ScreenHost::refresh` after letting go, so a callout that is due appears.
pub fn hold_tutorial(held: bool) {
    HELD.with(|h| h.set(held));
}

/// An island's name in the current language.
pub fn island_name(map: &MapDef) -> String {
    match map.id.as_str() {
        "dunholm" => res::str::map_dunholm(),
        _ => res::str::map_tarnholm(),
    }
    .format()
}

// ---------------------------------------------------------------- the screen

fn strings(game: &Game) -> Strings {
    use res::str as t;
    let ship = if game.is_race() {
        t::tutorial_ship_race(game.target_score as i64).format()
    } else {
        t::tutorial_ship_score().format()
    };
    Strings {
        score: t::score().format(),
        bonuses: t::strip_bonuses().format(),
        first_roll: t::first_roll().format(),
        second_roll: t::second_roll().format(),
        third_roll: t::third_roll().format(),
        play_again: t::play_again().format(),
        finish: t::finish().format(),
        done: t::done().format(),
        next_turn: t::next_turn().format(),
        got_it: t::tutorial_got_it().format(),
        skip: t::tutorial_skip().format(),
        tutorial: vec![
            (Step::Roll, t::tutorial_roll().format()),
            (Step::Keep, t::tutorial_keep().format()),
            (Step::Costs, t::tutorial_costs().format()),
            (Step::Build, t::tutorial_build().format()),
            (Step::Undo, t::tutorial_undo().format()),
            (Step::End, t::tutorial_end().format()),
            (Step::Ship, ship),
            (Step::Gold, t::tutorial_gold().format()),
            (Step::Bank, t::tutorial_bank().format()),
            (Step::BankClassic, t::tutorial_bank_classic().format()),
        ],
    }
}

/// Device pixels per point, from the primary display (the canvas is drawn for it).
fn device_scale() -> f64 {
    let monitors = day::monitors();
    monitors
        .iter()
        .find(|m| m.primary)
        .or(monitors.first())
        .map(|m| m.scale)
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0)
}

/// How long a tapped button shows pressed.
const PRESS_MS: u64 = 110;
/// How long after a dialog closes the board takes the keyboard back: past the dialog's own
/// button, which shows pressed for a moment before it acts.
const REFOCUS_MS: u32 = 300;

#[derive(Clone)]
struct Shown {
    bitmap: day::Bitmap,
    rect: Rect,
    sea: u32,
}

impl PartialEq for Shown {
    fn eq(&self, other: &Self) -> bool {
        self.bitmap == other.bitmap && self.rect == other.rect && self.sea == other.sea
    }
}

struct Screen {
    host: ScreenHost,
    scene: RefCell<Option<Scene>>,
    ui: RefCell<Ui>,
    layout: RefCell<Option<Layout>>,
    shown: Signal<Option<Shown>>,
    /// The canvas size in points.
    size: Cell<(f64, f64)>,
    dpr: Cell<f64>,
    /// What the finger now down went down on, on a platform that says so at once.
    touch: Cell<Option<Hit>>,
    /// A frame is being decoded; another render waits for it (`dirty`).
    in_flight: Cell<bool>,
    dirty: Cell<bool>,
    frame: RefCell<Option<day::frame::FrameHandle>>,
    frame_pending: Cell<bool>,
    /// Questions open: taps wait, and so does the tutorial.
    asking: Cell<u32>,
    revision: Cell<u64>,
    /// The board has the keyboard. Nothing else on this screen can take it, so it comes back
    /// here whenever whatever was over the game closes (`refocus`).
    focused: Signal<bool>,
}

/// The game screen for the shell to place: it fills what it is given.
pub fn game_screen(host: ScreenHost) -> impl Piece {
    let revision = host.game.borrow().revision();
    let screen = Rc::new(Screen {
        host,
        scene: RefCell::new(None),
        ui: RefCell::new(Ui::new()),
        layout: RefCell::new(None),
        shown: Signal::new(None),
        size: Cell::new((0.0, 0.0)),
        dpr: Cell::new(1.0),
        touch: Cell::new(None),
        in_flight: Cell::new(false),
        dirty: Cell::new(false),
        frame: RefCell::new(None),
        frame_pending: Cell::new(false),
        asking: Cell::new(0),
        revision: Cell::new(revision),
        focused: Signal::new(false),
    });

    // Anything the shell changes from outside, the dice side and the system bars redraw
    {
        let weak = Rc::downgrade(&screen);
        let (refresh, side) = (screen.host.refresh, screen.host.dice_side);
        Effect::new(move || {
            refresh.get();
            side.get();
            let _ = day::safe_area();
            if let Some(s) = weak.upgrade() {
                s.revision.set(s.host.game.borrow().revision());
                s.request();
                s.refocus();
            }
        });
    }

    let shown = screen.shown;
    let focused = screen.focused;
    // The canvas's own closures hold the screen, so it lives exactly as long as the canvas
    let draw = screen.clone();
    let tap = screen.clone();
    let keys = screen.clone();
    let drag = screen;
    canvas(move |d, size| {
        if draw.size.get() != (size.width, size.height) {
            draw.size.set((size.width, size.height));
            draw.request();
        }
        let current = shown.get();
        let sea = current.as_ref().map_or(0xFF0D3947, |c| c.sea);
        let color = Color::rgb(
            ((sea >> 16) & 0xFF) as f64 / 255.0,
            ((sea >> 8) & 0xFF) as f64 / 255.0,
            (sea & 0xFF) as f64 / 255.0,
        );
        d.fill(
            Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
            color,
        );
        if let Some(c) = current {
            d.image(&c.bitmap, c.rect);
        }
    })
    .on_tap_at(move |p| tap.tap(p))
    .on_key(move |k| keys.key(k))
    .focusable()
    .focused(focused)
    .on_drag(move |g| drag.drag(g))
    // The label goes on the canvas itself: after `.grow()` it would land on a layout wrapper
    // that has no element of its own, and screen readers would never hear it.
    .a11y(|a| {
        a.label(res::str::board_label())
            .hint(res::str::board_hint())
    })
    .id(day::id!(board))
    .grow()
}

impl Screen {
    /// Gives the board the keyboard back, unless something is still over the game. A moment
    /// later, as a dialog's default button does: the tap that closed the dialog is still
    /// finishing, and its last events would otherwise take the focus to whatever is under them.
    fn refocus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        day::task(async move {
            day::sleep(REFOCUS_MS).await;
            if let Some(s) = weak.upgrade()
                && s.asking.get() == 0
                && !HELD.with(Cell::get)
            {
                s.focused.set(true);
            }
        });
    }

    /// A render at the next frame, at most one waiting.
    fn request(self: &Rc<Self>) {
        // Before the canvas has a size there is nothing to draw, and a frame asked for while
        // the page is still being built may never come; the canvas asks once it is laid out
        let (w, h) = self.size.get();
        if w <= 0.0 || h <= 0.0 || self.frame_pending.replace(true) {
            return;
        }
        let weak: Weak<Screen> = Rc::downgrade(self);
        let handle = day::frame::request(move |_| {
            if let Some(s) = weak.upgrade() {
                s.frame_pending.set(false);
                s.render();
            }
        });
        *self.frame.borrow_mut() = Some(handle);
    }

    fn render(self: &Rc<Self>) {
        if self.in_flight.get() {
            self.dirty.set(true);
            return;
        }
        let (w, h) = self.size.get();
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let dpr = device_scale();
        self.dpr.set(dpr);
        let (dw, dh) = ((w * dpr).round() as u32, (h * dpr).round() as u32);
        let safe = day::safe_area();
        let px = |v: f64| (v * dpr).round().max(0.0) as u32;
        let insets = Insets {
            top: px(safe.top),
            bottom: px(safe.bottom),
            left: px(safe.leading),
            right: px(safe.trailing),
        };
        let now = crate::clock::now_ms();
        let (frame, layout, sea) = {
            let mut game = self.host.game.borrow_mut();
            if game.clock.is_none() {
                // The roll button's lockout after the game ends reads it
                game.clock = Some(Box::new(crate::clock::now_ms));
            }
            let side = self.host.dice_side.get_untracked();
            let layout = Layout::new(dw, dh, insets, game.map, side);
            let s = strings(&game);
            let mut scene = self.scene.borrow_mut();
            let scene = scene.get_or_insert_with(|| Scene::new(game.map));
            let mut ui = self.ui.borrow_mut();
            ui.tutorial_held = self.asking.get() > 0 || HELD.with(Cell::get);
            let frame = TUTORIAL
                .with(|t| scene.render(&mut game, &layout, &mut ui, &mut t.borrow_mut(), &s, now));
            // The rows left over below a portrait layout's bottom bar are of a piece with it
            let backdrop = if layout.landscape.is_none() {
                scene::WOOD_DARK
            } else {
                game.map.sea_fill
            };
            (frame, layout, backdrop)
        };
        if frame.tutorial_dirty {
            (self.host.on_changed)();
        }
        let (ox, oy, rw, rh) = layout.device_rect();
        let rect = Rect::new(
            ox as f64 / dpr,
            oy as f64 / dpr,
            rw as f64 / dpr,
            rh as f64 / dpr,
        );
        let png = frame.to_png(&layout);
        *self.layout.borrow_mut() = Some(layout);
        self.in_flight.set(true);
        if frame.animating {
            self.dirty.set(true);
        }
        let weak = Rc::downgrade(self);
        let shown = self.shown;
        day::decode_image_async(Arc::new(png), move |r| {
            if let Ok(bitmap) = r {
                shown.set(Some(Shown { bitmap, rect, sea }));
            }
            if let Some(s) = weak.upgrade() {
                s.in_flight.set(false);
                if s.dirty.replace(false) {
                    s.request();
                }
            }
        });
    }

    /// The spot under a point of the canvas.
    fn spot(&self, p: Point) -> Option<Spot> {
        let dpr = self.dpr.get();
        self.layout
            .borrow()
            .as_ref()
            .map(|l| l.to_spot(p.x * dpr, p.y * dpr))
    }

    fn haptic(&self, h: Haptic) {
        (self.host.haptic)(h);
    }

    /// The finger went down at `spot`, on a platform that says so at once. As on the Java app, a
    /// die holds now, and a button shows pressed until the lift with its click said now; what
    /// the button does waits for the lift (`tap`).
    fn touch_down(self: &Rc<Self>, spot: Spot) {
        let hit = self.hit(spot);
        let button = match hit {
            Hit::GotIt => Some(Button::GotIt),
            Hit::Skip => Some(Button::Skip),
            Hit::Menu => Some(Button::Menu),
            Hit::Undo => Some(Button::Undo),
            Hit::Roll if self.host.game.borrow().roll_press() != RollPress::Locked => {
                Some(Button::Roll)
            }
            _ => None,
        };
        self.touch.set(Some(hit));
        if let Some(b) = button {
            self.ui.borrow_mut().pressed = Some((b, u64::MAX));
            if b != Button::Undo {
                sound_sink::play(Sfx::Click);
                self.haptic(Haptic::Tap);
            }
            self.request();
        }
        if let Hit::Die(i) = hit {
            if self.host.game.borrow_mut().dice_touched(i) {
                self.haptic(Haptic::Tap);
            }
            self.changed();
        }
    }

    /// The finger moved or lifted: a roll button it slid off lets go at once, and any other
    /// button shows pressed until the lift.
    fn touch_moved(self: &Rc<Self>, spot: Spot, lifted: bool) {
        let mut ui = self.ui.borrow_mut();
        let Some((b, _)) = ui.pressed else {
            return;
        };
        let slid_off = b == Button::Roll && self.hit_with(&ui, spot) != Hit::Roll;
        if lifted || slid_off {
            ui.pressed = None;
            drop(ui);
            self.request();
        }
    }

    /// Shows a button pressed for a moment: not when a finger has already held it down.
    fn press(self: &Rc<Self>, b: Button) {
        if self.touch.get().is_some() {
            return;
        }
        self.ui.borrow_mut().pressed = Some((b, crate::clock::now_ms() + PRESS_MS));
        sound_sink::play(Sfx::Click);
        self.haptic(Haptic::Tap);
    }

    /// After anything that may have changed the game: save if it did, and redraw.
    fn changed(self: &Rc<Self>) {
        let rev = self.host.game.borrow().revision();
        if rev != self.revision.replace(rev) {
            (self.host.on_changed)();
        }
        self.request();
    }

    fn hit(&self, spot: Spot) -> Hit {
        self.hit_with(&self.ui.borrow(), spot)
    }

    fn hit_with(&self, ui: &Ui, spot: Spot) -> Hit {
        let game = self.host.game.borrow();
        let scene = self.scene.borrow();
        let layout = self.layout.borrow();
        match (scene.as_ref(), layout.as_ref()) {
            (Some(scene), Some(l)) => scene.hit(&game, l, ui, spot),
            _ => Hit::Nothing,
        }
    }

    fn tap(self: &Rc<Self>, p: Point) {
        if self.asking.get() > 0 {
            return;
        }
        let down = self.touch.take();
        let Some(spot) = self.spot(p) else {
            return;
        };
        let hit = self.hit(spot);
        if let Some(down) = down {
            // A die has held already, and a button only answers the finger that pressed it
            let button = matches!(
                down,
                Hit::GotIt | Hit::Skip | Hit::Menu | Hit::Undo | Hit::Roll
            );
            if matches!(down, Hit::Die(_)) || (button && hit != down) {
                return;
            }
        }
        // The press was shown from the finger going down, or is shown now
        let held = down.is_some();
        self.touch.set(down);
        self.act(hit);
        if held {
            self.touch.set(None);
        }
    }

    fn act(self: &Rc<Self>, hit: Hit) {
        match hit {
            Hit::GotIt | Hit::Skip => {
                let got_it = hit == Hit::GotIt;
                self.press(if got_it { Button::GotIt } else { Button::Skip });
                TUTORIAL.with(|t| {
                    let mut t = t.borrow_mut();
                    if got_it { t.got_it() } else { t.skip() }
                });
                (self.host.on_changed)();
                self.request();
            }
            Hit::Menu => {
                self.press(Button::Menu);
                self.request();
                (self.host.on_menu)();
            }
            Hit::Undo => {
                self.press(Button::Undo);
                self.host.game.borrow_mut().undo();
                self.changed();
            }
            Hit::Die(i) => {
                if self.host.game.borrow_mut().dice_touched(i) {
                    self.haptic(Haptic::Tap);
                }
                self.changed();
            }
            Hit::Roll => self.roll(),
            Hit::Piece(p) => self.build(p.kind, p.index),
            Hit::MiniDie(_) | Hit::Nothing => {}
        }
    }

    /// Keyboard play, for desktop and web players and for switch and keyboard access. Day
    /// delivers the digits and arrows on every platform but letters only on some, so every
    /// action has a digit: 1 to 6 keep or release a die, 0 rolls, 7 undoes, 8 dismisses the
    /// tutorial's callout, 9 builds the suggested spot (the one the tutorial would point at).
    /// Where letters arrive, R or Space rolls, U undoes, B builds, Enter dismisses the callout
    /// and M or Escape opens the menu. The walkthroughs drive the game this way too, the same
    /// on every screen size.
    fn key(self: &Rc<Self>, k: &day::KeyEvent) {
        self.touch.set(None);
        if self.asking.get() > 0 || k.modifiers & (day::KeyEvent::PRIMARY | day::KeyEvent::ALT) != 0
        {
            return;
        }
        match k.key.as_str() {
            d @ ("1" | "2" | "3" | "4" | "5" | "6") => {
                let i = d.parse::<usize>().unwrap_or(1) - 1;
                if self.host.game.borrow_mut().dice_touched(i) {
                    self.haptic(Haptic::Tap);
                }
                self.changed();
            }
            "0" | "r" | "R" | " " => self.roll(),
            "7" | "u" | "U" => {
                self.press(Button::Undo);
                self.host.game.borrow_mut().undo();
                self.changed();
            }
            "8" | "Enter" => {
                TUTORIAL.with(|t| t.borrow_mut().got_it());
                (self.host.on_changed)();
                self.request();
            }
            "9" | "b" | "B" => {
                let spot = {
                    let game = self.host.game.borrow();
                    self.scene
                        .borrow()
                        .as_ref()
                        .and_then(|scene| scene.pieces.offered_spot(&game))
                };
                if let Some(p) = spot
                    && self.host.game.borrow().can_build_at(p.kind, p.index)
                {
                    self.build(p.kind, p.index);
                }
            }
            "m" | "M" | "Escape" => {
                self.press(Button::Menu);
                self.request();
                (self.host.on_menu)();
            }
            _ => {}
        }
    }

    fn roll(self: &Rc<Self>) {
        let press = self.host.game.borrow().roll_press();
        match press {
            RollPress::Locked => {}
            RollPress::ConfirmEndTurn => {
                self.press(Button::Roll);
                let rev = self.host.game.borrow().revision();
                let s = self.clone();
                self.ask(async move {
                    let yes = ui::confirm(
                        res::str::end_turn_title().format(),
                        res::str::end_turn_text().format(),
                        res::str::yes().format(),
                        res::str::no().format(),
                    )
                    .await;
                    if yes && s.host.game.borrow().revision() == rev {
                        s.host.game.borrow_mut().roll();
                    }
                    s.changed();
                });
            }
            _ => {
                self.press(Button::Roll);
                let outcome = self.host.game.borrow_mut().press_roll_button();
                if outcome.rolled {
                    self.haptic(Haptic::Roll);
                }
                self.changed();
            }
        }
        self.request();
    }

    /// A tap on a spot on offer: the questions it needs first, one after the other, and the
    /// build if every answer is yes.
    fn build(self: &Rc<Self>, kind: Kind, index: usize) {
        let (prompts, rev) = {
            let game = self.host.game.borrow();
            (game.build_prompts(kind, index), game.revision())
        };
        let s = self.clone();
        let run = move |s: &Rc<Screen>| {
            let built = {
                let mut game = s.host.game.borrow_mut();
                game.build(kind, index);
                game.is_built(kind, index)
            };
            if built {
                s.haptic(Haptic::Build);
            }
            s.changed();
        };
        if prompts.is_empty() {
            run(&s);
            return;
        }
        self.ask(async move {
            for prompt in prompts {
                let (title, text) = match prompt {
                    BuildPrompt::EarlyBuild { rolls_made } => (
                        res::str::build_early_title(),
                        if rolls_made == 1 {
                            res::str::build_early_text_two()
                        } else {
                            res::str::build_early_text_one()
                        },
                    ),
                    BuildPrompt::SecondPair => (
                        res::str::build_second_pair_title(),
                        res::str::build_second_pair_text(),
                    ),
                };
                let yes = ui::confirm(
                    title.format(),
                    text.format(),
                    res::str::yes().format(),
                    res::str::no().format(),
                )
                .await;
                if !yes {
                    s.request();
                    return;
                }
            }
            // The game may have moved under the questions (the shell can replace it)
            if s.host.game.borrow().revision() == rev {
                run(&s);
            } else {
                s.request();
            }
        });
    }

    /// Runs a question: taps wait for it and the tutorial's callout steps aside.
    fn ask(self: &Rc<Self>, question: impl std::future::Future<Output = ()> + 'static) {
        self.asking.set(self.asking.get() + 1);
        self.request();
        let s = self.clone();
        day::task(async move {
            question.await;
            s.asking.set(s.asking.get().saturating_sub(1));
            s.request();
            s.refocus();
        });
    }

    /// Trader resource classic mode: a banked mini die picked up, carried and dropped on a
    /// die, which then shows its resource (a "?" first asks which).
    fn drag(self: &Rc<Self>, g: Drag) {
        // A carried die is the interface's, so it moves on the interface's pixels
        let Some(Spot { x, y, .. }) = self.spot(g.location) else {
            return;
        };
        match g.phase {
            DragPhase::Began => {
                self.touch.set(None);
                if self.asking.get() > 0 || self.ui.borrow().carry.is_some() {
                    return;
                }
                let press = Point::new(
                    g.location.x - g.translation.x,
                    g.location.y - g.translation.y,
                );
                let Some(Spot { x: px, y: py, .. }) = self.spot(press) else {
                    return;
                };
                let slot = {
                    let game = self.host.game.borrow();
                    let scene = self.scene.borrow();
                    let layout = self.layout.borrow();
                    match (scene.as_ref(), layout.as_ref()) {
                        (Some(sc), Some(l)) => sc
                            .mini_slot_at(&game, l, px, py)
                            .filter(|&slot| sc.may_carry(&game, slot)),
                        _ => None,
                    }
                };
                if let Some(slot) = slot {
                    self.ui.borrow_mut().carry = Some(Carry { slot, x, y });
                    self.haptic(Haptic::Tap);
                    self.request();
                } else if g.translation.x == 0.0
                    && g.translation.y == 0.0
                    && let Some(spot) = self.spot(g.location)
                {
                    self.touch_down(spot);
                }
            }
            DragPhase::Changed => {
                if let Some(spot) = self.spot(g.location) {
                    self.touch_moved(spot, false);
                }
                let mut ui = self.ui.borrow_mut();
                if let Some(c) = ui.carry.as_mut() {
                    c.x = x;
                    c.y = y;
                    drop(ui);
                    self.request();
                }
            }
            DragPhase::Ended => {
                if let Some(spot) = self.spot(g.location) {
                    self.touch_moved(spot, true);
                }
                let Some(c) = self.ui.borrow_mut().carry.take() else {
                    return;
                };
                self.drop_carry(c.slot, x, y);
            }
        }
    }

    fn drop_carry(self: &Rc<Self>, slot: usize, x: i32, y: i32) {
        let target = {
            let game = self.host.game.borrow();
            let scene = self.scene.borrow();
            let layout = self.layout.borrow();
            match (scene.as_ref(), layout.as_ref()) {
                (Some(sc), Some(l)) => sc.drop_die_at(&game, l, x, y, slot),
                _ => None,
            }
        };
        let Some(die) = target else {
            self.request();
            return;
        };
        let banked = self.host.game.borrow().playsheet.knight_resource(slot);
        if banked != DiceValue::Any {
            if self.host.game.borrow_mut().overwrite_die(die, slot, banked) {
                self.haptic(Haptic::Build);
            }
            self.changed();
            return;
        }
        let s = self.clone();
        self.ask(async move {
            let names = [
                ("wool", res::str::resource_wool(), ui::Icon::Wool),
                ("grain", res::str::resource_grain(), ui::Icon::Grain),
                ("brick", res::str::resource_brick(), ui::Icon::Brick),
                ("ore", res::str::resource_ore(), ui::Icon::Ore),
                ("lumber", res::str::resource_lumber(), ui::Icon::Lumber),
            ];
            let mut dialog = ui::Dialog::new(res::str::trader_choose_title().format()).stacked();
            for (v, (key, name, icon)) in DiceValue::RESOURCES.into_iter().zip(names) {
                dialog = dialog.icon_button(key, name.format(), icon, v);
            }
            // Cancelling leaves the die and the bank as they were
            let choice = ui::ask(dialog.cancel(res::str::settings_cancel().format())).await;
            if let Some(v) = choice {
                let done = {
                    let mut game = s.host.game.borrow_mut();
                    game.can_overwrite_die(die, slot) && game.overwrite_die(die, slot, v)
                };
                if done {
                    s.haptic(Haptic::Build);
                }
            }
            s.changed();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::scene::{Hit, Scene, Strings, Ui};
    use super::*;
    use crate::rules::rng::Rng;

    fn game() -> Game {
        Game::with_rng(MapDef::tarnholm(), Rng::new(7))
    }

    fn render(scene: &mut Scene, game: &mut Game, l: &Layout, ui: &mut Ui) -> scene::Frame {
        let mut t = Tutorial::new(false, 0);
        scene.render(game, l, ui, &mut t, &Strings::default(), 1_000)
    }

    #[test]
    fn renders_every_layout_at_its_size() {
        let mut g = game();
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        for (w, h, side) in [
            (1082, 2342, DiceSide::Right),
            (2342, 1082, DiceSide::Right),
            (2342, 1082, DiceSide::Left),
            (2560, 1600, DiceSide::Right),
            (2560, 1600, DiceSide::Left),
            (1440, 900, DiceSide::Right),
            (1366, 768, DiceSide::Left),
            (1284, 2778, DiceSide::Right),
        ] {
            let l = Layout::new(w, h, Insets::default(), g.map, side);
            let f = render(&mut scene, &mut g, &l, &mut ui);
            let screen = f.composed(&l);
            assert_eq!((screen.width, screen.height), l.screen_size());
            // The sea reaches every corner of the frame: nothing is left transparent
            assert!(screen.to_argb().iter().all(|&p| p >> 24 == 0xFF), "{w}x{h}");
            // The PNG shows the same picture
            assert_eq!(
                crate::art::Image::from_png(&f.to_png(&l)).unwrap(),
                screen,
                "{w}x{h}"
            );
        }
    }

    #[test]
    fn taps_through_the_layout_play_the_game() {
        for (w, h, side) in [
            (1082, 2342, DiceSide::Right),
            (2342, 1082, DiceSide::Right),
            (2342, 1082, DiceSide::Left),
            // The tall columns
            (2560, 1600, DiceSide::Right),
            (2560, 1600, DiceSide::Left),
            (1440, 900, DiceSide::Right),
            (1920, 1080, DiceSide::Left),
            // The island at another scale than the interface
            (1366, 768, DiceSide::Right),
            (1366, 768, DiceSide::Left),
            (1284, 2778, DiceSide::Right),
        ] {
            let mut g = game();
            let mut scene = Scene::new(g.map);
            let mut ui = Ui::new();
            let l = Layout::new(w, h, Insets::default(), g.map, side);
            render(&mut scene, &mut g, &l, &mut ui);
            let centre =
                |r: crate::rules::tutorial_layout::Rect| l.ui_spot(r.x + r.w / 2, r.y + r.h / 2);
            assert_eq!(scene.hit(&g, &l, &ui, centre(l.roll)), Hit::Roll);
            assert_eq!(scene.hit(&g, &l, &ui, centre(l.dice[4])), Hit::Die(4));
            assert_eq!(scene.hit(&g, &l, &ui, centre(l.menu)), Hit::Menu);

            // Roll until something is on offer, then tap it where it is drawn
            let mut found = None;
            for _ in 0..40 {
                g.press_roll_button();
                if let Some(p) = scene
                    .pieces
                    .all()
                    .find(|p| g.offer(p.kind, p.index).map(|o| o.now) == Some(true))
                {
                    found = Some(*p);
                    break;
                }
            }
            let p = found.expect("something was offered");
            let target = if p.kind == Kind::Road {
                p.anchor
            } else {
                crate::rules::pt::Pt::new(p.anchor.x, p.anchor.y - p.body_rise())
            };
            let spot = l.board_spot(target.x, target.y);
            match scene.hit(&g, &l, &ui, spot) {
                Hit::Piece(hit) => {
                    assert_eq!((hit.kind, hit.index), (p.kind, p.index), "{side:?} {w}x{h}")
                }
                other => panic!("{other:?} at {spot:?} for {p:?}"),
            }
        }
    }

    #[test]
    fn taps_go_to_the_region_on_top() {
        // A short laptop window: the interface at 3, the island at 2 under it
        let mut g = game();
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        for side in [DiceSide::Right, DiceSide::Left] {
            let l = Layout::new(1280, 680, Insets::default(), g.map, side);
            assert_eq!((l.fit.scale, l.isle.scale), (3, 2));
            render(&mut scene, &mut g, &l, &mut ui);
            // Every screen pixel of a die is that die, to its very edge, though the island's
            // pixels run on under the column at another pitch
            let d = l.dice[1];
            let k = l.ui_scale();
            for (sx, sy) in [
                (d.x * k, d.y * k),
                ((d.x + d.w) * k - 1, (d.y + d.h) * k - 1),
            ] {
                assert_eq!(scene.hit(&g, &l, &ui, l.screen_spot(sx, sy)), Hit::Die(1));
            }
            // Just inside the island's part of the screen, beside the column, is the island
            let (inner, outer) = match side {
                DiceSide::Right => (l.island.x + l.island.w - 1, l.island.x + l.island.w),
                DiceSide::Left => (l.island.x, l.island.x - 1),
            };
            let row = l.island.y + l.island.h / 2;
            assert!(l.island.contains(inner, row) && !l.island.contains(outer, row));
            // No piece lies beside the columns, but whatever is there the island answers for,
            // through its own pixel: a piece's every island pixel is that piece
            let b = crate::rules::board::Board::for_map(g.map);
            for v in &b.village {
                let spot = l.board_spot(v.x, v.y);
                assert!(l.island.contains(spot.x, spot.y));
                assert_eq!(l.spot_to_board(spot), (v.x, v.y));
                let kk = l.isle.scale as i32;
                let (ix, iy) = (spot.sx.div_euclid(kk), spot.sy.div_euclid(kk));
                for (sx, sy) in [(ix * kk, iy * kk), (ix * kk + kk - 1, iy * kk + kk - 1)] {
                    assert_eq!(l.spot_to_board(l.screen_spot(sx, sy)), (v.x, v.y));
                }
            }
        }
    }

    #[test]
    fn the_tutorial_points_at_a_piece_on_an_island_at_another_scale() {
        let mut g = game();
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        let l = Layout::new(1280, 680, Insets::default(), g.map, DiceSide::Right);
        g.press_roll_button();
        let p = scene.pieces.offered_spot(&g).expect("a spot on offer");
        // The outline's rectangle, in interface pixels, covers the piece's island pixels
        let o = crate::rules::tutorial_layout::Rect::new(p.anchor.x - 2, p.anchor.y - 2, 4, 4);
        let r = l.board_rect_to_ui(o);
        let f = l.isle.rect_to_frame(o);
        let (k, ui_k) = (l.isle.scale as i32, l.ui_scale());
        assert!(r.x * ui_k <= f.x * k && (r.x + r.w) * ui_k >= (f.x + f.w) * k);
        assert!(r.y * ui_k <= f.y * k && (r.y + r.h) * ui_k >= (f.y + f.h) * k);
        // And a frame with the tutorial on renders whole
        let mut t = Tutorial::new(true, 0);
        let frame = scene.render(&mut g, &l, &mut ui, &mut t, &Strings::default(), 5_000);
        assert!(frame.ui.is_some());
        assert_eq!(frame.composed(&l).width, l.screen_size().0);
    }

    #[test]
    fn a_turned_callout_reads_upright_and_takes_its_taps() {
        for side in [DiceSide::Right, DiceSide::Left] {
            let mut g = game();
            let mut scene = Scene::new(g.map);
            let mut ui = Ui::new();
            let l = Layout::new(2342, 1082, Insets::default(), g.map, side);
            let mut t = Tutorial::new(true, 0);
            let s = Strings {
                got_it: "Got it".into(),
                skip: "Skip".into(),
                ..Strings::default()
            };
            let frame = scene.render(&mut g, &l, &mut ui, &mut t, &s, 5_000);
            let p = ui.callout.clone().expect("a callout is due");
            // On the screen's art, inside it, which is wider than the art is
            let (w, h) = (l.fit.width as i32, l.fit.height as i32);
            assert_eq!(w, l.height);
            assert!(
                p.panel.x >= 0 && p.panel.x + p.panel.w <= w,
                "{:?}",
                p.panel
            );
            assert!(
                p.panel.y >= 0 && p.panel.y + p.panel.h <= h,
                "{:?}",
                p.panel
            );
            let k = l.ui_scale();
            let g_it = p.got_it;
            let spot = l.screen_spot((g_it.x + g_it.w / 2) * k, (g_it.y + g_it.h / 2) * k);
            assert_eq!(scene.hit(&g, &l, &ui, spot), Hit::GotIt);
            if let Ok(dir) = std::env::var("SNAPSHOT_DIR") {
                let png = frame.to_png(&l);
                std::fs::write(format!("{dir}/callout-{side:?}.png"), png).unwrap();
            }
        }
    }

    #[test]
    fn a_whole_game_renders_and_the_ship_sails_in() {
        let mut g = game();
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        let mut t = Tutorial::new(true, 0);
        let s = Strings::default();
        for (w, h) in [(1082, 2342), (2342, 1082)] {
            let l = Layout::new(w, h, Insets::default(), g.map, DiceSide::Right);
            let mut now = 10_000;
            let mut sailed = false;
            g.reset();
            while !g.is_game_done() {
                let stage = voyage_stage(&g);
                g.press_roll_button();
                // Build whatever the plan can, so flags, buildings and banked dice all show
                let offered: Vec<_> = scene
                    .pieces
                    .all()
                    .filter(|p| g.offer(p.kind, p.index).is_some_and(|o| o.now))
                    .copied()
                    .collect();
                for p in offered {
                    g.build(p.kind, p.index);
                }
                now += 50;
                let f = scene.render(&mut g, &l, &mut ui, &mut t, &s, now);
                if voyage_stage(&g) != stage && f.animating {
                    sailed = true;
                }
                now += 5_000;
                let settled = scene.render(&mut g, &l, &mut ui, &mut t, &s, now);
                assert!(!settled.animating || g.is_roll_locked());
            }
            assert!(sailed, "the ship never sailed at {w}x{h}");
            assert!(g.playsheet.score() > 0);
        }
    }

    /// What a whole frame costs at a few canvas sizes, the scene drawn and the PNG made, as
    /// `render` does it: `cargo test --release -- --ignored frame_cost_by_size --nocapture`.
    #[test]
    #[ignore]
    fn frame_cost_by_size() {
        let mut g = game();
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        g.press_roll_button();
        for (w, h) in [
            (1082, 2342),
            (2342, 1082),
            (2560, 1600),
            (1600, 2560),
            (1440, 900),
            (1920, 1080),
        ] {
            let l = Layout::new(w, h, Insets::default(), g.map, DiceSide::Right);
            eprintln!("{w}x{h}: interface {} island {}", l.fit.scale, l.isle.scale);
            let n = 60;
            let (mut draw, mut encode) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
            for i in 0..n {
                let t = std::time::Instant::now();
                let mut tut = Tutorial::new(false, 0);
                let f = scene.render(
                    &mut g,
                    &l,
                    &mut ui,
                    &mut tut,
                    &Strings::default(),
                    1_000 + i,
                );
                draw += t.elapsed();
                let t = std::time::Instant::now();
                let png = f.to_png(&l);
                encode += t.elapsed();
                assert!(png.len() > 1000);
            }
            eprintln!(
                "{w}x{h}: draw {:?} encode {:?} frame {:?}",
                draw / n as u32,
                encode / n as u32,
                (draw + encode) / n as u32
            );
        }
    }

    fn voyage_stage(g: &Game) -> crate::rules::voyage::Stage {
        crate::rules::voyage::stage_for(g.map, g.turns_taken, g.is_game_done(), g.playsheet.score())
    }

    /// Renders a game near its end and over, in both orientations on a phone and on a few
    /// larger screens, to PNGs in $SNAPSHOT_DIR, for looking at by eye:
    /// `SNAPSHOT_DIR=/tmp/x cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn snapshot_late_game() {
        let Ok(dir) = std::env::var("SNAPSHOT_DIR") else {
            return;
        };
        for map in MapDef::all() {
            snapshot_map(&dir, map);
        }
    }

    fn snapshot_map(dir: &str, map: &'static MapDef) {
        let mut g = Game::with_rng(map, Rng::new(7));
        let mut scene = Scene::new(g.map);
        let mut ui = Ui::new();
        let mut t = Tutorial::new(false, 0);
        let s = Strings {
            score: "Score".into(),
            play_again: "Play again".into(),
            next_turn: "Next turn".into(),
            finish: "Finish".into(),
            ..Strings::default()
        };
        let mut shots = 0;
        while !g.is_game_done() {
            g.press_roll_button();
            let offered: Vec<_> = scene
                .pieces
                .all()
                .filter(|p| g.offer(p.kind, p.index).is_some_and(|o| o.now))
                .copied()
                .collect();
            for p in offered {
                g.build(p.kind, p.index);
            }
            if g.turns_taken >= 13 && g.turns_taken % 4 == 1 || g.is_game_done() {
                for (w, h, name) in [
                    (1082, 2342, "p"),
                    (2342, 1082, "l"),
                    (2342, 1082, "ll"),
                    (2560, 1600, "tablet"),
                    (1440, 900, "desktop"),
                    (1366, 768, "laptop"),
                    (1284, 2778, "tall"),
                ] {
                    let side = if name == "ll" {
                        DiceSide::Left
                    } else {
                        DiceSide::Right
                    };
                    let l = Layout::new(w, h, Insets::default(), g.map, side);
                    let f = scene.render(&mut g, &l, &mut ui, &mut t, &s, 100_000 + shots * 10_000);
                    let png = f.to_png(&l);
                    std::fs::write(format!("{dir}/{}-{shots}-{name}.png", map.id), png).unwrap();
                }
                shots += 1;
            }
        }
    }
}
