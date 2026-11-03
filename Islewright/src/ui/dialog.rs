// SPDX-License-Identifier: GPL-3.0-or-later
//! Dialogs as pixel-art overlays over the game: a dimmed backdrop that takes the taps the game
//! would otherwise get, and a parchment panel with a title, the message and brass buttons (the
//! port of PixelDialog). `ask` presents one and resolves to the value of the button pressed, or
//! `None` when it was dismissed with Escape, the backdrop or a cancel button, so call sites read
//! like the native `Alert` code they replace.
//!
//! One dialog shows at a time; any asked for meanwhile wait their turn. When one opens, its title
//! and message are announced to screen readers and keyboard focus moves to its default button.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use day::prelude::*;

use super::metrics::{Metrics, metrics};
use super::paint::{self, Icon, kit};
use super::text::{Align, Block, Face};
use super::widget::{self, Button, Style, W};

/// Art pixels inside the panel's frame, between buttons, and a dialog's widest at default text.
const PAD: i32 = 8;
const GAP: i32 = 4;
const MAX_W: i32 = 216;
/// A button's width in a row, as PixelDialog.BUTTON_ART_W.
const ROW_BUTTON_W: i32 = 90;
/// The space round a dialog that leaves the game showing at its edges.
const MARGIN: i32 = 8;

struct Choice<T> {
    key: String,
    label: String,
    value: Option<T>,
    icon: Option<Icon>,
}

/// What builds a dialog's own content for the width it is given.
pub type Content = Rc<dyn Fn(&Metrics, i32) -> W>;

/// A dialog to `ask`: a title, a message or content of its own, and buttons that each answer
/// with a value.
pub struct Dialog<T> {
    title: String,
    paragraphs: Vec<String>,
    content: Option<Content>,
    choices: Vec<Choice<T>>,
    stacked: bool,
    focus: Option<String>,
    dismissable: bool,
}

impl<T: 'static> Dialog<T> {
    pub fn new(title: impl Into<String>) -> Dialog<T> {
        Dialog {
            title: title.into(),
            paragraphs: Vec::new(),
            content: None,
            choices: Vec::new(),
            stacked: false,
            focus: None,
            dismissable: true,
        }
    }

    /// The message. Blank lines split it into paragraphs.
    pub fn message(mut self, text: impl Into<String>) -> Dialog<T> {
        let text = text.into();
        self.paragraphs.extend(
            text.split("\n\n")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string),
        );
        self
    }

    /// Content of the dialog's own in place of a message: PixelDialog.custom.
    pub fn content(mut self, build: impl Fn(&Metrics, i32) -> W + 'static) -> Dialog<T> {
        self.content = Some(Rc::new(build));
        self
    }

    /// A button that answers `value`. `key` names it for scripts: its element id is
    /// `dialog_button:<key>`.
    pub fn button(mut self, key: &str, label: impl Into<String>, value: T) -> Dialog<T> {
        self.choices.push(Choice {
            key: key.to_string(),
            label: label.into(),
            value: Some(value),
            icon: None,
        });
        self
    }

    /// A button with one of the game's icons before its label.
    pub fn icon_button(
        mut self,
        key: &str,
        label: impl Into<String>,
        icon: Icon,
        value: T,
    ) -> Dialog<T> {
        self.choices.push(Choice {
            key: key.to_string(),
            label: label.into(),
            value: Some(value),
            icon: Some(icon),
        });
        self
    }

    /// A button that answers `None`, as Escape and a tap outside the panel do.
    pub fn cancel(mut self, label: impl Into<String>) -> Dialog<T> {
        self.choices.push(Choice {
            key: "cancel".to_string(),
            label: label.into(),
            value: None,
            icon: None,
        });
        self
    }

    /// Only the first button answers with its value; any other is the same as dismissing it, as
    /// PixelDialog.custom's Cancel is.
    pub fn ok_only(mut self) -> Dialog<T> {
        for c in self.choices.iter_mut().skip(1) {
            c.value = None;
        }
        self
    }

    /// The buttons one above the other, full width, whether or not they would fit in a row.
    pub fn stacked(mut self) -> Dialog<T> {
        self.stacked = true;
        self
    }

    /// The button that takes focus when the dialog opens (the first one otherwise).
    pub fn default_button(mut self, key: &str) -> Dialog<T> {
        self.focus = Some(key.to_string());
        self
    }

    /// Only a button closes it: no Escape, no tap outside.
    pub fn modal(mut self) -> Dialog<T> {
        self.dismissable = false;
        self
    }
}

