// SPDX-License-Identifier: GPL-3.0-or-later
//! The app around the game screen: the saved games, the settings, the menu and the pages it
//! opens, sound, haptics, and the accelerometer that places the dice.
//!
//! Everything here is drawn with the pixel-art kit (`crate::ui`): the menu, Settings and every
//! question are kit dialogs over the game, and How to play, Top Scores and About are kit pages in
//! a Day cover. Day's own Settings surface, About page and the web's window bar are not
//! used, so no part of the app shows in the platform's default style.

mod audio;
#[cfg(debug_assertions)]
mod debug;
pub mod orientation;
mod pages;
pub mod prefs;
pub mod store;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use day::prelude::*;

use crate::res;
use crate::rules::game::Game;
use crate::rules::high_scores::Theme;
use crate::rules::map_def::MapDef;
use crate::rules::palette::Palette;
use crate::ui::{self, Dialog};
use crate::view::{self, Haptic, ScreenHost};

use prefs::*;

day::routes! {
    /// The pages the menu opens over the game.
    pub(crate) enum Page {
        Rules => "rules",
        Scores => "scores",
        Games => "games",
        About => "about",
    }
}

thread_local! {
    /// The rank a finished game just took on its island's list (1 is a new best), until the
    /// shell has announced it; 0 when there is nothing to announce.
    static NEW_RANK: Cell<i32> = const { Cell::new(0) };
}

/// The island the player last had on screen.
fn current_island() -> &'static MapDef {
    MapDef::by_id(Some(&ISLAND.get_untracked()))
}

/// The other island, for the menu's "Go to" entry.
fn other_island(map: &MapDef) -> &'static MapDef {
    MapDef::all()
        .into_iter()
        .find(|m| m.id != map.id)
        .unwrap_or(MapDef::tarnholm())
}

/// The island's saved game, or a fresh one, wired to the app: scores go to the store, the
/// clock is the wall clock, and sounds go to the installed sink.
fn open_game(map: &'static MapDef) -> Game {
    let mut game = store::load_game(map).unwrap_or_else(|| {
        let mut g = Game::new(map);
        g.rng = crate::rules::rng::Rng::new(crate::clock::now_ms());
        g
    });
    game.recorder = Some(Box::new(|map_id: &str, result: i32| {
        let rank = store::record(map_id, result);
        NEW_RANK.with(|r| r.set(rank));
        rank
    }));
    game.clock = Some(Box::new(crate::clock::now_ms));
    apply_settings(&mut game);
    game
}

/// Copies the settings the rules read onto the game.
fn apply_settings(game: &mut Game) {
    game.player_color = PLAYER_COLOR.get() as u32;
    game.early_build_prompt = EARLY_BUILD_PROMPT.get() as i32;
    game.end_turn_prompt = END_TURN_PROMPT.get();
    game.set_trader_classic(TRADER_CLASSIC.get());
    game.set_race_target(RACE_TARGET.get() as i32);
}

/// What the menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Restart,
    OtherIsland,
    Settings,
    Rules,
    Scores,
    Games,
    About,
    #[cfg(debug_assertions)]
    Debug,
}

/// What the menu acts on.
#[derive(Clone)]
struct Shell {
    game: Rc<RefCell<Game>>,
    refresh: Signal<u64>,
    page: Signal<Option<Page>>,
    /// A played game shown on the board in place of the one in progress (`ScreenHost`).
    viewing: Rc<RefCell<Option<Game>>>,
}

impl Shell {
    /// The menu's entries, in order: a script key, the label, and what it does.
    fn entries(&self) -> Vec<(&'static str, String, Choice)> {
        let other = view::island_name(other_island(self.game.borrow().map));
        vec![
            (
                "restart",
                res::str::restart_title().format(),
                Choice::Restart,
            ),
            (
                "island",
                res::str::menu_go_to_island(other).format(),
                Choice::OtherIsland,
            ),
            (
                "settings",
                res::str::settings_menu().format(),
                Choice::Settings,
            ),
            ("rules", res::str::rules_menu().format(), Choice::Rules),
            (
                "scores",
                res::str::high_scores_menu().format(),
                Choice::Scores,
            ),
            ("games", res::str::games_menu().format(), Choice::Games),
            ("about", res::str::about_menu().format(), Choice::About),
            #[cfg(debug_assertions)]
            (
                "debug",
                res::str::debug_scenario_menu().format(),
                Choice::Debug,
            ),
        ]
    }

