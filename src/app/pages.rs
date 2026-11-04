// SPDX-License-Identifier: GPL-3.0-or-later
//! What the menu's pages and dialogs hold: How to play, Top Scores and About for `ui::page`, and
//! the content of the Settings, color picker and welcome dialogs, each as the Android app laid it
//! out. Settings are held in a `Draft` until OK saves them to the `Pref`s in `prefs.rs`.

use std::rc::Rc;

use day::prelude::*;

use super::prefs::*;
use crate::res;
use crate::rules::game::Game;
use crate::rules::high_scores::{
    self, EARLY_PROMPT_CONSERVATIVE, EARLY_PROMPT_OFF, EARLY_PROMPT_SMART, HighScoresAndSettings,
    MUSIC_VOLUME_MAX, MUSIC_VOLUME_MIN, RACE_TARGET_MIN, Theme, race_target_max,
};
use crate::rules::map_def::MapDef;
use crate::rules::player_color::{self, PRESETS};
use crate::rules::rules_markup::{self, Kind};
use crate::ui::paint::kit;
use crate::ui::text::{Align, Face};
use crate::ui::widget::{self, Lead, States, Tone};
use crate::ui::{Button, Dialog, Icon, Metrics, Style, W, ask, toast};
use crate::view::island_name;

/// The localized rules text, `resource/assets/text/<locale>/rules.txt`, compiled in so every
/// target reads it the same way (the web build has no synchronous asset reads).
fn rules_source(locale: &str) -> &'static str {
    match language(locale) {
        "de" => include_str!("../../resource/assets/text/de/rules.txt"),
        "es" => include_str!("../../resource/assets/text/es/rules.txt"),
        "fr" => include_str!("../../resource/assets/text/fr/rules.txt"),
        "id" | "in" => include_str!("../../resource/assets/text/id/rules.txt"),
        "it" => include_str!("../../resource/assets/text/it/rules.txt"),
        "pt" => include_str!("../../resource/assets/text/pt-BR/rules.txt"),
        _ => include_str!("../../resource/assets/text/en/rules.txt"),
    }
}

/// The localized About text, `resource/assets/text/<locale>/about.txt`.
fn about_source(locale: &str) -> &'static str {
    match language(locale) {
        "de" => include_str!("../../resource/assets/text/de/about.txt"),
        "es" => include_str!("../../resource/assets/text/es/about.txt"),
        "fr" => include_str!("../../resource/assets/text/fr/about.txt"),
        "id" | "in" => include_str!("../../resource/assets/text/id/about.txt"),
        "it" => include_str!("../../resource/assets/text/it/about.txt"),
        "pt" => include_str!("../../resource/assets/text/pt-BR/about.txt"),
        _ => include_str!("../../resource/assets/text/en/about.txt"),
    }
}

fn language(locale: &str) -> &str {
    locale.split(['-', '_']).next().unwrap_or("en")
}

/// The gap above each kind of block in the rules, as Rules.java laid them out.
fn rules_gap(kind: &Kind) -> i32 {
    match kind {
        Kind::Heading => 10,
        Kind::Bullet => 3,
        Kind::Icon => 4,
        Kind::Image => 8,
        _ => 5,
    }
}

/// How to play: the rules text as headings, paragraphs, bullets, icon lines and pictures, in the
/// current language. 4 art pixels stay clear at the right for the scroll thumb.
pub fn rules(m: &Metrics, width: i32) -> W {
    let locale = day::locale().get_untracked();
    let whole = width;
    let width = width - 4;
    let mut items = vec![widget::gap(m, whole, 0)];
    for b in rules_markup::parse(Some(rules_source(&locale))) {
        let text = b.text.clone().unwrap_or_default();
        let icon = b.name.as_deref().and_then(Icon::by_name);
        let top = if items.len() == 1 {
            3
        } else {
            rules_gap(&b.kind)
        };
        let item = match b.kind {
            Kind::Heading => widget::text(m, None, width, &text, Style::HEADING),
            Kind::Paragraph => widget::text(m, None, width, &text, Style::BODY),
            Kind::Bullet => widget::lead_text(m, None, width, Lead::Bullet, &text, Style::BODY),
            Kind::Icon => {
                let lead = icon.map_or(Lead::None, Lead::Icon);
                widget::lead_text(m, None, width, lead, &text, Style::BODY)
            }
            Kind::Image => {
                let Some(icon) = icon else { continue };
                widget::picture(m, width, icon)
            }
        };
        items.push(widget::gap(m, width, top));
        items.push(item);
        if matches!(b.kind, Kind::Image) {
            items.push(widget::gap(m, width, 2));
        }
    }
    widget::vstack(m, items, 0)
}

