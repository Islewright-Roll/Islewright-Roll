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
use super::paint::{self, Icon, Kit, Look, kit, palette};
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
        let _ = paint::palette_changes().get();
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

thread_local! {
    static KEYBOARD: Cell<Option<Signal<bool>>> = const { Cell::new(None) };
}

/// Whether the keyboard is in use. A focus ring shows only then: a phone's touches never show
/// one, as in the Android app.
fn keyboard() -> Signal<bool> {
    KEYBOARD.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global(!cfg!(target_os = "android"));
            c.set(Some(s));
            s
        }
    })
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
    move |k: &day::KeyEvent| {
        if !keyboard().get_untracked() {
            keyboard().set(true);
        }
        match k.key.as_str() {
            "Enter" | " " if !shim_activates => activate(),
            "Escape" | "Backspace" => {
                escape();
            }
            _ => {}
        }
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

/// A text color, looked up in the look as it draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ink,
    Secondary,
}

impl Tone {
    pub fn argb(self) -> u32 {
        match self {
            Tone::Ink => palette().ink,
            Tone::Secondary => palette().secondary,
        }
    }
}

/// How a piece of text looks and what it is to a screen reader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub face: Face,
    pub color: Tone,
    pub align: Align,
    /// A heading of this level, or plain text.
    pub heading: Option<u8>,
}

impl Style {
    pub const BODY: Style = Style {
        face: Face::Body,
        color: Tone::Ink,
        align: Align::Start,
        heading: None,
    };
    pub const NOTE: Style = Style {
        color: Tone::Secondary,
        ..Style::BODY
    };
    pub const HEADING: Style = Style {
        face: Face::Display,
        color: Tone::Ink,
        align: Align::Start,
        heading: Some(2),
    };
    pub const TITLE: Style = Style {
        face: Face::Display,
        color: Tone::Ink,
        align: Align::Center,
        heading: Some(1),
    };
    pub fn centered(mut self) -> Style {
        self.align = Align::Center;
        self
    }

    pub fn end(mut self) -> Style {
        self.align = Align::End;
        self
    }
}

/// What a line of text starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lead {
    None,
    Bullet,
    Icon(Icon),
    /// An icon whose magenta parts take the player's color.
    Player(Icon, u32),
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
        Lead::Bullet => 8 * t,
        Lead::Icon(_) | Lead::Player(..) => 16 * t,
    };
    let font = kit.font(style.face);
    let block = Block::new(font, style.face, t, s, width - gutter, style.align);
    let lead_h = match lead {
        Lead::Icon(_) | Lead::Player(..) => 12 * t,
        _ => 0,
    };
    let h = block.height.max(lead_h).max(1);
    let icon = match lead {
        Lead::Icon(i) => Some(i.image()),
        Lead::Player(i, color) => Some(crate::view::assets::recolored(&i.image(), color)),
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
                Lead::Bullet => paint::bullet(img, t, 6 * t, 3 * t),
                Lead::Icon(_) | Lead::Player(..) => {
                    if let Some(icon) = &icon {
                        img.sprite(&icon.upscale(t as u32), 0, t, 255);
                    }
                }
                Lead::None => {}
            }
            block.draw(font, img, gutter, 0, style.color.argb());
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

