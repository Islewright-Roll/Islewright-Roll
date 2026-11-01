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
 * Where each trader stands on the board: the map's anchor (the hex centre plus the map's
 * own offset), moved so that the traders of one hex are centred on that hex. A hex with
 * one trader already is (see Art.TRADER_AX and TRADER_AY), so it is not moved. A hex with
 * several (Dunholm's desert pairs, one cart above the other with the room between them for
 * the one resource they share) is moved so that the combined opaque box of its sprites is
 * centred on the hex centre, both ways.
 */
final class TraderLayout {
   private TraderLayout() {
   }

   /** The anchors, in the map's trader order. */
   static Pt[] anchors(MapDef map) {
      Pt[] out = new Pt[map.traders.length];
      for(int i = 0; i < out.length; i++) {
         int[] t = map.traders[i];
         int[] shift = pairShift(map, t[0]);
         out[i] = new Pt(map.hex[t[0]][0] + t[1] + shift[0], map.hex[t[0]][1] + t[2] + shift[1]);
      }
      return out;
   }

   /** How far the traders of a hex move to centre them; none for a hex with one trader. */
   static int[] pairShift(MapDef map, int hex) {
      int count = 0;
      int minX = Integer.MAX_VALUE, minY = Integer.MAX_VALUE, maxX = Integer.MIN_VALUE, maxY = Integer.MIN_VALUE;
      for(int[] t : map.traders) {
         if(t[0] != hex) {
            continue;
         }
         count++;
         // The sprite's opaque box, relative to the hex centre
         int left = t[1] - Art.TRADER_AX;
         int top = t[2] - Art.TRADER_AY;
         minX = Math.min(minX, left);
         maxX = Math.max(maxX, left + Art.TRADER_BOX_W);
         minY = Math.min(minY, top);
         maxY = Math.max(maxY, top + Art.TRADER_BOX_H);
      }
      if(count < 2) {
         return new int[] { 0, 0 };
      }
      return new int[] { (int) Math.floor(-(minX + maxX) / 2.0 + 0.5), (int) Math.floor(-(minY + maxY) / 2.0 + 0.5) };
   }

   /**
    * Where the one resource of a bank slot that more than one trader stands behind rests, as
    * the top left of its 12 px cell in board pixels: centred between the traders' carts, and
    * as far down as the middle of the carts' height less half the cell (see
    * Art.TRADER_SHARED_DROP), so that it lies on the ground between them. Null for a slot
    * with a single trader, whose resource rides on its own cart.
    */
   static Pt sharedAt(MapDef map, int slot) {
      int[] tiers = map.bankTraders[slot];
      if(tiers.length < 2) {
         return null;
      }
      Pt[] at = anchors(map);
      int sumX = 0, sumTop = 0;
      for(int tier : tiers) {
         sumX += at[tier - 1].x;
         sumTop += at[tier - 1].y - Art.TRADER_AY;
      }
      return new Pt(Math.floorDiv(sumX, tiers.length) - Art.SHARED_CELL / 2,
            Math.floorDiv(sumTop, tiers.length) + Art.TRADER_SHARED_DROP);
   }
}