/// About: the whole text as one block, its web addresses tappable, and the name carrying the
/// version the player is running. The block is centered when it is shorter than the page.
pub fn about(m: &Metrics, width: i32, height: i32) -> W {
    let locale = day::locale().get_untracked();
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let text = about_source(&locale).replace("{{version}}", &version);
    widget::linked_text(m, width, &text, height)
}

/// A date as this locale's short form, as DateFormat.SHORT wrote it. "-" for none.
fn short_date(ms: i64, locale: &str) -> String {
    if ms == 0 {
        return res::str::high_scores_default().format();
    }
    let iso = high_scores::format_date(ms);
    let part = |range: std::ops::Range<usize>| iso.get(range).unwrap_or("").to_string();
    let (year, month, day) = (part(0..4), part(5..7), part(8..10));
    let short = year.get(2..).unwrap_or("").to_string();
    let bare = |s: &str| s.trim_start_matches('0').to_string();
    match language(locale) {
        "de" => format!("{}.{}.{short}", day, month),
        "fr" => format!("{day}/{month}/{year}"),
        "es" | "pt" | "it" | "id" | "in" => format!("{day}/{month}/{short}"),
        _ => format!("{}/{}/{short}", bare(&month), bare(&day)),
    }
}

/// What a backup holds besides the scores: the settings as they stand.
fn backup_document() -> HighScoresAndSettings {
    let mut doc = super::store::load_scores();
    doc.player_color = PLAYER_COLOR.get() as u32;
    doc.player_color_was_specified = true;
    doc.map = ISLAND.get();
    doc.map_was_specified = true;
    doc.early_build_prompt = EARLY_BUILD_PROMPT.get() as i32;
    doc.end_turn_prompt = END_TURN_PROMPT.get();
    doc.haptic_feedback = HAPTICS.get();
    doc.haptic_feedback_was_specified = true;
    doc.sound_effects = SOUND_EFFECTS.get();
    doc.sound_effects_was_specified = true;
    doc.trader_classic = TRADER_CLASSIC.get();
    doc.trader_classic_was_specified = true;
    doc.theme = current_theme();
    doc.theme_was_specified = true;
    doc.show_welcome = SHOW_WELCOME.get();
    doc.show_welcome_was_specified = true;
    doc.tutorial = TUTORIAL.get();
    doc.tutorial_was_specified = true;
    doc.tutorial_seen = TUTORIAL_SEEN.get() as u32;
    doc.tutorial_seen_was_specified = true;
    doc.music_volume = MUSIC_VOLUME.get() as i32;
    doc.music_volume_was_specified = true;
    doc.race_target = RACE_TARGET.get() as i32;
    doc.race_target_was_specified = true;
    doc
}

/// Takes a parsed backup in: every island's results are merged, and each setting the file
/// actually names comes over. Returns how many scores were new.
fn import_backup(imported: &HighScoresAndSettings) -> usize {
    let mut store = backup_document();
    let added = store.import_from(imported);
    super::store::save_scores(&store);
    PLAYER_COLOR.set(store.player_color as i64);
    HAPTICS.set(store.haptic_feedback);
    SOUND_EFFECTS.set(store.sound_effects);
    TRADER_CLASSIC.set(store.trader_classic);
    THEME.set(store.theme.name().to_string());
    SHOW_WELCOME.set(store.show_welcome);
    TUTORIAL.set(store.tutorial);
    TUTORIAL_SEEN.set(store.tutorial_seen as i64);
    MUSIC_VOLUME.set(store.music_volume as i64);
    RACE_TARGET.set(store.race_target as i64);
    super::apply_theme(store.theme);
    added
}

/// What a backup is suggested to be called, HighScoresAndSettings.FILE_NAME.
const BACKUP_FILE_NAME: &str = "highscores-and-settings.json";

/// Export: where to write a copy of the document is the player's to pick.
fn start_export() {
    day::task(async move {
        let json = backup_document().to_json();
        let saved = save_file(json.into_bytes())
            .suggested_name(BACKUP_FILE_NAME)
            .filter("JSON", &["json"])
            .present()
            .await;
        // Day answers the same for a cancelled picker and a failed write: nothing was saved, and
        // the player can see that from no message having appeared
        if saved.is_some() {
            toast::show(res::str::high_scores_export_success().format(), true);
        }
    });
}

/// A dialog of a title, a message and OK.
fn tell(title: String, message: String) {
    day::task(async move {
        let _ = ask(Dialog::new(title).message(message).button(
            "ok",
            res::str::ok_button().format(),
            (),
        ))
        .await;
    });
}

