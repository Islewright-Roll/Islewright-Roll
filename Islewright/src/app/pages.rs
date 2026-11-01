// SPDX-License-Identifier: GPL-3.0-or-later
//! What the menu's pages hold: How to play, Top Scores, Settings and About, each built from the
//! pixel-art kit for the width its page gives it (`ui::page` draws the frame round them).
//! Settings are bound straight to the `Pref`s in `prefs.rs`, so a change applies at once.

use day::prelude::*;
use day_piece_settings::prelude::{Pref, PrefValue};

use super::prefs::*;
use crate::res;
use crate::rules::high_scores::{
    self, EARLY_PROMPT_CONSERVATIVE, EARLY_PROMPT_OFF, EARLY_PROMPT_SMART, HighScoresAndSettings,
    RACE_TARGET_MIN, race_target_max,
};
use crate::rules::map_def::MapDef;
use crate::rules::player_color::PRESETS;
use crate::rules::rules_markup::{self, Kind};
use crate::ui::paint::PALETTE;
use crate::ui::text::{Align, Face};
use crate::ui::widget::{self, Lead, States};
use crate::ui::{Icon, Metrics, Style, W};
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

/// The gap above a heading that follows other content, on top of the page's own spacing.
const SECTION_GAP: i32 = 6;

/// How to play: the rules text as headings, paragraphs, bullets, icon lines and pictures, in
/// the current language. The pictures are the game's own art at a whole-number size.
pub fn rules(m: &Metrics, width: i32) -> Vec<W> {
    let locale = day::locale().get_untracked();
    let mut items = Vec::new();
    for b in rules_markup::parse(Some(rules_source(&locale))) {
        let text = b.text.clone().unwrap_or_default();
        let icon = b.name.as_deref().and_then(Icon::by_name);
        match b.kind {
            Kind::Heading => {
                if !items.is_empty() {
                    items.push(widget::gap(m, width, SECTION_GAP));
                }
                items.push(widget::text(m, None, width, &text, Style::HEADING));
            }
            Kind::Paragraph => items.push(widget::text(m, None, width, &text, Style::BODY)),
            Kind::Bullet => items.push(widget::lead_text(
                m,
                None,
                width,
                Lead::Bullet,
                &text,
                Style::BODY,
            )),
            Kind::Icon => {
                let lead = icon.map_or(Lead::None, Lead::Icon);
                items.push(widget::lead_text(m, None, width, lead, &text, Style::BODY));
            }
            Kind::Image => {
                if let Some(icon) = icon {
                    items.push(widget::picture(m, width, icon, 2));
                }
            }
        }
    }
    items
}

/// Top Scores: each island's list, best first, one line per score read as one sentence.
pub fn scores(m: &Metrics, width: i32) -> Vec<W> {
    let scores: HighScoresAndSettings = super::store::load_scores();
    let mut items = Vec::new();
    for map in MapDef::all() {
        if !items.is_empty() {
            items.push(widget::gap(m, width, SECTION_GAP));
        }
        items.push(widget::text(
            m,
            None,
            width,
            &island_name(map),
            Style::HEADING,
        ));
        let list = scores.scores_for(&map.id);
        if list.iter().all(|s| s.score <= 0) {
            items.push(widget::text(
                m,
                None,
                width,
                &res::str::high_scores_default().format(),
                Style::BODY,
            ));
            continue;
        }
        let t = m.t;
        // The rank right aligned in a column that holds "10.", then a clear gap before the points
        let rank_w = 24 * t;
        let points_x = rank_w + 10 * t;
        for (i, s) in list.iter().enumerate().filter(|(_, s)| s.score > 0) {
            let rank = format!("{}.", i + 1);
            let score = s.score.to_string();
            // The day it was set; the time of day the document also keeps says nothing here
            let date: String = high_scores::format_date(s.date).chars().take(10).collect();
            let spoken = format!("{rank} {score}, {date}");
            let id = day::id!(score_line, format!("{}-{}", map.id, i + 1));
            items.push(widget::cells(
                m,
                Some(id),
                width,
                vec![
                    (rank, 0, rank_w, Align::End, PALETTE.ink, Face::Display),
                    (
                        score,
                        points_x,
                        width / 2 - points_x,
                        Align::Start,
                        PALETTE.ink,
                        Face::Display,
                    ),
                    (
                        date,
                        width / 2,
                        width - width / 2,
                        Align::End,
                        PALETTE.secondary,
                        Face::Body,
                    ),
                ],
                &spoken,
            ));
        }
    }
    items
}

