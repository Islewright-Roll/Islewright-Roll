// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TutorialLayoutTest: wrapping, and the placement of the panel and its pointer
//! for each of the steps' targets on both canvas sizes the game meets (216 x 480 and 240 x 426
//! art pixels). The targets are laid out here from the old game view's layout constants.

use dayapp::rules::tutorial_layout::{self, Canvas as Area, Placement, Rect, Side};

/// Five pixels a character, like the body font's average.
fn five(text: &str) -> i32 {
    text.chars().count() as i32 * 5
}

const TEXTS: [&str; 10] = [
    "Roll the dice to start your turn.",
    "Tap on dice to save them, then roll the rest. You get three rolls each turn.",
    "This is what each thing costs. Match your dice to it.",
    "Highlighted spots can be built with your dice. Tap one.",
    "Changed your mind? This takes back your last build, until your next roll.",
    "When you are done building, tap here to end the turn.",
    "The Crown's ship marks your turns. Score as much as you can before it reaches the pier.",
    "Two gold stand in for any one resource. A single gold can't be used on its own.",
    "Your trader banked this for you. It is spent by itself when a build needs it, on any turn.",
    "Das Schiff des Koenigs zaehlt deine Zuege. Erreiche 10 Punkte, bevor es den Anleger erreicht.",
];

// The old game view's layout, in art pixels
const DESIGN_W: i32 = 216;
const TOP_H: i32 = 64;
const BOTTOM_H: i32 = 66;
const DICE_GAP: i32 = 6;
const DIE: i32 = 28;
const ROLL_W: i32 = 96;
const ROLL_H: i32 = 20;
const UNDO_SIZE: i32 = 16;
const HUD_INSET: i32 = 6;
const BOX_GAP: i32 = 3;
const SCORE_W: i32 = 40;
const COSTS_W: i32 = 135;
const BOX_Y: i32 = 6;
const BOX_H: i32 = 34;
const STRIP_Y: i32 = 46;
const TRACK_H: i32 = 13;
const MINI_DIE: i32 = 16;
const MINI_GAP: i32 = 3;
const MINI_PER_ROW: i32 = 6;
const MINI_PAD: i32 = 3;
const MINI_MARGIN: i32 = 2;

/// One canvas: its size and the system bars' padding, with the game view's layout of the targets.
struct Canvas {
    w: i32,
    h: i32,
    pad_top: i32,
    pad_bottom: i32,
    hud_x: i32,
    dice_x: i32,
    dice_y: i32,
    roll_y: i32,
}

impl Canvas {
    fn new(w: i32, h: i32, pad_top: i32, pad_bottom: i32) -> Canvas {
        let dice_row = 6 * DIE + 5 * DICE_GAP;
        let dice_y = h - pad_bottom - BOTTOM_H + 8;
        Canvas {
            w,
            h,
            pad_top,
            pad_bottom,
            hud_x: (w - DESIGN_W) / 2,
            dice_x: (w - dice_row) / 2,
            dice_y,
            roll_y: dice_y + DIE + 6,
        }
    }

    fn roll(&self) -> Rect {
        Rect::new((self.w - ROLL_W) / 2, self.roll_y, ROLL_W, ROLL_H)
    }

    /// The roll button grown for a long label.
    fn wide_roll(&self, width: i32) -> Rect {
        Rect::new((self.w - width) / 2, self.roll_y, width, ROLL_H)
    }

    fn dice(&self) -> Rect {
        Rect::new(self.dice_x, self.dice_y, 6 * DIE + 5 * DICE_GAP, DIE)
    }

    fn die(&self, i: i32) -> Rect {
        Rect::new(self.dice_x + i * (DIE + DICE_GAP), self.dice_y, DIE, DIE)
    }

    fn costs(&self) -> Rect {
        Rect::new(
            self.hud_x + HUD_INSET + SCORE_W + BOX_GAP,
            self.pad_top + BOX_Y,
            COSTS_W,
            BOX_H,
        )
    }

    fn undo(&self) -> Rect {
        Rect::new(
            self.hud_x + DESIGN_W - UNDO_SIZE - 2,
            self.roll_y + (ROLL_H - UNDO_SIZE) / 2,
            UNDO_SIZE,
            UNDO_SIZE,
        )
    }

