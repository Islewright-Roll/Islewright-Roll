// SPDX-License-Identifier: GPL-3.0-or-later
//! Sound effects through Day's sound part. The rules decide what plays and when
//! (`rules::sound_playback`: the delay queue, the repeat guard, the retrigger); this module only
//! makes the noise, and ticks the queue while anything waits in it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::rules::sfx::Sfx;
use crate::rules::sound_playback::{SoundBackend, SoundPlayback};
use crate::rules::sound_sink::{self, SoundSink};

use super::prefs::SOUND_EFFECTS;

/// The clip for a sound, under `resource/assets/sfx/`.
fn clip(sfx: Sfx) -> day::AssetName {
    day::AssetName::dynamic(format!("sfx/sfx_{}.wav", sfx.file()))
}

/// Plays clips through day-part-sound, which mixes, limits voices and never fails loudly.
#[derive(Default)]
struct DaySound {
    /// A sound the retrigger stopped, which its next play restarts.
    restart: Option<Sfx>,
}

impl SoundBackend for DaySound {
    fn play(&mut self, sfx: Sfx) -> u32 {
        if !SOUND_EFFECTS.get() {
            return 0;
        }
        if self.restart.take() == Some(sfx) {
            day_part_sound::restart(&clip(sfx));
        } else {
            day_part_sound::play(&clip(sfx));
        }
        1
    }

    // The retrigger stops a sound and plays it again at once, which the part does in one call:
    // the next play of it silences what is still sounding and starts it from the top.
    fn stop(&mut self, sfx: Sfx, _stream_id: u32) {
        self.restart = Some(sfx);
    }
}

type Playback = SoundPlayback<DaySound, fn() -> u64>;

struct Inner {
    playback: RefCell<Playback>,
    /// When the pending wake-up is due, if one is pending.
    scheduled: Cell<Option<u64>>,
}

/// The sink the game and the UI play through: a handle on the one shared playback, so the
/// delay queue can be ticked from outside.
#[derive(Clone)]
pub struct Sounds(Rc<Inner>);

impl SoundSink for Sounds {
    fn play(&mut self, sfx: Sfx, delay_ms: i32) {
        // There are two roll sounds, a rattle and a throw: each roll plays one of them. The
        // clock's last bit will do for a coin: a roll comes when a finger says so.
        let sfx = if sfx == Sfx::Roll && crate::clock::now_ms() % 2 == 1 {
            Sfx::RollB
        } else {
            sfx
        };
        self.0.playback.borrow_mut().play(sfx, delay_ms);
        self.wake();
    }
}

impl Sounds {
    /// Loads the clips and installs this as the thread's sink, for sounds with no game of
    /// their own (a button click).
    pub fn install() -> Sounds {
        let clips: Vec<day::AssetName> = Sfx::ALL.iter().map(|s| clip(*s)).collect();
        day_part_sound::preload(&clips);
        let sounds = Sounds(Rc::new(Inner {
            playback: RefCell::new(SoundPlayback::new(
                DaySound::default(),
                crate::clock::now_ms as fn() -> u64,
            )),
            scheduled: Cell::new(None),
        }));
        sound_sink::set_sink(Some(Box::new(sounds.clone())));
        sounds
    }

    /// Schedules one wake-up for when the next waiting sound is due: the pending one stands
    /// unless the new sound is due sooner. A wake-up plays what has come due and schedules the
    /// next.
    fn wake(&self) {
        let Some(due) = self.0.playback.borrow().next_due() else {
            return;
        };
        if self.0.scheduled.get().is_some_and(|t| t <= due) {
            return;
        }
        self.0.scheduled.set(Some(due));
        let me = self.clone();
        day::task(async move {
            let wait = due.saturating_sub(crate::clock::now_ms());
            if wait > 0 {
                day::sleep(wait.min(u32::MAX as u64) as u32).await;
            }
            if me.0.scheduled.get() == Some(due) {
                me.0.scheduled.set(None);
            }
            me.0.playback.borrow_mut().poll();
            me.wake();
        });
    }
}