/// On and Off, as a switch's value is read.
fn switch_states() -> States {
    States {
        on: res::str::settings_on().format(),
        off: res::str::settings_off().format(),
    }
}

/// Selected and not, as a radio button's value is read.
fn radio_states() -> States {
    States {
        on: res::str::ui_selected().format(),
        off: res::str::ui_not_selected().format(),
    }
}

/// A check box bound to a boolean pref.
fn check_pref(m: &Metrics, id: ElementId, width: i32, label: &str, pref: &'static Pref<bool>) -> W {
    widget::check(
        m,
        id,
        width,
        label,
        switch_states(),
        move || pref.get(),
        move |on| pref.set(on),
    )
}

/// A group of radio buttons bound to a pref, each read with the group's name before its own.
fn radio_group<T: Clone + PartialEq + PrefValue + 'static>(
    m: &Metrics,
    width: i32,
    group: &str,
    pref: &'static Pref<T>,
    options: Vec<(T, String, Option<u32>, ElementId)>,
) -> Vec<W> {
    options
        .into_iter()
        .map(|(value, label, swatch, id)| {
            let (mine, pick) = (value.clone(), value);
            widget::radio(
                m,
                id,
                width,
                &label,
                &format!("{group}: {label}"),
                swatch,
                radio_states(),
                move || pref.get() == mine,
                move || pref.set(pick.clone()),
            )
        })
        .collect()
}

