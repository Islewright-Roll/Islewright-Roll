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
import android.graphics.Color;
import android.text.Editable;
import android.text.TextWatcher;
import android.view.LayoutInflater;
import android.view.View;
import android.widget.EditText;
import android.widget.SeekBar;
import android.widget.SeekBar.OnSeekBarChangeListener;

/**
 * The player colour picker dialog: six preset swatches, a hue/saturation wheel, a
 * brightness slider and an editable hex field, all two-way bound to one current colour.
 * Confirming saves through HighScoresAndSettings, the store PlayerColor already reads
 * and writes; cancelling changes nothing.
 */
public final class PlayerColorPickerDialog {

    /** Notified only when the dialog is confirmed. A cancel calls this never. */
    public interface OnColorChosen {
        void onColorChosen(int color);
    }

    private PlayerColorPickerDialog() {
    }

    public static void show(Activity activity, int initialColor, OnColorChosen callback) {
        new Controller(activity, initialColor, callback).show();
    }

    /**
     * One instance per shown dialog. All the two-way binding lives here: applyColor()
     * is the single place that changes the current colour, and every listener below
     * calls it with the views it already updated itself set to false, so the update
     * never bounces back into the view that started it.
     */
    private static final class Controller {

        private final Activity activity;
        private final OnColorChosen callback;
        private final View rootView;

        private final View preview;
        private final ColorWheelView wheel;
        private final SeekBar brightness;
        private final EditText hexField;

        // Guards the hex TextWatcher against its own programmatic setText calls (a
        // preset tap, a wheel drag or a slider move all write the canonical hex back
        // in), so those never get re-parsed as if the user had typed them.
        private boolean updatingHexProgrammatically = false;

        private int color;

        Controller(Activity activity, int initialColor, OnColorChosen callback) {
            this.activity = activity;
            this.callback = callback;
            this.color = initialColor;

            LayoutInflater inflater = LayoutInflater.from(activity);
            rootView = inflater.inflate(R.layout.player_color_picker, null);

            preview = rootView.findViewById(R.id.player_color_preview);
            wheel = (ColorWheelView) rootView.findViewById(R.id.player_color_wheel);
            brightness = (SeekBar) rootView.findViewById(R.id.player_color_brightness);
            hexField = (EditText) rootView.findViewById(R.id.player_color_hex);

            int[] swatchIds = {
                R.id.player_color_swatch_0, R.id.player_color_swatch_1, R.id.player_color_swatch_2,
                R.id.player_color_swatch_3, R.id.player_color_swatch_4, R.id.player_color_swatch_5,
            };
            for(int i = 0; i < swatchIds.length; i++) {
                View swatch = rootView.findViewById(swatchIds[i]);
                final int presetColor = PlayerColor.PRESETS[i];
                swatch.setBackgroundColor(presetColor);
                swatch.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        applyColor(presetColor, true, true, true);
                    }
                });
            }

            wheel.setOnHueSaturationChangeListener(new ColorWheelView.OnHueSaturationChangeListener() {
                @Override
                public void onHueSaturationChange(float hue, float saturation) {
                    int newColor = combineHsv(hue, saturation, currentValue());
                    applyColor(newColor, false, false, true);
                }
            });

            brightness.setOnSeekBarChangeListener(new OnSeekBarChangeListener() {
                @Override
                public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                    // setProgress() below (a preset tap, a valid typed hex) reports
                    // fromUser == false, so this only ever runs for an actual drag.
                    if(!fromUser) {
                        return;
                    }

                    float[] hsv = new float[3];
                    Color.colorToHSV(color, hsv);
                    int newColor = combineHsv(hsv[0], hsv[1], progress / 255f);
                    applyColor(newColor, false, false, true);
                }

                @Override
                public void onStartTrackingTouch(SeekBar seekBar) {
                }

                @Override
                public void onStopTrackingTouch(SeekBar seekBar) {
                }
            });

            hexField.addTextChangedListener(new TextWatcher() {
                @Override
                public void beforeTextChanged(CharSequence s, int start, int count, int after) {
                }

                @Override
                public void onTextChanged(CharSequence s, int start, int before, int count) {
                }

                @Override
                public void afterTextChanged(Editable s) {
                    if(updatingHexProgrammatically) {
                        return;
                    }

                    // Mid-typing states ("#1", "#1A", "#1A2") are not valid hex, and
                    // that is normal, not an error: parseHex throws for them, and the
                    // catch below simply leaves the colour, wheel, slider and preview
                    // exactly as they were until a full value appears. Nothing about
                    // the field itself is touched here either way, so the user's
                    // cursor and partial text are never disturbed while they type.
                    try {
                        int parsed = PlayerColor.parseHex(s.toString());
                        applyColor(parsed, true, true, false);
                    }
                    catch(IllegalArgumentException e) {
                        // Not a complete hex value yet. Nothing to do.
                    }
                }
            });

            hexField.setOnFocusChangeListener(new View.OnFocusChangeListener() {
                @Override
                public void onFocusChange(View v, boolean hasFocus) {
                    // On losing focus, the field always shows the canonical form of
                    // whatever colour actually ended up applied: uppercase "#RRGGBB"
                    // if the user's own typing landed on a valid value, or a revert to
                    // the last valid colour if it did not.
                    if(!hasFocus) {
                        setHexTextCanonical();
                    }
                }
            });

            applyColor(initialColor, true, true, true);
        }

        private static int combineHsv(float hue, float saturation, float value) {
            return 0xFF000000 | (Color.HSVToColor(new float[] { hue, saturation, value }) & 0xFFFFFF);
        }

        private float currentValue() {
            float[] hsv = new float[3];
            Color.colorToHSV(color, hsv);
            return hsv[2];
        }

        /**
         * The single place that changes the current colour. Each caller passes false
         * for whichever view already reflects the new value on its own (the wheel
         * after a drag, the slider after a drag, the hex field mid-typing), so that
         * view is never redundantly, and re-entrantly, written back into.
         */
        private void applyColor(int newColor, boolean updateWheel, boolean updateBrightness, boolean updateHex) {
            color = newColor;
            preview.setBackgroundColor(color);

            float[] hsv = new float[3];
            Color.colorToHSV(color, hsv);

            if(updateWheel) {
                wheel.setHueAndSaturation(hsv[0], hsv[1]);
            }

            if(updateBrightness) {
                brightness.setProgress(Math.round(hsv[2] * 255f));
            }

            if(updateHex) {
                setHexTextCanonical();
            }
        }

        private void setHexTextCanonical() {
            updatingHexProgrammatically = true;
            hexField.setText(PlayerColor.toHex(color));
            hexField.setSelection(hexField.getText().length());
            updatingHexProgrammatically = false;
        }

        void show() {
            PixelDialog.custom(activity, R.string.player_color_title, rootView, new Runnable() {
                @Override
                public void run() {
                    HighScoresAndSettings store = HighScoresAndSettings.load(activity);
                    store.playerColor = color;
                    store.save(activity);
                    if(callback != null) {
                        callback.onColorChosen(color);
                    }
                }
            }, new Runnable() {
                @Override
                public void run() {
                }
            });
        }
    }
}
