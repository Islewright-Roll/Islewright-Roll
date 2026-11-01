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

import java.util.HashMap;

import android.content.res.Resources;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Paint.Style;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffColorFilter;
import android.graphics.Rect;

/**
 * The art framebuffer: the whole game screen is drawn here at one art pixel per bitmap
 * pixel, then blitted to the view once per frame at an integer scale with nearest
 * neighbour sampling, so every art pixel lands as an exact scale x scale block of screen
 * pixels. Nothing is ever drawn straight to the view's own canvas except that blit.
 */
final class ArtCanvas {
   /** Narrowest and shortest art space the layout is designed for. */
   static final int DESIGN_W = 216;
   static final int MIN_H = 352;

   int scale = 1;
   int width;
   int height;
   // Where the scaled bitmap lands in the view: whatever the integer scale leaves over
   // (at most scale - 1 screen px per axis) is split either side
   int offsetX;
   int offsetY;

   Bitmap bitmap;
   Canvas canvas;

   private final Paint blitPaint = nearestPaint();
   private final Paint spritePaint = nearestPaint();
   private final Paint fillPaint = new Paint();
   private final Rect src = new Rect();
   private final Rect dest = new Rect();
   private final HashMap<Integer, PorterDuffColorFilter> tints = new HashMap<Integer, PorterDuffColorFilter>();

   ArtCanvas() {
      fillPaint.setStyle(Style.FILL);
      fillPaint.setAntiAlias(false);
   }

   static Paint nearestPaint() {
      Paint paint = new Paint();
      paint.setFilterBitmap(false);
      paint.setAntiAlias(false);
      paint.setDither(false);
      return paint;
   }

   /** Decodes a drawable-nodpi PNG at its own pixel size, whatever the screen density. */
   static Bitmap load(Resources resources, int id) {
      BitmapFactory.Options options = new BitmapFactory.Options();
      options.inScaled = false;
      options.inPreferredConfig = Bitmap.Config.ARGB_8888;
      return BitmapFactory.decodeResource(resources, id, options);
   }

