// SPDX-License-Identifier: GPL-3.0-or-later
//! The widgets: each one is its own Day canvas that shows its pixel art at one image pixel per
//! device pixel, and is its own accessible element with a role, a label and, where it has one, a
//! value. Layout stays with Day's containers, so screen readers walk the widgets in order and
//! scrolling is the platform's own.
//!
//! A widget is drawn at art resolution, enlarged `k` times with nearest-neighbor sampling and
//! decoded into a bitmap, exactly as the game screen presents its frames. Each widget keeps the
//! bitmaps of the looks it has shown, so a button that goes back to its normal look shows at
//! once. The cache is per widget: a bitmap belongs to the canvas that decoded it, and Day may
//! release it with that canvas.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use day::prelude::*;

use super::metrics::Metrics;
use super::paint::{self, FOCUS, Icon, Kit, Look, PALETTE, kit};
use super::text::{Align, Block, Face};
use crate::art::Image;
use crate::rules::sfx::Sfx;
use crate::rules::sound_sink;

/// A built widget and its size in art pixels, so the dialogs and pages that hold it can work
/// out their own sizes before Day lays anything out.
pub struct W {
    pub piece: AnyPiece,
    pub w: i32,
    pub h: i32,
}

impl W {
    pub fn new(piece: impl Piece, w: i32, h: i32) -> W {
        W {
            piece: piece.any(),
            w,
            h,
        }
    }

    /// A widget's canvas, with its accessibility and gestures already on it, sized to exactly
    /// `w` x `h` art pixels.
    pub fn framed(m: &Metrics, piece: impl Piece, w: i32, h: i32) -> W {
        W::new(piece.frame(m.pt(w), m.pt(h)), w, h)
    }
}

// ---------------------------------------------------------------- showing art

/// How many looks a widget keeps the bitmaps of before it starts over.
const CACHE_MAX: usize = 8;

fn key_of(img: &Image, k: i32) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (img.width, img.height, k).hash(&mut h);
    img.rgba.hash(&mut h);
    h.finish()
}

/// A canvas `w` x `h` art pixels that shows what `paint` draws for the current `look`, redrawn
/// whenever a signal `look` reads changes. It is not framed yet: Day applies `.a11y`, `.id`, the
/// gestures and focus to the node the previous modifier returned, so they go on the canvas
/// itself and `W::framed` sizes it last.
fn sprite(
    m: &Metrics,
    w: i32,
    h: i32,
    paint: impl Fn(&mut Image, Look) + 'static,
    look: impl Fn() -> Look + 'static,
) -> impl Piece {
    let shown: Signal<Option<day::Bitmap>> = Signal::new(None);
    let generation = Rc::new(Cell::new(0u64));
    {
        // Once the widget is gone, no decode still in flight shows anything
        let generation = generation.clone();
        day::reactive::Scope::current().on_cleanup(move || generation.set(u64::MAX));
    }
    let cache: Rc<RefCell<HashMap<u64, day::Bitmap>>> = Rc::default();
    let k = m.k;
    Effect::new(move || {
        let l = look();
        untrack(|| {
            let mut img = Image::new(w.max(1) as u32, h.max(1) as u32);
            paint(&mut img, l);
            let key = key_of(&img, k);
            let n = generation.get().saturating_add(1).min(u64::MAX - 1);
            generation.set(n);
            if let Some(b) = cache.borrow().get(&key).cloned() {
                shown.set(Some(b));
                return;
            }
            let png = img.to_png_scaled(k as u32);
            let generation = generation.clone();
            let cache = cache.clone();
            day::decode_image_async(Arc::new(png), move |r| {
                let Ok(b) = r else { return };
                {
                    let mut c = cache.borrow_mut();
                    if c.len() >= CACHE_MAX {
                        c.clear();
                    }
                    c.insert(key, b.clone());
                }
                // A later look may have been asked for while this one decoded
                if generation.get() == n {
                    shown.set(Some(b));
                }
            });
        });
    });
    let (wp, hp) = (m.pt(w), m.pt(h));
    canvas(move |d, _| {
        if let Some(b) = shown.get() {
            d.image(&b, Rect::new(0.0, 0.0, wp, hp));
        }
    })
}