    fn strip(&self) -> Rect {
        Rect::new(
            self.hud_x + HUD_INSET,
            self.pad_top + STRIP_Y,
            DESIGN_W - 2 * HUD_INSET,
            TRACK_H + 2,
        )
    }

    fn mini(&self, count: i32) -> Rect {
        let pitch = MINI_DIE + MINI_GAP;
        let in_row = MINI_PER_ROW.min(count);
        let rows = (count + MINI_PER_ROW - 1) / MINI_PER_ROW;
        let x = self.hud_x + MINI_MARGIN + MINI_PAD;
        let bottom = self.h - self.pad_bottom - BOTTOM_H - MINI_PAD;
        let top = bottom - MINI_DIE - (rows - 1) * pitch;
        Rect::new(x, top, in_row * pitch - MINI_GAP, bottom - top)
    }

    /// A village-sized spot at a board position, board origin about the middle of the free space.
    fn spot(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(
            self.w / 2 + dx - 12,
            (self.pad_top + TOP_H + self.h - BOTTOM_H - self.pad_bottom) / 2 + dy - 15,
            24,
            30,
        )
    }

    fn area(&self) -> Area {
        Area {
            w: self.w,
            top_limit: self.pad_top,
            bottom_limit: self.h - self.pad_bottom,
        }
    }
}

fn canvases() -> [Canvas; 4] {
    [
        Canvas::new(216, 480, 0, 0),
        Canvas::new(216, 480, 6, 10),
        Canvas::new(240, 426, 0, 0),
        Canvas::new(240, 426, 6, 10),
    ]
}

fn place(c: &Canvas, target: Rect, text: &str) -> Option<Placement> {
    tutorial_layout::place(target, text, "Got it", "Skip tutorial", c.area(), &five)
}

/// Every rule a placement keeps, whatever the target.
fn assert_sound(c: &Canvas, target: Rect, text: &str, p: Option<Placement>) {
    let what = format!(
        "target {target} on {}x{} pads {},{} text '{text}'",
        c.w, c.h, c.pad_top, c.pad_bottom
    );
    let p = p.unwrap_or_else(|| panic!("{what}"));
    let panel = p.panel;
    // On the canvas, clear of the system bars' padding
    assert!(panel.x >= tutorial_layout::MARGIN, "{what} panel {panel}");
    assert!(
        panel.x + panel.w <= c.w - tutorial_layout::MARGIN,
        "{what} panel {panel}"
    );
    assert!(
        panel.y >= c.pad_top + tutorial_layout::MARGIN,
        "{what} panel {panel}"
    );
    assert!(
        panel.y + panel.h <= c.h - c.pad_bottom - tutorial_layout::MARGIN,
        "{what} panel {panel}"
    );
    // Never over the target nor its outline
    let outlined = Rect::new(target.x - 1, target.y - 1, target.w + 2, target.h + 2);
    assert!(!panel.intersects(&outlined), "{what} panel {panel}");
    // The pointer's tip is next to the outline, and its base on the panel's edge
    match p.side {
        Side::Above => {
            assert_eq!(target.y - 2, p.tip_y, "{what}");
            assert_eq!(panel.y + panel.h, p.base_y, "{what}");
            assert_eq!(p.base_y + tutorial_layout::POINTER - 1, p.tip_y, "{what}");
            assert!(
                p.tip_x >= target.x && p.tip_x < target.x + target.w,
                "{what}"
            );
            assert!(
                p.base_x - tutorial_layout::HALF_BASE >= panel.x
                    && p.base_x + tutorial_layout::HALF_BASE < panel.x + panel.w,
                "{what}"
            );
        }
        Side::Below => {
            assert_eq!(target.y + target.h + 1, p.tip_y, "{what}");
            assert_eq!(panel.y - 1, p.base_y, "{what}");
            assert_eq!(p.base_y - (tutorial_layout::POINTER - 1), p.tip_y, "{what}");
            assert!(
                p.tip_x >= target.x && p.tip_x < target.x + target.w,
                "{what}"
            );
            assert!(
                p.base_x - tutorial_layout::HALF_BASE >= panel.x
                    && p.base_x + tutorial_layout::HALF_BASE < panel.x + panel.w,
                "{what}"
            );
        }
        Side::Left => {
            assert_eq!(target.x - 2, p.tip_x, "{what}");
            assert_eq!(panel.x + panel.w, p.base_x, "{what}");
            assert!(
                p.tip_y >= target.y && p.tip_y < target.y + target.h,
                "{what}"
            );
        }
        Side::Right => {
            assert_eq!(target.x + target.w + 1, p.tip_x, "{what}");
            assert_eq!(panel.x - 1, p.base_x, "{what}");
            assert!(
                p.tip_y >= target.y && p.tip_y < target.y + target.h,
                "{what}"
            );
        }
    }
    // The text is inside the panel, line by line
    assert!(p.text_x >= panel.x + tutorial_layout::PAD_X, "{what}");
    for line in &p.lines {
        assert!(
            p.text_x + five(line) <= panel.x + panel.w - tutorial_layout::PAD_X,
            "{what} line '{line}'"
        );
    }
    let text_bottom = p.text_y
        + (p.lines.len() as i32 - 1) * tutorial_layout::LINE_PITCH
        + tutorial_layout::LINE_INK;
    // The buttons: hit areas at least 14 tall, inside the panel, below the text, apart
    assert!(p.got_it.h >= 14 && p.skip.h >= 14, "{what}");
    for b in [p.got_it, p.skip] {
        assert!(
            b.x >= panel.x && b.x + b.w <= panel.x + panel.w,
            "{what} button {b}"
        );
        assert!(
            b.y >= text_bottom && b.y + b.h <= panel.y + panel.h,
            "{what} button {b}"
        );
    }
    assert!(!p.got_it.intersects(&p.skip), "{what}");
    assert!(p.got_it.w >= five("Got it"), "{what}");
}

