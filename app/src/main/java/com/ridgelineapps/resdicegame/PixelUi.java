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
import android.content.res.ColorStateList;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.graphics.ColorFilter;
import android.graphics.Paint;
import android.graphics.PixelFormat;
import android.graphics.Point;
import android.graphics.Rect;
import android.graphics.Typeface;
import android.content.res.Configuration;
import android.graphics.drawable.Drawable;
import android.text.Layout;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.EditText;
import android.widget.SeekBar;
import android.widget.TextView;

/**
 * The pixel-art UI kit shared by every screen that is not the game canvas: one integer
 * scale, two pixel fonts and the 9-slice button and panel art, all drawn nearest
 * neighbour so that each art pixel is an exact scale x scale block of screen pixels.
 *
 * Layout convention: in a layout that goes through skin(), every dimension is written
 * in px and means ART pixels. skin() multiplies margins, paddings, fixed sizes and
 * minimum sizes by the scale, which is what keeps every view on the art grid. Text sizes
 * are never written in XML; skin() sets them from the fonts' native sizes.
 */
final class PixelUi {

    /** The design width and the minimum height in art pixels. Must agree with the game canvas. */
    static final int DESIGN_W = 216;
    static final int MIN_H = 352;

    /** Native pixel sizes of the two fonts: an exact pixel grid at these sizes. */
    static final int BODY_PX = 10;
    static final int DISPLAY_PX = 8;

    /** Extra leading between lines, in art pixels, (Ark Pixel's own 14 px line is already roomy). */
    private static final int BODY_LEADING = 0;
    private static final int DISPLAY_LEADING = 3;

    /** The brass buttons look the same in both modes, so their label colours are constants. */
    static final int BUTTON_TEXT = 0xFFFFF4D6;
    static final int BUTTON_TEXT_DISABLED = 0xFFC0BCB0;

    /** The look screens are drawn in right now. See refreshTheme. */
    private static volatile Palette current = Palette.LIGHT;

    private static Typeface displayFace;
    private static Typeface bodyFace;
    private static final Paint NEAREST = new Paint();

    static {
        NEAREST.setFilterBitmap(false);
        NEAREST.setAntiAlias(false);
        NEAREST.setDither(false);
    }

    private PixelUi() {
    }

    // ---------------------------------------------------------------------------------
    // Theme
    // ---------------------------------------------------------------------------------

    /** The palette for the current mode. The only place a screen asks what colour things are. */
    static Palette palette() {
        return current;
    }

    /**
     * Works out the mode from the stored Theme setting and the OS night mode of this
     * context, and makes it the current palette. Called when an Activity starts or comes
     * back to the front, and when the OS night mode changes under a running one.
     */
    static void refreshTheme(Context context) {
        preview(context, HighScoresAndSettings.load(context).theme);
    }

    /**
     * What an Activity that handles the uiMode configuration change itself does when the
     * OS night mode flips: the new palette, and everything on screen repainted in place,
     * the content (null for the game canvas, which has no kit views) and any dialog open.
     */
    static void themeChanged(Context context, View contentRoot) {
        refreshTheme(context);
        if (contentRoot != null) {
            restyle(contentRoot);
        }
        PixelDialog.restyleOpen();
    }