// ---------------------------------------------------------------- feedback and escape

/// An Escape handler and the id it was registered under.
type Escape = (u64, Rc<dyn Fn()>);

thread_local! {
    static FEEDBACK: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    static ESCAPES: RefCell<Vec<Escape>> = const { RefCell::new(Vec::new()) };
    static NEXT_ESCAPE: Cell<u64> = const { Cell::new(1) };
}

/// What a press does besides its action: the shell plays its haptic here.
pub fn set_feedback(f: impl Fn() + 'static) {
    FEEDBACK.with(|c| *c.borrow_mut() = Some(Rc::new(f)));
}

fn feedback() {
    sound_sink::play(Sfx::Click);
    if let Some(f) = FEEDBACK.with(|c| c.borrow().clone()) {
        f();
    }
}

/// What Escape does while the current scope is alive: the newest one wins, so a dialog over a
/// page closes before the page does.
pub fn on_escape(f: impl Fn() + 'static) {
    let id = NEXT_ESCAPE.with(|c| c.replace(c.get() + 1));
    ESCAPES.with(|e| e.borrow_mut().push((id, Rc::new(f))));
    day::reactive::Scope::current()
        .on_cleanup(move || ESCAPES.with(|e| e.borrow_mut().retain(|(i, _)| *i != id)));
}

/// Runs the newest Escape handler; true when there was one.
pub fn escape() -> bool {
    let top = ESCAPES.with(|e| e.borrow().last().map(|(_, f)| f.clone()));
    match top {
        Some(f) => {
            f();
            true
        }
        None => false,
    }
}

/// How long a pressed button shows pressed before it acts.
const PRESS_MS: u32 = 90;
/// How long after a dialog opens its default button takes the focus.
const AUTOFOCUS_MS: u32 = 250;

/// The keys a focusable widget answers: Enter and Space activate it (except where the web shim
/// already activates a button for them, which would act twice), and Escape closes what it is in.
/// The web delivers neither to a canvas, only the arrows, the digits, Delete and Backspace, so
/// Backspace closes too, as a browser's Back does.
fn keys(activate: Rc<dyn Fn()>, native_button: bool) -> impl Fn(&day::KeyEvent) + 'static {
    let shim_activates = native_button && cfg!(target_arch = "wasm32");
    move |k: &day::KeyEvent| match k.key.as_str() {
        "Enter" | " " if !shim_activates => activate(),
        "Escape" | "Backspace" => {
            escape();
        }
        _ => {}
    }
}

/// A press that shows for a moment, then acts. Repeated taps while it shows are ignored.
fn press_action(pressed: Signal<bool>, action: Rc<dyn Fn()>) -> Rc<dyn Fn()> {
    Rc::new(move || {
        if pressed.get_untracked() {
            return;
        }
        pressed.set(true);
        feedback();
        let action = action.clone();
        day::task(async move {
            day::sleep(PRESS_MS).await;
            if pressed.try_get().is_some() {
                pressed.set(false);
            }
            action();
        });
    })
}

// ---------------------------------------------------------------- text

/// How a piece of text looks and what it is to a screen reader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub face: Face,
    pub color: u32,
    pub align: Align,
    /// A heading of this level, or plain text.
    pub heading: Option<u8>,
}

impl Style {
    pub const BODY: Style = Style {
        face: Face::Body,
        color: PALETTE.ink,
        align: Align::Start,
        heading: None,
    };
    pub const NOTE: Style = Style {
        color: PALETTE.secondary,
        ..Style::BODY
    };
    pub const HEADING: Style = Style {
        face: Face::Display,
        color: PALETTE.ink,
        align: Align::Start,
        heading: Some(2),
    };
    pub const TITLE: Style = Style {
        face: Face::Display,
        color: PALETTE.ink,
        align: Align::Center,
        heading: Some(1),
    };
    /// A page's title, on the dark wood.
    pub const WOOD_TITLE: Style = Style {
        face: Face::Display,
        color: paint::WOOD_TEXT,
        align: Align::Start,
        heading: Some(1),
    };

