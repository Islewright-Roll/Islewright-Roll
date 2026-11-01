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

/**
 * The player's chosen board colour, stored as one opaque ARGB int in memory and one
 * "#RRGGBB" hex string on disk. Presets live here rather than in colors.xml because the
 * JSON default (HighScoresAndSettings) needs a value with no Context available, and the
 * board tint does not vary with the light/dark theme.
 */
public final class PlayerColor {

    // Material 700 shades, chosen to stay distinct from each other and from the board
    // artwork's sand, sea and the green "can build" highlight.
    public static final int[] PRESETS = {
        0xFFD32F2F, // red
        0xFFF5BB30, // orange-yellow
        0xFF1B8E26, // green
        0xFF1718E1, // blue
        0xFF6922A2, // purple
        0xFF763410, // brown
    };

    public static final int DEFAULT = PRESETS[0];

    private PlayerColor() {
    }

    /**
     * Parses a hex colour string permissively, so a power user's hand-edited JSON still
     * loads. Accepts an optional leading "#", surrounding whitespace, any case, and 3, 6
     * or 8 hex digits ("F00" expands to "FF0000"; 8 digits are AARRGGBB with the alpha
     * discarded, since the store never keeps an alpha).
     */
    public static int parseHex(String s) {
        if(s == null) {
            throw new IllegalArgumentException("null color");
        }

        String hex = s.trim();
        if(hex.startsWith("#")) {
            hex = hex.substring(1);
        }

        if(hex.length() == 3) {
            StringBuilder expanded = new StringBuilder(6);
            for(int i = 0; i < 3; i++) {
                char c = hex.charAt(i);
                expanded.append(c).append(c);
            }
            hex = expanded.toString();
        }

        if(hex.length() != 6 && hex.length() != 8) {
            throw new IllegalArgumentException("not a hex color: " + s);
        }

        // Keep only the last 6 digits (RRGGBB); an 8 digit value's leading pair is alpha.
        String rgb = hex.substring(hex.length() - 6);
        long parsed = Long.parseLong(rgb, 16);
        return 0xFF000000 | (int) parsed;
    }

    /** Always "#RRGGBB" uppercase, alpha dropped. */
    public static String toHex(int color) {
        return String.format("#%06X", color & 0xFFFFFF);
    }

    public static int withAlpha(int color, int alpha) {
        return (color & 0xFFFFFF) | ((alpha & 0xFF) << 24);
    }

    /** SRC_OVER composite of color at alpha over an opaque background, for previews. */
    public static int over(int color, int alpha, int background) {
        float a = (alpha & 0xFF) / 255f;

        int r = Math.round(((color >> 16) & 0xFF) * a + ((background >> 16) & 0xFF) * (1 - a));
        int g = Math.round(((color >> 8) & 0xFF) * a + ((background >> 8) & 0xFF) * (1 - a));
        int b = Math.round((color & 0xFF) * a + (background & 0xFF) * (1 - a));

        return 0xFF000000 | (r << 16) | (g << 8) | b;
    }
}
