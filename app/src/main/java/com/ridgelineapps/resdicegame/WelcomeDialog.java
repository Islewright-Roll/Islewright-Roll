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
import android.content.Intent;
import android.graphics.Bitmap;
import android.view.Gravity;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.TextView;

/**
 * The first-run welcome: what the game is about on the map being played, a line on the
 * other island and where to switch to it, four short hints each with its glyph, a "don't show this again" box, and the way to the Rules.
 * A PixelDialog like any other, so it follows the theme. Dismissing it by back or by a
 * tap outside is the same as Play. The box is only read when the dialog closes: if it is
 * ticked then, the "show welcome" setting is turned off.
 *
 * Laid out in art pixels (PixelDialog skins the content): lines of text at the right of
 * a 12 px glyph, a 4 px gap between the glyph and its text.
 */
final class WelcomeDialog {

    private WelcomeDialog() {
    }

    static void show(final Activity activity, Game game) {
        int s = PixelUi.scale(activity);
        MapDef map = game.map;

        LinearLayout column = new LinearLayout(activity);
        column.setOrientation(LinearLayout.VERTICAL);

        // The goal depends on how the island ends: a race to a score, or a turn limit
        // A race names the target this game is played to, which the player may have set
        String lead = game.isRace()
                ? activity.getString(R.string.welcome_lead_race, game.targetScore)
                : activity.getString(R.string.welcome_lead_turns, map.turnCap);
        TextView leadView = new PixelTextView(activity, null);
        leadView.setText(lead);
        column.addView(leadView, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));

        // The other island in one line, and where to switch to it. Built from the map itself,
        // so a third island would need nothing here but a line of its own to name.
        MapDef other = otherMap(map);
        if (other != null) {
            TextView otherView = new PixelTextView(activity, null);
            // The other island's game is not loaded, so a race there is named by the setting
            otherView.setText(other.targetScore > 0
                    ? activity.getString(R.string.welcome_other_race, activity.getString(other.nameRes),
                            HighScoresAndSettings.load(activity).raceTarget)
                    : activity.getString(R.string.welcome_other_turns, activity.getString(other.nameRes), other.turnCap));
            LinearLayout.LayoutParams otherLp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            otherLp.topMargin = 4;
            column.addView(otherView, otherLp);
        }

        int[][] rows = {
                { R.drawable.icon_any_12, R.string.welcome_dice },
                { R.drawable.icon_grain_12, R.string.welcome_costs },
                { R.drawable.glyph_road_12, R.string.welcome_roads },
                { R.drawable.glyph_trader_12, R.string.welcome_traders },
        };
        for (int i = 0; i < rows.length; i++) {
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            lp.topMargin = i == 0 ? 6 : 4;
            column.addView(iconLine(activity, s, rows[i][0], activity.getString(rows[i][1])), lp);
        }

        final int scale = s;
        final PixelCheckBox box = new PixelCheckBox(activity, null) {
            // The dialog body is an even number of art pixels wide (PixelUi.dialogWidth), so
            // a centred child must be even too, or its left edge lands on a half art pixel.
            @Override
            protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
                super.onMeasure(widthMeasureSpec, heightMeasureSpec);
                int width = getMeasuredWidth();
                if ((width / scale) % 2 != 0) {
                    width += scale;
                }
                setMeasuredDimension(width, getMeasuredHeight());
            }
        };
        box.setText(R.string.welcome_dont_show);
        box.setPadding(0, 2, 0, 2);
        // Centred above the buttons, so it reads as belonging to them
        LinearLayout.LayoutParams boxLp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        boxLp.gravity = Gravity.CENTER_HORIZONTAL;
        boxLp.topMargin = 6;
        column.addView(box, boxLp);

        PixelDialog.custom(activity, R.string.welcome_title, column,
                new int[] { R.string.welcome_how_to_play, R.string.welcome_play },
                new Runnable[] { new Runnable() {
                    @Override
                    public void run() {
                        activity.startActivity(new Intent(activity, Rules.class));
                    }
                }, null },
                new Runnable() {
                    @Override
                    public void run() {
                        if (box.isChecked()) {
                            HighScoresAndSettings store = HighScoresAndSettings.load(activity);
                            store.showWelcome = false;
                            store.save(activity);
                        }
                    }
                });
    }

    /** The first island in {@link MapDef#ALL} that is not this one; null if there is only one. */
    static MapDef otherMap(MapDef map) {
        for (MapDef candidate : MapDef.ALL) {
            if (candidate != map) {
                return candidate;
            }
        }
        return null;
    }

    /**
     * One line of text at the right of a glyph, the glyph level with the first line. Sizes
     * are art pixels for PixelUi.skin to scale, except the glyph, which is already its
     * real size. Rules uses it too, for the "[icon] text" lines.
     */
    static View iconLine(Activity activity, int scale, int iconRes, CharSequence text) {
        // A glyph with team magenta in it (the road's kerbs) shows in the player's colour
        Bitmap bitmap = TeamColor.recolor(PixelUi.decode(activity, iconRes), TeamColor.MAGENTA,
                HighScoresAndSettings.load(activity).playerColor);
        LinearLayout row = new LinearLayout(activity);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.TOP);

        PixelImage icon = new PixelImage(activity, bitmap, scale);
        LinearLayout.LayoutParams iconLp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        iconLp.rightMargin = 4;
        // One art pixel down, so the icon sits level with the first line's letters
        iconLp.topMargin = 1;
        row.addView(icon, iconLp);

        TextView label = new PixelTextView(activity, null);
        label.setText(text);
        row.addView(label, new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));
        return row;
    }
}