/// A picked file is read whole before it is parsed, so a cap keeps a wrong pick from filling
/// memory. The real document is at most a few kilobytes.
const MAX_IMPORT_BYTES: usize = 1024 * 1024;

/// Import: the picked document merged in, or nothing changed if any step fails.
fn start_import(changed: Signal<u64>) {
    day::task(async move {
        let Some(file) = open_file().filter("JSON", &["json"]).present().await else {
            return;
        };
        let parsed = match file.read_limited(MAX_IMPORT_BYTES).await {
            Ok(bytes) => String::from_utf8(bytes)
                .ok()
                .and_then(|s| HighScoresAndSettings::from_json(&s).ok()),
            Err(_) => None,
        };
        let Some(imported) = parsed else {
            tell(
                res::str::high_scores_import_failed_title().format(),
                res::str::high_scores_import_failed_message().format(),
            );
            return;
        };
        let added = import_backup(&imported);
        changed.update(|n| *n += 1);
        tell(
            res::str::high_scores_import_title().format(),
            res::str::high_scores_import_success(added as i64).format(),
        );
    });
}

/// How tall a score row is, with its rule: 8 above it, 6 below, and the rule.
const SCORE_ROW_ABOVE: i32 = 8;
const SCORE_ROW_BELOW: i32 = 6;

/// Top Scores: a radio button per island, opening on the one in play, then its five rows and the
/// backup buttons, all set in the middle of the page (high_scores.xml).
pub fn scores(m: &Metrics, width: i32, height: i32) -> W {
    use res::str as s;
    let inset = 6;
    let list_w = width - 2 * inset;
    let shown = Signal::new(ISLAND.get());
    let changed = Signal::new(0u64);
    let maps = MapDef::all();
    let each = list_w / maps.len().max(1) as i32;
    let radios: Vec<W> = maps
        .iter()
        .map(|map| {
            let name = island_name(map);
            let id = map.id.to_string();
            let (mine, pick) = (id.clone(), id.clone());
            widget::radio(
                m,
                day::id!(scores_map, id),
                each,
                &name,
                &name,
                None,
                radio_states(),
                move || shown.get() == mine,
                move || shown.set(pick.clone()),
            )
        })
        .collect();
    let radios = widget::hstack(m, radios, 0);
    let row_h = kit().body.height() * m.t;
    let list_h = 1 + 5 * (SCORE_ROW_ABOVE + row_h + SCORE_ROW_BELOW + 1);
    let m2 = *m;
    let list = switch(
        move || (shown.get(), changed.get()),
        move |(map_id, _)| score_rows(&m2, map_id, list_w, row_h).piece,
    )
    .frame(m.pt(list_w), m.pt(list_h));
    let list = W::new(list, list_w, list_h);

    let note = widget::text(
        m,
        None,
        width,
        &s::high_scores_backup_note().format(),
        Style::NOTE.centered(),
    );
    let half = (width - 4) / 2;
    let export = Button::new(
        day::id!(scores_export),
        s::high_scores_export_button().format(),
    )
    .width(half)
    .build(m, start_export);
    let import = Button::new(
        day::id!(scores_import),
        s::high_scores_import_button().format(),
    )
    .width(half)
    .build(m, move || start_import(changed));
    let buttons = widget::hstack(m, vec![export, import], 4);

    let radios = widget::inset(m, radios, inset);
    let list = widget::inset(m, list, inset);
    let used = radios.h + 4 + list.h + 20 + note.h + 8 + buttons.h;
    let spare = (height - used).max(0);
    // The two spacers of high_scores.xml share the window's device pixels, so half of what is
    // left of a whole art pixel lands above the list
    let nudge = (m.extra_h / 2) as f64 / m.dpr;
    let column = widget::vstack(
        m,
        vec![
            widget::gap(m, width, spare / 2),
            radios,
            widget::gap(m, width, 4),
            list,
            widget::gap(m, width, 20),
            note,
            widget::gap(m, width, 8),
            buttons,
        ],
        0,
    );
    let (w, h) = (column.w, column.h);
    W::new(
        column.piece.padding(Insets::new(nudge, 0.0, 0.0, 0.0)),
        w,
        h,
    )
}