    pub fn centered(mut self) -> Style {
        self.align = Align::Center;
        self
    }
}

/// What a line of text starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lead {
    None,
    Bullet,
    Icon(Icon),
}

/// Text wrapped to `width` art pixels. Its accessible label is the whole text as written.
pub fn text(m: &Metrics, id: Option<ElementId>, width: i32, s: &str, style: Style) -> W {
    lead_text(m, id, width, Lead::None, s, style)
}

/// Text after a bullet or an icon, which hang in a column of their own, as a list's do.
pub fn lead_text(
    m: &Metrics,
    id: Option<ElementId>,
    width: i32,
    lead: Lead,
    s: &str,
    style: Style,
) -> W {
    let kit = kit();
    let t = m.t;
    let gutter = match lead {
        Lead::None => 0,
        Lead::Bullet => 10 * t,
        Lead::Icon(_) => 16 * t,
    };
    let font = kit.font(style.face);
    let block = Block::new(font, style.face, t, s, width - gutter, style.align);
    let lead_h = match lead {
        Lead::Icon(_) => 12 * t,
        _ => 0,
    };
    let h = block.height.max(lead_h).max(1);
    let icon = match lead {
        Lead::Icon(i) => Some(i.image()),
        _ => None,
    };
    let piece = sprite(
        m,
        width,
        h,
        move |img, _| {
            let kit = kit.clone();
            let font = kit.font(style.face);
            match lead {
                Lead::Bullet => paint::bullet(img, 2 * t, 5 * t, 3 * t),
                Lead::Icon(_) => {
                    if let Some(icon) = &icon {
                        img.sprite(&icon.upscale(t as u32), 0, t, 255);
                    }
                }
                Lead::None => {}
            }
            block.draw(font, img, gutter, 0, style.color);
        },
        Look::default,
    );
    let label = s.to_string();
    let piece = piece.a11y(move |a| {
        let a = a.label(label);
        match style.heading {
            Some(level) => a.role(Role::Heading(level)),
            None => a,
        }
    });
    W::framed(m, with_id(piece, id), width, h)
}

fn with_id(piece: impl Piece, id: Option<ElementId>) -> AnyPiece {
    match id {
        Some(id) => piece.id(id).any(),
        None => piece.any(),
    }
}

/// A picture from the game's art, centered in `width`, enlarged as far as whole art pixels let
/// it fit (at most `max_mult`). Decorative: the text beside it says what it shows.
pub fn picture(m: &Metrics, width: i32, icon: Icon, max_mult: i32) -> W {
    let art = icon.image();
    let mult = (width / art.width.max(1) as i32).clamp(1, max_mult.max(1));
    let (aw, ah) = (art.width as i32 * mult, art.height as i32 * mult);
    let big = art.upscale(mult as u32);
    let piece = sprite(
        m,
        width,
        ah,
        move |img, _| img.sprite(&big, (width - aw) / 2, 0, 255),
        Look::default,
    )
    .a11y(|a| a.decorative());
    W::framed(m, piece, width, ah)
}

/// Art that is only decoration: a panel, a frame. Hidden from screen readers.
pub fn art(m: &Metrics, w: i32, h: i32, paint: impl Fn(&mut Image) + 'static) -> AnyPiece {
    sprite(m, w, h, move |img, _| paint(img), Look::default)
        .a11y(|a| a.decorative())
        .frame(m.pt(w), m.pt(h))
        .any()
}

/// A one pixel line across `width`, as under a dialog's title. Decorative.
pub fn rule(m: &Metrics, width: i32) -> W {
    let piece = sprite(
        m,
        width,
        1,
        |img, _| img.fill_argb(0, 0, img.width as i32, 1, PALETTE.rule),
        Look::default,
    )
    .a11y(|a| a.decorative());
    W::framed(m, piece, width, 1)
}