    /** As refreshTheme for a Theme value that is not stored (yet): the Settings dialog's live preview. */
    static void preview(Context context, int theme) {
        boolean night = (context.getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                == Configuration.UI_MODE_NIGHT_YES;
        current = Palette.forMode(theme, night);
    }

    /** The integer scale: max(1, floor(min(widthPx / 216, heightPx / 352))) of the real display. */
    @SuppressWarnings("deprecation")
    static int scale(Context context) {
        WindowManager wm = (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
        Point size = new Point();
        wm.getDefaultDisplay().getRealSize(size);
        return Math.max(1, Math.min(size.x / DESIGN_W, size.y / MIN_H));
    }

    static synchronized Typeface display(Context context) {
        if (displayFace == null) {
            displayFace = Typeface.createFromAsset(context.getApplicationContext().getAssets(), "fonts/PressStart2P-Regular.ttf");
        }
        return displayFace;
    }

    static synchronized Typeface body(Context context) {
        if (bodyFace == null) {
            bodyFace = Typeface.createFromAsset(context.getApplicationContext().getAssets(), "fonts/ArkPixel10-Regular.otf");
        }
        return bodyFace;
    }

    static Bitmap decode(Context context, int resId) {
        BitmapFactory.Options options = new BitmapFactory.Options();
        options.inScaled = false;
        options.inPreferredConfig = Bitmap.Config.ARGB_8888;
        return BitmapFactory.decodeResource(context.getResources(), resId, options);
    }

    static Paint nearestPaint() {
        return NEAREST;
    }

    /** The width of a dialog in screen px: min(screen width - 16 art px, 216 art px), on the grid. */
    static int dialogWidth(Context context, int scale) {
        WindowManager wm = (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
        Point size = new Point();
        wm.getDefaultDisplay().getRealSize(size);
        // Even, so that centred children (an even number of art pixels wide) land on the grid.
        int art = Math.min(size.x / scale - 16, DESIGN_W) & ~1;
        return art * scale;
    }

    // ---------------------------------------------------------------------------------
    // Skinning
    // ---------------------------------------------------------------------------------

    /** Skins a whole tree and gives its root the panel background. */
    static void skin(View root, int scale) {
        skin(root, scale, true);
    }

    /**
     * As skin(View, int), but the root's background is only set when panel is true, for
     * content that sits inside a frame somebody else draws.
     */
    static void skin(View root, int scale, boolean panel) {
        skinTree(root, scale);
        if (panel) {
            root.setBackground(PixelFrameDrawable.panel(root.getContext(), scale));
        }
    }

    /**
     * Skins the content view of a full-screen Activity: the whole tree, the panel frame,
     * and padding so the content area is a whole, even number of art pixels wide (any
     * leftover screen pixels go to the right edge, where the dark window shows as outline).
     * Returns the content width in art pixels.
     */
    @SuppressWarnings("deprecation")
    static int skinScreen(android.app.Activity activity) {
        int scale = scale(activity);
        View root = ((ViewGroup) activity.findViewById(android.R.id.content)).getChildAt(0);
        skin(root, scale);

        Point size = new Point();
        activity.getWindowManager().getDefaultDisplay().getRealSize(size);
        int leftover = size.x % scale;
        int contentArt = size.x / scale - (root.getPaddingLeft() + root.getPaddingRight()) / scale;
        int extra = leftover;
        if ((contentArt & 1) != 0) {
            extra += scale;
            contentArt--;
        }
        root.setPadding(root.getPaddingLeft(), root.getPaddingTop(), root.getPaddingRight() + extra, root.getPaddingBottom());
        keepAboveTheNavigationBar(root, scale);
        return contentArt;
    }

    /**
     * Pads a screen's bottom by the navigation bar's height, rounded up to the art grid, so
     * its last row (the OK button) is never under the three button bar.
     */
    private static void keepAboveTheNavigationBar(final View root, final int scale) {
        final int base = root.getPaddingBottom();
        root.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
            @Override
            public android.view.WindowInsets onApplyWindowInsets(View v, android.view.WindowInsets insets) {
                int bar = (insets.getSystemWindowInsetBottom() + scale - 1) / scale * scale;
                v.setPadding(v.getPaddingLeft(), v.getPaddingTop(), v.getPaddingRight(), base + bar);
                return insets;
            }
        });
        root.requestApplyInsets();
    }

    private static void skinTree(View v, int scale) {
        scaleBox(v, scale);

        // A divider or an outline is a plain coloured box that follows the palette
        if ("rule".equals(v.getTag())) {
            v.setBackground(new PaletteColor(PaletteColor.RULE));
        }
        else if ("frame".equals(v.getTag())) {
            v.setBackground(new PaletteColor(PaletteColor.FRAME));
        }

        if (v instanceof SeekBar) {
            skinSeekBar((SeekBar) v, scale);
        }
        else if (v instanceof TextView) {
            style((TextView) v, scale);
        }

        if (v instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) v;
            for (int i = 0; i < group.getChildCount(); i++) {
                skinTree(group.getChildAt(i), scale);
            }
        }
    }

