// SPDX-License-Identifier: GPL-3.0-or-later
//! How the game shows the turn limit: the King's ship sails toward a pier. The pure numbers
//! of that, apart from any drawing: which turn the ship has reached, where it is on the
//! board, where the dock is, and how the HUD strip lays out its cells.
//!
//! The ship's position is a "cell": 0 is the first turn, turn_cap is the pier. It moves one
//! cell on from the turn in progress as each turn is taken. It is on the board for the last
//! three turns, sailing in on the first and moving up on the second, and is moored at the
//! dock for the whole last turn; the game ending by the turn limit puts a flag up on the dock.
//!
//! A map with no turn cap (Dunholm, which ends only when the target is reached) has no voyage
//! on the board at all. Its HUD strip is a window onto the latest turns instead, see
//! FLAG_WINDOW.

use super::art;
use super::board::{self, VERTEX};
use super::map_def::MapDef;
use super::pt::Pt;

/// The ship is on the island from this many turns before the end, the one being played included.
pub const APPROACH_TURNS: i32 = 3;
/// It sails this many times on the board: from the far stop to the middle one, and on to the dock.
pub const SAIL_STEPS: i32 = 2;

/// Where the ship is on the board. Moored is the last turn, played with the ship at the dock;
/// done is the game over by the turn limit: moored, and the flag is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    None = 0,
    Far = 1,
    Mid = 2,
    Moored = 3,
    Done = 4,
}

impl Stage {
    fn from_index(i: i32) -> Stage {
        match i {
            1 => Stage::Far,
            2 => Stage::Mid,
            3 => Stage::Moored,
            4 => Stage::Done,
            _ => Stage::None,
        }
    }
}

/// The ship sails in from off the screen, and between its stops, in this many milliseconds.
pub const SAIL_IN_MS: i32 = 1100;
pub const SAIL_STEP_MS: i32 = 650;

/// At the far stop this much of the ship's sprite, half of it, lies beyond the left edge of
/// what the screen shows of the board: the ship comes in bow first, the way it heads.
pub const FAR_OUT_X: i32 = 30;
/// The ship sails at least this far between one stop and the next.
pub const MIN_STEP: i32 = 20;
/// However much of the board shows, a stop is never further than this from the next, each way.
pub const MAX_STEP: i32 = 24;
/// The dock is part of the baked board (tools/bake_board.py, the same on every map): a jetty
/// running up the screen from the vertex where the start road meets the sea. These are that
/// vertex, the point the ship's bowsprit comes to, and the top left of dock.png (the dock's
/// above water art alone, which the game draws over the roads), in board pixels; they are
/// dock_foot, dock_mooring and dock_sprite in tools/board_geometry_*.txt.
pub const DOCK_FOOT: Pt = Pt::new(216, 180);
pub const DOCK_MOORING: Pt = Pt::new(214, 152);
pub const DOCK_SPRITE: Pt = Pt::new(210, 134);
/// The board row that has to show under the top bar for the moored ship to be seen whole
/// (its masthead flag is one row below), and the row just under the island's lowest building.
pub const HARBOUR_TOP: i32 = 94;
pub const ISLAND_FOOT: i32 = 366;
/// Rows of sea wanted between the top bar and the harbour row on a screen that has room.
pub const SEA_ABOVE_HARBOUR: i32 = 24;

// ---------------------------------------------------------------- where the ship is

/// The cell the ship has reached. Not over: the turn being played. Over by running out of
/// turns: the pier. Over by reaching a race's target: the turn it was reached in, so the King
/// has not arrived yet. A map with no turn cap has no pier to count toward: the cell is just
/// the turn being played, or the last one taken once it is over.
pub fn cell(turn_cap: i32, turns_taken: i32, done: bool, reached_target: bool) -> i32 {
    if turn_cap == 0 {
        return (if done { turns_taken - 1 } else { turns_taken }).max(0);
    }
    if !done {
        return turns_taken.min(turn_cap - 1).max(0);
    }
    if reached_target {
        return (turns_taken - 1).min(turn_cap - 1).max(0);
    }
    turn_cap
}

