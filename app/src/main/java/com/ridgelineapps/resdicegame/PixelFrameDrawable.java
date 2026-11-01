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

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.ColorFilter;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.graphics.drawable.Drawable;

/**
 * A 9-slice frame drawn nearest neighbour at an integer scale. The art is 24x24 with a 6
 * px corner; the edges and the centre of that art are flat along their stretch, so each
 * is stretched from a one pixel sliver and the result is always whole art pixels. The
 * drawn rectangle is snapped down to the art grid, so a bounds that is not a multiple of
 * the scale never produces half-size pixel blocks.
 *
 * Stateful: a button frame swaps to the pressed art while pressed and to the disabled
 * art while disabled.
 *
 * Three kinds follow the palette as they draw, so a screen changes look without being
 * rebuilt: the panel swaps to the dark panel art; the recessed field (and the radio and
 * check box, which are the same recess) and the slider thumb use the same button art in
 * both looks, recoloured once at load for the dark look by Palette (nearest neighbour,
 * one colour at a time, cached), so the dark shapes are the light shapes. The brass
 * buttons themselves are the same in both.
 */
final class PixelFrameDrawable extends Drawable {

    private static final int ART = 24;
    private static final int CORNER = 6;

    private static final int PANEL = 0;
    private static final int BUTTON = 1;
    private static final int FIELD = 2;
    private static final int THUMB = 3;

    private static Bitmap panelArt;
    private static Bitmap darkPanelArt;
    private static Bitmap buttonArt;
    private static Bitmap pressedArt;
    private static Bitmap disabledArt;
    private static Bitmap darkFieldArt;
    private static Bitmap darkThumbNormal;
    private static Bitmap darkThumbPressed;
    private static Bitmap darkThumbDisabled;

    private final int scale;
    private final int kind;
    private final Bitmap normal;
    private final Bitmap pressed;
    private final Bitmap disabled;

    private Bitmap current;
    private int intrinsicWidth = -1;
    private int intrinsicHeight = -1;

    private final Rect src = new Rect();
    private final Rect dst = new Rect();

    private PixelFrameDrawable(int scale, int kind, Bitmap normal, Bitmap pressed, Bitmap disabled) {
        this.scale = scale;
        this.kind = kind;
        this.normal = normal;
        this.pressed = pressed;
        this.disabled = disabled;
        this.current = normal;
    }

    static synchronized PixelFrameDrawable panel(Context context, int scale) {
        if (panelArt == null) {
            panelArt = PixelUi.decode(context, R.drawable.ui_panel);
            darkPanelArt = PixelUi.decode(context, R.drawable.ui_panel_dark);
        }
        return new PixelFrameDrawable(scale, PANEL, panelArt, panelArt, panelArt);
    }

    static synchronized PixelFrameDrawable button(Context context, int scale) {
        if (buttonArt == null) {
            buttonArt = PixelUi.decode(context, R.drawable.ui_button_normal);
            pressedArt = PixelUi.decode(context, R.drawable.ui_button_pressed);
            disabledArt = PixelUi.decode(context, R.drawable.ui_button_disabled);
        }
        return new PixelFrameDrawable(scale, BUTTON, buttonArt, pressedArt, disabledArt);
    }

    /** A recessed frame for text fields and boxes: the same in every state. */
    static synchronized PixelFrameDrawable field(Context context, int scale) {
        if (pressedArt == null) {
            button(context, scale);
        }
        return new PixelFrameDrawable(scale, FIELD, pressedArt, pressedArt, pressedArt);
    }

    /** The slider thumb: the button art, recoloured in the dark look. */
    static synchronized PixelFrameDrawable thumb(Context context, int scale) {
        if (buttonArt == null) {
            button(context, scale);
        }
        return new PixelFrameDrawable(scale, THUMB, buttonArt, pressedArt, disabledArt);
    }

    /** For a drawable used as a slider thumb, which has no layout of its own. */
    void setSize(int width, int height) {
        intrinsicWidth = width;
        intrinsicHeight = height;
    }

    @Override
    public int getIntrinsicWidth() {
        return intrinsicWidth;
    }

    @Override
    public int getIntrinsicHeight() {
        return intrinsicHeight;
    }

    @Override
    public boolean isStateful() {
        return true;
    }

