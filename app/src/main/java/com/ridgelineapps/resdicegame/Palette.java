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
 * Every colour the pixel kit paints with, in one place, for one look. PixelUi hands out
 * the palette for the current mode (PixelUi.palette()); nothing else in the kit carries
 * a light or dark branch of its own. The dark values are the "dark kit" palette in
 * tools/pieces.txt, with its contrast ratios.
 *
 * The brass buttons are the same in both looks, so their label colours are not here.
 */
final class Palette {

    static final int THEME_SYSTEM = HighScoresAndSettings.THEME_SYSTEM;
    static final int THEME_LIGHT = HighScoresAndSettings.THEME_LIGHT;
    static final int THEME_DARK = HighScoresAndSettings.THEME_DARK;

    /** True for the dark look. Used only by the drawables whose art differs, never by screens. */
    final boolean dark;

    /** Body text, titles, the score rows. */
    final int ink;
    /** Descriptions and other quiet text. */
    final int secondary;
    /** Links. */
    final int link;
    /** The line under a title and the dividers between rows. */
    final int rule;
    /** Text typed into a recessed field, and its hint. */
    final int fieldText;
    final int fieldHint;
    /** The mark in a radio box or a check box. */
    final int mark;
    /** A recessed field, radio box or check box: face, outline, bottom and right inner light (the dark look recolours the pressed button art with these). */
    final int fieldFace;
    final int fieldShadow;
    final int fieldLight;
    /** The inner shade along the top and left of a dark recess, between its outline and its face. */
    final int fieldInner;
    /** The slider thumb's other tones in the dark look: lip, inner edge, shade, pressed face. */
    final int thumbLip;
    final int thumbEdge;
    final int thumbShade;
    final int thumbPressed;
    /** The slider: groove outline, groove face, groove top edge, thumb outline, face, top edge. */
    final int trackOutline;
    final int trackFace;
    final int trackTop;
    final int thumbOutline;
    final int thumbFace;
    final int thumbTop;
    /** The scroll thumb. */
    final int scrollThumb;
    /** The outline around the colour preview. */
    final int frame;

    private Palette(boolean dark, int ink, int secondary, int link, int rule, int fieldText, int fieldHint, int mark,
            int fieldFace, int fieldShadow, int fieldLight, int fieldInner, int trackOutline, int trackFace, int trackTop,
            int thumbOutline, int thumbFace, int thumbTop, int thumbLip, int thumbEdge, int thumbShade, int thumbPressed, int scrollThumb, int frame) {
        this.dark = dark;
        this.ink = ink;
        this.secondary = secondary;
        this.link = link;
        this.rule = rule;
        this.fieldText = fieldText;
        this.fieldHint = fieldHint;
        this.mark = mark;
        this.fieldFace = fieldFace;
        this.fieldShadow = fieldShadow;
        this.fieldLight = fieldLight;
        this.fieldInner = fieldInner;
        this.trackOutline = trackOutline;
        this.trackFace = trackFace;
        this.trackTop = trackTop;
        this.thumbOutline = thumbOutline;
        this.thumbFace = thumbFace;
        this.thumbTop = thumbTop;
        this.thumbLip = thumbLip;
        this.thumbEdge = thumbEdge;
        this.thumbShade = thumbShade;
        this.thumbPressed = thumbPressed;
        this.scrollThumb = scrollThumb;
        this.frame = frame;
    }

    /** Parchment and brass: the look the game shipped with. The field and the thumb are art; these values just document it. */
    static final Palette LIGHT = new Palette(false,
            0xFF2A1A0E, 0xFF5A3C14, 0xFF8C3A10, 0xFFC8A860,
            0xFFFFF4D6, 0xFFC0BCB0, 0xFFFFF4D6,
            0xFF7A5A2C, 0xFF1E140A, 0xFFA0783C, 0xFF5A3C14,
            0xFF1E140A, 0xFF7A5A2C, 0xFF5A3C14,
            0xFF1E140A, 0xFFA0783C, 0xFFE0B870, 0xFFB48C4C, 0xFF86622E, 0xFF5A3C14, 0xFF7A5A2C,
            0xFF5A3C14, 0xFF1E140A);

    /** Darker wood: tools/pieces.txt, "dark kit". */
    static final Palette DARK = new Palette(true,
            0xFFD2BE94, 0xFFB8A47C, 0xFF7DB8D8, 0xFFB8914F,
            0xFFD2BE94, 0xFFB8A47C, 0xFFE0B870,
            0xFF1E150C, 0xFF120B05, 0xFF5A4630, 0xFF181008,
            0xFF120B05, 0xFF1E150C, 0xFF120B05,
            0xFF120B05, 0xFFB8914F, 0xFFE0B870, 0xFFC9A45E, 0xFF8F6F3A, 0xFF6E5530, 0xFF9A7A40,
            0xFFB8A47C, 0xFFB8914F);

    /**
     * One colour of the pressed button art (the recessed field, radio box and check box)
     * as this palette paints it. The art has four colours: transparent, outline, face and
     * one shade used for the top and left bevel and the bottom and right inner line, told
     * apart by {@code lowerRight}. The light palette returns the colour untouched.
     */
    int recolourField(int argb, boolean lowerRight) {
        if (!dark || (argb >>> 24) == 0) {
            return argb;
        }
        switch (argb & 0xFFFFFF) {
            case 0x1E140A: return fieldShadow;
            case 0x7A5A2C: return fieldFace;
            case 0x5A3C14: return lowerRight ? fieldLight : fieldInner;
            default: return argb;
        }
    }

    /** One colour of the button art (the slider thumb, any state) as this palette paints it. */
    int recolourThumb(int argb) {
        if (!dark || (argb >>> 24) == 0) {
            return argb;
        }
        switch (argb & 0xFFFFFF) {
            case 0x1E140A: return thumbOutline;
            case 0xE0B870: return thumbTop;
            case 0xFFF4D6: return thumbTop;
            case 0xA0783C: return thumbFace;
            case 0x5A3C14: return thumbShade;
            case 0xB48C4C: return thumbLip;
            case 0x86622E: return thumbEdge;
            case 0x7A5A2C: return thumbPressed;
            case 0x8C8880: return 0xFF6A655A;
            case 0x6E6A60: return 0xFF4A463E;
            case 0x7C7870: return 0xFF5A564C;
            case 0x5E5A52: return 0xFF3E3A33;
            default: return argb;
        }
    }

    /** Whether a Theme setting, with the OS in night mode or not, means the dark look. */
    static boolean isDark(int theme, boolean osNight) {
        if (theme == THEME_DARK) {
            return true;
        }
        if (theme == THEME_LIGHT) {
            return false;
        }
        return osNight;
    }

    static Palette forMode(int theme, boolean osNight) {
        return isDark(theme, osNight) ? DARK : LIGHT;
    }
}