/// The rule the first row hangs from, and a map's five rows, "-" beyond its stored scores.
fn score_rows(m: &Metrics, map_id: &str, width: i32, row_h: i32) -> W {
    let locale = day::locale().get_untracked();
    let store: HighScoresAndSettings = super::store::load_scores();
    let map = MapDef::by_id(Some(map_id));
    let list = store.scores_for(&map.id);
    let mut items = vec![widget::rule(m, width)];
    for i in 0..5 {
        let (score, date) = match list.get(i) {
            Some(e) if e.score > 0 => {
                let score = if map.lower_is_better {
                    res::str::turns_count(e.score as i64).format()
                } else {
                    e.score.to_string()
                };
                (score, short_date(e.date, &locale))
            }
            _ => {
                let none = res::str::high_scores_default().format();
                (none.clone(), none)
            }
        };
        let spoken = format!("{} {score}, {date}", i + 1);
        let id = day::id!(score_line, format!("{}-{}", map.id, i + 1));
        let cell = widget::cells(
            m,
            Some(id),
            width,
            vec![
                (score, 0, width / 2, Align::Start, Tone::Ink, Face::Display),
                (
                    date,
                    width / 2,
                    width - width / 2,
                    Align::End,
                    Tone::Ink,
                    Face::Body,
                ),
            ],
            &spoken,
        );
        items.push(widget::gap(m, width, SCORE_ROW_ABOVE));
        items.push(widget::vstack(
            m,
            vec![widget::gap(m, width, row_h - cell.h), cell],
            0,
        ));
        items.push(widget::gap(m, width, SCORE_ROW_BELOW));
        items.push(widget::rule(m, width));
    }
    widget::vstack(m, items, 0)
}

/// Played games: every finished game, newest first, as its island, turns, score and date.
/// Tapping one puts its end on the board (`show`).
pub fn games(m: &Metrics, width: i32, show: Rc<dyn Fn(Game)>) -> W {
    use res::str as s;
    let locale = day::locale().get_untracked();
    // 4 art pixels stay clear at the right for the scroll thumb
    let width = width - 4;
    let log = super::store::load_games();
    if log.games.is_empty() {
        return widget::text(
            m,
            None,
            width,
            &s::games_none().format(),
            Style::BODY.centered(),
        );
    }
    let col = |n: i32| width * n / 20;
    let columns = |island: String, turns: String, score: String, date: String, tone, face| {
        vec![
            (island, 0, col(7), Align::Start, tone, face),
            (turns, col(7), col(4), Align::End, tone, face),
            (score, col(11), col(4), Align::End, tone, face),
            (date, col(15), width - col(15), Align::End, tone, Face::Body),
        ]
    };
    let mut items = vec![
        widget::cells(
            m,
            None,
            width,
            columns(
                s::games_island().format(),
                s::games_turns().format(),
                s::games_score().format(),
                s::games_date().format(),
                Tone::Secondary,
                Face::Body,
            ),
            "",
        ),
        widget::gap(m, width, 2),
        widget::rule(m, width),
    ];
    let count = log.games.len();
    for (n, played) in log.games.into_iter().rev().enumerate() {
        let Some(map) = MapDef::find(&played.map) else {
            continue;
        };
        let island = island_name(map);
        let date = short_date(played.date, &locale);
        let spoken = s::games_row_spoken(s::GamesRowSpoken {
            arg1: island.clone(),
            arg2: played.turns as i64,
            arg3: played.score as i64,
            arg4: date.clone(),
        })
        .format();
        let row = columns(
            island,
            played.turns.to_string(),
            played.score.to_string(),
            date,
            Tone::Ink,
            Face::Display,
        );
        let show = show.clone();
        items.push(widget::row_button(
            m,
            day::id!(game_row, (count - n).to_string()),
            width,
            SCORE_ROW_BELOW,
            row,
            &spoken,
            move || {
                if let Some(game) = played.restore() {
                    show(game);
                }
            },
        ));
        items.push(widget::rule(m, width));
    }
    widget::vstack(m, items, 0)
}

/// Selected and not, as a radio button's value is read.
fn radio_states() -> States {
    States {
        on: res::str::ui_selected().format(),
        off: res::str::ui_not_selected().format(),
    }
}

/// On and Off, as a switch's value is read.
fn switch_states() -> States {
    States {
        on: res::str::settings_on().format(),
        off: res::str::settings_off().format(),
    }
}

/// The Theme setting.
pub fn current_theme() -> Theme {
    Theme::from_name(&THEME.get()).unwrap_or_default()
}

/// A group of radio buttons bound to a signal, one above the other.
fn radios<T: Clone + PartialEq + 'static>(
    m: &Metrics,
    width: i32,
    group: &str,
    value: Signal<T>,
    options: Vec<(T, String, ElementId)>,
) -> Vec<W> {
    options
        .into_iter()
        .map(|(v, label, id)| {
            let (mine, pick) = (v.clone(), v);
            widget::radio(
                m,
                id,
                width,
                &label,
                &format!("{group}: {label}"),
                None,
                radio_states(),
                move || value.get() == mine,
                move || value.set(pick.clone()),
            )
        })
        .collect()
}