// ---- wrapping -------------------------------------------------------------------------------

#[test]
fn wrap_breaks_on_spaces_within_the_width() {
    assert_eq!(
        vec!["aaaa bbbb", "cccc dddd"],
        tutorial_layout::wrap("aaaa bbbb cccc dddd", 45, &five)
    );
}

#[test]
fn wrap_keeps_a_long_word_whole() {
    assert_eq!(
        vec!["hi", "supercalifragilistic", "ok"],
        tutorial_layout::wrap("hi supercalifragilistic ok", 40, &five)
    );
}

#[test]
fn wrap_of_nothing_is_no_lines() {
    assert!(tutorial_layout::wrap("   ", 40, &five).is_empty());
    assert_eq!(vec!["one"], tutorial_layout::wrap("one", 40, &five));
}

#[test]
fn wrap_never_loses_a_word() {
    for text in TEXTS {
        for width in (60..220).step_by(20) {
            assert_eq!(text, tutorial_layout::wrap(text, width, &five).join(" "));
        }
    }
}

// ---- the steps on each canvas ---------------------------------------------------------------

#[test]
fn every_step_finds_its_place_on_both_canvases() {
    for c in canvases() {
        let targets = [
            c.roll(),
            c.dice(),
            c.costs(),
            c.spot(0, 0),
            c.undo(),
            c.roll(),
            c.strip(),
            c.die(2),
            c.mini(1),
        ];
        for (i, t) in targets.into_iter().enumerate() {
            assert_sound(&c, t, TEXTS[i], place(&c, t, TEXTS[i]));
        }
    }
}

#[test]
fn every_text_fits_every_target_on_both_canvases() {
    for c in canvases() {
        let targets = [
            c.roll(),
            c.wide_roll(130),
            c.dice(),
            c.die(0),
            c.die(5),
            c.costs(),
            c.undo(),
            c.strip(),
            c.mini(1),
            c.mini(6),
            c.mini(7),
            c.spot(0, 0),
            c.spot(-90, -120),
            c.spot(90, 120),
            c.spot(-95, 130),
            c.spot(95, -120),
        ];
        for t in targets {
            for text in TEXTS {
                assert_sound(&c, t, text, place(&c, t, text));
            }
        }
    }
}