/// A picture from the game's art at its own size, centered in `width`, made smaller by a whole
/// factor when it is wider than that (PixelImage.fit). Decorative: the text beside it says what
/// it shows.
pub fn picture(m: &Metrics, width: i32, icon: Icon) -> W {
    let art = icon.image();
    let factor = ((art.width as i32 + width - 1) / width.max(1)).max(1) as u32;
    let big = art.shrink(factor);
    let (aw, ah) = (big.width as i32, big.height as i32);
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
        |img, _| img.fill_argb(0, 0, img.width as i32, 1, palette().rule),
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
    /// The whole widget's width; `None` hugs the label.
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

    /// The width the label needs.
    pub fn natural_width(&self, m: &Metrics) -> i32 {
        let kit = kit();
        let ink = kit.display.width(&super::text::printable(&self.label)) * m.t;
        let icon = if self.icon.is_some() { 16 * m.t } else { 0 };
        ink + icon + 2 * PAD_X
    }

    /// The height the button will have, before it is built.
    pub fn height(&self, m: &Metrics) -> i32 {
        let kit = kit();
        let t = m.t;
        let w = self.width.unwrap_or_else(|| self.natural_width(m)).max(24);
        let icon_w = if self.icon.is_some() { 16 * t } else { 0 };
        let inner = w - 2 * PAD_X - icon_w;
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
        (body_h + 2 * PAD_Y).max(BUTTON_MIN_H)
    }

    /// Builds the button; `action` runs after the press has shown.
    pub fn build(self, m: &Metrics, action: impl Fn() + 'static) -> W {
        let kit = kit();
        let t = m.t;
        let w = self.width.unwrap_or_else(|| self.natural_width(m)).max(24);
        let icon_w = if self.icon.is_some() { 16 * t } else { 0 };
        let inner = w - 2 * PAD_X - icon_w;
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
        let h = (body_h + 2 * PAD_Y).max(BUTTON_MIN_H);
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
        let paint = move |img: &mut Image, look: Look| {
            let kit: &Kit = &kit;
            paint::button(img, kit, 0, 0, w, h, look);
            if look.focused {
                paint::focus_ring(img, 0, 0, w, h);
            }
            let nudge = if look.pressed { 1 } else { 0 };
            let y0 = (h - body_h) / 2 + nudge;
            let color = if look.disabled {
                paint::BUTTON_TEXT_DISABLED
            } else {
                paint::BUTTON_TEXT
            };
            let mut x0 = PAD_X;
            if let Some(icon) = &icon {
                let content = block.line_width(&kit.display) + icon_w;
                x0 = (w - content) / 2;
                img.sprite(icon, x0, y0 + (body_h - icon.height as i32) / 2, 255);
                x0 += icon_w;
            }
            let ty = y0 + (body_h - block.height) / 2;
            block.draw(&kit.display, img, x0, ty, color);
        };
        let action: Rc<dyn Fn()> = Rc::new(action);
        let act = press_action(pressed, action.clone());
        let tap = act.clone();
        let label = self.label.clone();
        let hint = self.hint.clone();
        // Where a platform says when the finger goes down, the button shows pressed for as long
        // as the finger is on it, and its tick and its action come with the lift
        let held = Rc::new(Cell::new(None::<bool>));
        let (wp, hp) = (m.pt(w), m.pt(h));
        let on_touch = {
            let held = held.clone();
            move |d: Drag| {
                let inside = d.location.x >= 0.0
                    && d.location.y >= 0.0
                    && d.location.x < wp
                    && d.location.y < hp;
                match d.phase {
                    DragPhase::Began if d.translation.x == 0.0 && d.translation.y == 0.0 => {
                        held.set(Some(true));
                        pressed.set(true);
                    }
                    DragPhase::Began => {}
                    DragPhase::Changed => {
                        if held.get().is_some() {
                            held.set(Some(inside));
                            pressed.set(inside);
                        }
                    }
                    DragPhase::Ended => {
                        if held.get().is_some() {
                            held.set(Some(inside));
                            pressed.set(false);
                        }
                    }
                }
            }
        };
        let piece = sprite(m, w, h, paint, move || Look {
            pressed: pressed.get(),
            focused: focused.get() && keyboard().get(),
            disabled: false,
        })
        .on_tap(move || tap())
        .on_key(keys(act.clone(), true))
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
        // Touch comes to a layer of its own: a view that follows a finger loses the click a
        // screen reader activates it by, so that stays on the button under it
        let touch = canvas(|_, _| {})
            .on_drag(on_touch)
            .on_tap(move || match held.take() {
                Some(true) => {
                    feedback();
                    action();
                }
                Some(false) => {}
                None => act(),
            })
            .a11y(|a| a.hidden())
            .frame(wp, hp);
        let layered = zstack((piece, touch)).align(Alignment::TopLeading);
        W::framed(m, layered, w, h)
    }
}

// ---------------------------------------------------------------- choices