/// A slider with its value beside it, as settings.xml has it: 6 clear at the right, the value in
/// 22 art pixels, right aligned, 2 after the slider.
fn slider_row(
    m: &Metrics,
    id: ElementId,
    width: i32,
    label: &str,
    range: std::ops::RangeInclusive<i64>,
    value: Signal<i64>,
) -> W {
    let bar_w = width - 6 - 22 - 2;
    let bar = widget::slider(
        m,
        id,
        bar_w,
        label,
        range,
        move || value.get(),
        move |v| value.set(v),
    );
    let m2 = *m;
    let digits = switch(
        move || value.get(),
        move |v| widget::text(&m2, None, 22, &v.to_string(), Style::BODY.end()).piece,
    )
    .frame(m.pt(22), m.pt(kit().body.height() * m.t));
    let digits = W::new(digits, 22, kit().body.height() * m.t);
    let digits = widget::vstack(
        m,
        vec![widget::gap(m, 22, (bar.h - digits.h) / 2), digits],
        0,
    );
    widget::hstack(m, vec![bar, widget::gap(m, 2, 1), digits], 0)
}

/// What the Settings dialog holds until OK saves it.
#[derive(Clone, Copy)]
pub struct Draft {
    early: Signal<i64>,
    end_turn: Signal<bool>,
    trader: Signal<bool>,
    haptics: Signal<bool>,
    water: Signal<bool>,
    music: Signal<i64>,
    sound: Signal<bool>,
    theme: Signal<String>,
    welcome: Signal<bool>,
    target: Signal<i64>,
    /// Show tutorial was pressed: everything is saved and the tutorial starts over.
    pub tutorial: Signal<bool>,
}

impl Draft {
    pub fn new() -> Draft {
        Draft {
            early: Signal::global(EARLY_BUILD_PROMPT.get()),
            end_turn: Signal::global(END_TURN_PROMPT.get()),
            trader: Signal::global(TRADER_CLASSIC.get()),
            haptics: Signal::global(HAPTICS.get()),
            water: Signal::global(ANIMATE_WATER.get()),
            music: Signal::global(MUSIC_VOLUME.get()),
            sound: Signal::global(SOUND_EFFECTS.get()),
            theme: Signal::global(THEME.get()),
            welcome: Signal::global(SHOW_WELCOME.get()),
            target: Signal::global(RACE_TARGET.get()),
            tutorial: Signal::global(false),
        }
    }

    /// Saves what the dialog shows: what OK does.
    pub fn apply(&self) {
        EARLY_BUILD_PROMPT.set(self.early.get_untracked());
        END_TURN_PROMPT.set(self.end_turn.get_untracked());
        TRADER_CLASSIC.set(self.trader.get_untracked());
        HAPTICS.set(self.haptics.get_untracked());
        ANIMATE_WATER.set(self.water.get_untracked());
        MUSIC_VOLUME.set(self.music.get_untracked());
        SOUND_EFFECTS.set(self.sound.get_untracked());
        THEME.set(self.theme.get_untracked());
        SHOW_WELCOME.set(self.welcome.get_untracked());
        RACE_TARGET.set(self.target.get_untracked());
    }
}

