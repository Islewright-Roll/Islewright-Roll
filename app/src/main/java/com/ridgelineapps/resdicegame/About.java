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

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.os.Bundle;
import android.text.Spannable;
import android.text.Spanned;
import android.text.TextPaint;
import android.text.style.URLSpan;
import android.text.util.Linkify;
import android.view.View;
import android.view.View.OnClickListener;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.TextView;

public class About extends Activity {

    private View root;

    @Override
    public void onConfigurationChanged(android.content.res.Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        PixelUi.themeChanged(this, root);
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);        
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().setFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN, WindowManager.LayoutParams.FLAG_FULLSCREEN);
        
        super.onCreate(savedInstanceState);
        PixelUi.refreshTheme(this);
        setContentView(R.layout.about);
        PixelUi.skinScreen(this);
        root = ((android.view.ViewGroup) findViewById(android.R.id.content)).getChildAt(0);
        
        ((Button)findViewById(R.id.about_ok)).setOnClickListener(new OnClickListener() {
            @Override
            public void onClick(View v) {
                finish();
            }
        });
        
        String text = readFile(this, R.raw.about).toString().replace("{{version}}", version());
        TextView t = (TextView) findViewById(R.id.about_text);
        t.setText(text);
        Linkify.addLinks(t, Linkify.WEB_URLS);
        stripLinkUnderlines(t);
    }

    /**
     * Linkify underlines links with a hairline that is not on the art grid. The link is
     * still tappable and is coloured differently from the body; only the line goes.
     */
    private static void stripLinkUnderlines(TextView t) {
        Spannable spannable = (Spannable) t.getText();
        for (URLSpan span : spannable.getSpans(0, spannable.length(), URLSpan.class)) {
            int start = spannable.getSpanStart(span);
            int end = spannable.getSpanEnd(span);
            spannable.removeSpan(span);
            spannable.setSpan(new URLSpan(span.getURL()) {
                @Override
                public void updateDrawState(TextPaint ds) {
                    super.updateDrawState(ds);
                    ds.setUnderlineText(false);
                }
            }, start, end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        }
    }

    /**
     * The version the user is actually running.
     *
     * A release shows its name alone, because the name is already unique. A pre-release
     * build shows the build number too: several of those share one name (1.19-rc might be
     * built a dozen times) and the number is the only thing telling them apart, which is
     * exactly what a bug report needs. The suffix is what distinguishes the two, so
     * "1.19-rc" gets the number and "1.19" does not.
     */
    private String version() {
        try {
            android.content.pm.PackageInfo info =
                    getPackageManager().getPackageInfo(getPackageName(), 0);
            String name = info.versionName;
            if (name != null && name.indexOf('-') >= 0) {
                return "v" + name + " (" + info.versionCode + ")";
            }
            return "v" + name;
        }
        catch (Exception e) {
            // Cannot happen for our own package, but a blank line beats a crash on About.
            return "";
        }
    }

    static CharSequence readFile(Activity activity, int id) {
        BufferedReader in = null;
        try {
            in = new BufferedReader(new InputStreamReader(
                    activity.getResources().openRawResource(id)));
            String line;
            StringBuilder buffer = new StringBuilder();
            while ((line = in.readLine()) != null) buffer.append(line).append('\n');
            return buffer;
        } catch (IOException e) {
            return "";
        } finally {
           if (in != null) {
              try {
                  in.close();
              } catch (IOException e) {
                  // Ignore
              }
          }
        }
    }
}

