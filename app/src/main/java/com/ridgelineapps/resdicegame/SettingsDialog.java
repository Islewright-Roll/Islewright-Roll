/*
 * Copyright (C) 2012 Resource Dice Game (http://code.google.com/p/android-dice-game)
 *
 * This program is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as published
 * by the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This source code is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
package com.ridgelineapps.resdicegame;

import android.app.Activity;
import android.view.LayoutInflater;
import android.view.View;
import android.widget.CheckBox;
import android.widget.LinearLayout;
import android.widget.RadioGroup;
import android.widget.SeekBar;
import android.widget.TextView;

/**
 * The settings dialog: the two prompt options, the Dunholm target slider, the trader
 * resource classic mode check box, a Player Color button, the haptics switch, the sound
 * effects switch, the music volume slider, the theme and the welcome switch, each a radio
 * group (the target and the volume sliders), and a Show tutorial button. Confirming saves
 * the prompts, target, trader mode, haptics, music volume, theme, welcome and tutorial
 * through HighScoresAndSettings and reports the game's values back so the caller can copy
 * them onto the live Game (the trader mode it reads back from the store); cancelling
 * changes nothing.
 *
 * The theme is previewed as it is chosen: the dialog (and nothing else is open under it)
 * is repainted at once in the look picked. Cancelling, or any other way out, puts the
 * stored look back; confirming has already saved the new one.
 *
 * The player colour is the one value this dialog does not save. Its button opens the
 * colour picker over this dialog, and the picker saves the colour itself and reports it
 * to its own callback; OK and Cancel here leave it as the picker left it.
 */
public final class SettingsDialog {

    /** Notified only when the dialog is confirmed. A cancel calls this never. */
    public interface OnSettingsChosen {
        void onSettingsChosen(int earlyBuildPrompt, boolean endTurnPrompt, boolean hapticFeedback, int raceTarget);
    }

    private SettingsDialog() {
    }

    /** onColorChosen is handed to the colour picker the Player Color button opens. */
    public static void show(Activity activity, int initialEarlyBuildPrompt, boolean initialEndTurnPrompt,
            boolean initialHapticFeedback, OnSettingsChosen callback, PlayerColorPickerDialog.OnColorChosen onColorChosen) {
        new Controller(activity, initialEarlyBuildPrompt, initialEndTurnPrompt, initialHapticFeedback, callback, onColorChosen).show();
    }

    /** One instance per shown dialog. */
    private static final class Controller {

        private final Activity activity;
        private final OnSettingsChosen callback;
        private final PlayerColorPickerDialog.OnColorChosen onColorChosen;
        private final View rootView;

        private final RadioGroup earlyBuildPromptGroup;
        private final RadioGroup endTurnPromptGroup;
        private final RadioGroup hapticFeedbackGroup;
        private final RadioGroup soundEffectsGroup;
        private final CheckBox traderClassic;
        private final RadioGroup themeGroup;
        private final RadioGroup welcomeGroup;
        private final View showTutorial;
        private android.app.Dialog dialog;
        private final SeekBar musicVolumeBar;
        private final TextView musicVolumeValue;
        private final SeekBar raceTargetBar;
        private final TextView raceTargetValue;
        private final View playerColor;