/// The Settings dialog's content, in settings.xml's order, with the language after it.
pub fn settings(m: &Metrics, width: i32, d: Draft, on_color: Rc<dyn Fn()>) -> W {
    use res::str as s;
    let dunholm = MapDef::dunholm();
    let mut items: Vec<W> = Vec::new();
    let gap = |items: &mut Vec<W>, h: i32| items.push(widget::gap(m, width, h));
    let line = |items: &mut Vec<W>, text: String, style: Style| {
        items.push(widget::text(m, None, width, &text, style));
    };
    let off = || s::settings_off().format();
    let on = || s::settings_on().format();

    // Warn before building early
    let early = s::settings_early_build_prompt_label().format();
    gap(&mut items, 8);
    line(&mut items, early.clone(), Style::BODY);
    gap(&mut items, 1);
    line(
        &mut items,
        s::settings_early_build_prompt_description().format(),
        Style::NOTE,
    );
    gap(&mut items, 4);
    items.extend(radios(
        m,
        width,
        &early,
        d.early,
        vec![
            (
                EARLY_PROMPT_OFF as i64,
                off(),
                day::id!(settings_early, "off"),
            ),
            (
                EARLY_PROMPT_SMART as i64,
                s::settings_early_build_prompt_smart().format(),
                day::id!(settings_early, "smart"),
            ),
            (
                EARLY_PROMPT_CONSERVATIVE as i64,
                s::settings_early_build_prompt_conservative().format(),
                day::id!(settings_early, "careful"),
            ),
        ],
    ));

    // Warn before ending a turn
    let end_turn = s::settings_end_turn_prompt_label().format();
    gap(&mut items, 8);
    line(&mut items, end_turn.clone(), Style::BODY);
    gap(&mut items, 4);
    items.extend(radios(
        m,
        width,
        &end_turn,
        d.end_turn,
        vec![
            (false, off(), day::id!(settings_end_turn, "off")),
            (true, on(), day::id!(settings_end_turn, "on")),
        ],
    ));

    // Trader resource classic mode
    gap(&mut items, 8);
    {
        let trader = d.trader;
        items.push(widget::check(
            m,
            day::id!(settings_trader_classic),
            width,
            &s::settings_trader_classic_label().format(),
            switch_states(),
            move || trader.get(),
            move |v| trader.set(v),
        ));
    }
    gap(&mut items, 1);
    line(
        &mut items,
        s::settings_trader_classic_description().format(),
        Style::NOTE,
    );

    // Haptic feedback
    let haptic = s::settings_haptic_feedback_label().format();
    gap(&mut items, 8);
    line(&mut items, haptic.clone(), Style::BODY);
    gap(&mut items, 4);
    items.extend(radios(
        m,
        width,
        &haptic,
        d.haptics,
        vec![
            (
                false,
                s::settings_haptic_feedback_off().format(),
                day::id!(settings_haptics, "off"),
            ),
            (
                true,
                s::settings_haptic_feedback_on().format(),
                day::id!(settings_haptics, "on"),
            ),
        ],
    ));

    // Animate water
    gap(&mut items, 8);
    {
        let water = d.water;
        items.push(widget::check(
            m,
            day::id!(settings_animate_water),
            width,
            &s::settings_animate_water_label().format(),
            switch_states(),
            move || water.get(),
            move |v| water.set(v),
        ));
    }

    // Music volume
    let music = s::settings_music_volume_label().format();
    gap(&mut items, 8);
    line(&mut items, music.clone(), Style::BODY);
    gap(&mut items, 2);
    items.push(slider_row(
        m,
        day::id!(settings_music_volume),
        width,
        &music,
        MUSIC_VOLUME_MIN as i64..=MUSIC_VOLUME_MAX as i64,
        d.music,
    ));
    gap(&mut items, 4);

    // Sound effects
    let sound = s::settings_sound_effects_label().format();
    gap(&mut items, 8);
    line(&mut items, sound.clone(), Style::BODY);
    gap(&mut items, 4);
    items.extend(radios(
        m,
        width,
        &sound,
        d.sound,
        vec![
            (false, off(), day::id!(settings_sound, "off")),
            (true, on(), day::id!(settings_sound, "on")),
        ],
    ));

    // Theme, previewed as it is chosen, in three columns of whole art pixels
    let theme_label = s::settings_theme_label().format();
    gap(&mut items, 8);
    line(&mut items, theme_label.clone(), Style::BODY);
    gap(&mut items, 4);
    let themes = [
        (Theme::System, s::settings_theme_system().format()),
        (Theme::Light, s::settings_theme_light().format()),
        (Theme::Dark, s::settings_theme_dark().format()),
    ];
    let third = width / 3;
    let row: Vec<W> = themes
        .into_iter()
        .enumerate()
        .map(|(i, (theme, label))| {
            let w = if i < 2 { third } else { width - 2 * third };
            let value = d.theme;
            let name = theme.name();
            widget::radio(
                m,
                day::id!(settings_theme, name),
                w,
                &label,
                &format!("{theme_label}: {label}"),
                None,
                radio_states(),
                move || value.get() == name,
                move || {
                    value.set(name.to_string());
                    super::apply_theme(theme);
                },
            )
        })
        .collect();
    items.push(widget::hstack(m, row, 0));

    // Show welcome at start
    let welcome_label = s::settings_welcome_label().format();
    gap(&mut items, 8);
    line(&mut items, welcome_label.clone(), Style::BODY);
    gap(&mut items, 4);
    let half = width / 2;
    let row: Vec<W> = [
        (false, off(), day::id!(settings_welcome, "off"), half),
        (true, on(), day::id!(settings_welcome, "on"), width - half),
    ]
    .into_iter()
    .map(|(v, label, id, w)| {
        let value = d.welcome;
        widget::radio(
            m,
            id,
            w,
            &label,
            &format!("{welcome_label}: {label}"),
            None,
            radio_states(),
            move || value.get() == v,
            move || value.set(v),
        )
    })
    .collect();
    items.push(widget::hstack(m, row, 0));

    // The Dunholm target
    let target = s::settings_race_target_label(island_name(dunholm)).format();
    gap(&mut items, 8);
    line(&mut items, target.clone(), Style::BODY);
    gap(&mut items, 1);
    line(
        &mut items,
        s::settings_race_target_description(dunholm.target_score as i64).format(),
        Style::NOTE,
    );
    gap(&mut items, 2);
    items.push(slider_row(
        m,
        day::id!(settings_race_target),
        width,
        &target,
        RACE_TARGET_MIN as i64..=race_target_max() as i64,
        d.target,
    ));
    gap(&mut items, 4);

    // Player Color, and Show tutorial
    gap(&mut items, 10);
    items.push(
        Button::new(
            day::id!(settings_player_color),
            s::player_color_menu().format(),
        )
        .width(width)
        .build(m, move || on_color()),
    );
    gap(&mut items, 4);
    {
        let tutorial = d.tutorial;
        items.push(
            Button::new(day::id!(settings_tutorial), s::settings_tutorial().format())
                .width(width)
                .build(m, move || {
                    tutorial.set(true);
                    crate::ui::dialog::dismiss_top();
                }),
        );
    }
    gap(&mut items, 2);

    // The language: this build's own addition, last so the rest stays where it was
    let lang = s::settings_language().format();
    gap(&mut items, 8);
    line(&mut items, lang.clone(), Style::BODY);
    gap(&mut items, 4);
    let mut languages = vec![(
        String::new(),
        s::settings_theme_system().format(),
        day::id!(settings_language, "system"),
    )];
    for (tag, name) in res::locales::ALL {
        languages.push((
            tag.to_string(),
            name.to_string(),
            day::id!(settings_language, *tag),
        ));
    }
    items.extend(languages.into_iter().map(|(tag, label, id)| {
        let (mine, pick) = (tag.clone(), tag);
        widget::radio(
            m,
            id,
            width,
            &label,
            &format!("{lang}: {label}"),
            None,
            radio_states(),
            move || day_piece_settings::LANGUAGE.get() == mine,
            move || day_piece_settings::LANGUAGE.set(pick.clone()),
        )
    }));
    widget::vstack(m, items, 0)
}

