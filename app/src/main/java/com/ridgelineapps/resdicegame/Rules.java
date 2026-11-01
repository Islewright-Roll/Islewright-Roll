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
/*
 * Originally from the Photostream portion of the The Android Open 
 * Source Project - http://code.google.com/p/apps-for-android/
 * 
 * Copyright (C) 2008 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package com.ridgelineapps.resdicegame;

import java.util.List;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.os.Build;
import android.os.Bundle;
import android.util.Log;
import android.view.View;
import android.view.View.OnClickListener;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

/**
 * The Rules screen. It lays out res/raw/rules.txt (res/raw-de/rules.txt in German), a
 * plain UTF-8 file in a small markup, so the content can be changed without touching
 * Java. RulesMarkup reads it; this class draws it in the pixel kit.
 *
 * THE FORMAT. One construct per line; a line is judged after trimming its ends.
 *
 *   plain text        Running text. Consecutive plain lines run together as one paragraph
 *                     (so a file may be wrapped by hand); a blank line ends it.
 *   # Heading         A section heading, in the display font. Needs the space after "#".
 *   - Item            A bullet with a hanging indent: wrapped lines stay inside the text.
 *   [icon_name] text  A line that starts with a 12 px icon, the text beside it, wrapping
 *                     inside its own column. icon_name is the name of a PNG in
 *                     res/drawable-nodpi, usually a 12 px one (icon_gold_12, glyph_road_12).
 *   ![name]           On a line of its own: the drawable-nodpi PNG called name, centred, at
 *                     the art scale with nearest neighbour; its width in art pixels is the
 *                     PNG's pixel width (piece_village is 20 px wide, so 20 art pixels).
 *
 * Names are letters, digits and underscores, with no ".png". A picture wider than the
 * column is made smaller by a whole factor (2, 3, ...), keeping the middle pixel of every
 * block, never by a fraction; a name that matches no drawable is left out (and logged).
 * There is no inline styling and no nesting: a bullet is one line of text, and a line that
 * starts with "[" but is not "[name] text" is plain text.
 *
 * The screen scrolls, with the kit's pixel scroll thumb at the right edge.
 */
public class Rules extends Activity {

    private static final String TAG = "Rules";

    private View root;

    @Override
    public void onCreate(Bundle savedInstanceState) {
        setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().setFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN, WindowManager.LayoutParams.FLAG_FULLSCREEN);

        super.onCreate(savedInstanceState);
        PixelUi.refreshTheme(this);
        setContentView(R.layout.rules);
        int contentArt = PixelUi.skinScreen(this);
        root = ((android.view.ViewGroup) findViewById(android.R.id.content)).getChildAt(0);

        ((Button)findViewById(R.id.about_ok)).setOnClickListener(new OnClickListener() {
            @Override
            public void onClick(View v) {
                finish();
            }
        });

        ScrollView scroll = (ScrollView) findViewById(R.id.rules_scroll);
        if (Build.VERSION.SDK_INT >= 29) {
            scroll.setVerticalScrollBarEnabled(true);
            scroll.setScrollbarFadingEnabled(false);
            scroll.setScrollBarSize(2 * PixelUi.scale(this));
            scroll.setVerticalScrollbarThumbDrawable(new PixelUi.PaletteColor(PixelUi.PaletteColor.SCROLL_THUMB));
        }
        else {
            scroll.setVerticalScrollBarEnabled(false);
        }

        String text = About.readFile(this, R.raw.rules).toString();
        // 4 art px on the right are kept clear for the scroll thumb (rules.xml)
        build((LinearLayout) findViewById(R.id.rules_content), RulesMarkup.parse(text), contentArt - 4);
    }

    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        PixelUi.themeChanged(this, root);
    }

    /** Lays the blocks out one under another. Sizes are art pixels, skinned block by block. */
    private void build(LinearLayout column, List<RulesMarkup.Block> blocks, int columnArt) {
        int scale = PixelUi.scale(this);
        boolean first = true;
        for (RulesMarkup.Block block : blocks) {
            View view;
            int top;
            int left = 0;
            switch (block.kind) {
                case HEADING: {
                    TextView t = new PixelTextView(this, null);
                    t.setTag("title");
                    t.setText(block.text);
                    view = t;
                    top = 10;
                    break;
                }
                case BULLET: {
                    TextView t = new Bullet(this);
                    t.setText(block.text);
                    t.setPadding(8, 0, 0, 0);
                    view = t;
                    top = 3;
                    break;
                }
                case ICON: {
                    int id = drawableId(block.name);
                    if (id == 0) {
                        TextView t = new PixelTextView(this, null);
                        t.setText(block.text);
                        view = t;
                    }
                    else {
                        view = WelcomeDialog.iconLine(this, scale, id, block.text);
                    }
                    top = 4;
                    break;
                }
                case IMAGE: {
                    int id = drawableId(block.name);
                    if (id == 0) {
                        continue;
                    }
                    Bitmap bitmap = PixelImage.fit(PixelUi.decode(this, id), columnArt);
                    view = new PixelImage(this, bitmap, scale);
                    // Centred by hand: gravity could put an odd width on half an art pixel
                    left = (columnArt - bitmap.getWidth()) / 2;
                    top = 8;
                    break;
                }
                default: {
                    TextView t = new PixelTextView(this, null);
                    t.setText(block.text);
                    view = t;
                    top = 5;
                    break;
                }
            }

            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                    block.kind == RulesMarkup.Kind.IMAGE ? LinearLayout.LayoutParams.WRAP_CONTENT : LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT);
            lp.topMargin = first ? 3 : top;
            lp.leftMargin = left;
            if (block.kind == RulesMarkup.Kind.IMAGE) {
                lp.bottomMargin = 2;
            }
            view.setLayoutParams(lp);
            PixelUi.skin(view, scale, false);
            column.addView(view);
            first = false;
        }
    }

    /** The drawable-nodpi image with this name, or 0. */
    private int drawableId(String name) {
        int id = getResources().getIdentifier(name, "drawable", getPackageName());
        if (id == 0) {
            Log.w(TAG, "rules.txt names an image that does not exist: " + name);
        }
        return id;
    }

    /**
     * A line with a hanging bullet: the padding on the left holds a 3 art px square, so
     * every wrapped line stays indented. The square sits level with the x-height of the first line.
     */
    private static final class Bullet extends PixelTextView {

        private final int scale;
        private final Paint paint = new Paint();

        Bullet(Activity activity) {
            super(activity, null);
            scale = PixelUi.scale(activity);
            paint.setAntiAlias(false);
        }

        @Override
        protected void onDraw(Canvas canvas) {
            if (getLayout() != null) {
                int baseline = getExtendedPaddingTop() + getLayout().getLineBaseline(0);
                int r = baseline % scale;
                baseline += r < scale / 2f ? -r : scale - r;
                int top = baseline - 5 * scale;
                paint.setColor(PixelUi.palette().ink);
                canvas.drawRect(scale, top, 4 * scale, top + 3 * scale, paint);
            }
            super.onDraw(canvas);
        }
    }
}
