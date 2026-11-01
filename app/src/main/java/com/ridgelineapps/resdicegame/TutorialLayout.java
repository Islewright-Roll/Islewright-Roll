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
import java.util.List;

/**
 * Where a tutorial callout goes, in art pixels: pure arithmetic with no Android types, so
 * it is unit tested on both canvas sizes. Given the rectangle of the thing a callout
 * describes, the callout's text and its two buttons' labels, it wraps the text, sizes the
 * panel and puts it beside the target (never over it, never off the canvas), with a small
 * pointer on the panel's edge that faces the target. The target's own 1 px outline is the
 * caller's to draw; the panel stays clear of it.
 */
final class TutorialLayout {

   /** How wide a piece of text is, in art pixels (BitmapFont.inkWidth). */
   interface Measure {
      int width(String text);
   }

   /** Which side of the target the panel sits. */
   enum Side { ABOVE, BELOW, LEFT, RIGHT }

   // The pointer: a triangle POINTER pixels deep whose base is 2 * HALF_BASE + 1 wide
   static final int POINTER = 5;
   static final int HALF_BASE = 4;
   // The target's outline is one pixel outside its rectangle
   static final int OUTLINE = 1;
   // The least space left between a panel and the canvas edge
   static final int MARGIN = 3;
   // The panel's frame is 4 px of art; text starts this far in from the panel's edge
   static final int PAD_X = 8;
   static final int PAD_TOP = 7;
   static final int PAD_BOTTOM = 5;
   // Distance from one line of text's top to the next, and a line's height in ink (the dots over
   // an umlaut down to a descender: Ark Pixel's rows 1 to 12). The pitch leaves a clear row
   // between one line's descenders and the next line's accents.
   static final int LINE_PITCH = 12;
   static final int LINE_INK = 12;
   // The button row: hit areas this tall (at least 14), the plates drawn inside them
   static final int BUTTON_H = 14;
   static final int BUTTON_PAD = 5;
   static final int BUTTON_GAP = 8;
   // Space between the last line of text and the buttons
   static final int TEXT_TO_BUTTONS = 3;
   // The widest a panel is allowed to be (the HUD's width less a margin each side)
   static final int MAX_PANEL_W = 210;
   // The narrowest panel that is worth drawing beside a target (left and right)
   static final int MIN_SIDE_W = 90;

   /** A rectangle in art pixels. */
   static final class Rect {
      final int x, y, w, h;

      Rect(int x, int y, int w, int h) {
         this.x = x;
         this.y = y;
         this.w = w;
         this.h = h;
      }

      boolean contains(int px, int py) {
         return px >= x && px < x + w && py >= y && py < y + h;
      }

      boolean intersects(Rect o) {
         return x < o.x + o.w && o.x < x + w && y < o.y + o.h && o.y < y + h;
      }

      @Override
      public String toString() {
         return "[" + x + "," + y + " " + w + "x" + h + "]";
      }
   }

   /** Everything GameView needs to draw one callout. */
   static final class Placement {
      Side side;
      /** The panel, frame included. */
      Rect panel;
      /** The wrapped text, one line per entry, the first line's top at (textX, textY). */
      List<String> lines;
      int textX;
      int textY;
      /** The pointer's tip, on the target's outline, and the centre of its base on the panel's edge. */
      int tipX;
      int tipY;
      int baseX;
      int baseY;
      /** The hit areas of the two buttons, and where each label's top left goes. */
      Rect gotIt;
      Rect skip;
      int gotItLabelX, gotItLabelY;
      int skipLabelX, skipLabelY;
   }

   private TutorialLayout() {
   }

   /**
    * The text broken into lines no wider than maxWidth. Breaks fall on spaces; a single word
    * wider than the line is left whole on its own line.
    */
   static List<String> wrap(String text, int maxWidth, Measure m) {
      ArrayList<String> lines = new ArrayList<String>();
      String line = "";
      for(String word : text.split(" +")) {
         if(word.length() == 0) {
            continue;
         }
         String joined = line.length() == 0 ? word : line + " " + word;
         if(line.length() > 0 && m.width(joined) > maxWidth) {
            lines.add(line);
            line = word;
         }
         else {
            line = joined;
         }
      }
      if(line.length() > 0) {
         lines.add(line);
      }
      return lines;
   }

   /**
    * Places a callout for a target rectangle on a canvas of canvasW x canvasH art pixels, whose
    * usable height runs from topLimit to bottomLimit (the system bars' padding is outside
    * it). A target in the lower half gets its panel above if there is room, then below, then
    * to a side; one in the upper half the other way round. Null only when there is room
    * on no side at all, which the two canvas sizes never have.
    */
   static Placement place(Rect target, String text, String gotItLabel, String skipLabel,
         int canvasW, int canvasH, int topLimit, int bottomLimit, Measure m) {
      boolean lower = target.y + target.h / 2 > (topLimit + bottomLimit) / 2;
      Side[] order = lower
            ? new Side[] { Side.ABOVE, Side.BELOW, Side.LEFT, Side.RIGHT }
            : new Side[] { Side.BELOW, Side.ABOVE, Side.RIGHT, Side.LEFT };
      for(Side side : order) {
         Placement p = tryPlace(side, target, text, gotItLabel, skipLabel, canvasW, topLimit, bottomLimit, m);
         if(p != null) {
            return p;
         }
      }
      return null;
   }