   /**
    * Picks the scale for a view size and (re)allocates the bitmap. The bitmap covers
    * the whole view; obscuredH is the part of its height the layout cannot use, so the
    * scale is chosen for what is left. True if anything changed.
    */
   boolean resize(int viewW, int viewH, int obscuredH) {
      int newScale = Math.max(1, Math.min(viewW / DESIGN_W, (viewH - obscuredH) / MIN_H));
      int newW = Math.max(1, viewW / newScale);
      int newH = Math.max(1, viewH / newScale);
      offsetX = (viewW - newW * newScale) / 2;
      offsetY = (viewH - newH * newScale) / 2;
      if(bitmap != null && newScale == scale && newW == width && newH == height) {
         return false;
      }
      scale = newScale;
      width = newW;
      height = newH;
      bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888);
      canvas = new Canvas(bitmap);
      return true;
   }

   void blit(Canvas screen) {
      src.set(0, 0, width, height);
      dest.set(offsetX, offsetY, offsetX + width * scale, offsetY + height * scale);
      screen.drawBitmap(bitmap, src, dest, blitPaint);
   }

   int toArtX(float screenX) {
      return (int) Math.floor((screenX - offsetX) / scale);
   }

   int toArtY(float screenY) {
      return (int) Math.floor((screenY - offsetY) / scale);
   }

   void fill(int x, int y, int w, int h, int argb) {
      fillPaint.setColor(argb);
      canvas.drawRect(x, y, x + w, y + h, fillPaint);
   }

   /** A filled box with its four corner pixels left off. */
   void plate(int x, int y, int w, int h, int argb) {
      fill(x + 1, y, w - 2, h, argb);
      fill(x, y + 1, 1, h - 2, argb);
      fill(x + w - 1, y + 1, 1, h - 2, argb);
   }

   void frame(int x, int y, int w, int h, int argb) {
      fill(x, y, w, 1, argb);
      fill(x, y + h - 1, w, 1, argb);
      fill(x, y + 1, 1, h - 2, argb);
      fill(x + w - 1, y + 1, 1, h - 2, argb);
   }

   void sprite(Bitmap sprite, int x, int y) {
      sprite(sprite, x, y, 255);
   }

   void sprite(Bitmap sprite, int x, int y, int alpha) {
      spritePaint.setColorFilter(null);
      spritePaint.setAlpha(alpha);
      canvas.drawBitmap(sprite, x, y, spritePaint);
   }

   /** Part of a sprite, uncut rows and columns only: (sx, sy, w, h) of it at (x, y). */
   void spritePart(Bitmap sprite, int sx, int sy, int w, int h, int x, int y) {
      spritePaint.setColorFilter(null);
      spritePaint.setAlpha(255);
      src.set(sx, sy, sx + w, sy + h);
      dest.set(x, y, x + w, y + h);
      canvas.drawBitmap(sprite, src, dest, spritePaint);
   }

   /**
    * A sprite repeated left to right across w pixels from x, the last copy cut short. For
    * a sprite that is seamless along its width; rows and columns are only copied.
    */
   void tile(Bitmap sprite, int x, int y, int w) {
      int tileW = sprite.getWidth();
      for(int done = 0; done < w; done += tileW) {
         spritePart(sprite, 0, 0, Math.min(tileW, w - done), sprite.getHeight(), x + done, y);
      }
   }

   /** A sprite's shape filled with one colour; the colour's own alpha is kept. */
   void silhouette(Bitmap mask, int x, int y, int argb) {
      PorterDuffColorFilter tint = tints.get(argb);
      if(tint == null) {
         tint = new PorterDuffColorFilter(argb, PorterDuff.Mode.SRC_IN);
         tints.put(argb, tint);
      }
      spritePaint.setAlpha(255);
      spritePaint.setColorFilter(tint);
      canvas.drawBitmap(mask, x, y, spritePaint);
   }

   /**
    * Draws a square 9-slice at any size: the corners are copied, the edges and the
    * middle are stretched. The sources are flat along the direction they stretch in,
    * so the result is the same as repeating pixels and nothing is resampled.
    */
   void nineSlice(Bitmap source, int x, int y, int w, int h, int corner) {
      spritePaint.setColorFilter(null);
      spritePaint.setAlpha(255);
      int size = source.getWidth();
      int[] sx = { 0, corner, size - corner, size };
      int[] dx = { x, x + corner, x + w - corner, x + w };
      int[] dy = { y, y + corner, y + h - corner, y + h };
      for(int row = 0; row < 3; row++) {
         for(int col = 0; col < 3; col++) {
            src.set(sx[col], sx[row], sx[col + 1], sx[row + 1]);
            dest.set(dx[col], dy[row], dx[col + 1], dy[row + 1]);
            canvas.drawBitmap(source, src, dest, spritePaint);
         }
      }
   }

   /** Opaque white wherever the sprite is at least half solid, so soft shadows drop out. */
   static Bitmap mask(Bitmap sprite) {
      int w = sprite.getWidth();
      int h = sprite.getHeight();
      int[] pixels = new int[w * h];
      sprite.getPixels(pixels, 0, w, 0, 0, w, h);
      for(int i = 0; i < pixels.length; i++) {
         pixels[i] = (pixels[i] >>> 24) >= 128 ? 0xFFFFFFFF : 0;
      }
      Bitmap out = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
      out.setPixels(pixels, 0, w, 0, 0, w, h);
      return out;
   }

   /** How many rows at the bottom of a sprite hold nothing at least half solid. */
   static int clearRowsBelow(Bitmap sprite) {
      int w = sprite.getWidth();
      int h = sprite.getHeight();
      for(int y = h - 1; y >= 0; y--) {
         for(int x = 0; x < w; x++) {
            if((sprite.getPixel(x, y) >>> 24) >= 128) {
               return h - 1 - y;
            }
         }
      }
      return h;
   }

   /** The mask's inner rim: its solid pixels that touch a clear one or the bitmap edge. */
   static Bitmap rim(Bitmap mask) {
      int w = mask.getWidth();
      int h = mask.getHeight();
      int[] in = new int[w * h];
      int[] out = new int[w * h];
      mask.getPixels(in, 0, w, 0, 0, w, h);
      for(int y = 0; y < h; y++) {
         for(int x = 0; x < w; x++) {
            if(in[y * w + x] == 0) {
               continue;
            }
            boolean edge = x == 0 || y == 0 || x == w - 1 || y == h - 1
                  || in[y * w + x - 1] == 0 || in[y * w + x + 1] == 0
                  || in[(y - 1) * w + x] == 0 || in[(y + 1) * w + x] == 0;
            if(edge) {
               out[y * w + x] = 0xFFFFFFFF;
            }
         }
      }
      Bitmap rim = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
      rim.setPixels(out, 0, w, 0, 0, w, h);
      return rim;
   }
}