/// What answers a dialog: called with the index of the button pressed, or `None`.
type Resolve = Box<dyn FnOnce(Option<usize>)>;

/// A dialog with its values taken out, waiting in the queue or on screen.
struct Open {
    id: u64,
    title: String,
    paragraphs: Vec<String>,
    content: Option<Content>,
    buttons: Vec<(String, String, Option<Icon>)>,
    stacked: bool,
    focus: usize,
    dismissable: bool,
    /// Called once, with the index of the button pressed or `None`.
    resolve: RefCell<Option<Resolve>>,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<Rc<Open>>> = const { RefCell::new(VecDeque::new()) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static SHOWING: Cell<Option<Signal<u64>>> = const { Cell::new(None) };
    /// The one dialog opened over the dialog on screen, as the color picker opens over Settings.
    static OVER: RefCell<Option<Rc<Open>>> = const { RefCell::new(None) };
    static OVER_SHOWING: Cell<Option<Signal<u64>>> = const { Cell::new(None) };
    static ANNOUNCED: Cell<u64> = const { Cell::new(0) };
}

/// The id of the dialog on screen, 0 for none.
fn showing() -> Signal<u64> {
    SHOWING.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global(0);
            c.set(Some(s));
            s
        }
    })
}

/// The id of the dialog over the dialog on screen, 0 for none.
fn over_showing() -> Signal<u64> {
    OVER_SHOWING.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global(0);
            c.set(Some(s));
            s
        }
    })
}

/// Forgets every dialog: what waited on their answers went with the tree they were asked from.
pub fn reset() {
    QUEUE.with(|q| q.borrow_mut().clear());
    OVER.with(|o| *o.borrow_mut() = None);
    over_showing().set(0);
    showing().set(0);
    ANNOUNCED.with(|a| a.set(0));
}

/// Whether a dialog is on screen. Tracked.
pub fn is_open() -> bool {
    showing().get() != 0
}

/// The answer a dialog will give.
struct Shared<T> {
    value: Option<Option<T>>,
    waker: Option<Waker>,
}

/// The future `ask` returns.
pub struct Answer<T> {
    shared: Rc<RefCell<Shared<T>>>,
}

impl<T> Future for Answer<T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut s = self.shared.borrow_mut();
        match s.value.take() {
            Some(v) => Poll::Ready(v),
            None => {
                s.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Takes the dialog's values out, leaving what is shown.
fn opened<T: 'static>(dialog: Dialog<T>) -> (Rc<Open>, Answer<T>) {
    let shared = Rc::new(RefCell::new(Shared {
        value: None,
        waker: None,
    }));
    let focus = dialog
        .focus
        .as_ref()
        .and_then(|k| dialog.choices.iter().position(|c| &c.key == k))
        .unwrap_or(0);
    let mut values: Vec<Option<T>> = Vec::new();
    let mut buttons = Vec::new();
    for c in dialog.choices {
        buttons.push((c.key, c.label, c.icon));
        values.push(c.value);
    }
    let answer = shared.clone();
    let id = NEXT_ID.with(|n| n.replace(n.get() + 1));
    let open = Rc::new(Open {
        id,
        title: dialog.title,
        paragraphs: dialog.paragraphs,
        content: dialog.content,
        buttons,
        stacked: dialog.stacked,
        focus,
        dismissable: dialog.dismissable,
        resolve: RefCell::new(Some(Box::new(move |i: Option<usize>| {
            let v = i.and_then(|i| values.get_mut(i).and_then(Option::take));
            let waker = {
                let mut s = answer.borrow_mut();
                s.value = Some(v);
                s.waker.take()
            };
            if let Some(w) = waker {
                w.wake();
            }
        }))),
    });
    (open, Answer { shared })
}

/// Presents `dialog` (after any already showing) and resolves to the value of the button
/// pressed, or `None` if it was dismissed.
pub fn ask<T: 'static>(dialog: Dialog<T>) -> Answer<T> {
    let (open, answer) = opened(dialog);
    let id = open.id;
    let first = QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        q.push_back(open);
        q.len() == 1
    });
    if first {
        showing().set(id);
    }
    answer
}