    /// Opens the menu: a dialog of the entries, one above the other, on every platform.
    fn open_menu(&self) {
        let me = self.clone();
        day::task(async move {
            let mut menu = Dialog::new(res::str::menu_title().format()).stacked();
            for (key, label, choice) in me.entries() {
                menu = menu.button(key, label, choice);
            }
            view::hold_tutorial(true);
            let choice = ui::ask(menu).await;
            view::hold_tutorial(false);
            me.refresh.update(|n| *n += 1);
            if let Some(choice) = choice {
                me.choose(choice);
            }
        });
    }

    /// The welcome, once per launch while the setting is on: this island's goal, the other
    /// island, and the basics. How to play opens the rules. The box is read when it closes.
    fn welcome(&self) {
        let me = self.clone();
        day::task(async move {
            let hide = Signal::global(false);
            let dialog = pages::welcome_dialog(&me.game.borrow(), RACE_TARGET.get() as i32, hide);
            view::hold_tutorial(true);
            let choice = ui::ask(dialog).await;
            view::hold_tutorial(false);
            if hide.get_untracked() {
                SHOW_WELCOME.set(false);
            }
            if choice == Some(pages::WelcomeChoice::HowToPlay) {
                me.page.set(Some(Page::Rules));
            }
            me.refresh.update(|n| *n += 1);
        });
    }

    /// Settings: a dialog of its own over the game. The theme previews as it is chosen; OK saves
    /// the rest, and however it closes the stored look is the one that stands. The player color
    /// opens over it and saves itself.
    fn settings(&self) {
        let me = self.clone();
        day::task(async move {
            use res::str as s;
            let draft = pages::Draft::new();
            let on_color: Rc<dyn Fn()> = Rc::new(pick_color);
            let dialog = Dialog::new(s::settings_title().format())
                .content(move |m, w| pages::settings(m, w, draft, on_color.clone()))
                .button("ok", s::ok_button().format(), ())
                .button("cancel", s::settings_cancel().format(), ());
            // Cancel answers like a tap outside: only OK reports a value of its own
            let dialog = dialog.ok_only();
            view::hold_tutorial(true);
            let answer = ui::ask(dialog).await;
            view::hold_tutorial(false);
            let tutorial = draft.tutorial.get_untracked();
            if answer.is_some() || tutorial {
                draft.apply();
            }
            if tutorial {
                TUTORIAL.set(true);
                TUTORIAL_SEEN.set(0);
                view::configure_tutorial(true, 0);
            }
            apply_theme(pages::current_theme());
            me.refresh.update(|n| *n += 1);
        });
    }

    fn choose(&self, choice: Choice) {
        let me = self.clone();
        match choice {
            Choice::Restart => {
                day::task(async move {
                    view::hold_tutorial(true);
                    let sure = ui::ask(
                        Dialog::new(res::str::restart_title().format())
                            .message(res::str::restart_text().format())
                            .button("yes", res::str::yes().format(), true)
                            .cancel(res::str::no().format())
                            .default_button("cancel"),
                    )
                    .await
                    .unwrap_or(false);
                    view::hold_tutorial(false);
                    if sure {
                        let map = me.game.borrow().map;
                        me.game.borrow_mut().start_new_game(map);
                        store::save_game(&me.game.borrow());
                    }
                    me.refresh.update(|n| *n += 1);
                });
            }
            Choice::OtherIsland => {
                // This island's game is kept, and the other's picked up where it was left.
                store::save_game(&self.game.borrow());
                let next = other_island(self.game.borrow().map);
                ISLAND.set(next.id.clone());
                *self.game.borrow_mut() = open_game(next);
                self.refresh.update(|n| *n += 1);
            }
            Choice::Settings => self.settings(),
            Choice::Rules => self.page.set(Some(Page::Rules)),
            Choice::Scores => self.page.set(Some(Page::Scores)),
            Choice::Games => self.page.set(Some(Page::Games)),
            Choice::About => self.page.set(Some(Page::About)),
            #[cfg(debug_assertions)]
            Choice::Debug => {
                debug::apply_scenario(&mut self.game.borrow_mut());
                self.refresh.update(|n| *n += 1);
            }
        }
    }