        Controller(Activity activity, int initialEarlyBuildPrompt, boolean initialEndTurnPrompt,
                boolean initialHapticFeedback, OnSettingsChosen callback, PlayerColorPickerDialog.OnColorChosen onColorChosen) {
            this.activity = activity;
            this.callback = callback;
            this.onColorChosen = onColorChosen;

            LayoutInflater inflater = LayoutInflater.from(activity);
            rootView = inflater.inflate(R.layout.settings, null);

            earlyBuildPromptGroup = (RadioGroup) rootView.findViewById(R.id.settings_early_build_prompt_group);
            endTurnPromptGroup = (RadioGroup) rootView.findViewById(R.id.settings_end_turn_prompt_group);

            earlyBuildPromptGroup.check(idForEarlyBuildPrompt(initialEarlyBuildPrompt));
            hapticFeedbackGroup = (RadioGroup) rootView.findViewById(R.id.settings_haptic_feedback_group);
            endTurnPromptGroup.check(initialEndTurnPrompt ? R.id.settings_end_turn_prompt_on : R.id.settings_end_turn_prompt_off);
            hapticFeedbackGroup.check(initialHapticFeedback ? R.id.settings_haptic_feedback_on : R.id.settings_haptic_feedback_off);

            HighScoresAndSettings stored = HighScoresAndSettings.load(activity);
            themeGroup = (RadioGroup) rootView.findViewById(R.id.settings_theme_group);
            welcomeGroup = (RadioGroup) rootView.findViewById(R.id.settings_welcome_group);
            // Three equal columns would be 66.67 art px each. Whole art pixels, so the second
            // and third label start on the grid: the last one takes what is left over.
            int innerArt = PixelUi.dialogWidth(activity, PixelUi.scale(activity)) / PixelUi.scale(activity) - 16;
            int[] themeIds = { R.id.settings_theme_system, R.id.settings_theme_light, R.id.settings_theme_dark };
            for(int i = 0; i < themeIds.length; i++) {
                View radio = rootView.findViewById(themeIds[i]);
                LinearLayout.LayoutParams lp = (LinearLayout.LayoutParams) radio.getLayoutParams();
                lp.weight = 0;
                lp.width = i < 2 ? innerArt / 3 : innerArt - 2 * (innerArt / 3);
            }
            themeGroup.check(idForTheme(stored.theme));
            welcomeGroup.check(stored.showWelcome ? R.id.settings_welcome_on : R.id.settings_welcome_off);
            showTutorial = rootView.findViewById(R.id.settings_show_tutorial);
            soundEffectsGroup = (RadioGroup) rootView.findViewById(R.id.settings_sound_effects_group);
            soundEffectsGroup.check(stored.soundEffects ? R.id.settings_sound_effects_on : R.id.settings_sound_effects_off);
            traderClassic = (CheckBox) rootView.findViewById(R.id.settings_trader_classic);
            traderClassic.setChecked(stored.traderClassic);

            // The slider previews as it moves: the music follows it at once. Cancel, or any
            // other way out, puts the stored volume back (the dismissal callback below).
            musicVolumeBar = (SeekBar) rootView.findViewById(R.id.settings_music_volume);
            musicVolumeValue = (TextView) rootView.findViewById(R.id.settings_music_volume_value);
            musicVolumeBar.setMax(HighScoresAndSettings.MUSIC_VOLUME_MAX);
            musicVolumeBar.setProgress(stored.musicVolume);
            musicVolumeValue.setText(String.valueOf(stored.musicVolume));
            // The target slider runs from 1, where a SeekBar runs from 0: progress is the target
            // less one. Its top is everything the island has to build, so no target is out of reach.
            String dunholm = activity.getString(R.string.map_dunholm);
            ((TextView) rootView.findViewById(R.id.settings_race_target_label)).setText(
                    activity.getString(R.string.settings_race_target_label, dunholm));
            ((TextView) rootView.findViewById(R.id.settings_race_target_description)).setText(
                    activity.getString(R.string.settings_race_target_description, MapDef.DUNHOLM.targetScore));
            raceTargetBar = (SeekBar) rootView.findViewById(R.id.settings_race_target);
            raceTargetValue = (TextView) rootView.findViewById(R.id.settings_race_target_value);
            raceTargetBar.setMax(HighScoresAndSettings.raceTargetMax() - HighScoresAndSettings.RACE_TARGET_MIN);
            raceTargetBar.setProgress(stored.raceTarget - HighScoresAndSettings.RACE_TARGET_MIN);
            raceTargetValue.setText(String.valueOf(stored.raceTarget));
            raceTargetBar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
                @Override
                public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                    raceTargetValue.setText(String.valueOf(progress + HighScoresAndSettings.RACE_TARGET_MIN));
                }

                @Override
                public void onStartTrackingTouch(SeekBar seekBar) {
                }