/// As `ask`, for a dialog opened over the one on screen, which stays open beneath it.
pub fn ask_over<T: 'static>(dialog: Dialog<T>) -> Answer<T> {
    let (open, answer) = opened(dialog);
    let id = open.id;
    OVER.with(|o| *o.borrow_mut() = Some(open));
    over_showing().set(id);
    answer
}

/// Closes the dialog on top, as a tap outside it does.
pub fn dismiss_top() {
    let over = over_showing().get_untracked();
    let id = if over != 0 {
        over
    } else {
        showing().get_untracked()
    };
    if id != 0 {
        close(id, None);
    }
}

/// A question with Yes and No, true for Yes.
pub async fn confirm(
    title: impl Into<String>,
    text: impl Into<String>,
    yes: impl Into<String>,
    no: impl Into<String>,
) -> bool {
    ask(Dialog::new(title)
        .message(text)
        .button("yes", yes, true)
        .cancel(no))
    .await
    .unwrap_or(false)
}

/// Closes the dialog on screen with an answer and shows the next one waiting.
fn close(id: u64, answer: Option<usize>) {
    let over = OVER.with(|o| {
        let mut o = o.borrow_mut();
        match o.as_ref() {
            Some(open) if open.id == id => o.take(),
            _ => None,
        }
    });
    if let Some(open) = over {
        over_showing().set(0);
        let resolve = open.resolve.borrow_mut().take();
        if let Some(r) = resolve {
            r(answer);
        }
        return;
    }
    let open = QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        match q.front() {
            Some(o) if o.id == id => q.pop_front(),
            _ => None,
        }
    });
    let Some(open) = open else {
        return;
    };
    let next = QUEUE.with(|q| q.borrow().front().map_or(0, |o| o.id));
    showing().set(next);
    let resolve = open.resolve.borrow_mut().take();
    if let Some(r) = resolve {
        r(answer);
    }
}

// ---------------------------------------------------------------- layout

/// How a dialog's buttons are arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    /// Side by side, centered.
    Row,
    /// Full width, one above the other, in this many columns.
    Columns(i32),
}

/// How wide a dialog's panel is and how its buttons are arranged, in art pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub panel_w: i32,
    /// The tallest the panel may be.
    pub max_h: i32,
    /// The width of each button, in order.
    pub button_w: Vec<i32>,
    pub arrange: Arrange,
}

impl Plan {
    /// The panel width for the window, and the buttons' arrangement for their natural widths.
    pub fn new(m: &Metrics, natural: &[i32], stacked: bool) -> Plan {
        Plan::with_width(m, natural, stacked, Plan::widths(m)[0])
    }

    /// The panel widths to try, narrowest first: the design width (the whole window for large
    /// text), then wider ones a short, wide window has room for, so a dialog that would scroll
    /// in a short, wide window takes the width instead.
    pub fn widths(m: &Metrics) -> Vec<i32> {
        let avail_w = m.width - m.safe.left - m.safe.right - 2 * MARGIN;
        // Even, so a centered button lands on a whole art pixel (PixelUi.dialogWidth)
        let even = |w: i32| (w.min(avail_w).max(64)) & !1;
        let first = even(if m.t > 1 { avail_w } else { MAX_W });
        let mut out = vec![first];
        for w in [280, 360, avail_w] {
            let w = even(w);
            if w > *out.last().unwrap_or(&first) {
                out.push(w);
            }
        }
        out
    }

