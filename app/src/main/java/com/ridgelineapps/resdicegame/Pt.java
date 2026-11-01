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

/** A board position in board pixels. Plain Java, so the geometry runs under JUnit. */
final class Pt {
   public final int x;
   public final int y;

   Pt(int x, int y) {
      this.x = x;
      this.y = y;
   }

   @Override
   public boolean equals(Object o) {
      if(!(o instanceof Pt)) {
         return false;
      }
      Pt p = (Pt) o;
      return p.x == x && p.y == y;
   }

   @Override
   public int hashCode() {
      return x * 31 + y;
   }

   @Override
   public String toString() {
      return "(" + x + "," + y + ")";
   }
}