/// Where on the board a ship at this cell is: none until three turns remain, then the far
/// stop, the middle one, moored for the turn that is left, and done once the turns have run
/// out. A map with no turn cap never runs out of turns, so its ship never comes onto the board.
pub fn stage(turn_cap: i32, cell: i32) -> Stage {
    if turn_cap == 0 {
        return Stage::None;
    }
    let remaining = turn_cap - cell;
    if remaining <= 0 {
        return Stage::Done;
    }
    if remaining > APPROACH_TURNS {
        return Stage::None;
    }
    // 3 turns remaining is the far stop, 1 the dock
    Stage::from_index(Stage::Done as i32 - remaining)
}

/// The same two steps from a game's own numbers.
pub fn stage_for(map: &MapDef, turns_taken: i32, done: bool, score: i32) -> Stage {
    let reached = map.target_score > 0 && score >= map.target_score;
    stage(map.turn_cap, cell(map.turn_cap, turns_taken, done, reached))
}

/// Board position of the ship sprite's top left for a stage. Moored (and done), its contact
/// pixel is on the dock's mooring point. The two stops before that are evenly spaced on a
/// straight line over open sea from the far stop to the mooring. The far stop is set by what
/// the screen shows: visible_left is the board coordinate of the screen's left edge, and the
/// far stop hangs half off it. Where the screen is wide enough for two good steps the ship
/// sails in level; where it is not, it starts higher up instead, and comes down from under
/// the top bar.
pub fn ship_origin(stage: Stage, mooring: Pt, visible_left: i32) -> Pt {
    let moored_x = mooring.x - art::SHIP_CONTACT_X;
    let moored_y = mooring.y - art::SHIP_CONTACT_Y;
    if stage >= Stage::Moored || stage == Stage::None {
        return Pt::new(moored_x, moored_y);
    }
    let reach = SAIL_STEPS * MAX_STEP;
    let across = moored_x - (moored_x - reach).max(moored_x.min(visible_left - FAR_OUT_X));
    // The whole way is a little over two least steps, so each half still rounds to one
    let way = SAIL_STEPS * MIN_STEP + 2;
    let mut down = 0;
    while down < reach && across * across + down * down < way * way {
        down += 1;
    }
    let steps_out = Stage::Moored as i32 - stage as i32;
    Pt::new(
        moored_x - part(across, steps_out),
        moored_y - part(down, steps_out),
    )
}

/// steps_out halves of a distance, rounded to the nearest pixel.
fn part(distance: i32, steps_out: i32) -> i32 {
    (2 * distance * steps_out + SAIL_STEPS) / (2 * SAIL_STEPS)
}

// ---------------------------------------------------------------- the ship under sail

/// How long the ship takes to sail from one stage to the next, or 0 when it is not a sail and
/// the ship is just there: the stage before the first (from is None), going back (an undo, a
/// new game), skipping a stage, or the flag going up. From none to the far stop it sails in
/// from off the screen, which is further.
pub fn sail_ms(from: Option<Stage>, to: Stage) -> i32 {
    let Some(from) = from else {
        return 0;
    };
    if to as i32 != from as i32 + 1 || to > Stage::Moored {
        return 0;
    }
    if from == Stage::None {
        SAIL_IN_MS
    } else {
        SAIL_STEP_MS
    }
}

/// Where a sail from this stage starts. From a stop, that stop. From none, the ship starts
/// wholly off the left edge of what the screen shows (visible_left, as for ship_origin),
/// level with the far stop: so it comes in on the line it has to follow, bow first.
pub fn sail_start(from: Stage, mooring: Pt, visible_left: i32) -> Pt {
    if from == Stage::None {
        return Pt::new(
            visible_left - art::SHIP_W,
            ship_origin(Stage::Far, mooring, visible_left).y,
        );
    }
    ship_origin(from, mooring, visible_left)
}

/// Where the ship is elapsed_ms into a sail of duration_ms from one place to another: at the
/// start before it, at the end after it, and between them eased in and out (smoothstep),
/// rounded to a whole board pixel so it moves on the art grid.
pub fn sail_at(from: Pt, to: Pt, elapsed_ms: i32, duration_ms: i32) -> Pt {
    if duration_ms <= 0 || elapsed_ms >= duration_ms {
        return to;
    }
    if elapsed_ms <= 0 {
        return from;
    }
    let t = elapsed_ms as f64 / duration_ms as f64;
    let eased = t * t * (3.0 - 2.0 * t);
    Pt::new(
        from.x + java_round((to.x - from.x) as f64 * eased),
        from.y + java_round((to.y - from.y) as f64 * eased),
    )
}

