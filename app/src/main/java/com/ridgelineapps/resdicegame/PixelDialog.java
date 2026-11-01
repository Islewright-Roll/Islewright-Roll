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

import java.util.ArrayList;

import android.app.Activity;
import android.app.Dialog;
import android.content.DialogInterface;
import android.graphics.Point;
import android.graphics.drawable.ColorDrawable;
import android.os.Build;
import android.view.Gravity;
import android.view.View;
import android.view.View.MeasureSpec;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

/**
 * Every dialog in the game: a panel frame, a display-font title, a body and a row of
 * buttons, all in the pixel kit at the shared integer scale. These are plain
 * android.app.Dialog windows with a custom view; the window background is transparent so
 * the panel art is the frame, and the window origin is placed on the art grid.
 *
 * Layout in art pixels (multiply by the scale): 8 of padding inside the frame, the title,
 * a 1 px rule, the body, then the buttons 8 below it.
 */
final class PixelDialog {

    /** Reports which entry of a choose() list was picked. */
    interface OnChoice {
        void onChoice(int index);
    }

    /** The root views of the dialogs on screen, so a theme change can repaint them. */
    private static final ArrayList<View> OPEN = new ArrayList<View>();

    /** Every dialog built and not yet gone, so the game screen can tell that one is open. */
    private static final ArrayList<Dialog> LIVE = new ArrayList<Dialog>();

    /**
     * Whether any dialog is on screen. The game's tutorial callouts hide while one is. Every
     * dialog is shown the moment it is built, on the UI thread, so the first frame after
     * a dialog opens already counts it, and it counts until it is dismissed; one whose
     * Activity went away without dismissing it is dropped.
     */
    static boolean isOpen() {
        for (int i = LIVE.size() - 1; i >= 0; i--) {
            Dialog d = LIVE.get(i);
            Activity owner = d.getOwnerActivity();
            if (owner != null && owner.isDestroyed()) {
                LIVE.remove(i);
            }
        }
        return !LIVE.isEmpty();
    }

    private static final int PAD = 8;
    private static final int GAP = 4;
    private static final int BUTTON_ART_W = 90;

    private PixelDialog() {
    }

    /** Repaints every dialog now on screen for the current palette. */
    static void restyleOpen() {
        for (int i = OPEN.size() - 1; i >= 0; i--) {
            View root = OPEN.get(i);
            if (root.isAttachedToWindow()) {
                PixelUi.restyle(root);
            }
            else {
                OPEN.remove(i);
            }
        }
    }

    static void confirm(Activity a, int titleRes, int messageRes, Runnable onYes) {
        confirm(a, titleRes, messageRes, onYes, null);
    }

    static void confirm(Activity a, int titleRes, int messageRes, Runnable onYes, Runnable onNo) {
        int s = PixelUi.scale(a);
        TextView message = text(a, s, a.getResources().getString(messageRes));
        show(a, titleRes, message, new int[] { R.string.yes, R.string.no }, new Runnable[] { onYes, onNo });
    }

    /** As confirm above, for a message that had values formatted into it. */
    static void confirm(Activity a, int titleRes, CharSequence message, Runnable onYes, Runnable onNo) {
        int s = PixelUi.scale(a);
        show(a, titleRes, text(a, s, message), new int[] { R.string.yes, R.string.no }, new Runnable[] { onYes, onNo });
    }

    /** A title, a message and one OK button. Not part of the shared contract; used by Top Scores. */
    static void message(Activity a, int titleRes, CharSequence message) {
        int s = PixelUi.scale(a);
        show(a, titleRes, text(a, s, message), new int[] { R.string.ok_button }, new Runnable[] { null });
    }

    /** A column of full-width buttons, one per entry; tapping outside dismisses with no choice. */
    static void choose(Activity a, int titleRes, int[] itemRes, final OnChoice onChoice) {
        CharSequence[] items = new CharSequence[itemRes.length];
        for (int i = 0; i < itemRes.length; i++) {
            items[i] = a.getText(itemRes[i]);
        }
        choose(a, titleRes, items, onChoice);
    }

    /** As choose above, for entries that had values formatted into them. */
    static void choose(Activity a, int titleRes, CharSequence[] items, final OnChoice onChoice) {
        final int s = PixelUi.scale(a);
        final Dialog[] holder = new Dialog[1];
        LinearLayout column = new LinearLayout(a);
        column.setOrientation(LinearLayout.VERTICAL);
        for (int i = 0; i < items.length; i++) {
            final int index = i;
            TextView button = button(a, s, items[i], new Runnable() {
                @Override
                public void run() {
                    holder[0].dismiss();
                    if (onChoice != null) {
                        onChoice.onChoice(index);
                    }
                }
            });
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            if (i > 0) {
                lp.topMargin = GAP * s;
            }
            column.addView(button, lp);
        }
        holder[0] = build(a, titleRes, column, null, null, null);
        holder[0].show();
    }

