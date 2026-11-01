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
import android.graphics.Rect;
import android.view.View;

/**
 * A piece of pixel art shown at the kit's integer scale, nearest neighbour: every art
 * pixel of the bitmap is an exact scale x scale block, and the view is exactly that big.
 * Used for the 12 px glyphs beside a line of text and for the pictures in the rules.
 *
 * The bitmap must have been decoded unscaled (PixelUi.decode). A picture that is wider
 * than its column is made smaller by a whole factor first (fit), never by a fraction.
 */
final class PixelImage extends View {

    private final Bitmap bitmap;
    private final int scale;
    private final Rect src;
    private final Rect dst = new Rect();

    PixelImage(Context context, Bitmap bitmap, int scale) {
        super(context);
        this.bitmap = bitmap;
        this.scale = scale;
        this.src = new Rect(0, 0, bitmap.getWidth(), bitmap.getHeight());
    }

    /** Width and height in art pixels. */
    int artWidth() {
        return bitmap.getWidth();
    }

    int artHeight() {
        return bitmap.getHeight();
    }

    @Override
    protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
        setMeasuredDimension(bitmap.getWidth() * scale, bitmap.getHeight() * scale);
    }

    @Override
    protected void onDraw(Canvas canvas) {
        dst.set(0, 0, bitmap.getWidth() * scale, bitmap.getHeight() * scale);
        canvas.drawBitmap(bitmap, src, dst, PixelUi.nearestPaint());
    }

    // ---------------------------------------------------------------------------------
    // Fitting a picture into a column
    // ---------------------------------------------------------------------------------

    /** The whole factor a picture artWidth wide is made smaller by to fit maxArt: 1 when it already fits. */
    static int shrinkFactor(int artWidth, int maxArt) {
        if (maxArt <= 0 || artWidth <= maxArt) {
            return 1;
        }
        return (artWidth + maxArt - 1) / maxArt;
    }

    /**
     * Makes pixels (width x height, row by row) k times smaller each way by keeping the
     * pixel at the centre of every k x k block. The result is width / k by height / k;
     * the few leftover columns and rows are cropped evenly from both sides, so a picture
     * is never smeared and never given a fractional pixel.
     */
    static int[] shrink(int[] pixels, int width, int height, int k) {
        if (k <= 1) {
            return pixels;
        }
        int w = width / k;
        int h = height / k;
        int offX = (width - w * k) / 2;
        int offY = (height - h * k) / 2;
        int[] out = new int[w * h];
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                out[y * w + x] = pixels[(offY + y * k + k / 2) * width + offX + x * k + k / 2];
            }
        }
        return out;
    }

    /** The bitmap made to fit a column maxArt art px wide, by a whole factor; the same bitmap if it fits. */
    static Bitmap fit(Bitmap source, int maxArt) {
        int k = shrinkFactor(source.getWidth(), maxArt);
        if (k == 1) {
            return source;
        }
        int w = source.getWidth();
        int h = source.getHeight();
        int[] pixels = new int[w * h];
        source.getPixels(pixels, 0, w, 0, 0, w, h);
        int[] small = shrink(pixels, w, h, k);
        Bitmap out = Bitmap.createBitmap(w / k, h / k, Bitmap.Config.ARGB_8888);
        out.setPixels(small, 0, w / k, 0, 0, w / k, h / k);
        return out;
    }
}