/// Java's Math.round: halves go up, toward positive infinity.
pub(crate) fn java_round(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// Where the board's top left goes on the art canvas. Across, the board is centred. Down, the
/// island's centre goes in the middle of the space between the top bar (ending at bar_bottom)
/// and the bottom bar (starting at tray_top), unless that would put the moored ship's
/// masthead under the top bar: then the board sits lower, as far as it needs to and no
/// further than keeps the island's foot above the bottom bar. Where the screen is tall
/// enough, it also sits low enough to show SEA_ABOVE_HARBOUR rows of sea over the harbour
/// row, so that the ship's far stop is well in view, while the island's foot stays a little
/// above the bottom bar.
pub fn board_origin(art_width: i32, bar_bottom: i32, tray_top: i32) -> Pt {
    let mut y = (bar_bottom + tray_top) / 2 - board::CENTRE_Y;
    let for_sea = (tray_top - 6 - ISLAND_FOOT).min(bar_bottom - (HARBOUR_TOP - SEA_ABOVE_HARBOUR));
    if for_sea > y {
        y = for_sea;
    }
    let for_harbour = bar_bottom - HARBOUR_TOP;
    let for_foot = tray_top - ISLAND_FOOT;
    if y < for_harbour {
        y = y.max(for_harbour.min(for_foot));
    }
    Pt::new(art_width / 2 - board::CENTRE_X, y)
}

// ---------------------------------------------------------------- the dock

/// The end of the start road that touches the sea: the one vertex that belongs to no land
/// hex beyond the two the road runs between. The road has to have exactly one, and it has to
/// be DOCK_FOOT, where the board has its dock; a map whose start road does not meet the sea
/// there is a bug, which the error says.
pub fn coastal_end(hex: &[[i32; 2]], road_ends: &[Pt]) -> Result<Pt, String> {
    let mut found = None;
    for end in road_ends {
        let hexes = hex
            .iter()
            .filter(|centre| {
                VERTEX
                    .iter()
                    .any(|v| centre[0] + v[0] == end.x && centre[1] + v[1] == end.y)
            })
            .count();
        if hexes == 2 {
            if found.is_some() {
                return Err("Both ends of the start road touch the sea".into());
            }
            found = Some(*end);
        }
    }
    found.ok_or_else(|| "Neither end of the start road touches the sea".into())
}

// ---------------------------------------------------------------- the HUD strip

/// The strip as the widest margin lets the island's own turn count fit in 216 px.
pub const STRIP_MARGIN: i32 = 3;
pub const STRIP_LEAD: i32 = 2;
pub const MAX_PITCH: i32 = 14;
/// Where the ship lies once the turns are over, from the pier's left edge: in harbour, its
/// stern behind the pier.
pub const PIER_SHIP_IN: i32 = 4;

/// How many cells the strip of a map with no turn cap has. The same as Tarnholm's fifteen
/// turns, so both islands' strips look alike. There is no last turn to count down to, so the
/// strip is a window onto the latest turns: once it is full the oldest flag falls off the
/// left as each new one goes up.
pub const FLAG_WINDOW: i32 = 15;

/// The first turn a window strip shows, counted from 0: its flag is in cell 0. While the game
/// is on, the ship sails in the cell after the last flag and is never allowed into the last
/// cell, whose bare water lies beside the pier, so the window holds FLAG_WINDOW - 2 flags.
/// The turn that reaches the target puts its flag up in the cell the ship was in, and the
/// ship sails on into harbour, so no flag moves on the win and the cell beside the pier stays
/// bare water.
pub fn window_start(turns_taken: i32, done: bool) -> i32 {
    (turns_taken
        - if done {
            FLAG_WINDOW - 1
        } else {
            FLAG_WINDOW - 2
        })
    .max(0)
}

/// How many flags a window strip shows, in cells 0 onward.
pub fn window_flags(turns_taken: i32, done: bool) -> i32 {
    turns_taken - window_start(turns_taken, done)
}

/// The cell of a window strip the ship is in: the one after the last flag while the game is
/// on, and FLAG_WINDOW, in harbour behind the pier, once the target is reached.
pub fn window_ship_cell(turns_taken: i32, done: bool) -> i32 {
    if done {
        FLAG_WINDOW
    } else {
        window_flags(turns_taken, false)
    }
}

/// One row of sea across the HUD: a cell per turn, then the pier. Two kinds. A strip with
/// flags (one island) puts a flag in the cell of each finished turn, so a cell is at least as
/// wide as the flag's pole and field, and the flags overlap by the width of the pole, the
/// next flag's pole standing where the last one's edge was. A plain strip has only buoys and
/// the ship, centred in cells as narrow as the fit allows.
///
/// All x values are relative to the strip's left edge; the strip itself is centred in the
/// HUD. The water is as wide as it needs to be, ending at the pier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strip {
    pub turns: i32,
    pub flags: bool,
    pub pitch: i32,
    pub pad: i32,
    /// Water width, from the left end to the pier's right edge.
    pub width: i32,
    /// Where the strip's left edge is within a HUD of the given width.
    pub left: i32,
}