                @Override
                public void onStopTrackingTouch(SeekBar seekBar) {
                }
            });

            playerColor = rootView.findViewById(R.id.settings_player_color);

            musicVolumeBar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
                @Override
                public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                    musicVolumeValue.setText(String.valueOf(progress));
                    MusicPlayer music = MusicPlayer.get();
                    if(music != null) {
                        music.setVolumeSetting(progress, fromUser);
                    }
                }

                @Override
                public void onStartTrackingTouch(SeekBar seekBar) {
                }

                @Override
                public void onStopTrackingTouch(SeekBar seekBar) {
                }
            });
            themeGroup.setOnCheckedChangeListener(new RadioGroup.OnCheckedChangeListener() {
                @Override
                public void onCheckedChanged(RadioGroup group, int checkedId) {
                    PixelUi.preview(Controller.this.activity, themeForId(checkedId));
                    PixelDialog.restyleOpen();
                }
            });
        }

        private static int idForTheme(int theme) {
            if(theme == HighScoresAndSettings.THEME_LIGHT) {
                return R.id.settings_theme_light;
            }
            if(theme == HighScoresAndSettings.THEME_DARK) {
                return R.id.settings_theme_dark;
            }
            return R.id.settings_theme_system;
        }

        private static int themeForId(int checkedId) {
            if(checkedId == R.id.settings_theme_light) {
                return HighScoresAndSettings.THEME_LIGHT;
            }
            if(checkedId == R.id.settings_theme_dark) {
                return HighScoresAndSettings.THEME_DARK;
            }
            return HighScoresAndSettings.THEME_SYSTEM;
        }

        /** Anything not recognised (a stray value from a hand-edited file) falls back to Smart. */
        private static int idForEarlyBuildPrompt(int earlyBuildPrompt) {
            if(earlyBuildPrompt == HighScoresAndSettings.EARLY_PROMPT_OFF) {
                return R.id.settings_early_build_prompt_off;
            }
            if(earlyBuildPrompt == HighScoresAndSettings.EARLY_PROMPT_CONSERVATIVE) {
                return R.id.settings_early_build_prompt_conservative;
            }
            return R.id.settings_early_build_prompt_smart;
        }

        private static int earlyBuildPromptForId(int checkedId) {
            if(checkedId == R.id.settings_early_build_prompt_off) {
                return HighScoresAndSettings.EARLY_PROMPT_OFF;
            }
            if(checkedId == R.id.settings_early_build_prompt_conservative) {
                return HighScoresAndSettings.EARLY_PROMPT_CONSERVATIVE;
            }
            return HighScoresAndSettings.EARLY_PROMPT_SMART;
        }

        /**
         * Saves what the dialog shows and reports it back: what OK does. Show tutorial does
         * the same and also starts the tutorial over, so nothing chosen in the dialog is lost.
         */
        private void apply(boolean startTutorial) {
            {
                {
                    int earlyBuildPrompt = earlyBuildPromptForId(earlyBuildPromptGroup.getCheckedRadioButtonId());
                    boolean endTurnPrompt = endTurnPromptGroup.getCheckedRadioButtonId() == R.id.settings_end_turn_prompt_on;

                    boolean hapticFeedback = hapticFeedbackGroup.getCheckedRadioButtonId() == R.id.settings_haptic_feedback_on;

                    HighScoresAndSettings store = HighScoresAndSettings.load(activity);
                    store.earlyBuildPrompt = earlyBuildPrompt;
                    store.endTurnPrompt = endTurnPrompt;
                    store.hapticFeedback = hapticFeedback;
                    store.soundEffects = soundEffectsGroup.getCheckedRadioButtonId() == R.id.settings_sound_effects_on;
                    store.traderClassic = traderClassic.isChecked();
                    store.theme = themeForId(themeGroup.getCheckedRadioButtonId());
                    store.showWelcome = welcomeGroup.getCheckedRadioButtonId() == R.id.settings_welcome_on;
                    if(startTutorial) {
                        store.tutorial = true;
                        store.tutorialSeen = 0;
                    }
                    store.musicVolume = musicVolumeBar.getProgress();
                    int raceTarget = raceTargetBar.getProgress() + HighScoresAndSettings.RACE_TARGET_MIN;
                    store.raceTarget = raceTarget;
                    store.save(activity);
                    // Off releases the pool and plays nothing; On loads it (see SoundEffects)
                    SoundEffects effects = SoundEffects.get();
                    if(effects != null) {
                        effects.setEnabled(store.soundEffects);
                    }
                    // The dialog is already gone, and its dismissal put the old look back
                    PixelUi.refreshTheme(activity);

                    if(callback != null) {
                        callback.onSettingsChosen(earlyBuildPrompt, endTurnPrompt, hapticFeedback, raceTarget);
                    }
                }
            }
        }

        void show() {
            // Over this dialog, which stays open beneath it with whatever has been chosen so far
            playerColor.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    PlayerColorPickerDialog.show(activity, HighScoresAndSettings.load(activity).playerColor, onColorChosen);
                }
            });
            showTutorial.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    apply(true);
                    if(dialog != null) {
                        dialog.dismiss();
                    }
                }
            });
            dialog = PixelDialog.custom(activity, R.string.settings_title, rootView, new Runnable() {
                @Override
                public void run() {
                    apply(false);
                }
            }, new Runnable() {
                @Override
                public void run() {
                }
            }, new Runnable() {
                @Override
                public void run() {
                    // However the dialog closed, the stored look and music volume are the ones that stand
                    PixelUi.refreshTheme(activity);
                    PixelDialog.restyleOpen();
                    MusicPlayer music = MusicPlayer.get();
                    if(music != null) {
                        music.setVolumeSetting(HighScoresAndSettings.load(activity).musicVolume, false);
                    }
                }
            });
        }
    }
}