    @Override
    protected boolean onStateChange(int[] state) {
        boolean enabled = false;
        boolean isPressed = false;
        for (int s : state) {
            if (s == android.R.attr.state_enabled) {
                enabled = true;
            }
            else if (s == android.R.attr.state_pressed) {
                isPressed = true;
            }
        }
        Bitmap next = !enabled ? disabled : (isPressed ? pressed : normal);
        if (next != current) {
            current = next;
            invalidateSelf();
            return true;
        }
        return false;
    }

    @Override
    public void draw(Canvas canvas) {
        Rect b = getBounds();
        int artW = b.width() / scale;
        int artH = b.height() / scale;
        if (artW < 2 || artH < 2) {
            return;
        }
        int x0 = Math.floorDiv(b.left, scale) * scale;
        int y0 = Math.floorDiv(b.top, scale) * scale;

        Palette palette = PixelUi.palette();
        Bitmap art = current;
        if (palette.dark) {
            art = darkArt(kind, art);
        }

        int cw = Math.min(CORNER, artW / 2);
        int ch = Math.min(CORNER, artH / 2);
        int midW = artW - 2 * cw;
        int midH = artH - 2 * ch;
        int mid = CORNER;

        // Columns: left corner, stretched middle, right corner. Rows likewise.
        int[] srcX = { 0, mid, ART - cw };
        int[] srcW = { cw, 1, cw };
        int[] dstX = { 0, cw, artW - cw };
        int[] dstW = { cw, midW, cw };
        int[] srcY = { 0, mid, ART - ch };
        int[] srcH = { ch, 1, ch };
        int[] dstY = { 0, ch, artH - ch };
        int[] dstH = { ch, midH, ch };

        for (int row = 0; row < 3; row++) {
            if (dstH[row] <= 0) {
                continue;
            }
            for (int col = 0; col < 3; col++) {
                if (dstW[col] <= 0) {
                    continue;
                }
                src.set(srcX[col], srcY[row], srcX[col] + srcW[col], srcY[row] + srcH[row]);
                dst.set(x0 + dstX[col] * scale, y0 + dstY[row] * scale,
                        x0 + (dstX[col] + dstW[col]) * scale, y0 + (dstY[row] + dstH[row]) * scale);
                canvas.drawBitmap(art, src, dst, PixelUi.nearestPaint());
            }
        }
    }

    /**
     * The dark version of a piece of light art: the same pixels with each colour swapped
     * by Palette, made once and kept. The panel has its own dark art; the buttons do not
     * change.
     */
    private Bitmap darkArt(int kind, Bitmap art) {
        if (kind == PANEL) {
            return darkPanelArt;
        }
        synchronized (PixelFrameDrawable.class) {
            if (kind == FIELD) {
                if (darkFieldArt == null) {
                    darkFieldArt = recolour(art, true);
                }
                return darkFieldArt;
            }
            if (kind == THUMB) {
                if (art == buttonArt) {
                    if (darkThumbNormal == null) {
                        darkThumbNormal = recolour(art, false);
                    }
                    return darkThumbNormal;
                }
                if (art == pressedArt) {
                    if (darkThumbPressed == null) {
                        darkThumbPressed = recolour(art, false);
                    }
                    return darkThumbPressed;
                }
                if (darkThumbDisabled == null) {
                    darkThumbDisabled = recolour(art, false);
                }
                return darkThumbDisabled;
            }
        }
        return art;
    }

    /** Every pixel through Palette.DARK's colour swap; nearest neighbour by construction. */
    static Bitmap recolour(Bitmap art, boolean field) {
        int w = art.getWidth();
        int h = art.getHeight();
        int[] px = new int[w * h];
        art.getPixels(px, 0, w, 0, 0, w, h);
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                int c = px[y * w + x];
                px[y * w + x] = field ? Palette.DARK.recolourField(c, x + y >= w) : Palette.DARK.recolourThumb(c);
            }
        }
        return Bitmap.createBitmap(px, w, h, Bitmap.Config.ARGB_8888);
    }

    @Override
    public void setAlpha(int alpha) {
    }

    @Override
    public void setColorFilter(ColorFilter colorFilter) {
    }

    @Override
    public int getOpacity() {
        return PixelFormat.TRANSLUCENT;
    }
}
