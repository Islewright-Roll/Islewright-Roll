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

/**
 * The pure parts of the board's flat layers, kept free of Android so a unit test can reach
 * them: how far the glow round a buildable road is greyed, and how the footprints split
 * into two layers that never share a pixel.
 */
final class BoardLayers {
   private BoardLayers() {
   }

   /**
    * A colour with its saturation scaled by factor (1 leaves it alone, 0 is its grey). Each
    * colour channel is mixed toward the colour's own luma, so the brightness stays put and
    * only the colour drains out. The alpha byte is kept. Integer rounding, no Android.
    */
   static int desaturate(int argb, float factor) {
      int r = (argb >> 16) & 0xFF;
      int g = (argb >> 8) & 0xFF;
      int b = argb & 0xFF;
      float luma = (r * 299 + g * 587 + b * 114) / 1000f;
      return (argb & 0xFF000000)
            | (mix(luma, r, factor) << 16)
            | (mix(luma, g, factor) << 8)
            | mix(luma, b, factor);
   }

   private static int mix(float luma, float channel, float factor) {
      int v = Math.round(luma + (channel - luma) * factor);
      return Math.max(0, Math.min(255, v));
   }

   /**
    * A colour mixed toward white by factor (0 leaves it alone, 1 is white). Each channel moves
    * that share of the way to 255; the alpha byte is kept. The tint of the glow round a
    * resource being banked, which is the player's colour but lighter than a buildable spot's.
    */
   static int lighten(int argb, float factor) {
      int r = (argb >> 16) & 0xFF;
      int g = (argb >> 8) & 0xFF;
      int b = argb & 0xFF;
      return (argb & 0xFF000000)
            | (toward(r, factor) << 16)
            | (toward(g, factor) << 8)
            | toward(b, factor);
   }

   private static int toward(int channel, float factor) {
      return Math.max(0, Math.min(255, Math.round(channel + (255 - channel) * factor)));
   }

   /**
    * The colour of the soft ring outside a glow's outline: black round a colour light enough
    * to stand out of a shadow (brighter than lightAbove, 0 to 255), white round one that is not.
    */
   static int haloRgb(int rgb, int lightAbove) {
      int brightness = (((rgb >> 16) & 0xFF) * 299 + ((rgb >> 8) & 0xFF) * 587 + (rgb & 0xFF) * 114) / 1000;
      return brightness > lightAbove ? 0x000000 : 0xFFFFFF;
   }

   /**
    * Grows a mask outward, one step per pass. `dist` holds 0 on the mask's pixels and -1
    * elsewhere; after the call, a pixel that is `step` steps outside the mask (4 way) holds
    * `step`, up to `reach`. `tier` holds each mask pixel's strength and `roadOnly` whether
    * it is road alone; a grown pixel takes the strongest tier of the pixels it grew from and
    * is road only when all of them are. A step grows only from the one before it, never from
    * pixels it has just marked. The one outline-and-ring maker: the glow round buildable spots
    * and the glow round a resource being banked are both this.
    */
   static void grow(int[] dist, int[] tier, boolean[] roadOnly, int w, int h, int reach) {
      for(int step = 1; step <= reach; step++) {
         for(int y = 0; y < h; y++) {
            for(int x = 0; x < w; x++) {
               int p = y * w + x;
               if(dist[p] != -1) {
                  continue;
               }
               int best = 0;
               boolean fromRoadsOnly = true;
               if(x > 0 && dist[p - 1] == step - 1) {
                  best = Math.max(best, tier[p - 1]);
                  fromRoadsOnly &= roadOnly[p - 1];
               }
               if(x < w - 1 && dist[p + 1] == step - 1) {
                  best = Math.max(best, tier[p + 1]);
                  fromRoadsOnly &= roadOnly[p + 1];
               }
               if(y > 0 && dist[p - w] == step - 1) {
                  best = Math.max(best, tier[p - w]);
                  fromRoadsOnly &= roadOnly[p - w];
               }
               if(y < h - 1 && dist[p + w] == step - 1) {
                  best = Math.max(best, tier[p + w]);
                  fromRoadsOnly &= roadOnly[p + w];
               }
               if(best > 0) {
                  // Marked now, settled after the pass, so a step only grows from the one before
                  dist[p] = -2 - step;
                  tier[p] = best;
                  roadOnly[p] = fromRoadsOnly;
               }
            }
         }
         for(int p = 0; p < dist.length; p++) {
            if(dist[p] == -2 - step) {
               dist[p] = step;
            }
         }
      }
   }

   /**
    * The colour of a pixel at distance d from the mask (1 the outline, 2 the ring), or 0 for
    * a pixel that holds no glow. strength scales the opacity (1 leaves it as is).
    */
   static int glowArgb(int d, int outlineRgb, int ringRgb, int outlineAlpha, int ringAlpha, float strength) {
      if(d != 1 && d != 2) {
         return 0;
      }
      int alpha = Math.round((d == 1 ? outlineAlpha : ringAlpha) * strength);
      return (alpha << 24) | ((d == 1 ? outlineRgb : ringRgb) & 0xFFFFFF);
   }

   /**
    * The outline and ring round one flat mask (w by h, with at least two clear pixels all
    * round it), as ARGB pixels: grow, then colour. For a single silhouette in a single tier.
    */
   static int[] outline(boolean[] mask, int w, int h, int outlineRgb, int ringRgb,
         int outlineAlpha, int ringAlpha, float strength) {
      int[] dist = new int[w * h];
      int[] tier = new int[dist.length];
      boolean[] roadOnly = new boolean[dist.length];
      for(int p = 0; p < dist.length; p++) {
         dist[p] = mask[p] ? 0 : -1;
         tier[p] = mask[p] ? 1 : 0;
      }
      grow(dist, tier, roadOnly, w, h, 2);
      int[] out = new int[dist.length];
      for(int p = 0; p < out.length; p++) {
         out[p] = glowArgb(dist[p], outlineRgb, ringRgb, outlineAlpha, ringAlpha, strength);
      }
      return out;
   }

   /**
    * The road footprint layer: every pixel of the roads' mask that no standing piece's mask
    * holds. The standing footprints draw over it from their own layer, so a pixel is in at
    * most one of the two and is never darkened twice.
    */
   static boolean[] cutOut(boolean[] roads, boolean[] standing) {
      boolean[] out = new boolean[roads.length];
      for(int i = 0; i < out.length; i++) {
         out[i] = roads[i] && !standing[i];
      }
      return out;
   }
}