/// The color picker's content: six presets, a wheel, a brightness slider and a hex field, all
/// bound to one color (player_color_picker.xml). `color` holds it until OK saves it.
pub fn color_picker(m: &Metrics, width: i32, color: Signal<u32>) -> W {
    use res::str as s;
    let hsv = Signal::global(player_color::argb_to_hsv(color.get_untracked()));
    let hex = Signal::global(player_color::to_hex(color.get_untracked()));
    // Whatever is typed takes effect once it is a whole value; a half typed one changes nothing
    Effect::new(move || {
        let typed = hex.get();
        if let Ok(parsed) = player_color::parse_hex(&typed)
            && parsed != untrack(|| color.get())
        {
            color.set(parsed);
            hsv.set(player_color::argb_to_hsv(parsed));
        }
    });
    let set = move |c: u32, new_hsv: (f64, f64, f64)| {
        color.set(c);
        hsv.set(new_hsv);
        hex.set(player_color::to_hex(c));
    };

    let preview = widget::preview(m, width, 20, move || color.get());
    let names = [
        s::color_red(),
        s::color_yellow(),
        s::color_green(),
        s::color_blue(),
        s::color_purple(),
        s::color_brown(),
    ];
    let swatches: Vec<W> = PRESETS
        .iter()
        .zip(names)
        .enumerate()
        .map(|(i, (c, name))| {
            let c = *c;
            widget::swatch(
                m,
                day::id!(picker_swatch, i.to_string()),
                (26, 14),
                0xFF000000 | c,
                &name.format(),
                move || set(c, player_color::argb_to_hsv(c)),
            )
        })
        .collect();
    let swatches = widget::hstack(m, swatches, 4);
    let swatches = widget::center(m, swatches, width);

    let title = s::player_color_title().format();
    let wheel = widget::color_wheel(
        m,
        day::id!(picker_wheel),
        56,
        &title,
        move || {
            let (h, sat, _) = hsv.get();
            (h, sat)
        },
        move |h, sat| {
            let (_, _, v) = hsv.get_untracked();
            set(player_color::hsv_to_argb(h, sat, v), (h, sat, v));
        },
    );
    let wheel = widget::center(m, wheel, width);

    let brightness_label = s::player_color_brightness().format();
    let brightness = widget::slider(
        m,
        day::id!(picker_brightness),
        width,
        &brightness_label,
        0..=255,
        move || (hsv.get().2 * 255.0).round() as i64,
        move |v| {
            let (h, sat, _) = hsv.get_untracked();
            let value = v as f64 / 255.0;
            set(player_color::hsv_to_argb(h, sat, value), (h, sat, value));
        },
    );

    let hex_label = s::player_color_hex_label().format();
    let label_w = kit().body.width(&hex_label) * m.t;
    let field = widget::text_entry(
        m,
        day::id!(picker_hex),
        width - label_w - 6,
        &hex_label,
        hex,
    );
    let label = widget::text(m, None, label_w, &hex_label, Style::BODY);
    let label = widget::vstack(
        m,
        vec![widget::gap(m, label_w, (field.h - label.h) / 2), label],
        0,
    );
    let hex_row = widget::hstack(m, vec![label, widget::gap(m, 6, 1), field], 0);

    let brightness_text = widget::text(m, None, width, &brightness_label, Style::BODY);
    widget::vstack(
        m,
        vec![
            preview,
            widget::gap(m, width, 6),
            swatches,
            widget::gap(m, width, 8),
            wheel,
            widget::gap(m, width, 8),
            brightness_text,
            widget::gap(m, width, 2),
            brightness,
            widget::gap(m, width, 6),
            hex_row,
        ],
        0,
    )
}