/// The row a check box or a radio button draws: its box beside wrapped body text, 2 art pixels
/// above and below (settings.xml), at least 16 tall.
struct Row {
    block: Block,
    w: i32,
    h: i32,
    t: i32,
}

/// The room a box and its gap take, and a row's padding above and below.
const BOX_ROOM: i32 = 16;
const ROW_PAD: i32 = 2;

impl Row {
    fn new(m: &Metrics, width: i32, label: &str) -> Row {
        let kit = kit();
        let t = m.t;
        let text_w = width - BOX_ROOM * t;
        let block = Block::new(&kit.body, Face::Body, t, label, text_w, Align::Start);
        let h = (block.height + 2 * ROW_PAD).max(BOX_ROOM * t);
        Row {
            block,
            w: width,
            h,
            t,
        }
    }

    /// Draws the frame of the row and its text, and returns where its box goes: centered on the
    /// first line's letters, as PixelRadioButton had it.
    fn paint(&self, img: &mut Image, look: Look) -> (i32, i32) {
        let kit = kit();
        if look.focused {
            paint::row_focus(img, 0, 0, self.w, self.h);
            paint::focus_ring(img, 0, 0, self.w, self.h);
        }
        let t = self.t;
        self.block
            .draw(&kit.body, img, BOX_ROOM * t, ROW_PAD, palette().ink);
        (0, ROW_PAD + 11 * t - 9 * t)
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

/// The width a check box or radio button needs for `label` on one line, even so that it centers
/// on the art grid (WelcomeDialog).
pub fn row_width(m: &Metrics, label: &str) -> i32 {
    let w = kit().body.width(&super::text::printable(label)) * m.t + BOX_ROOM * m.t;
    w + (w & 1)
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
        Rc::new(move || set(!get()))
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
            focused: focused.get() && keyboard().get(),
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
    let pick: Rc<dyn Fn()> = Rc::new(pick);
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
            focused: focused.get() && keyboard().get(),
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

/// A number chosen between `min` and `max` with the kit's slider: an adjustable slider to screen
/// readers, whose arrow keys change it. It reports as it moves. A tap jumps there, and a drag
/// follows the finger, with the thumb's middle under it.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    m: &Metrics,
    id: ElementId,
    width: i32,
    label: &str,
    range: std::ops::RangeInclusive<i64>,
    get: impl Fn() -> i64 + 'static,
    set: impl Fn(i64) + 'static,
) -> W {
    let kit = kit();
    let (min, max) = (*range.start(), *range.end());
    let span = (max - min).max(1);
    let get = Rc::new(get);
    let set: Rc<dyn Fn(i64)> = Rc::new(move |v| set(v.clamp(min, max)));
    let room = (width - paint::THUMB_W).max(1) as f64;
    let (art_pt, half) = (m.pt(1), paint::THUMB_W as f64 / 2.0);
    let at = {
        let set = set.clone();
        Rc::new(move |x: f64| {
            let f = ((x / art_pt - half) / room).clamp(0.0, 1.0);
            set(min + (f * span as f64).round() as i64);
        })
    };
    let pressed = Signal::new(false);
    let focused = Signal::new(false);
    let shown = get.clone();
    let value = get.clone();
    let step = {
        let (get, set) = (get.clone(), set.clone());
        move |d: i64| set(get() + d)
    };
    let tap_at = at.clone();
    let piece = sprite(
        m,
        width,
        paint::SLIDER_H,
        move |img, look| {
            let f = (shown() - min) as f64 / span as f64;
            let x = (f * room).round() as i32;
            paint::slider(img, &kit, 0, width, x, look.pressed);
            if look.focused {
                paint::focus_ring(img, x, 0, paint::THUMB_W, paint::SLIDER_H);
            }
        },
        {
            let get = get.clone();
            move || {
                // Tracked: the value changing redraws it
                let _ = get();
                Look {
                    pressed: pressed.get(),
                    focused: focused.get() && keyboard().get(),
                    disabled: false,
                }
            }
        },
    )
    .on_tap_at(move |p| tap_at(p.x))
    .on_drag(move |d: Drag| {
        at(d.location.x);
        pressed.set(!matches!(d.phase, DragPhase::Ended));
    })
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
                .value(move || value().to_string())
        }
    })
    .id(id);
    W::framed(m, piece, width, paint::SLIDER_H)
}