    /// The plan for a panel `panel_w` art pixels wide. At the design width, as PixelDialog had
    /// it: a stacked dialog's buttons are full width, one above the other; otherwise buttons in a
    /// row are one width when every label fits that, and if one does not, each is as wide as its
    /// label and the last takes the rest of the row. On a panel wider than that, up to three
    /// buttons share a row, and more go in two columns.
    pub fn with_width(m: &Metrics, natural: &[i32], stacked: bool, panel_w: i32) -> Plan {
        let inner = panel_w - 2 * PAD;
        let n = natural.len().max(1) as i32;
        let wide = panel_w > MAX_W;
        let max_h = m.height - m.safe.top - m.safe.bottom - 2 * MARGIN;
        let plan = |button_w: Vec<i32>, arrange| Plan {
            panel_w,
            max_h,
            button_w,
            arrange,
        };
        if !wide {
            if stacked {
                return plan(vec![inner; natural.len()], Arrange::Columns(1));
            }
            let each = if n == 1 {
                ROW_BUTTON_W
            } else {
                ROW_BUTTON_W.min((inner - GAP) / 2)
            };
            if natural.iter().all(|&w| w <= each) {
                return plan(vec![each; natural.len()], Arrange::Row);
            }
            let mut widths = natural.to_vec();
            if let Some(last) = widths.last_mut() {
                let used: i32 = natural[..natural.len() - 1].iter().sum::<i32>() + (n - 1) * GAP;
                *last = inner - used;
            }
            if widths.iter().all(|&w| w >= 16) {
                return plan(widths, Arrange::Row);
            }
            return plan(vec![inner; natural.len()], Arrange::Columns(1));
        }
        if n <= 3 {
            let each = inner.min((inner - (n - 1) * GAP) / n);
            if natural.iter().all(|&w| w <= each) {
                return plan(vec![each; natural.len()], Arrange::Row);
            }
            // Each as wide as its label, the room left shared out
            let total: i32 = natural.iter().sum::<i32>() + (n - 1) * GAP;
            if total <= inner {
                let extra = (inner - total) / n;
                return plan(natural.iter().map(|w| w + extra).collect(), Arrange::Row);
            }
        }
        let half = (inner - GAP) / 2;
        let cols = if n > 3 && natural.iter().all(|&w| w <= half) {
            2
        } else {
            1
        };
        let each = (inner - (cols - 1) * GAP) / cols;
        plan(vec![each; natural.len()], Arrange::Columns(cols))
    }

    /// The height of the buttons arranged by this plan, given each one's height.
    pub fn buttons_h(&self, heights: &[i32]) -> i32 {
        match self.arrange {
            Arrange::Row => heights.iter().copied().max().unwrap_or(0),
            Arrange::Columns(cols) => {
                let rows: Vec<i32> = heights
                    .chunks(cols.max(1) as usize)
                    .map(|row| row.iter().copied().max().unwrap_or(0))
                    .collect();
                rows.iter().sum::<i32>() + (rows.len() as i32 - 1).max(0) * GAP
            }
        }
    }

    /// The panel's height for this plan, worked out from the text without building anything.
    fn height(&self, m: &Metrics, open: &Open) -> i32 {
        let kit = kit();
        let inner = self.inner_w();
        let block = |face: Face, s: &str, align: Align| {
            Block::new(kit.font(face), face, m.t, s, inner, align).height
        };
        let head = block(Face::Display, &open.title, Align::Center) + 1 + 6;
        let body: i32 = open
            .paragraphs
            .iter()
            .map(|p| block(Face::Body, p, Align::Start) + 6)
            .sum();
        let body = if body > 0 { body - 6 + 8 } else { 0 };
        let heights: Vec<i32> = open
            .buttons
            .iter()
            .zip(&self.button_w)
            .map(|((_, label, icon), &w)| {
                let b = Button::new(day::id!(dialog_button, "measure"), label.clone()).width(w);
                match icon {
                    Some(i) => b.icon(*i),
                    None => b,
                }
                .height(m)
            })
            .collect();
        2 * PAD + head + body + self.buttons_h(&heights)
    }

    pub fn inner_w(&self) -> i32 {
        self.panel_w - 2 * PAD
    }
}

/// The dialog layer, over the game: the dialog showing and any opened over it, or nothing.
pub fn layer() -> impl Piece {
    // The dim reaches under the navigation bar and the system's back comes to the dialog, as
    // with a dialog window of its own
    Effect::new(move || {
        super::window::dim(showing().get() != 0, || {
            super::widget::escape();
        })
    });
    zstack((
        keyed(
            move || {
                let id = showing().get();
                (id != 0).then(|| (id, metrics(), day::locale().get()))
            },
            |(id, m, _)| view(*id, m),
        ),
        keyed(
            move || {
                let id = over_showing().get();
                (id != 0).then(|| (id, metrics(), day::locale().get()))
            },
            |(id, m, _)| view(*id, m),
        ),
    ))
    .align(Alignment::TopLeading)
}

