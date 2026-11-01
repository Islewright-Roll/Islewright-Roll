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
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.widget.CheckBox;

/**
 * A CheckBox drawn as the same 12 art px recessed box a PixelRadioButton has, with a
 * pixel-art tick when checked, instead of the platform check box. PixelUi.skin leaves 16
 * art px of left padding for the box and its gap. See PixelUi.snapText.
 */
public class PixelCheckBox extends CheckBox {

    /** The tick, 7 x 6 art px, drawn inside the box. */
    private static final String[] TICK = {
            "......X",
            ".....XX",
            "X...XX.",
            "XX.XX..",
            ".XXX...",
            "..X....",
    };

    private final int scale;
    private final PixelFrameDrawable box;
    private final Paint fill = new Paint();

    public PixelCheckBox(Context context, AttributeSet attrs) {
        super(context, attrs);
        scale = PixelUi.scale(context);
        box = PixelFrameDrawable.field(context, scale);
        fill.setAntiAlias(false);
    }

    /** As PixelRadioButton: nothing else asks for a redraw when the checked state flips. */
    @Override
    public void setChecked(boolean checked) {
        boolean was = isChecked();
        super.setChecked(checked);
        if (was != checked) {
            invalidate();
        }
    }

    @Override
    protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
        super.onMeasure(widthMeasureSpec, heightMeasureSpec);
        setMeasuredDimension(PixelUi.grid(getMeasuredWidth(), widthMeasureSpec, scale), PixelUi.grid(getMeasuredHeight(), heightMeasureSpec, scale));
    }

    @Override
    protected void onDraw(Canvas canvas) {
        // The box is centred on the x-height of the first line, on the grid.
        int size = 12 * scale;
        int left = getPaddingLeft() - 16 * scale;
        int top = getPaddingTop() - scale;
        if (getLayout() != null) {
            top = ((getExtendedPaddingTop() + getLayout().getLineBaseline(0) - 9 * scale) / scale) * scale;
        }
        box.setBounds(left, top, left + size, top + size);
        box.draw(canvas);
        if (isChecked()) {
            fill.setColor(PixelUi.palette().mark);
            int x0 = left + 3 * scale;
            int y0 = top + 3 * scale;
            for (int row = 0; row < TICK.length; row++) {
                for (int col = 0; col < TICK[row].length(); col++) {
                    if (TICK[row].charAt(col) == 'X') {
                        canvas.drawRect(x0 + col * scale, y0 + row * scale, x0 + (col + 1) * scale, y0 + (row + 1) * scale, fill);
                    }
                }
            }
        }

        int saved = PixelUi.snapText(this, canvas, scale, 0);
        super.onDraw(canvas);
        canvas.restoreToCount(saved);
    }
}