/// A flat, tappable block of color with no art of its own: a color picker's swatch.
pub fn swatch(
    m: &Metrics,
    id: ElementId,
    (w, h): (i32, i32),
    argb: u32,
    label: &str,
    tap: impl Fn() + 'static,
) -> W {
    let piece = sprite(
        m,
        w,
        h,
        move |img, _| img.fill_argb(0, 0, w, h, argb),
        Look::default,
    )
    .on_tap(tap)
    .a11y({
        let label = label.to_string();
        move |a| a.role(Role::Button).label(label)
    })
    .id(id);
    W::framed(m, piece, w, h)
}

/// A box filled with what `color` says, in a one pixel outline: the color preview.
pub fn preview(m: &Metrics, w: i32, h: i32, color: impl Fn() -> u32 + 'static) -> W {
    let color = Rc::new(color);
    let tracked = color.clone();
    let piece = sprite(
        m,
        w,
        h,
        move |img, _| {
            img.fill_argb(0, 0, w, h, palette().frame);
            img.fill_argb(1, 1, w - 2, h - 2, 0xFF000000 | color());
        },
        move || {
            let _ = tracked();
            Look::default()
        },
    )
    .a11y(|a| a.decorative());
    W::framed(m, piece, w, h)
}

/// Text in a recessed field, its text typed through an invisible native field laid over it, so
/// the keyboard and the system's editing work while the letters stay the kit's. The native caret
/// does not show.
pub fn text_entry(m: &Metrics, id: ElementId, width: i32, label: &str, value: Signal<String>) -> W {
    let kit = kit();
    let t = m.t;
    let h = kit.body.height() * t + 2 * PAD_Y;
    let art = sprite(
        m,
        width,
        h,
        move |img, _| {
            paint::field(img, &kit, 0, 0, width, h);
            let s = value.get_untracked();
            kit.body.draw(img, &s, PAD_X, PAD_Y, palette().field_text);
        },
        move || {
            let _ = value.get();
            Look::default()
        },
    )
    .a11y(|a| a.decorative());
    let label = label.to_string();
    let native = text_field(value)
        .a11y(move |a| a.label(label))
        .id(id)
        .opacity(0.02)
        .frame(m.pt(width), m.pt(h));
    let stack = zstack((art.frame(m.pt(width), m.pt(h)), native)).frame(m.pt(width), m.pt(h));
    W::new(stack, width, h)
}

/// A hue and saturation wheel, `art` art pixels across, drawn smooth at device pixels (a gradient
/// is the one thing that should not be pixel art): hue round the rim, white at the center, with a
/// ring at the color `at` says. `set` gets the hue in degrees and the saturation from 0 to 1.
pub fn color_wheel(
    m: &Metrics,
    id: ElementId,
    art: i32,
    label: &str,
    at: impl Fn() -> (f64, f64) + 'static,
    set: impl Fn(f64, f64) + 'static,
) -> W {
    let size = art * m.k;
    let device = Metrics { k: 1, ..*m };
    let wheel = Rc::new(wheel_image(size));
    let at = Rc::new(at);
    let tracked = at.clone();
    let radius = size as f64 / 2.0;
    let ring = 9.0 * m.dpr;
    let stroke = 2.0 * m.dpr;
    let piece = sprite(
        &device,
        size,
        size,
        move |img, _| {
            img.sprite(&wheel, 0, 0, 255);
            let (hue, sat) = at();
            let x = radius + sat * radius * hue.to_radians().cos();
            let y = radius + sat * radius * hue.to_radians().sin();
            draw_ring(img, x, y, ring, stroke, 0xFFFFFFFF);
            draw_ring(img, x, y, ring - stroke, stroke, 0xFF000000);
        },
        move || {
            let _ = tracked();
            Look::default()
        },
    );
    let dpr = m.dpr;
    let set = Rc::new(set);
    let touch = {
        let set = set.clone();
        Rc::new(move |x: f64, y: f64| {
            let (dx, dy) = (x * dpr - radius, y * dpr - radius);
            let d = (dx * dx + dy * dy).sqrt().min(radius);
            let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
            set(hue, d / radius);
        })
    };
    let tap = touch.clone();
    let piece = piece
        .on_tap_at(move |p| tap(p.x, p.y))
        .on_drag(move |d: Drag| touch(d.location.x, d.location.y))
        .a11y({
            let label = label.to_string();
            move |a| a.label(label)
        })
        .id(id);
    W::new(piece.frame(m.pt(art), m.pt(art)), art, art)
}

