// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from SoundPlaybackTest: the guard, the retrigger of the banked sound, delayed
//! requests, and closing.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dayapp::rules::sound_cues::BANKED_GAP_MS;
use dayapp::rules::sound_playback::{GUARD_MS, SoundBackend, SoundPlayback};
use dayapp::rules::{Sfx, SoundSink};

#[derive(Clone)]
struct FakeBackend {
    log: Rc<RefCell<Vec<String>>>,
    loaded: Rc<Cell<bool>>,
    next: Rc<Cell<u32>>,
}

impl SoundBackend for FakeBackend {
    fn play(&mut self, sfx: Sfx) -> u32 {
        if !self.loaded.get() {
            return 0;
        }
        let id = self.next.get();
        self.next.set(id + 1);
        self.log.borrow_mut().push(format!("play {sfx} {id}"));
        id
    }

    fn stop(&mut self, sfx: Sfx, stream_id: u32) {
        self.log
            .borrow_mut()
            .push(format!("stop {sfx} {stream_id}"));
    }
}

type Clock = Rc<Cell<u64>>;

struct Rig {
    backend: FakeBackend,
    clock: Clock,
    playback: SoundPlayback<FakeBackend, Box<dyn Fn() -> u64>>,
}

impl Rig {
    fn new() -> Rig {
        let backend = FakeBackend {
            log: Rc::default(),
            loaded: Rc::new(Cell::new(true)),
            next: Rc::new(Cell::new(1)),
        };
        let clock: Clock = Rc::new(Cell::new(1000));
        let c = clock.clone();
        let playback = SoundPlayback::new(
            backend.clone(),
            Box::new(move || c.get()) as Box<dyn Fn() -> u64>,
        );
        Rig {
            backend,
            clock,
            playback,
        }
    }

    fn log(&self) -> String {
        format!("[{}]", self.backend.log.borrow().join(", "))
    }

    /// Advances time, playing whatever came due on the way, each at its own time.
    fn advance(&mut self, ms: u64) {
        let target = self.clock.get() + ms;
        while let Some(due) = self.playback.next_due().filter(|&d| d <= target) {
            self.clock.set(due.max(self.clock.get()));
            self.playback.poll();
        }
        self.clock.set(target);
    }
}

#[test]
fn a_sound_plays_and_logs_its_stream() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Click, 0);
    assert_eq!("[play CLICK 1]", r.log());
}

#[test]
fn the_same_sound_twice_within_the_guard_plays_once() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Click, 0);
    r.advance(GUARD_MS - 1);
    r.playback.play(Sfx::Click, 0);
    assert_eq!("[play CLICK 1]", r.log());
    r.advance(1);
    r.playback.play(Sfx::Click, 0);
    assert_eq!("[play CLICK 1, play CLICK 2]", r.log());
}

#[test]
fn different_sounds_at_once_all_play() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Castle, 0);
    r.playback.play(Sfx::Banked, 0);
    r.playback.play(Sfx::Gold, 0);
    assert_eq!("[play CASTLE 1, play BANKED 2, play GOLD 3]", r.log());
}

#[test]
fn banked_retriggers_by_stopping_the_stream_still_playing() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Banked, 0);
    r.advance(110);
    r.playback.play(Sfx::Banked, 0);
    r.advance(110);
    r.playback.play(Sfx::Banked, 0);
    assert_eq!(
        "[play BANKED 1, stop BANKED 1, play BANKED 2, stop BANKED 2, play BANKED 3]",
        r.log()
    );
}

#[test]
fn banked_after_its_sample_has_ended_just_plays() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Banked, 0);
    r.advance(Sfx::Banked.retrigger_ms());
    r.playback.play(Sfx::Banked, 0);
    assert_eq!("[play BANKED 1, play BANKED 2]", r.log());
}

#[test]
fn other_sounds_do_not_retrigger() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Village, 0);
    r.advance(100);
    r.playback.play(Sfx::Village, 0);
    assert_eq!("[play VILLAGE 1, play VILLAGE 2]", r.log());
}

#[test]
fn a_delayed_sound_plays_when_its_time_comes() {
    let mut r = Rig::new();
    r.playback.play(Sfx::ShipDocks, 300);
    assert_eq!("[]", r.log());
    r.advance(299);
    assert_eq!("[]", r.log());
    r.advance(1);
    assert_eq!("[play SHIP_DOCKS 1]", r.log());
}

#[test]
fn delayed_banked_uses_stutter_the_opening() {
    let mut r = Rig::new();
    r.playback.play(Sfx::Banked, 0);
    r.playback.play(Sfx::Banked, BANKED_GAP_MS);
    r.playback.play(Sfx::Banked, 2 * BANKED_GAP_MS);
    r.advance(1000);
    assert_eq!(
        "[play BANKED 1, stop BANKED 1, play BANKED 2, stop BANKED 2, play BANKED 3]",
        r.log()
    );
}

#[test]
fn a_sound_not_loaded_yet_is_dropped_and_does_not_count_as_played() {
    let mut r = Rig::new();
    r.backend.loaded.set(false);
    r.playback.play(Sfx::Click, 0);
    r.backend.loaded.set(true);
    // Straight away, inside the guard: it did not play, so it is not guarded
    r.playback.play(Sfx::Click, 0);
    assert_eq!("[play CLICK 1]", r.log());
}

#[test]
fn a_dropped_banked_sound_leaves_nothing_to_stop() {
    let mut r = Rig::new();
    r.backend.loaded.set(false);
    r.playback.play(Sfx::Banked, 0);
    r.advance(100);
    r.backend.loaded.set(true);
    r.playback.play(Sfx::Banked, 0);
    assert_eq!("[play BANKED 1]", r.log());
}

#[test]
fn closing_stops_waiting_sounds_too() {
    let mut r = Rig::new();
    r.playback.play(Sfx::ShipDocks, 300);
    r.playback.close();
    r.advance(1000);
    r.playback.play(Sfx::Click, 0);
    assert_eq!("[]", r.log());
}