    /** Art pixels (written as px in the layout) to screen pixels: margins, padding, sizes. */
    private static void scaleBox(View v, int scale) {
        if ((v instanceof Button && !(v instanceof CompoundButton)) || v instanceof EditText || v instanceof SeekBar) {
            // The theme gives these dp-based padding and minimum sizes. Not art: drop them,
            // style() and skinSeekBar() set the kit's own.
            v.setPadding(0, 0, 0, 0);
        }
        if (v instanceof Button || v instanceof EditText || v instanceof SeekBar) {
            v.setMinimumWidth(0);
            v.setMinimumHeight(0);
        }
        v.setPadding(v.getPaddingLeft() * scale, v.getPaddingTop() * scale, v.getPaddingRight() * scale, v.getPaddingBottom() * scale);
        v.setMinimumWidth(v.getMinimumWidth() * scale);
        v.setMinimumHeight(v.getMinimumHeight() * scale);

        ViewGroup.LayoutParams lp = v.getLayoutParams();
        if (lp != null) {
            if (lp.width > 0) {
                lp.width *= scale;
            }
            if (lp.height > 0) {
                lp.height *= scale;
            }
            if (lp instanceof ViewGroup.MarginLayoutParams) {
                ViewGroup.MarginLayoutParams m = (ViewGroup.MarginLayoutParams) lp;
                m.setMargins(m.leftMargin * scale, m.topMargin * scale, m.rightMargin * scale, m.bottomMargin * scale);
            }
        }
    }

    /**
     * Fonts, colours, background and padding floor for one text view, by kind: Button
     * (display font on the button frame), EditText (body font on a recessed frame),
     * RadioButton, a TextView tagged "title" (display font), else plain body text.
     * Code-built views call this directly; their sizes are already in screen px.
     */
    static void style(TextView tv, int scale) {
        Context context = tv.getContext();
        boolean isButton = tv instanceof Button && !(tv instanceof CompoundButton);
        boolean isTitle = "title".equals(tv.getTag());
        boolean isField = tv instanceof EditText;

        if (isButton || isTitle) {
            applyFont(tv, display(context), DISPLAY_PX, DISPLAY_LEADING, scale);
        }
        else {
            applyFont(tv, body(context), BODY_PX, BODY_LEADING, scale);
        }

        if (isButton) {
            tv.setAllCaps(false);
            tv.setStateListAnimator(null);
            tv.setMinWidth(0);
            tv.setMinHeight(0);
            tv.setGravity(Gravity.CENTER);
            tv.setBackground(PixelFrameDrawable.button(context, scale));
            tv.setMinimumHeight(22 * scale);
            // The 9-slice corner is 6 art px: keep the label clear of the bevel.
            tv.setPadding(Math.max(tv.getPaddingLeft(), 8 * scale), Math.max(tv.getPaddingTop(), 6 * scale),
                    Math.max(tv.getPaddingRight(), 8 * scale), Math.max(tv.getPaddingBottom(), 6 * scale));
        }
        else if (isField) {
            tv.setBackground(PixelFrameDrawable.field(context, scale));
            tv.setPadding(8 * scale, 6 * scale, 8 * scale, 6 * scale);
            tv.setGravity(Gravity.CENTER_VERTICAL);
            tv.setHighlightColor(0x66FFF4D6);
        }
        else if (tv instanceof CompoundButton) {
            ((CompoundButton) tv).setButtonDrawable(null);
            tv.setBackground(null);
            tv.setMinWidth(0);
            tv.setMinHeight(16 * scale);
            tv.setGravity(Gravity.TOP);
            // Room for the 12 art px box and a 4 art px gap, inside the left padding.
            tv.setPadding(tv.getPaddingLeft() + 16 * scale, tv.getPaddingTop(), tv.getPaddingRight(), tv.getPaddingBottom());
        }
        colorize(tv);
    }