/// What the player chose in the welcome. Any other way out is the same as Play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WelcomeChoice {
    HowToPlay,
    Play,
}

/// The first-run welcome: the goal of this game, the other island in a line, four hints each with
/// its glyph, and a "don't show this again" box whose state is read when the dialog closes.
pub fn welcome_dialog(
    game: &Game,
    other_race_target: i32,
    hide: Signal<bool>,
) -> Dialog<WelcomeChoice> {
    use res::str as s;
    let lead = if game.is_race() {
        s::welcome_lead_race(game.target_score as i64).format()
    } else {
        s::welcome_lead_turns(game.map.turn_cap as i64).format()
    };
    let other = MapDef::all()
        .into_iter()
        .find(|m| m.id != game.map.id)
        .map(|other| {
            let name = island_name(other);
            if other.target_score > 0 {
                s::welcome_other_race(s::WelcomeOtherRace {
                    arg1: name,
                    arg2: other_race_target as i64,
                })
                .format()
            } else {
                s::welcome_other_turns(s::WelcomeOtherTurns {
                    arg1: name,
                    arg2: other.turn_cap as i64,
                })
                .format()
            }
        });
    Dialog::new(s::welcome_title().format())
        .content(move |m, width| {
            let mut items = vec![widget::text(m, None, width, &lead, Style::BODY)];
            if let Some(other) = &other {
                items.push(widget::gap(m, width, 4));
                items.push(widget::text(m, None, width, other, Style::BODY));
            }
            let hints = [
                (Icon::Any, s::welcome_dice().format()),
                (Icon::Grain, s::welcome_costs().format()),
                (Icon::Road, s::welcome_roads().format()),
                (Icon::Trader, s::welcome_traders().format()),
            ];
            for (i, (icon, text)) in hints.into_iter().enumerate() {
                items.push(widget::gap(m, width, if i == 0 { 6 } else { 4 }));
                let lead = match icon {
                    Icon::Road => Lead::Player(icon, PLAYER_COLOR.get() as u32),
                    _ => Lead::Icon(icon),
                };
                items.push(widget::lead_text(m, None, width, lead, &text, Style::BODY));
            }
            let label = s::welcome_dont_show().format();
            let box_w = widget::row_width(m, &label).min(width);
            let check = widget::check(
                m,
                day::id!(welcome_dont_show),
                box_w,
                &label,
                switch_states(),
                move || hide.get(),
                move |v| hide.set(v),
            );
            items.push(widget::gap(m, width, 6));
            items.push(widget::center(m, check, width));
            widget::vstack(m, items, 0)
        })
        .button(
            "how_to_play",
            s::welcome_how_to_play().format(),
            WelcomeChoice::HowToPlay,
        )
        .button("play", s::welcome_play().format(), WelcomeChoice::Play)
        .default_button("play")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_its_rules_and_about() {
        for locale in ["en", "de", "es", "fr", "id", "it", "pt-BR"] {
            assert!(rules_source(locale).len() > 500, "{locale}");
            let about = about_source(locale);
            assert!(about.contains("{{version}}"), "{locale}");
            assert!(
                about.contains("https://github.com/Islewright-Roll/Islewright-Roll"),
                "{locale}"
            );
        }
        assert_eq!(language("pt-BR"), "pt");
    }
}