/// Settings: the game's options, the player color, and the language, each bound to its pref.
pub fn settings(m: &Metrics, width: i32) -> Vec<W> {
    use res::str as s;
    let mut items = Vec::new();
    let note = |items: &mut Vec<W>, text: String| {
        items.push(widget::text(m, None, width, &text, Style::NOTE));
    };
    let heading = |items: &mut Vec<W>, text: String| {
        if !items.is_empty() {
            items.push(widget::gap(m, width, SECTION_GAP));
        }
        items.push(widget::text(m, None, width, &text, Style::HEADING));
    };

    heading(&mut items, s::settings_game_group().format());
    items.push(check_pref(
        m,
        day::id!(settings_sound),
        width,
        &s::settings_sound_effects_label().format(),
        &SOUND_EFFECTS,
    ));
    items.push(check_pref(
        m,
        day::id!(settings_haptics),
        width,
        &s::settings_haptic_feedback_label().format(),
        &HAPTICS,
    ));

    let early = s::settings_early_build_prompt_label().format();
    items.push(widget::text(m, None, width, &early, Style::BODY));
    items.extend(radio_group(
        m,
        width,
        &early,
        &EARLY_BUILD_PROMPT,
        vec![
            (
                EARLY_PROMPT_OFF as i64,
                s::settings_off().format(),
                None,
                day::id!(settings_early, "off"),
            ),
            (
                EARLY_PROMPT_SMART as i64,
                s::settings_early_smart().format(),
                None,
                day::id!(settings_early, "smart"),
            ),
            (
                EARLY_PROMPT_CONSERVATIVE as i64,
                s::settings_early_careful().format(),
                None,
                day::id!(settings_early, "careful"),
            ),
        ],
    ));
    note(&mut items, s::settings_early_caption().format());

    items.push(check_pref(
        m,
        day::id!(settings_end_turn),
        width,
        &s::settings_end_turn_short().format(),
        &END_TURN_PROMPT,
    ));
    note(&mut items, s::settings_end_turn_caption().format());

    items.push(check_pref(
        m,
        day::id!(settings_trader_classic),
        width,
        &s::settings_trader_classic_short().format(),
        &TRADER_CLASSIC,
    ));
    note(
        &mut items,
        s::settings_trader_classic_description().format(),
    );

    let dunholm = MapDef::dunholm();
    let target = s::settings_race_target_label(island_name(dunholm)).format();
    items.push(widget::text(m, None, width, &target, Style::BODY));
    items.push(widget::stepper(
        m,
        day::id!(settings_race_target),
        &target,
        RACE_TARGET_MIN as i64..=race_target_max() as i64,
        (
            day::id!(settings_race_target_less),
            day::id!(settings_race_target_more),
        ),
        (s::ui_decrease().format(), s::ui_increase().format()),
        move || RACE_TARGET.get(),
        move |v| RACE_TARGET.set(v),
    ));
    note(
        &mut items,
        s::settings_race_target_description(dunholm.target_score as i64).format(),
    );

    items.push(check_pref(
        m,
        day::id!(settings_tutorial),
        width,
        &s::settings_tutorial().format(),
        &TUTORIAL,
    ));
    items.push(check_pref(
        m,
        day::id!(settings_welcome),
        width,
        &s::settings_welcome_label().format(),
        &SHOW_WELCOME,
    ));

    let color = s::player_color_title().format();
    heading(&mut items, color.clone());
    let names = [
        s::color_red(),
        s::color_yellow(),
        s::color_green(),
        s::color_blue(),
        s::color_purple(),
        s::color_brown(),
    ];
    let colors = PRESETS
        .iter()
        .zip(names)
        .enumerate()
        .map(|(i, (c, name))| {
            (
                *c as i64,
                name.format(),
                Some(*c & 0xFFFFFF),
                day::id!(settings_color, i.to_string()),
            )
        })
        .collect();
    items.extend(radio_group(m, width, &color, &PLAYER_COLOR, colors));

    let lang = s::settings_language().format();
    heading(&mut items, lang.clone());
    let mut languages = vec![(
        String::new(),
        s::settings_theme_system().format(),
        None,
        day::id!(settings_language, "system"),
    )];
    for (tag, name) in res::locales::ALL {
        languages.push((
            tag.to_string(),
            name.to_string(),
            None,
            day::id!(settings_language, *tag),
        ));
    }
    items.extend(radio_group(
        m,
        width,
        &lang,
        &day_piece_settings::LANGUAGE,
        languages,
    ));
    items
}

/// About: the name and version, what the game is, its links, and the credits, in the current
/// language. A line of the text that is only a link becomes a link.
pub fn about(m: &Metrics, width: i32) -> Vec<W> {
    let locale = day::locale().get_untracked();
    let version = env!("CARGO_PKG_VERSION");
    let source = about_source(&locale).replace("{{version}}", version);
    let mut items = Vec::new();
    let mut links = 0;
    for (i, paragraph) in source.split("\n\n").enumerate() {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        if i == 0 {
            items.push(widget::text(m, None, width, paragraph, Style::HEADING));
            continue;
        }
        let mut text = String::new();
        let flush = |items: &mut Vec<W>, text: &mut String| {
            if !text.is_empty() {
                items.push(widget::text(m, None, width, text, Style::BODY));
                text.clear();
            }
        };
        for line in paragraph.lines() {
            let line = line.trim();
            if line.starts_with("https://") && !line.contains(' ') {
                flush(&mut items, &mut text);
                items.push(widget::link(
                    m,
                    day::id!(about_url, links.to_string()),
                    width,
                    line,
                    line,
                ));
                links += 1;
            } else {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(line);
            }
        }
        flush(&mut items, &mut text);
    }
    items.push(widget::gap(m, width, SECTION_GAP));
    items.push(widget::text(
        m,
        None,
        width,
        &res::str::about_website().format(),
        Style::BODY,
    ));
    items.push(widget::link(
        m,
        day::id!(about_website),
        width,
        "https://islewright-roll.github.io/islewright/",
        "https://islewright-roll.github.io/islewright/",
    ));
    items
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
                about.contains("https://github.com/islewright-roll/islewright"),
                "{locale}"
            );
        }
        assert_eq!(language("pt-BR"), "pt");
    }
}
