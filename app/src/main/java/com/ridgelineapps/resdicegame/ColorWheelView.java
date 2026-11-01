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
import android.graphics.Color;
import android.graphics.ComposeShader;
import android.graphics.Paint;
import android.graphics.PorterDuff;
import android.graphics.RadialGradient;
import android.graphics.Shader;
import android.graphics.SweepGradient;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.View;

/**
 * A colour wheel: hue around the circumference, saturation from centre (white, s=0) to
 * rim (full saturation, s=1), both painted at value 1. A disc can only ever hold two of
 * HSV's three dimensions, so this view never reports a value; the picker dialog gets
 * brightness from a separate slider and combines it with the hue and saturation this
 * view reports.
 *
 * The wheel is built once, in onSizeChanged, onto a Bitmap-backed Canvas (always
 * software rendered, regardless of whether the view hierarchy above it is hardware
 * accelerated), then blitted every frame. That sidesteps ComposeShader's hardware
 * acceleration restrictions at API levels below 23 (minSdk here is 21): the shader only
 * ever touches the offscreen software canvas, never a hardware layer, and the wheel
 * never changes after that first build, so there is nothing to redo per frame anyway.
 */
public class ColorWheelView extends View {

    /** Fired only from a user touch. Programmatic moves (setHueAndSaturation) do not fire it, so a preset tap or a typed hex does not loop back into itself. */
    public interface OnHueSaturationChangeListener {
        void onHueSaturationChange(float hue, float saturation);
    }

    private static final int HUE_STOPS = 12;
    private static final float INDICATOR_RADIUS_DP = 9f;
    private static final float INDICATOR_STROKE_DP = 2f;

    private final Paint bitmapPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint indicatorOuterPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint indicatorInnerPaint = new Paint(Paint.ANTI_ALIAS_FLAG);

    private Bitmap wheelBitmap;
    private float hue = 0f;
    private float saturation = 0f;
    private OnHueSaturationChangeListener listener;

    public ColorWheelView(Context context, AttributeSet attrs) {
        super(context, attrs);

        float density = context.getResources().getDisplayMetrics().density;

        // A ring stroked in both white and black (the black ring drawn smaller, inside
        // the white one) stays visible whether the indicator sits over the wheel's
        // light centre or its darker, fully saturated rim.
        indicatorOuterPaint.setStyle(Paint.Style.STROKE);
        indicatorOuterPaint.setColor(Color.WHITE);
        indicatorOuterPaint.setStrokeWidth(INDICATOR_STROKE_DP * density);

        indicatorInnerPaint.setStyle(Paint.Style.STROKE);
        indicatorInnerPaint.setColor(Color.BLACK);
        indicatorInnerPaint.setStrokeWidth(INDICATOR_STROKE_DP * density);
    }

    public void setOnHueSaturationChangeListener(OnHueSaturationChangeListener listener) {
        this.listener = listener;
    }

    /**
     * Moves the indicator without notifying the listener. Callers that already know the
     * new hue and saturation (a preset tap, a valid hex typed into the field) use this,
     * so this view's own change callback never fires for a move it did not originate.
     */
    public void setHueAndSaturation(float hue, float saturation) {
        this.hue = hue;
        this.saturation = saturation;
        invalidate();
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);

        int size = Math.min(w, h);
        wheelBitmap = size > 0 ? buildWheelBitmap(size) : null;
    }

    private static Bitmap buildWheelBitmap(int size) {
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bitmap);
        float radius = size / 2f;

        int[] hueColors = new int[HUE_STOPS + 1];
        float[] hsv = { 0f, 1f, 1f };
        for(int i = 0; i <= HUE_STOPS; i++) {
            hsv[0] = 360f * i / HUE_STOPS;
            hueColors[i] = Color.HSVToColor(hsv);
        }

        // The sweep gradient alone paints full-saturation hue all the way round. The
        // radial gradient, composed SRC_OVER on top, fades from opaque white at the
        // centre to fully transparent at the rim; blending opaque white over a colour
        // is exactly what desaturating that colour toward white looks like, and the
        // radial gradient's alpha is linear in distance, so the visible saturation at
        // any point is exactly (distance from centre) / radius.
        SweepGradient sweep = new SweepGradient(radius, radius, hueColors, null);
        RadialGradient radial = new RadialGradient(radius, radius, radius, Color.WHITE, 0x00FFFFFF, Shader.TileMode.CLAMP);
        ComposeShader compose = new ComposeShader(sweep, radial, PorterDuff.Mode.SRC_OVER);

        Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        paint.setShader(compose);
        canvas.drawCircle(radius, radius, radius, paint);

        return bitmap;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        if(wheelBitmap == null) {
            return;
        }

        float left = (getWidth() - wheelBitmap.getWidth()) / 2f;
        float top = (getHeight() - wheelBitmap.getHeight()) / 2f;
        canvas.drawBitmap(wheelBitmap, left, top, bitmapPaint);

        float radius = wheelBitmap.getWidth() / 2f;
        float centerX = left + radius;
        float centerY = top + radius;

        double angle = Math.toRadians(hue);
        float indicatorRadius = saturation * radius;
        float x = centerX + indicatorRadius * (float) Math.cos(angle);
        float y = centerY + indicatorRadius * (float) Math.sin(angle);

        float density = getResources().getDisplayMetrics().density;
        float ringRadius = INDICATOR_RADIUS_DP * density;
        canvas.drawCircle(x, y, ringRadius, indicatorOuterPaint);
        canvas.drawCircle(x, y, ringRadius - indicatorInnerPaint.getStrokeWidth(), indicatorInnerPaint);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        if(wheelBitmap == null) {
            return false;
        }

        switch(event.getAction()) {
            case MotionEvent.ACTION_DOWN:
            case MotionEvent.ACTION_MOVE:
                updateFromTouch(event.getX(), event.getY());
                return true;
            default:
                return false;
        }
    }

    private void updateFromTouch(float touchX, float touchY) {
        float radius = wheelBitmap.getWidth() / 2f;
        float left = (getWidth() - wheelBitmap.getWidth()) / 2f;
        float top = (getHeight() - wheelBitmap.getHeight()) / 2f;
        float centerX = left + radius;
        float centerY = top + radius;

        float dx = touchX - centerX;
        float dy = touchY - centerY;
        float distance = (float) Math.sqrt(dx * dx + dy * dy);

        // A drag past the rim still fully saturates at that angle, rather than being
        // ignored, which is what a finger sliding off the edge expects.
        if(distance > radius && distance > 0) {
            dx = dx * radius / distance;
            dy = dy * radius / distance;
            distance = radius;
        }

        double angleRadians = Math.atan2(dy, dx);
        float degrees = (float) Math.toDegrees(angleRadians);
        if(degrees < 0) {
            degrees += 360f;
        }

        hue = degrees;
        saturation = radius == 0 ? 0 : distance / radius;
        invalidate();

        if(listener != null) {
            listener.onHueSaturationChange(hue, saturation);
        }
    }
}