/// The wheel itself, antialiased at its rim.
fn wheel_image(size: i32) -> Image {
    let mut img = Image::new(size.max(1) as u32, size.max(1) as u32);
    let r = size as f64 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f64 + 0.5 - r, y as f64 + 0.5 - r);
            let d = (dx * dx + dy * dy).sqrt();
            let edge = (r - d + 0.5).clamp(0.0, 1.0);
            if edge <= 0.0 {
                continue;
            }
            let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
            let argb = crate::rules::player_color::hsv_to_argb(hue, (d / r).min(1.0), 1.0);
            let a = (edge * 255.0).round() as u32;
            img.fill_argb(x, y, 1, 1, (a << 24) | (argb & 0xFFFFFF));
        }
    }
    img
}

/// A circle `stroke` wide at radius `r`, antialiased.
fn draw_ring(img: &mut Image, cx: f64, cy: f64, r: f64, stroke: f64, argb: u32) {
    let reach = (r + stroke).ceil() as i32 + 1;
    let (x0, y0) = (cx.floor() as i32, cy.floor() as i32);
    for y in y0 - reach..=y0 + reach {
        for x in x0 - reach..=x0 + reach {
            if x < 0 || y < 0 || x >= img.width as i32 || y >= img.height as i32 {
                continue;
            }
            let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
            let d = ((dx * dx + dy * dy).sqrt() - r).abs();
            let cover = (stroke / 2.0 - d + 0.5).clamp(0.0, 1.0);
            if cover > 0.0 {
                let a = (cover * 255.0).round() as u32;
                img.fill_argb(x, y, 1, 1, (a << 24) | (argb & 0xFFFFFF));
            }
        }
    }
}

/// One run of a wrapped line that belongs to a web address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlRun {
    pub line: usize,
    /// First and past-last character on that line.
    pub from: usize,
    pub to: usize,
    /// The whole address, which may go on across lines.
    pub url: String,
}

/// Body text with web addresses in it, in the link color and tappable where they stand, as
/// Linkify made them (without its underline). Each address on each line is its own button to
/// screen readers. `fill_h` is the least height, the text centered in it (about.xml).
pub fn linked_text(m: &Metrics, width: i32, s: &str, fill_h: i32) -> W {
    let kit = kit();
    let block = Block::new(&kit.body, Face::Body, m.t, s, width, Align::Start);
    let h = block.height.max(fill_h);
    let top = (h - block.height) / 2;
    let runs = url_runs(&block.lines);
    let drawn = block.clone();
    let draw_runs = runs.clone();
    let font_kit = kit.clone();
    let piece = sprite(
        m,
        width,
        h,
        move |img, _| {
            let body = &font_kit.body;
            let p = palette();
            for (i, line) in drawn.lines.iter().enumerate() {
                let y = top + i as i32 * drawn.line_pitch();
                let chars: Vec<char> = line.chars().collect();
                let (mut x, mut at) = (0, 0);
                let mut put = |text: &[char], color: u32, x: &mut i32| {
                    let text: String = text.iter().collect();
                    super::text::draw_line(body, img, &text, *x, y, drawn.mult, color);
                    *x += body.width(&text) * drawn.mult;
                };
                for run in draw_runs.iter().filter(|r| r.line == i) {
                    put(&chars[at..run.from], p.ink, &mut x);
                    put(&chars[run.from..run.to], p.link, &mut x);
                    at = run.to;
                }
                put(&chars[at..], p.ink, &mut x);
            }
        },
        Look::default,
    );
    let label = s.to_string();
    let text = piece.a11y(move |a| a.label(label));
    let mut layers = vec![W::framed(m, text, width, h).piece];
    for (n, run) in runs.into_iter().enumerate() {
        let chars: Vec<char> = block.lines[run.line].chars().collect();
        let before: String = chars[..run.from].iter().collect();
        let link: String = chars[run.from..run.to].iter().collect();
        let x = kit.body.width(&before) * m.t;
        let w = kit.body.width(&link) * m.t;
        let y = top + run.line as i32 * block.line_pitch();
        let target = run.url.clone();
        let hit = canvas(|_, _| {})
            .on_tap(move || open_url(&target))
            .a11y(move |a| a.role(Role::Button).label(run.url))
            .id(day::id!(text_link, n.to_string()))
            .frame(m.pt(w), m.pt(block.line_pitch()))
            .padding(Insets::new(m.pt(y), m.pt(x), 0.0, 0.0));
        layers.push(hit.any());
    }
    let stack = zstack(layers)
        .align(Alignment::TopLeading)
        .frame(m.pt(width), m.pt(h));
    W::new(stack, width, h)
}