impl Strip {
    pub fn new(turns: i32, flags: bool, hud_width: i32) -> Strip {
        let room = hud_width - 2 * STRIP_MARGIN;
        let mut p = MAX_PITCH;
        while p > 1 && Strip::measure(turns, flags, p).width > room {
            p -= 1;
        }
        let m = Strip::measure(turns, flags, p);
        Strip {
            left: (hud_width - m.width) / 2,
            ..m
        }
    }

    /// At a given pitch, before it is centred: the way the fit is measured.
    fn measure(turns: i32, flags: bool, pitch: i32) -> Strip {
        let mut s = Strip {
            turns,
            flags,
            pitch,
            // A plain strip's first ship, centred in its cell, starts STRIP_LEAD in
            pad: if flags {
                STRIP_LEAD
            } else {
                STRIP_LEAD - (pitch - art::TRACK_SHIP_W) / 2
            },
            width: 0,
            left: 0,
        };
        s.width = (s.pier_x() + art::TRACK_PIER_W).max(s.ship_x(turns) + art::TRACK_SHIP_W);
        s
    }

    /// The strip a map shows. Every finished turn gets its flag. A map with no turn cap shows
    /// the latest of them in a window of FLAG_WINDOW cells; a race with a cap has no per-turn
    /// scores to show, so its strip is the narrow kind.
    pub fn for_map(map: &MapDef, hud_width: i32) -> Strip {
        if map.turn_cap == 0 {
            return Strip::new(FLAG_WINDOW, true, hud_width);
        }
        Strip::new(map.turn_cap, map.target_score == 0, hud_width)
    }

    /// Left edge of cell i's flag, or of its buoy's room.
    pub fn cell_x(&self, i: i32) -> i32 {
        self.pad + self.pitch * i
    }

    /// Left edge of a buoy centred in cell i.
    pub fn buoy_x(&self, i: i32) -> i32 {
        self.cell_x(i) + (self.pitch - art::TRACK_BUOY_W) / 2
    }

    /// Whether cell i has a buoy while its turn is still to come. A strip with flags marks
    /// every cell, because each becomes a flag. A plain strip marks every second one, the
    /// even turns (cells 1, 3, 5 and so on), so the last buoy is the last turn before the
    /// pier and none of them moves as the ship advances.
    pub fn marked(&self, i: i32) -> bool {
        self.flags || i % 2 == 1
    }

    /// Left edge of the ship at a cell, 0 to turns. In a strip with flags the ship sits where
    /// a flag will; in a plain one it is centred on the cell. In the last turn's cell (the last
    /// turn is played with the ship at the dock) it lies alongside the pier, which directly
    /// follows the last cell; once the turns are over it is in harbour behind it.
    pub fn ship_x(&self, cell: i32) -> i32 {
        if cell >= self.turns {
            // Over: in harbour, its stern behind the pier, so the last flag shows whole
            return self.pier_x() + PIER_SHIP_IN;
        }
        if cell == self.turns - 1 {
            // The last turn: alongside the pier, in the last cell's bare water
            return self.pier_x() - art::TRACK_SHIP_W + 1;
        }
        if self.flags {
            return self.cell_x(cell);
        }
        self.pad + self.pitch * cell + (self.pitch - art::TRACK_SHIP_W) / 2
    }

    /// The pier directly follows the last cell: with flags, one column past the last flag;
    /// without, where the last cell's room ends less the half of it round a buoy, but never
    /// touching the last buoy.
    pub fn pier_x(&self) -> i32 {
        if self.flags {
            return self.cell_x(self.turns - 1) + art::TRACK_FLAG_W + 1;
        }
        let beside = self.cell_x(self.turns) - (self.pitch - art::TRACK_BUOY_W) / 2;
        beside.max(self.buoy_x(self.turns - 1) + art::TRACK_BUOY_W + 1)
    }
}