    /// A page the menu opened, in the kit's page frame with a Done button that closes it.
    /// Built again when the window turns, the text size changes or the language does.
    fn page_view(&self, page: &Page) -> AnyPiece {
        let page = *page;
        let open = self.page;
        let (viewing, refresh) = (self.viewing.clone(), self.refresh);
        switch(
            move || (ui::metrics(), day::locale().get()),
            move |(m, _)| {
                let done: Rc<dyn Fn()> = Rc::new(move || open.set(None));
                let m = *m;
                let ok = res::str::ok_button().format();
                match page {
                    Page::Rules => ui::page::page(
                        &m,
                        day::id!(rules_page),
                        &res::str::rules_title().format(),
                        &ok,
                        done,
                        ui::page::Fill::Scroll { thumb: true },
                        |w, _| pages::rules(&m, w),
                    ),
                    Page::Scores => ui::page::page(
                        &m,
                        day::id!(scores_page),
                        &res::str::high_scores_title().format(),
                        &ok,
                        done,
                        ui::page::Fill::Exact,
                        |w, h| pages::scores(&m, w, h),
                    ),
                    Page::Games => {
                        let viewing = viewing.clone();
                        let show: Rc<dyn Fn(Game)> = Rc::new(move |game| {
                            *viewing.borrow_mut() = Some(game);
                            open.set(None);
                            refresh.update(|n| *n += 1);
                        });
                        ui::page::page(
                            &m,
                            day::id!(games_page),
                            &res::str::games_title().format(),
                            &ok,
                            done,
                            ui::page::Fill::Scroll { thumb: true },
                            |w, _| pages::games(&m, w, show),
                        )
                    }
                    Page::About => ui::page::page(
                        &m,
                        day::id!(about_screen),
                        &res::str::about_menu().format(),
                        &ok,
                        done,
                        ui::page::Fill::Scroll { thumb: false },
                        |w, h| pages::about(&m, w, h),
                    ),
                }
            },
        )
        .any()
    }
}

/// A finished game made its island's list: say so with a toast, as the Android app did. The
/// fanfare is the rules' own (SoundCues).
fn announce_rank(rank: i32) {
    let title = if rank == 1 {
        res::str::high_scores_toast()
    } else {
        res::str::high_scores_top_5_toast()
    };
    ui::toast::show(title.format(), true);
}

/// The color picker, over Settings: OK saves the color, which reaches the game at once.
fn pick_color() {
    day::task(async move {
        use res::str as s;
        let color = Signal::global(PLAYER_COLOR.get() as u32);
        let dialog = Dialog::new(s::player_color_title().format())
            .content(move |m, w| pages::color_picker(m, w, color))
            .button("ok", s::ok_button().format(), ())
            .button("cancel", s::settings_cancel().format(), ())
            .ok_only();
        if ui::ask_over(dialog).await.is_some() {
            PLAYER_COLOR.set((0xFF000000 | color.get_untracked()) as i64);
        }
    });
}

/// Makes the look follow a Theme setting and the system's night mode.
fn apply_theme(theme: Theme) {
    ui::paint::set_palette(*Palette::for_mode(theme, day::dark_mode()));
}

/// The app's root: the game screen, with the kit's dialogs over it and the menu's pages in a
/// cover above that.
/// A build of `root` and what it leaves running. On Android a theme or language switch runs `root`
/// again on a new tree, and everything built on the root scope would live on: its keyed pieces see
/// the old node ids alive in the new tree, which repeats them, and build into it a second time.
struct Mounted<P> {
    scope: Scope,
    piece: P,
}

impl<P: Piece> Piece for Mounted<P> {
    fn build(self, cx: &mut day::BuildCx) -> day::RNode {
        self.scope.enter(|| self.piece.build(cx))
    }
}

pub fn root() -> impl Piece {
    thread_local! {
        static MOUNT: Cell<Option<Scope>> = const { Cell::new(None) };
    }
    if let Some(last) = MOUNT.take() {
        last.dispose();
        ui::dialog::reset();
    }
    let scope = Scope::child();
    MOUNT.set(Some(scope));
    Mounted {
        scope,
        piece: scope.enter(build_root),
    }
}