    /**
     * A dialog around caller-supplied content, which this skins (so layouts are written in
     * art pixels, see PixelUi). OK is always shown; Cancel only when onCancel is not null.
     * Returns the dialog already shown, so a caller can dismiss it.
     */
    static Dialog custom(Activity a, int titleRes, View content, Runnable onOk, Runnable onCancel) {
        int s = PixelUi.scale(a);
        PixelUi.skin(content, s, false);
        if (onCancel != null) {
            return show(a, titleRes, content, new int[] { R.string.ok_button, R.string.settings_cancel }, new Runnable[] { onOk, onCancel });
        }
        return show(a, titleRes, content, new int[] { R.string.ok_button }, new Runnable[] { onOk });
    }

    /**
     * As custom above, with any buttons, in the order given (the content is skinned here
     * too). onDismiss runs however the dialog closes: a button, back, or a tap outside.
     * Returns the dialog already shown.
     */
    static Dialog custom(Activity a, int titleRes, View content, int[] labels, Runnable[] actions, Runnable onDismiss) {
        int s = PixelUi.scale(a);
        PixelUi.skin(content, s, false);
        Dialog d = build(a, titleRes, content, labels, actions, onDismiss);
        d.show();
        return d;
    }

    /** As custom(Activity, int, View, Runnable, Runnable), also telling the caller when the dialog is gone. */
    static Dialog custom(Activity a, int titleRes, View content, Runnable onOk, Runnable onCancel, Runnable onDismiss) {
        int s = PixelUi.scale(a);
        PixelUi.skin(content, s, false);
        Dialog d = build(a, titleRes, content, new int[] { R.string.ok_button, R.string.settings_cancel }, new Runnable[] { onOk, onCancel }, onDismiss);
        d.show();
        return d;
    }

    private static Dialog show(Activity a, int titleRes, View body, int[] labels, Runnable[] actions) {
        Dialog d = build(a, titleRes, body, labels, actions, null);
        d.show();
        return d;
    }

    private static TextView text(Activity a, int s, CharSequence value) {
        PixelTextView tv = new PixelTextView(a, null);
        tv.setText(value);
        PixelUi.style(tv, s);
        return tv;
    }

    private static TextView button(Activity a, int s, int labelRes, final Runnable action) {
        return button(a, s, a.getText(labelRes), action);
    }