   private static Placement tryPlace(Side side, Rect t, String text, String gotItLabel, String skipLabel,
         int canvasW, int topLimit, int bottomLimit, Measure m) {
      int gap = OUTLINE + POINTER;
      // The room on this side of the target, as a width and a height the panel may use
      int maxW;
      int maxH;
      switch(side) {
         case ABOVE:
            maxW = Math.min(MAX_PANEL_W, canvasW - 2 * MARGIN);
            maxH = t.y - gap - (topLimit + MARGIN);
            break;
         case BELOW:
            maxW = Math.min(MAX_PANEL_W, canvasW - 2 * MARGIN);
            maxH = (bottomLimit - MARGIN) - (t.y + t.h + gap);
            break;
         case LEFT:
            maxW = Math.min(MAX_PANEL_W, t.x - gap - MARGIN);
            maxH = bottomLimit - topLimit - 2 * MARGIN;
            break;
         default:
            maxW = Math.min(MAX_PANEL_W, canvasW - MARGIN - (t.x + t.w + gap));
            maxH = bottomLimit - topLimit - 2 * MARGIN;
            break;
      }
      if(maxW < MIN_SIDE_W) {
         return null;
      }

      int gotW = m.width(gotItLabel) + 2 * BUTTON_PAD;
      int skipW = m.width(skipLabel) + 2 * BUTTON_PAD;
      int inner = maxW - 2 * PAD_X;
      List<String> lines = wrap(text, inner, m);
      int textW = 0;
      for(String line : lines) {
         textW = Math.max(textW, m.width(line));
      }
      // One row of buttons if both fit with a gap between, else the two stacked
      boolean oneRow = gotW + BUTTON_GAP + skipW <= inner;
      int buttonsW = oneRow ? gotW + BUTTON_GAP + skipW : Math.max(gotW, skipW);
      int w = Math.min(maxW, Math.max(textW, buttonsW) + 2 * PAD_X);
      // Space for the pointer's base inside the panel's width
      w = Math.max(w, 2 * (PAD_X + HALF_BASE) + 1);
      int buttonRows = oneRow ? 1 : 2;
      int h = PAD_TOP + (lines.size() - 1) * LINE_PITCH + LINE_INK + TEXT_TO_BUTTONS + buttonRows * BUTTON_H + PAD_BOTTOM;
      if(h > maxH) {
         return null;
      }

      int tcx = t.x + t.w / 2;
      int tcy = t.y + t.h / 2;
      int px;
      int py;
      switch(side) {
         case ABOVE:
            px = clamp(tcx - w / 2, MARGIN, canvasW - MARGIN - w);
            py = t.y - gap - h;
            break;
         case BELOW:
            px = clamp(tcx - w / 2, MARGIN, canvasW - MARGIN - w);
            py = t.y + t.h + gap;
            break;
         case LEFT:
            px = t.x - gap - w;
            py = clamp(tcy - h / 2, topLimit + MARGIN, bottomLimit - MARGIN - h);
            break;
         default:
            px = t.x + t.w + gap;
            py = clamp(tcy - h / 2, topLimit + MARGIN, bottomLimit - MARGIN - h);
            break;
      }

      Placement p = new Placement();
      p.side = side;
      p.panel = new Rect(px, py, w, h);
      p.lines = lines;
      p.textX = px + PAD_X;
      p.textY = py + PAD_TOP;

      // The pointer faces the target at its centre, pulled in to keep its base inside the
      // panel's straight edge, and kept within the target's own span where the target allows
      if(side == Side.ABOVE || side == Side.BELOW) {
         int lo = Math.max(px + PAD_X, t.x);
         int hi = Math.min(px + w - PAD_X - 1, t.x + t.w - 1);
         int bx = lo <= hi ? clamp(tcx, lo, hi) : clamp(tcx, px + PAD_X, px + w - PAD_X - 1);
         p.baseX = bx;
         p.tipX = bx;
         if(side == Side.ABOVE) {
            p.baseY = py + h;
            p.tipY = t.y - OUTLINE - 1;
         }
         else {
            p.baseY = py - 1;
            p.tipY = t.y + t.h + OUTLINE;
         }
      }
      else {
         int lo = Math.max(py + PAD_X, t.y);
         int hi = Math.min(py + h - PAD_X - 1, t.y + t.h - 1);
         int by = lo <= hi ? clamp(tcy, lo, hi) : clamp(tcy, py + PAD_X, py + h - PAD_X - 1);
         p.baseY = by;
         p.tipY = by;
         if(side == Side.LEFT) {
            p.baseX = px + w;
            p.tipX = t.x - OUTLINE - 1;
         }
         else {
            p.baseX = px - 1;
            p.tipX = t.x + t.w + OUTLINE;
         }
      }

      // The buttons: Got it at the left, Skip at the right, or stacked at the left
      int rowY = py + h - PAD_BOTTOM - buttonRows * BUTTON_H;
      int leftX = px + PAD_X;
      p.gotIt = new Rect(leftX, rowY, gotW, BUTTON_H);
      if(oneRow) {
         p.skip = new Rect(px + w - PAD_X - skipW, rowY, skipW, BUTTON_H);
      }
      else {
         p.skip = new Rect(leftX, rowY + BUTTON_H, skipW, BUTTON_H);
      }
      p.gotItLabelX = p.gotIt.x + BUTTON_PAD;
      p.gotItLabelY = p.gotIt.y;
      p.skipLabelX = p.skip.x + BUTTON_PAD;
      p.skipLabelY = p.skip.y;
      return p;
   }

   private static int clamp(int v, int lo, int hi) {
      return Math.max(lo, Math.min(hi, v));
   }
}