fn build_root() -> impl Piece {
    // The saved language applies from the start, as Day's settings surface did when it was
    // installed; the Settings page's Language choice writes the same pref.
    day_piece_settings::wire_general();
    let _sounds = audio::Sounds::install();

    #[cfg(debug_assertions)]
    debug::apply_settings();

    apply_theme(pages::current_theme());

    let game = Rc::new(RefCell::new(open_game(current_island())));
    #[cfg(debug_assertions)]
    debug::apply_scenario(&mut game.borrow_mut());
    let refresh = Signal::new(0u64);
    let page: Signal<Option<Page>> = Signal::new(None);
    let dice_side = orientation::dice_side();

    // A settings change reaches the game on screen at once.
    {
        let game = game.clone();
        Effect::new(move || {
            // Tracked reads: any of these changing re-runs this.
            let _ = (
                PLAYER_COLOR.signal().get(),
                EARLY_BUILD_PROMPT.signal().get(),
                END_TURN_PROMPT.signal().get(),
                TRADER_CLASSIC.signal().get(),
                RACE_TARGET.signal().get(),
            );
            untrack(|| {
                apply_settings(&mut game.borrow_mut());
                store::save_game(&game.borrow());
                refresh.update(|n| *n += 1);
            });
        });
    }

    // The tutorial is the screen's; its settings and progress are the shell's to keep.
    view::configure_tutorial(TUTORIAL.get(), TUTORIAL_SEEN.get() as u32);
    Effect::new(move || {
        let on = TUTORIAL.signal().get();
        untrack(|| {
            if view::tutorial_progress().0 != on {
                view::configure_tutorial(on, TUTORIAL_SEEN.get() as u32);
                refresh.update(|n| *n += 1);
            }
        });
    });

    // No tutorial callout shows under a page; one that came due shows when it closes.
    Effect::new(move || {
        let open = page.get().is_some();
        untrack(|| {
            view::hold_tutorial(open);
            if !open {
                refresh.update(|n| *n += 1);
            }
        });
    });

    let on_changed: Rc<dyn Fn()> = {
        let game = game.clone();
        Rc::new(move || {
            let rank = NEW_RANK.with(|r| r.replace(0));
            if rank > 0 {
                announce_rank(rank);
            }
            if let Some(done) = game.borrow_mut().take_finished() {
                store::log_game(done);
            }
            store::save_game(&game.borrow());
            let (on, seen) = view::tutorial_progress();
            if TUTORIAL.get() != on {
                TUTORIAL.set(on);
            }
            if TUTORIAL_SEEN.get() != seen as i64 {
                TUTORIAL_SEEN.set(seen as i64);
            }
        })
    };

    let viewing: Rc<RefCell<Option<Game>>> = Rc::new(RefCell::new(None));
    let shell = Shell {
        game: game.clone(),
        refresh,
        page,
        viewing: viewing.clone(),
    };
    let on_menu: Rc<dyn Fn()> = {
        let shell = shell.clone();
        Rc::new(move || shell.open_menu())
    };
    if SHOW_WELCOME.get() {
        shell.welcome();
    }

    let haptic: Rc<dyn Fn(Haptic)> = Rc::new(|h| {
        if !HAPTICS.get() {
            return;
        }
        day_part_haptics::play(match h {
            Haptic::Tap => day_part_haptics::Haptic::Selection,
            Haptic::Roll | Haptic::Build => day_part_haptics::Haptic::Medium,
        });
    });
    // A press in a dialog or on a page clicks and taps as the game's own buttons do
    {
        let haptic = haptic.clone();
        ui::widget::set_feedback(move || haptic(Haptic::Tap));
    }

    let screen = view::game_screen(ScreenHost {
        game,
        viewing,
        refresh,
        dice_side,
        on_menu,
        on_changed,
        haptic,
    });
    // The game has the whole screen, as the Android app's fullscreen window did
    ui::host(
        zstack((
            screen,
            cover(page, move |p: &Page| {
                zstack((shell.page_view(p), ui::overlays())).align(Alignment::TopLeading)
            })
            .background(|_| ui::page::surround()),
        )),
        move || page.get().is_some(),
    )
    .status_bar_hidden(true)
}