    private static TextView button(Activity a, int s, CharSequence label, final Runnable action) {
        PixelButton b = new PixelButton(a, null);
        b.setText(label);
        PixelUi.style(b, s);
        b.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                action.run();
            }
        });
        return b;
    }

    /** Builds the window without showing it. labels and actions may be null for none. */
    private static Dialog build(Activity a, int titleRes, View body, int[] labels, final Runnable[] actions, final Runnable onDismiss) {
        final int s = PixelUi.scale(a);
        final Dialog d = new Dialog(a);
        d.requestWindowFeature(Window.FEATURE_NO_TITLE);

        Point screen = new Point();
        a.getWindowManager().getDefaultDisplay().getRealSize(screen);
        int width = PixelUi.dialogWidth(a, s);
        // The dialog has to fit between the system bars, not on the whole display: with the
        // three button navigation bar on, the display's own height reaches under that bar
        int safeTop = 0;
        int safeBottom = 0;
        if (android.os.Build.VERSION.SDK_INT >= 23) {
            android.view.WindowInsets insets = a.getWindow().getDecorView().getRootWindowInsets();
            if (insets != null) {
                safeTop = insets.getSystemWindowInsetTop();
                safeBottom = insets.getSystemWindowInsetBottom();
            }
        }
        int safeHeight = screen.y - safeTop - safeBottom;
        int maxHeight = (safeHeight / s - 16) * s;

        final LinearLayout root = new LinearLayout(a);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(PAD * s, PAD * s, PAD * s, PAD * s);
        root.setBackground(PixelFrameDrawable.panel(a, s));

        PixelTextView title = new PixelTextView(a, null);
        title.setTag("title");
        title.setText(titleRes);
        PixelUi.style(title, s);
        title.setGravity(Gravity.CENTER_HORIZONTAL);
        root.addView(title, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));

        View rule = new View(a);
        rule.setBackground(new PixelUi.PaletteColor(PixelUi.PaletteColor.RULE));
        LinearLayout.LayoutParams ruleLp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, s);
        ruleLp.bottomMargin = 6 * s;
        root.addView(rule, ruleLp);

        ScrollView scroll = new ScrollView(a);
        scroll.setVerticalScrollBarEnabled(false);
        scroll.setOverScrollMode(View.OVER_SCROLL_NEVER);
        scroll.addView(body, new ScrollView.LayoutParams(ScrollView.LayoutParams.MATCH_PARENT, ScrollView.LayoutParams.WRAP_CONTENT));
        LinearLayout.LayoutParams scrollLp = new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        root.addView(scroll, scrollLp);

        if (labels != null) {
            int innerArt = width / s - 2 * PAD;
            LinearLayout row = new LinearLayout(a);
            row.setOrientation(LinearLayout.HORIZONTAL);
            int each = labels.length == 1 ? BUTTON_ART_W : Math.min(BUTTON_ART_W, (innerArt - GAP) / 2);
            TextView[] buttons = new TextView[labels.length];
            int[] widths = new int[labels.length];
            boolean fits = true;
            for (int i = 0; i < labels.length; i++) {
                final Runnable action = actions[i];
                buttons[i] = button(a, s, labels[i], new Runnable() {
                    @Override
                    public void run() {
                        d.dismiss();
                        if (action != null) {
                            action.run();
                        }
                    }
                });
                buttons[i].measure(MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED), MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED));
                widths[i] = buttons[i].getMeasuredWidth() / s;
                fits &= widths[i] <= each;
            }
            // Buttons of one width when every label fits that; otherwise each as wide as its
            // label needs and the last one takes the rest of the row.
            int total;
            if (fits) {
                total = labels.length * each + (labels.length - 1) * GAP;
                for (int i = 0; i < labels.length; i++) {
                    widths[i] = each;
                }
            }
            else {
                total = innerArt;
                int used = (labels.length - 1) * GAP;
                for (int i = 0; i < labels.length - 1; i++) {
                    used += widths[i];
                }
                widths[labels.length - 1] = innerArt - used;
            }
            for (int i = 0; i < labels.length; i++) {
                LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(widths[i] * s, LinearLayout.LayoutParams.WRAP_CONTENT);
                if (i > 0) {
                    lp.leftMargin = GAP * s;
                }
                row.addView(buttons[i], lp);
            }
            LinearLayout.LayoutParams rowLp = new LinearLayout.LayoutParams(total * s, LinearLayout.LayoutParams.WRAP_CONTENT);
            rowLp.topMargin = 8 * s;
            rowLp.leftMargin = ((innerArt - total) / 2) * s;
            root.addView(row, rowLp);
        }

        // Measure once to centre on the grid, and to let long content scroll rather than overflow.
        root.measure(MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED));
        int height = PixelUi.roundUp(root.getMeasuredHeight(), s);
        if (height > maxHeight) {
            height = maxHeight;
            scrollLp.height = 0;
            scrollLp.weight = 1;
            if (Build.VERSION.SDK_INT >= 29) {
                scroll.setVerticalScrollBarEnabled(true);
                scroll.setScrollBarSize(2 * s);
                scroll.setVerticalScrollbarThumbDrawable(new PixelUi.PaletteColor(PixelUi.PaletteColor.SCROLL_THUMB));
            }
        }
        LIVE.add(d);
        d.setContentView(root, new ViewGroup.LayoutParams(width, height));
        d.setCanceledOnTouchOutside(true);
        d.setOnShowListener(new DialogInterface.OnShowListener() {
            @Override
            public void onShow(DialogInterface dialog) {
                OPEN.add(root);
            }
        });
        d.setOnDismissListener(new DialogInterface.OnDismissListener() {
            @Override
            public void onDismiss(DialogInterface dialog) {
                OPEN.remove(root);
                LIVE.remove(d);
                if (onDismiss != null) {
                    onDismiss.run();
                }
            }
        });

        Window w = d.getWindow();
        w.setBackgroundDrawable(new ColorDrawable(0));
        w.setLayout(width, height);
        WindowManager.LayoutParams wlp = w.getAttributes();
        wlp.gravity = Gravity.TOP | Gravity.LEFT;
        wlp.x = ((screen.x - width) / 2 / s) * s;
        wlp.y = safeTop + ((safeHeight - height) / 2 / s) * s;
        w.setAttributes(wlp);
        return d;
    }
}