/// Empty space. Decorative.
pub fn gap(m: &Metrics, w: i32, h: i32) -> W {
    let (wp, hp) = (m.pt(w), m.pt(h));
    W::new(spacer().frame(wp, hp).a11y(|a| a.hidden()), w, h)
}

// ---------------------------------------------------------------- buttons

/// The inside of a button: room for the bevel round the label (PixelUi: 8 by 6 art pixels).
const PAD_X: i32 = 8;
const PAD_Y: i32 = 6;
const BUTTON_MIN_H: i32 = 22;

/// A brass button.
pub struct Button {
    pub id: ElementId,
    pub label: String,
    /// The whole widget's width, focus margin included; `None` hugs the label.
    pub width: Option<i32>,
    pub icon: Option<Icon>,
    /// Take keyboard focus when shown, as a dialog's default button does.
    pub autofocus: bool,
    /// A longer description for screen readers.
    pub hint: Option<String>,
}

impl Button {
    pub fn new(id: ElementId, label: impl Into<String>) -> Button {
        Button {
            id,
            label: label.into(),
            width: None,
            icon: None,
            autofocus: false,
            hint: None,
        }
    }

    pub fn width(mut self, w: i32) -> Button {
        self.width = Some(w);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Button {
        self.icon = Some(icon);
        self
    }

    pub fn autofocus(mut self, on: bool) -> Button {
        self.autofocus = on;
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Button {
        self.hint = Some(hint.into());
        self
    }

    /// The width the label needs, focus margin included.
    pub fn natural_width(&self, m: &Metrics) -> i32 {
        let kit = kit();
        let ink = kit.display.ink_width(&super::text::printable(&self.label)) * m.t;
        let icon = if self.icon.is_some() { 16 * m.t } else { 0 };
        ink + icon + 2 * PAD_X + 2 * FOCUS
    }

    /// The height the button will have, focus margin included, before it is built.
    pub fn height(&self, m: &Metrics) -> i32 {
        let kit = kit();
        let t = m.t;
        let w = self.width.unwrap_or_else(|| self.natural_width(m)).max(24);
        let icon_w = if self.icon.is_some() { 16 * t } else { 0 };
        let inner = w - 2 * FOCUS - 2 * PAD_X - icon_w;
        let block = Block::new(
            &kit.display,
            Face::Display,
            t,
            &self.label,
            inner,
            Align::Center,
        );
        let body_h = block
            .height
            .max(if self.icon.is_some() { 12 * t } else { 0 });
        (body_h + 2 * PAD_Y).max(BUTTON_MIN_H) + 2 * FOCUS
    }

    /// Builds the button; `action` runs after the press has shown.
    pub fn build(self, m: &Metrics, action: impl Fn() + 'static) -> W {
        let kit = kit();
        let t = m.t;
        let w = self.width.unwrap_or_else(|| self.natural_width(m)).max(24);
        let icon_w = if self.icon.is_some() { 16 * t } else { 0 };
        let inner = w - 2 * FOCUS - 2 * PAD_X - icon_w;
        let block = Block::new(
            &kit.display,
            Face::Display,
            t,
            &self.label,
            inner,
            Align::Center,
        );
        let body_h = block
            .height
            .max(if self.icon.is_some() { 12 * t } else { 0 });
        let frame_h = (body_h + 2 * PAD_Y).max(BUTTON_MIN_H);
        let h = frame_h + 2 * FOCUS;
        let icon = self.icon.map(|i| i.image().upscale(t as u32));
        let pressed = Signal::new(false);
        let focused = Signal::new(false);
        if self.autofocus {
            // A moment after the dialog opens: the tap that opened it is still finishing, and
            // its last events would otherwise take the focus back to whatever is under them.
            day::task(async move {
                day::sleep(AUTOFOCUS_MS).await;
                if focused.try_get().is_some() {
                    focused.set(true);
                }
            });
        }
        let ink = block.ink_width(&kit.display);
        let paint = move |img: &mut Image, look: Look| {
            let kit: &Kit = &kit;
            if look.focused {
                paint::focus_ring(img, 0, 0, w, h);
            }
            paint::button(img, kit, FOCUS, FOCUS, w - 2 * FOCUS, frame_h, look);
            let nudge = if look.pressed { 1 } else { 0 };
            // The label and icon together, centered in the frame
            let content = ink + icon_w;
            let x0 = FOCUS + (w - 2 * FOCUS - content) / 2;
            let y0 = FOCUS + (frame_h - body_h) / 2 + nudge;
            if let Some(icon) = &icon {
                img.sprite(icon, x0, y0 + (body_h - icon.height as i32) / 2, 255);
            }
            let color = if look.disabled {
                paint::BUTTON_TEXT_DISABLED
            } else {
                paint::BUTTON_TEXT
            };
            let mut lines = block.clone();
            lines.width = ink;
            let ty = y0 + (body_h - block.height) / 2;
            lines.draw(&kit.display, img, x0 + icon_w, ty, color);
        };
        let action: Rc<dyn Fn()> = Rc::new(action);
        let act = press_action(pressed, action);
        let tap = act.clone();
        let label = self.label.clone();
        let hint = self.hint.clone();
        let piece = sprite(m, w, h, paint, move || Look {
            pressed: pressed.get(),
            focused: focused.get(),
            disabled: false,
        })
        .on_tap(move || tap())
        .on_key(keys(act, true))
        .focusable()
        .focused(focused)
        .a11y(move |a| {
            let a = a.role(Role::Button).label(label);
            match hint {
                Some(h) => a.hint(h),
                None => a,
            }
        })
        .id(self.id);
        W::framed(m, piece, w, h)
    }
}

// ---------------------------------------------------------------- choices

/// The row a check box or a radio button draws: its box beside wrapped body text.
struct Row {
    block: Block,
    w: i32,
    h: i32,
    t: i32,
}

/// The gap between a box and its text.
const BOX_GAP: i32 = 4;

impl Row {
    fn new(m: &Metrics, width: i32, label: &str) -> Row {
        let kit = kit();
        let t = m.t;
        let text_w = width - 2 * FOCUS - 2 - (paint::BOX + BOX_GAP) * t;
        let block = Block::new(&kit.body, Face::Body, t, label, text_w, Align::Start);
        let h = block.height.max(paint::BOX * t) + 2 * FOCUS + 2;
        Row {
            block,
            w: width,
            h,
            t,
        }
    }

    /// Draws the frame of the row and its text, and returns where its box goes.
    fn paint(&self, img: &mut Image, look: Look) -> (i32, i32) {
        let kit = kit();
        if look.focused {
            paint::row_focus(img, FOCUS, FOCUS, self.w - 2 * FOCUS, self.h - 2 * FOCUS);
            paint::focus_ring(img, 0, 0, self.w, self.h);
        }
        let t = self.t;
        let x = FOCUS + 1;
        let y = FOCUS + 1;
        // The box sits on the first line, centered on its height
        let line = kit.body.height() * t;
        let by = y + ((line - paint::BOX * t) / 2).max(0);
        self.block.draw(
            &kit.body,
            img,
            x + (paint::BOX + BOX_GAP) * t,
            y,
            PALETTE.ink,
        );
        (x, by)
    }
}

/// A box drawn at its own size, then enlarged `t` times onto the row.
fn put_box(img: &mut Image, t: i32, x: i32, y: i32, draw: impl FnOnce(&mut Image)) {
    let mut b = Image::new(paint::BOX as u32, paint::BOX as u32);
    draw(&mut b);
    img.sprite(&b.upscale(t as u32), x, y, 255);
}

/// The role a check box takes: a switch, except on the web (see `check`).
const CHECK_ROLE: Role = if cfg!(target_arch = "wasm32") {
    Role::Button
} else {
    Role::Toggle
};

/// The words a screen reader says for a switch's state, so they follow the app's language.
#[derive(Clone)]
pub struct States {
    pub on: String,
    pub off: String,
}

/// A check box bound to `get` and `set`: a switch to screen readers, with On or Off as its value.
/// On the web it is a button with that value instead: the browser shim gives Enter and Space to
/// buttons only, and a switch there could not be toggled from the keyboard.
pub fn check(
    m: &Metrics,
    id: ElementId,
    width: i32,
    label: &str,
    states: States,
    get: impl Fn() -> bool + 'static,
    set: impl Fn(bool) + 'static,
) -> W {
    let row = Rc::new(Row::new(m, width, label));
    let (w, h, t) = (row.w, row.h, row.t);
    let get = Rc::new(get);
    let focused = Signal::new(false);
    let toggle: Rc<dyn Fn()> = {
        let get = get.clone();
        Rc::new(move || {
            feedback();
            set(!get());
        })
    };
    let paint_row = row.clone();
    let on_now = get.clone();
    let value_get = get.clone();
    let tap = toggle.clone();
    let label = label.to_string();
    let piece = sprite(
        m,
        w,
        h,
        move |img, look| {
            let (bx, by) = paint_row.paint(img, look);
            let on = look.pressed;
            put_box(img, t, bx, by, |b| paint::check_box(b, &kit(), 0, 0, on));
        },
        // The look's pressed flag carries the checked state here
        move || Look {
            pressed: on_now(),
            focused: focused.get(),
            disabled: false,
        },
    )
    .on_tap(move || tap())
    .on_key(keys(toggle, CHECK_ROLE == Role::Button))
    .focusable()
    .focused(focused)
    .a11y(move |a| {
        let states = states.clone();
        a.role(CHECK_ROLE).label(label).value(move || {
            if value_get() {
                states.on.clone()
            } else {
                states.off.clone()
            }
        })
    })
    .id(id);
    W::framed(m, piece, w, h)
}

/// One radio button of a group: chosen when `chosen` says so, `pick` chooses it. A button to
/// screen readers, with its selected state and a value that says it. `spoken` is its accessible
/// label: the visible `label` after the group's name, since a lone "Off" says nothing.
#[allow(clippy::too_many_arguments)]
pub fn radio(
    m: &Metrics,
    id: ElementId,
    width: i32,
    label: &str,
    spoken: &str,
    swatch: Option<u32>,
    states: States,
    chosen: impl Fn() -> bool + 'static,
    pick: impl Fn() + 'static,
) -> W {
    let row = Rc::new(Row::new(m, width, label));
    let (w, h, t) = (row.w, row.h, row.t);
    let chosen = Rc::new(chosen);
    let focused = Signal::new(false);
    let pick: Rc<dyn Fn()> = Rc::new(move || {
        feedback();
        pick();
    });
    let paint_row = row.clone();
    let on_now = chosen.clone();
    let (sel, val) = (chosen.clone(), chosen.clone());
    let tap = pick.clone();
    let label = spoken.to_string();
    let piece = sprite(
        m,
        w,
        h,
        move |img, look| {
            let (bx, by) = paint_row.paint(img, look);
            let on = look.pressed;
            put_box(img, t, bx, by, |b| {
                paint::radio_box(b, &kit(), 0, 0, on, swatch)
            });
        },
        move || Look {
            pressed: on_now(),
            focused: focused.get(),
            disabled: false,
        },
    )
    .on_tap(move || tap())
    .on_key(keys(pick, true))
    .focusable()
    .focused(focused)
    .a11y(move |a| {
        let states = states.clone();
        a.role(Role::Button)
            .label(label)
            .selected(move || sel())
            .value(move || {
                if val() {
                    states.on.clone()
                } else {
                    states.off.clone()
                }
            })
    })
    .id(id);
    W::framed(m, piece, w, h)
}

/// A number chosen between `min` and `max`: the value in a recessed field, an adjustable slider
/// to screen readers (the arrow keys change it), with a minus and a plus button either side.
#[allow(clippy::too_many_arguments)]
pub fn stepper(
    m: &Metrics,
    id: ElementId,
    label: &str,
    range: std::ops::RangeInclusive<i64>,
    ids: (ElementId, ElementId),
    words: (String, String),
    get: impl Fn() -> i64 + 'static,
    set: impl Fn(i64) + 'static,
) -> W {
    let kit = kit();
    let t = m.t;
    let (min, max) = (*range.start(), *range.end());
    let get = Rc::new(get);
    let set: Rc<dyn Fn(i64)> = Rc::new(move |v| set(v.clamp(min, max)));
    let step = {
        let (get, set) = (get.clone(), set.clone());
        Rc::new(move |d: i64| set(get() + d))
    };
    let minus = {
        let step = step.clone();
        Button::new(ids.0, "-")
            .hint(words.0)
            .width(24 * t + 2 * FOCUS)
            .build(m, move || step(-1))
    };
    let plus = {
        let step = step.clone();
        Button::new(ids.1, "+")
            .hint(words.1)
            .width(24 * t + 2 * FOCUS)
            .build(m, move || step(1))
    };
    let digits = max.to_string().len().max(2) as i32;
    let field_w = digits * 8 * t + 2 * PAD_X + 2 * FOCUS;
    let h = minus.h;
    let focused = Signal::new(false);
    let shown = get.clone();
    let value_text = get.clone();
    let piece = sprite(
        m,
        field_w,
        h,
        move |img, look| {
            if look.focused {
                paint::focus_ring(img, 0, 0, field_w, h);
            }
            paint::frame(
                img,
                &kit.pressed,
                FOCUS,
                FOCUS,
                field_w - 2 * FOCUS,
                h - 2 * FOCUS,
            );
            let s = shown().to_string();
            let block = Block::new(
                &kit.display,
                Face::Display,
                t,
                &s,
                field_w - 2 * FOCUS,
                Align::Center,
            );
            block.draw(
                &kit.display,
                img,
                FOCUS,
                (h - block.height) / 2,
                PALETTE.field_text,
            );
        },
        {
            let get = get.clone();
            move || {
                // Tracked: the value changing redraws the field
                let _ = get();
                Look {
                    focused: focused.get(),
                    ..Look::default()
                }
            }
        },
    )
    .on_key(move |k: &day::KeyEvent| match k.key.as_str() {
        "ArrowLeft" | "ArrowDown" | "-" => step(-1),
        "ArrowRight" | "ArrowUp" | "+" => step(1),
        "Escape" | "Backspace" => {
            escape();
        }
        _ => {}
    })
    .focusable()
    .focused(focused)
    .a11y({
        let label = label.to_string();
        move |a| {
            a.role(Role::Slider)
                .label(label)
                .value(move || value_text().to_string())
        }
    })
    .id(id)
    .frame(m.pt(field_w), m.pt(h));
    let gap = 2;
    let w = minus.w + gap + field_w + gap + plus.w;
    let row = row((minus.piece, piece, plus.piece))
        .spacing(m.pt(gap))
        .align(VAlign::Top);
    W::new(row, w, h)
}

/// A link: underlined text that opens `url` in the browser. A button to screen readers, since
/// Day has no link role for a canvas.
pub fn link(m: &Metrics, id: ElementId, width: i32, label: &str, url: &str) -> W {
    let kit = kit();
    let t = m.t;
    let block = Block::new(
        &kit.body,
        Face::Body,
        t,
        label,
        width - 2 * FOCUS,
        Align::Start,
    );
    let h = block.height + 2 * FOCUS + t;
    let ink = block.ink_width(&kit.body);
    let w = (ink + 2 * FOCUS + 2).min(width);
    let focused = Signal::new(false);
    let pressed = Signal::new(false);
    let url = url.to_string();
    let open: Rc<dyn Fn()> = Rc::new(move || open_url(&url));
    let act = press_action(pressed, open);
    let tap = act.clone();
    let line_h = kit.body.height() * t;
    let piece = sprite(
        m,
        w,
        h,
        move |img, look| {
            if look.focused {
                paint::focus_ring(img, 0, 0, w, h);
            }
            let color = if look.pressed {
                PALETTE.ink
            } else {
                PALETTE.link
            };
            block.draw(&kit.body, img, FOCUS + 1, FOCUS, color);
            // The underline, a pixel under each line's glyphs
            for (i, l) in block.lines.iter().enumerate() {
                let lw = kit.body.ink_width(l) * t;
                let y = FOCUS + i as i32 * line_h + (kit.body.height() - 2) * t;
                img.fill_argb(FOCUS + 1, y, lw, t, color);
            }
        },
        move || Look {
            pressed: pressed.get(),
            focused: focused.get(),
            disabled: false,
        },
    )
    .on_tap(move || tap())
    .on_key(keys(act, true))
    .focusable()
    .focused(focused)
    .a11y({
        let label = label.to_string();
        move |a| a.role(Role::Button).label(label)
    })
    .id(id);
    W::framed(m, piece, w, h)
}

/// One column of a `cells` row: its text, where it starts and how wide it is in art pixels, how it
/// sits there, its color and its font.
pub type Column = (String, i32, i32, Align, u32, Face);

/// A line of text in columns, such as a score's rank, points and date, read by screen readers as
/// one element, `label`.
pub fn cells(m: &Metrics, id: Option<ElementId>, width: i32, cells: Vec<Column>, label: &str) -> W {
    let kit = kit();
    let t = m.t;
    let blocks: Vec<(Block, i32, u32)> = cells
        .into_iter()
        .map(|(s, x, w, align, color, face)| {
            let block = Block::new(kit.font(face), face, t, &s, w, align);
            (block, x, color)
        })
        .collect();
    let h = blocks
        .iter()
        .map(|(b, ..)| b.height)
        .max()
        .unwrap_or(1)
        .max(1);
    let piece = sprite(
        m,
        width,
        h,
        move |img, _| {
            for (block, x, color) in &blocks {
                let font = kit.font(block.face);
                // The display font's glyphs are shorter than the body font's line: they drop
                // to share its baseline
                let y = match block.face {
                    Face::Display => (h - block.height).min(3 * t),
                    Face::Body => 0,
                };
                block.draw(font, img, *x, y, *color);
            }
        },
        Look::default,
    );
    let label = label.to_string();
    let piece = piece.a11y(move |a| a.label(label));
    W::framed(m, with_id(piece, id), width, h)
}

// ---------------------------------------------------------------- stacking

/// Widgets one above the other, `gap` art pixels apart, left aligned.
pub fn vstack(m: &Metrics, items: Vec<W>, gap: i32) -> W {
    let w = items.iter().map(|i| i.w).max().unwrap_or(0);
    let h = items.iter().map(|i| i.h).sum::<i32>() + gap * (items.len() as i32 - 1).max(0);
    let pieces: Vec<AnyPiece> = items.into_iter().map(|i| i.piece).collect();
    W::new(
        column(pieces).spacing(m.pt(gap)).align(HAlign::Leading),
        w,
        h,
    )
}

/// Widgets side by side, `gap` art pixels apart, top aligned.
pub fn hstack(m: &Metrics, items: Vec<W>, gap: i32) -> W {
    let h = items.iter().map(|i| i.h).max().unwrap_or(0);
    let w = items.iter().map(|i| i.w).sum::<i32>() + gap * (items.len() as i32 - 1).max(0);
    let pieces: Vec<AnyPiece> = items.into_iter().map(|i| i.piece).collect();
    W::new(row(pieces).spacing(m.pt(gap)).align(VAlign::Top), w, h)
}

/// A widget moved `left` art pixels to the right.
pub fn inset(m: &Metrics, item: W, left: i32) -> W {
    if left <= 0 {
        return item;
    }
    let h = item.h;
    hstack(m, vec![gap(m, left, h), item], 0)
}

/// A widget centered across `width` art pixels, on whole art pixels.
pub fn center(m: &Metrics, item: W, width: i32) -> W {
    let left = (width - item.w) / 2;
    inset(m, item, left)
}