/// The runs of wrapped lines that are web addresses. An address starts "https://" and runs to a
/// space, going on to the next line when it was broken at the line's end, less any closing
/// punctuation.
pub fn url_runs(lines: &[String]) -> Vec<UrlRun> {
    let mut out = Vec::new();
    let (mut line, mut col) = (0, 0);
    while line < lines.len() {
        let chars: Vec<char> = lines[line].chars().collect();
        let text: String = chars[col.min(chars.len())..].iter().collect();
        let Some(byte) = text.find("https://") else {
            line += 1;
            col = 0;
            continue;
        };
        let start = col + text[..byte].chars().count();
        let mut parts: Vec<(usize, usize, usize)> = Vec::new();
        let mut url = String::new();
        let (mut l, mut c) = (line, start);
        let (end_line, end_col) = loop {
            let cs: Vec<char> = lines[l].chars().collect();
            let mut e = c;
            while e < cs.len() && !cs[e].is_whitespace() {
                e += 1;
            }
            parts.push((l, c, e));
            url.extend(&cs[c..e]);
            match lines.get(l + 1) {
                Some(next) if e == cs.len() && url_continues(&url, next) => {
                    l += 1;
                    c = 0;
                }
                _ => break (l, e),
            }
        };
        // Closing punctuation belongs to the sentence
        let mut cut = 0;
        while url.ends_with([')', ',', '.', ';', ':']) {
            url.pop();
            cut += 1;
        }
        while cut > 0 {
            let Some(last) = parts.last_mut() else { break };
            let trim = cut.min(last.2 - last.1);
            last.2 -= trim;
            cut -= trim;
            if last.1 == last.2 {
                parts.pop();
            }
        }
        for (line, from, to) in parts {
            out.push(UrlRun {
                line,
                from,
                to,
                url: url.clone(),
            });
        }
        line = end_line;
        col = end_col;
    }
    out
}

/// Whether the line after a line that ended in an address goes on with it: a break inside an
/// address falls where `text::pieces` allows one, after "//" or before its punctuation.
fn url_continues(url: &str, next: &str) -> bool {
    url.ends_with("//") || next.starts_with(['/', '.', '-', '_', '~', '?', '#', '%', ',', '=', '&'])
}

/// One column of a `cells` row: its text, where it starts and how wide it is in art pixels, how it
/// sits there, its color and its font.
pub type Column = (String, i32, i32, Align, Tone, Face);

/// A line of text in columns, such as a score's rank, points and date, read by screen readers as
/// one element, `label`.
pub fn cells(m: &Metrics, id: Option<ElementId>, width: i32, cells: Vec<Column>, label: &str) -> W {
    let kit = kit();
    let t = m.t;
    let blocks: Vec<(Block, i32, Tone)> = cells
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
                block.draw(font, img, *x, y, color.argb());
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