    /**
     * Text colours for the current palette, by kind of view. style() calls it once; a
     * theme change calls it again through restyle() on a screen that is already up, which
     * is why it touches nothing but colours.
     */
    private static void colorize(TextView tv) {
        Palette p = current;
        if (tv instanceof Button && !(tv instanceof CompoundButton)) {
            tv.setTextColor(new ColorStateList(
                    new int[][] { { -android.R.attr.state_enabled }, {} },
                    new int[] { BUTTON_TEXT_DISABLED, BUTTON_TEXT }));
        }
        else if (tv instanceof EditText) {
            tv.setTextColor(p.fieldText);
            tv.setHintTextColor(p.fieldHint);
        }
        else if ("secondary".equals(tv.getTag())) {
            tv.setTextColor(p.secondary);
            tv.setLinkTextColor(p.link);
        }
        else {
            tv.setTextColor(p.ink);
            tv.setLinkTextColor(p.link);
        }
    }

    /**
     * Repaints a tree that is already on screen for the current palette. Everything the
     * palette colours except text (panels, fields, the slider, rules, scroll thumbs) is a
     * drawable that reads the palette as it draws, so this only has to redo text colours
     * and ask for a redraw.
     */
    static void restyle(View root) {
        if (root instanceof SeekBar) {
            root.invalidate();
        }
        else if (root instanceof TextView) {
            colorize((TextView) root);
        }
        if (root instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) root;
            for (int i = 0; i < group.getChildCount(); i++) {
                restyle(group.getChildAt(i));
            }
        }
        root.invalidate();
    }

    /**
     * A flat colour that is looked up in the palette every time it is drawn, so a box
     * painted with it follows a theme change without anyone touching it.
     */
    static final class PaletteColor extends Drawable {
        static final int RULE = 0;
        static final int SCROLL_THUMB = 1;
        static final int FRAME = 2;
        static final int INK = 3;

        private final int which;
        private final Paint paint = new Paint();

        PaletteColor(int which) {
            this.which = which;
            paint.setAntiAlias(false);
        }

        private int color() {
            Palette p = current;
            switch (which) {
                case SCROLL_THUMB:
                    return p.scrollThumb;
                case FRAME:
                    return p.frame;
                case INK:
                    return p.ink;
                default:
                    return p.rule;
            }
        }

        @Override
        public void draw(Canvas canvas) {
            paint.setColor(color());
            canvas.drawRect(getBounds(), paint);
        }

        @Override
        public void setAlpha(int alpha) {
        }

        @Override
        public void setColorFilter(ColorFilter colorFilter) {
        }

        @Override
        public int getOpacity() {
            return PixelFormat.OPAQUE;
        }
    }

    /**
     * Native size times scale, with every flag that could blur or reposition glyphs off,
     * and a line pitch that is a whole number of art pixels.
     */
    static void applyFont(TextView tv, Typeface face, int nativePx, int leadingArtPx, int scale) {
        tv.setTypeface(face);
        tv.setTextSize(TypedValue.COMPLEX_UNIT_PX, nativePx * scale);
        Paint p = tv.getPaint();
        p.setAntiAlias(false);
        p.setSubpixelText(false);
        // Hinting ON, not off: with hinting off Skia/FreeType places Press Start 2P one screen
        // pixel to the right of the grid. With it on, both fonts land exactly on the art grid.
        p.setHinting(Paint.HINTING_ON);
        p.setLinearText(false);
        p.setFilterBitmap(false);
        tv.setIncludeFontPadding(false);

        Paint.FontMetricsInt fm = p.getFontMetricsInt();
        int natural = fm.descent - fm.ascent;
        int pitch = roundUp(natural, scale) + leadingArtPx * scale;
        tv.setLineSpacing(pitch - natural, 1f);
    }

    static int roundUp(int px, int scale) {
        return ((px + scale - 1) / scale) * scale;
    }

    /** For the Pixel* views' onMeasure: a wrapped size becomes a whole number of art pixels. */
    static int grid(int measured, int spec, int scale) {
        if (View.MeasureSpec.getMode(spec) == View.MeasureSpec.EXACTLY) {
            return measured;
        }
        return roundUp(measured, scale);
    }

    /**
     * Moves the canvas so the first line of text starts on the art grid, both across
     * (centred text can land on a half art pixel) and down (the baseline). Returns the
     * saved state; the caller restores it after TextView.onDraw. extraDown is the
     * pressed-button nudge, in px.
     */
    static int snapText(TextView tv, Canvas canvas, int scale, int extraDown) {
        int saved = canvas.save();
        Layout layout = tv.getLayout();
        if (layout != null) {
            float x = tv.getCompoundPaddingLeft() + layout.getLineLeft(0);
            float rx = x % scale;
            float dx = rx < scale / 2f ? -rx : scale - rx;

            int boxH = tv.getHeight() - tv.getExtendedPaddingTop() - tv.getExtendedPaddingBottom();
            int voffset = 0;
            int vg = tv.getGravity() & Gravity.VERTICAL_GRAVITY_MASK;
            if (layout.getHeight() < boxH) {
                if (vg == Gravity.CENTER_VERTICAL) {
                    voffset = (boxH - layout.getHeight()) >> 1;
                }
                else if (vg == Gravity.BOTTOM) {
                    voffset = boxH - layout.getHeight();
                }
            }
            int y = tv.getExtendedPaddingTop() + voffset + layout.getLineBaseline(0);
            int ry = y % scale;
            int dy = ry < scale / 2f ? -ry : scale - ry;
            canvas.translate(dx, dy + extraDown);
        }
        return saved;
    }

    // ---------------------------------------------------------------------------------
    // Drawables made of whole art pixels
    // ---------------------------------------------------------------------------------

    /** The slider track: a 1 art px outline around a groove, 6 art px tall, in the palette's colours. */
    private static final class Track extends Drawable {
        private final int scale;
        private final Paint paint = new Paint();

        Track(int scale) {
            this.scale = scale;
            paint.setAntiAlias(false);
        }

        @Override
        public void draw(Canvas canvas) {
            Rect b = getBounds();
            int h = 6 * scale;
            int top = b.top + ((b.height() - h) / 2 / scale) * scale;
            int left = b.left;
            int right = b.left + (b.width() / scale) * scale;
            Palette p = current;
            paint.setColor(p.trackOutline);
            canvas.drawRect(left, top, right, top + h, paint);
            paint.setColor(p.trackFace);
            canvas.drawRect(left + scale, top + scale, right - scale, top + h - scale, paint);
            paint.setColor(p.trackTop);
            canvas.drawRect(left + scale, top + scale, right - scale, top + 2 * scale, paint);
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

        @Override
        public int getIntrinsicHeight() {
            return 6 * scale;
        }
    }

    private static void skinSeekBar(SeekBar bar, int scale) {
        Context context = bar.getContext();
        PixelFrameDrawable thumb = PixelFrameDrawable.thumb(context, scale);
        thumb.setSize(12 * scale, 16 * scale);
        bar.setBackground(null);
        bar.setProgressDrawable(new Track(scale));
        bar.setThumb(thumb);
        bar.setThumbOffset(0);
        bar.setSplitTrack(false);
        bar.setMinimumHeight(16 * scale);
    }
}
