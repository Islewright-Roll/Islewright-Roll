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

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.HashMap;

import android.content.res.Resources;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffColorFilter;
import android.graphics.Rect;

/**
 * Text from a baked glyph atlas, for drawing onto the art bitmap at one art pixel per
 * font pixel. The atlas and metrics come from tools/bake_font.py, whose header is the
 * format's specification: a white, alpha only atlas, and one "codepoint x y w h advance"
 * line per glyph after a "height baseline" first line.
 */
final class BitmapFont {
   private static final int X = 0, Y = 1, W = 2, H = 3, ADVANCE = 4, INK = 5;

   private final Bitmap atlas;
   private final HashMap<Integer, int[]> glyphs = new HashMap<Integer, int[]>();
   private final HashMap<Integer, PorterDuffColorFilter> tints = new HashMap<Integer, PorterDuffColorFilter>();
   private final Paint paint = new Paint();
   private final Rect src = new Rect();
   private final Rect dest = new Rect();
   private int height;

   private BitmapFont(Bitmap atlas) {
      this.atlas = atlas;
      paint.setFilterBitmap(false);
      paint.setAntiAlias(false);
      paint.setDither(false);
   }

   static BitmapFont load(Resources resources, int atlasRes, int metricsRes) {
      BitmapFont font = new BitmapFont(ArtCanvas.load(resources, atlasRes));
      try {
         BufferedReader reader = new BufferedReader(new InputStreamReader(resources.openRawResource(metricsRes), "UTF-8"));
         try {
            font.height = Integer.parseInt(reader.readLine().trim().split(" ")[0]);
            String line;
            while((line = reader.readLine()) != null) {
               String[] f = line.trim().split(" ");
               if(f.length < 6) {
                  continue;
               }
               int[] g = new int[6];
               for(int i = 0; i < 5; i++) {
                  g[i] = Integer.parseInt(f[i + 1]);
               }
               g[INK] = font.inkRight(g);
               font.glyphs.put(Integer.parseInt(f[0]), g);
            }
         }
         finally {
            reader.close();
         }
      }
      catch(IOException e) {
         throw new IllegalStateException("font metrics unreadable", e);
      }
      return font;
   }

   /** One past the last inked column of a glyph's cell, so text can be centred on its ink. */
   private int inkRight(int[] g) {
      for(int x = g[W] - 1; x >= 0; x--) {
         for(int y = 0; y < g[H]; y++) {
            if((atlas.getPixel(g[X] + x, g[Y] + y) >>> 24) != 0) {
               return x + 1;
            }
         }
      }
      return 0;
   }

   private int[] glyph(char c) {
      int[] g = glyphs.get((int) c);
      return g != null ? g : glyphs.get((int) '?');
   }

   /** Pen advance of the whole string. */
   int width(String s) {
      int width = 0;
      for(int i = 0; i < s.length(); i++) {
         int[] g = glyph(s.charAt(i));
         if(g != null) {
            width += g[ADVANCE];
         }
      }
      return width;
   }

   /** Width up to the last inked pixel, without the final glyph's trailing space. */
   int inkWidth(String s) {
      int pen = 0;
      int ink = 0;
      for(int i = 0; i < s.length(); i++) {
         int[] g = glyph(s.charAt(i));
         if(g != null) {
            if(g[INK] > 0) {
               ink = pen + g[INK];
            }
            pen += g[ADVANCE];
         }
      }
      return ink;
   }

   int height() {
      return height;
   }

   /** (x, y) is the top left of the line box. */
   void draw(Canvas art, String s, int x, int y, int argb) {
      PorterDuffColorFilter tint = tints.get(argb);
      if(tint == null) {
         tint = new PorterDuffColorFilter(argb, PorterDuff.Mode.SRC_IN);
         tints.put(argb, tint);
      }
      paint.setColorFilter(tint);
      int pen = x;
      for(int i = 0; i < s.length(); i++) {
         int[] g = glyph(s.charAt(i));
         if(g == null) {
            continue;
         }
         src.set(g[X], g[Y], g[X] + g[W], g[Y] + g[H]);
         dest.set(pen, y, pen + g[W], y + g[H]);
         art.drawBitmap(atlas, src, dest, paint);
         pen += g[ADVANCE];
      }
   }

   /** The string with a 1 px outline all round, for text that sits on terrain. */
   void drawOutlined(Canvas art, String s, int x, int y, int argb, int outlineArgb) {
      for(int dy = -1; dy <= 1; dy++) {
         for(int dx = -1; dx <= 1; dx++) {
            if(dx != 0 || dy != 0) {
               draw(art, s, x + dx, y + dy, outlineArgb);
            }
         }
      }
      draw(art, s, x, y, argb);
   }
}
