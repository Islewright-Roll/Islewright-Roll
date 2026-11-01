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
import java.util.IdentityHashMap;

import android.graphics.Bitmap;

/**
 * Battle for Wesnoth team colouring, ported from src/color_range.cpp recolor_palette().
 * A sprite carries a fixed magenta palette (the horseman's trim, the house roofs, the
 * road kerbs) whose first entry is the reference shade. Each palette colour maps onto a ramp
 * through the player's colour: at or below the reference's brightness it runs from the
 * player colour down to black, above it from the player colour up to white.
 */
final class TeamColor {
   static final int[] MAGENTA = {
      0xF49AC1, 0x3F0016, 0x55002A, 0x690039, 0x7B0045, 0x8C0051, 0x9E005D, 0xB10069, 0xC30074, 0xD6007F,
      0xEC008C, 0xEE3D96, 0xEF5BA1, 0xF172AC, 0xF287B6, 0xF6ADCD, 0xF8C1D9, 0xFAD5E5, 0xFDE9F1,
   };

   // Edge pixels are partly transparent, and Android stores them premultiplied, so they
   // come back a shade or two off their palette colour. Match them this loosely.
   private static final int SOFT_TOLERANCE = 8;

   // One cache per colour in use: the roads are drawn in a lighter version of the player's
   // colour and everything else in the colour itself, so two colours are asked for every frame.
   // A colour picked and dropped (the picker's preview) is forgotten once there are too many.
   private static final int MAX_COLORS = 4;
   private static final HashMap<Integer, IdentityHashMap<Bitmap, Bitmap>> caches = new HashMap<Integer, IdentityHashMap<Bitmap, Bitmap>>();

   private TeamColor() {
   }

   /** The sprite in the given colour. Cached per sprite and colour. */
   static Bitmap recolor(Bitmap sprite, int[] palette, int playerColor) {
      playerColor |= 0xFF000000;
      IdentityHashMap<Bitmap, Bitmap> cache = caches.get(playerColor);
      if(cache == null) {
         if(caches.size() >= MAX_COLORS) {
            caches.clear();
         }
         cache = new IdentityHashMap<Bitmap, Bitmap>();
         caches.put(playerColor, cache);
      }
      Bitmap done = cache.get(sprite);
      if(done == null) {
         done = recolorUncached(sprite, palette, playerColor);
         cache.put(sprite, done);
      }
      return done;
   }

   static Bitmap recolorUncached(Bitmap sprite, int[] palette, int playerColor) {
      int[] ramp = ramp(palette, playerColor);
      int w = sprite.getWidth();
      int h = sprite.getHeight();
      int[] pixels = new int[w * h];
      sprite.getPixels(pixels, 0, w, 0, 0, w, h);
      for(int i = 0; i < pixels.length; i++) {
         int alpha = pixels[i] >>> 24;
         if(alpha == 0) {
            continue;
         }
         int rgb = pixels[i] & 0xFFFFFF;
         int match = find(palette, rgb, alpha == 255 ? 0 : SOFT_TOLERANCE);
         if(match >= 0) {
            pixels[i] = (alpha << 24) | ramp[match];
         }
      }
      Bitmap out = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
      out.setPixels(pixels, 0, w, 0, 0, w, h);
      return out;
   }

   /**
    * The sprite without its team colour: only the fully opaque pixels that are not in
    * the palette, everything else clear. For a road that is the cobbled surface with the
    * kerb taken off. Independent of the player colour, so the caller keeps the result.
    */
   static Bitmap unpainted(Bitmap sprite, int[] palette) {
      int w = sprite.getWidth();
      int h = sprite.getHeight();
      int[] pixels = new int[w * h];
      sprite.getPixels(pixels, 0, w, 0, 0, w, h);
      for(int i = 0; i < pixels.length; i++) {
         if((pixels[i] >>> 24) != 255 || find(palette, pixels[i] & 0xFFFFFF, 0) >= 0) {
            pixels[i] = 0;
         }
      }
      Bitmap out = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
      out.setPixels(pixels, 0, w, 0, 0, w, h);
      return out;
   }

   private static int[] ramp(int[] palette, int playerColor) {
      int refAvg = average(palette[0]);
      int[] ramp = new int[palette.length];
      for(int i = 0; i < palette.length; i++) {
         int avg = average(palette[i]);
         int out = 0;
         for(int shift = 16; shift >= 0; shift -= 8) {
            int mid = (playerColor >> shift) & 0xFF;
            int value;
            if(avg <= refAvg) {
               value = Math.round(mid * (avg / (float) refAvg));
            }
            else {
               value = Math.round(255 + (mid - 255) * ((255 - avg) / (float) (255 - refAvg)));
            }
            out |= value << shift;
         }
         ramp[i] = out;
      }
      return ramp;
   }

   private static int average(int rgb) {
      return (((rgb >> 16) & 0xFF) + ((rgb >> 8) & 0xFF) + (rgb & 0xFF)) / 3;
   }

   private static int find(int[] palette, int rgb, int tolerance) {
      int best = -1;
      int bestDistance = tolerance + 1;
      for(int i = 0; i < palette.length; i++) {
         if(palette[i] == rgb) {
            return i;
         }
         if(tolerance > 0) {
            int d = Math.max(Math.abs(((palette[i] >> 16) & 0xFF) - ((rgb >> 16) & 0xFF)),
                  Math.max(Math.abs(((palette[i] >> 8) & 0xFF) - ((rgb >> 8) & 0xFF)),
                        Math.abs((palette[i] & 0xFF) - (rgb & 0xFF))));
            if(d < bestDistance) {
               bestDistance = d;
               best = i;
            }
         }
      }
      return best;
   }
}
