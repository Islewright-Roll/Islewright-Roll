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
import android.util.AttributeSet;
import android.widget.EditText;

/**
 * An EditText whose size and text sit on the art grid. See PixelUi.snapText.
 */
public class PixelEditText extends EditText {

    private final int scale;

    public PixelEditText(Context context, AttributeSet attrs) {
        super(context, attrs);
        scale = PixelUi.scale(context);
    }

    @Override
    protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
        super.onMeasure(widthMeasureSpec, heightMeasureSpec);
        setMeasuredDimension(PixelUi.grid(getMeasuredWidth(), widthMeasureSpec, scale), PixelUi.grid(getMeasuredHeight(), heightMeasureSpec, scale));
    }

    @Override
    protected void onDraw(Canvas canvas) {
        int saved = PixelUi.snapText(this, canvas, scale, 0);
        super.onDraw(canvas);
        canvas.restoreToCount(saved);
    }
}