fn view(id: u64, m: &Metrics) -> AnyPiece {
    let over = OVER.with(|o| o.borrow().clone()).filter(|o| o.id == id);
    let Some(open) =
        over.or_else(|| QUEUE.with(|q| q.borrow().iter().find(|o| o.id == id).cloned()))
    else {
        return spacer().any();
    };
    if ANNOUNCED.with(|a| a.replace(id)) != id {
        let mut said = open.title.clone();
        for p in &open.paragraphs {
            said.push_str(". ");
            said.push_str(p);
        }
        announce(&said);
    }
    if open.dismissable {
        widget::on_escape(move || close(id, None));
    }

    let natural: Vec<i32> = open
        .buttons
        .iter()
        .map(|(_, label, icon)| {
            let b = Button::new(day::id!(dialog_button, "measure"), label.clone());
            let b = match icon {
                Some(i) => b.icon(*i),
                None => b,
            };
            b.natural_width(m)
        })
        .collect();
    // The narrowest width at which it all fits, else the widest there is
    let plans: Vec<Plan> = Plan::widths(m)
        .into_iter()
        .map(|w| Plan::with_width(m, &natural, open.stacked, w))
        .collect();
    let plan = plans
        .iter()
        .find(|p| p.height(m, &open) <= p.max_h)
        .or(plans.last())
        .cloned()
        .unwrap_or_else(|| Plan::new(m, &natural, open.stacked));
    let inner = plan.inner_w();

    let title = widget::text(
        m,
        Some(day::id!(dialog_title)),
        inner,
        &open.title,
        Style::TITLE,
    );
    let head_h = title.h + 1 + 6;
    let head = widget::vstack(
        m,
        vec![title, widget::rule(m, inner), widget::gap(m, inner, 6)],
        0,
    );

    let body = match &open.content {
        Some(build) => vec![build(m, inner)],
        None => open
            .paragraphs
            .iter()
            .map(|p| widget::text(m, None, inner, p, Style::BODY))
            .collect::<Vec<W>>(),
    };
    let has_body = !body.is_empty();
    let body = widget::vstack(m, body, 6);

    let buttons: Vec<W> = open
        .buttons
        .iter()
        .enumerate()
        .map(|(i, (key, label, icon))| {
            let b = Button::new(day::id!(dialog_button, key.as_str()), label.clone())
                .width(plan.button_w[i])
                .autofocus(i == open.focus);
            let b = match icon {
                Some(ic) => b.icon(*ic),
                None => b,
            };
            b.build(m, move || close(id, Some(i)))
        })
        .collect();
    let buttons = match plan.arrange {
        Arrange::Row => {
            let row = widget::hstack(m, buttons, GAP);
            // Whole art pixels from the left, as PixelDialog's row margin
            widget::inset(
                m,
                row,
                (inner
                    - plan.button_w.iter().sum::<i32>()
                    - (plan.button_w.len() as i32 - 1) * GAP)
                    / 2,
            )
        }
        Arrange::Columns(cols) => {
            let mut rows = Vec::new();
            let mut it = buttons.into_iter().peekable();
            while it.peek().is_some() {
                let row: Vec<W> = it.by_ref().take(cols.max(1) as usize).collect();
                rows.push(widget::hstack(m, row, GAP));
            }
            widget::vstack(m, rows, GAP)
        }
    };

    // What does not fit the window scrolls: the content alone while the buttons leave it three
    // lines, else the content and the buttons together under the title.
    let pitch = kit().body.height() * m.t;
    let below = if has_body { 8 } else { 0 };
    let top = 2 * PAD + head_h;
    let mut parts = vec![head];
    if top + body.h + below + buttons.h <= plan.max_h {
        if has_body {
            parts.push(body);
            parts.push(widget::gap(m, inner, below));
        }
        parts.push(buttons);
    } else if plan.max_h - top - below - buttons.h >= 3 * pitch {
        let room = plan.max_h - top - below - buttons.h;
        parts.push(super::page::scroller(m, body, inner, room, true));
        parts.push(widget::gap(m, inner, below));
        parts.push(buttons);
    } else {
        let mut rest = Vec::new();
        if has_body {
            rest.push(body);
            rest.push(widget::gap(m, inner, below));
        }
        rest.push(buttons);
        let rest = widget::vstack(m, rest, 0);
        parts.push(super::page::scroller(
            m,
            rest,
            inner,
            plan.max_h - top,
            true,
        ));
    }
    let content = widget::vstack(m, parts, 0);
    let (pw, ph) = (plan.panel_w, content.h + 2 * PAD);

    let k = kit();
    let panel = W::new(
        widget::art(m, pw, ph, move |img| {
            paint::frame(img, k.panel_art(), 0, 0, pw, ph)
        }),
        pw,
        ph,
    );
    let avail_w = m.width - m.safe.left - m.safe.right;
    let avail_h = m.height - m.safe.top - m.safe.bottom;
    let x = m.safe.left + (avail_w - pw) / 2;
    let y = m.safe.top + ((avail_h - ph) / 2).max(0);
    let dismissable = open.dismissable;
    let backdrop = canvas(|d, size| {
        d.fill(
            Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
            Color::rgba(0.0, 0.0, 0.0, 0.6),
        );
    })
    .on_tap(move || {
        if dismissable {
            close(id, None);
        }
    })
    .a11y(|a| a.hidden())
    .grow();
    // An overlay, so the panel never sizes the window: just after the screen turns it still has
    // the old window's height, and the window probe would hand that height straight back
    backdrop
        .overlay_aligned(
            Alignment::TopLeading,
            zstack((panel.piece, content.piece.padding(m.pt(PAD))))
                .align(Alignment::TopLeading)
                .padding(Insets::new(m.pt(y), m.pt(x), 0.0, 0.0)),
        )
        .id(day::id!(dialog))
        .grow()
        .any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone() -> Metrics {
        Metrics::new(412.0, 892.0, 2.625, Insets::ZERO, 1.0)
    }

    #[test]
    fn a_dialog_is_the_design_width_on_a_phone() {
        let p = Plan::new(&phone(), &[40, 40], false);
        assert_eq!(p.panel_w, 200);
        assert_eq!(p.inner_w(), 184);
        assert_eq!(p.arrange, Arrange::Row);
        // Two buttons at PixelDialog's width
        assert_eq!(p.button_w, vec![ROW_BUTTON_W; 2]);
        assert_eq!(p.panel_w % 2, 0);
    }

    #[test]
    fn a_long_label_keeps_its_width_and_the_last_takes_the_rest() {
        let p = Plan::new(&phone(), &[104, 48], false);
        assert_eq!(p.arrange, Arrange::Row);
        assert_eq!(p.button_w, vec![104, 184 - 104 - GAP]);
        let s = Plan::new(&phone(), &[40, 40], true);
        assert_eq!(s.arrange, Arrange::Columns(1));
    }

    #[test]
    fn a_short_wide_window_offers_wider_panels_and_spreads_the_buttons() {
        let wide = Metrics::new(892.0, 412.0, 2.625, Insets::ZERO, 1.0);
        let widths = Plan::widths(&wide);
        assert_eq!(widths[0], MAX_W);
        assert!(widths.windows(2).all(|w| w[0] < w[1]));
        let last = *widths.last().unwrap();
        // Three stacked labels of different lengths sit in one row, each as wide as it needs
        let p = Plan::with_width(&wide, &[110, 190, 50], true, last);
        assert_eq!(p.arrange, Arrange::Row);
        assert!(p.button_w.iter().zip([110, 190, 50]).all(|(w, n)| *w >= n));
        assert!(p.button_w.iter().sum::<i32>() + 2 * GAP <= p.inner_w());
        // Seven short ones go in two columns
        let menu = Plan::with_width(&wide, &[100; 7], true, last);
        assert_eq!(menu.arrange, Arrange::Columns(2));
        assert_eq!(menu.buttons_h(&[22; 7]), 4 * 22 + 3 * GAP);
    }

    #[test]
    fn large_text_takes_the_window_width() {
        let big = Metrics::new(412.0, 892.0, 2.625, Insets::ZERO, 2.0);
        let p = Plan::new(&big, &[40], false);
        assert_eq!(p.panel_w, (big.width - 2 * MARGIN) & !1);
    }

    #[test]
    fn answers_arrive_in_order() {
        let a = ask(Dialog::new("One").button("a", "A", 1).button("b", "B", 2));
        let b = ask(Dialog::<i32>::new("Two").cancel("Cancel"));
        let first = QUEUE.with(|q| q.borrow().front().map(|o| o.id)).unwrap();
        close(first, Some(1));
        assert_eq!(a.shared.borrow_mut().value.take(), Some(Some(2)));
        let second = QUEUE.with(|q| q.borrow().front().map(|o| o.id)).unwrap();
        assert_ne!(first, second);
        close(second, Some(0));
        // The cancel button answers nothing
        assert_eq!(b.shared.borrow_mut().value.take(), Some(None));
        assert!(QUEUE.with(|q| q.borrow().is_empty()));
    }
}