#[test]
fn the_panel_sits_above_targets_in_the_lower_half_and_below_the_ones_at_the_top() {
    for c in canvases() {
        assert_eq!(Side::Above, place(&c, c.roll(), TEXTS[0]).unwrap().side);
        assert_eq!(Side::Above, place(&c, c.dice(), TEXTS[1]).unwrap().side);
        assert_eq!(Side::Above, place(&c, c.undo(), TEXTS[4]).unwrap().side);
        assert_eq!(Side::Above, place(&c, c.mini(1), TEXTS[8]).unwrap().side);
        assert_eq!(Side::Above, place(&c, c.die(3), TEXTS[7]).unwrap().side);
        assert_eq!(Side::Below, place(&c, c.costs(), TEXTS[2]).unwrap().side);
        assert_eq!(Side::Below, place(&c, c.strip(), TEXTS[6]).unwrap().side);
    }
}

#[test]
fn a_spot_near_the_top_of_the_island_gets_the_panel_below_and_one_near_the_bottom_above() {
    let c = Canvas::new(216, 480, 0, 0);
    assert_eq!(
        Side::Below,
        place(&c, c.spot(0, -100), TEXTS[3]).unwrap().side
    );
    assert_eq!(
        Side::Above,
        place(&c, c.spot(0, 100), TEXTS[3]).unwrap().side
    );
}

// ---- the 216 x 480 canvas, step by step --------------------------------------------------------

#[test]
fn on_the_narrow_canvas_the_roll_callout_is_one_centred_panel_above_the_button() {
    let c = Canvas::new(216, 480, 0, 0);
    let roll = c.roll();
    assert_eq!(Rect::new(60, 456, 96, 20), roll);
    let p = place(&c, roll, TEXTS[0]).unwrap();
    assert_eq!(Side::Above, p.side);
    // Bottom edge: outline and pointer above the button
    assert_eq!(roll.y - 1 - tutorial_layout::POINTER, p.panel.y + p.panel.h);
    // Centred on the button, pointer straight down its middle
    assert_eq!(roll.x + roll.w / 2, p.base_x);
    assert_eq!(roll.x + roll.w / 2, p.panel.x + p.panel.w / 2);
}

#[test]
fn the_undo_callout_leans_away_from_the_right_edge_but_points_at_the_button() {
    let c = Canvas::new(216, 480, 0, 0);
    let undo = c.undo();
    let p = place(&c, undo, TEXTS[4]).unwrap();
    assert_eq!(Side::Above, p.side);
    assert_eq!(c.w - tutorial_layout::MARGIN, p.panel.x + p.panel.w);
    assert!(p.tip_x >= undo.x && p.tip_x < undo.x + undo.w);
}

#[test]
fn a_panel_never_needs_more_width_than_the_hud() {
    for c in canvases() {
        for text in TEXTS {
            assert!(place(&c, c.roll(), text).unwrap().panel.w <= tutorial_layout::MAX_PANEL_W);
        }
    }
}

#[test]
fn both_buttons_share_a_row_when_they_fit_and_stack_when_they_do_not() {
    let c = Canvas::new(216, 480, 0, 0);
    let whole = Area {
        w: c.w,
        top_limit: 0,
        bottom_limit: c.h,
    };
    let one = tutorial_layout::place(c.roll(), TEXTS[0], "Got it", "Skip tutorial", whole, &five)
        .unwrap();
    assert_eq!(one.got_it.y, one.skip.y);
    assert!(one.got_it.x < one.skip.x);
    let two = tutorial_layout::place(
        c.roll(),
        TEXTS[0],
        "Understood completely",
        "Skip the whole tutorial please",
        whole,
        &five,
    );
    let two_p = two.clone().unwrap();
    assert!(two_p.skip.y >= two_p.got_it.y + two_p.got_it.h);
    assert_eq!(two_p.got_it.x, two_p.skip.x);
    assert_sound(&c, c.roll(), TEXTS[0], two);
}

#[test]
fn a_target_with_no_room_anywhere_gets_no_placement() {
    let c = Canvas::new(216, 480, 0, 0);
    let whole = Area {
        w: c.w,
        top_limit: 0,
        bottom_limit: c.h,
    };
    assert!(
        tutorial_layout::place(
            Rect::new(0, 0, 216, 480),
            TEXTS[0],
            "Got it",
            "Skip",
            whole,
            &five
        )
        .is_none()
    );
}
