package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import java.util.Arrays;
import java.util.List;

import org.junit.Test;

import com.ridgelineapps.resdicegame.TutorialLayout.Placement;
import com.ridgelineapps.resdicegame.TutorialLayout.Rect;
import com.ridgelineapps.resdicegame.TutorialLayout.Side;

/**
 * Where a callout's panel goes: wrapping, and the placement of the panel and its pointer for
 * each of the nine steps' targets on both canvas sizes the game meets (216 x 480 and
 * 240 x 426 art pixels). The targets are laid out here from GameView's own layout constants,
 * the way GameView.layout() does.
 */
public class TutorialLayoutTest {

   /** Five pixels a character, like the body font's average. */
   private static final TutorialLayout.Measure FIVE = new TutorialLayout.Measure() {
      @Override
      public int width(String text) {
         return text.length() * 5;
      }
   };

   private static final String[] TEXTS = {
      "Roll the dice to start your turn.",
      "Tap on dice to save them, then roll the rest. You get three rolls each turn.",
      "This is what each thing costs. Match your dice to it.",
      "Highlighted spots can be built with your dice. Tap one.",
      "Changed your mind? This takes back your last build, until your next roll.",
      "When you are done building, tap here to end the turn.",
      "The King's ship marks your turns. Score as much as you can before it reaches the pier.",
      "Two gold stand in for any one resource. A single gold can't be used on its own.",
      "Your trader banked this for you. It is spent by itself when a build needs it, on any turn.",
      "Das Schiff des Koenigs zaehlt deine Zuege. Erreiche 10 Punkte, bevor es den Anleger erreicht.",
   };

   // ---- wrapping ----------------------------------------------------------------------------

   @Test
   public void wrapBreaksOnSpacesWithinTheWidth() {
      List<String> lines = TutorialLayout.wrap("aaaa bbbb cccc dddd", 45, FIVE);
      assertEquals(Arrays.asList("aaaa bbbb", "cccc dddd"), lines);
   }

   @Test
   public void wrapKeepsALongWordWhole() {
      List<String> lines = TutorialLayout.wrap("hi supercalifragilistic ok", 40, FIVE);
      assertEquals(Arrays.asList("hi", "supercalifragilistic", "ok"), lines);
   }

   @Test
   public void wrapOfNothingIsNoLines() {
      assertTrue(TutorialLayout.wrap("   ", 40, FIVE).isEmpty());
      assertEquals(Arrays.asList("one"), TutorialLayout.wrap("one", 40, FIVE));
   }

   @Test
   public void wrapNeverLosesAWord() {
      for(String text : TEXTS) {
         for(int width = 60; width < 220; width += 20) {
            StringBuilder joined = new StringBuilder();
            for(String line : TutorialLayout.wrap(text, width, FIVE)) {
               joined.append(joined.length() == 0 ? "" : " ").append(line);
            }
            assertEquals(text, joined.toString());
         }
      }
   }

   // ---- the canvases ---------------------------------------------------------------------------

   /** One canvas: its size and the system bars' padding, with GameView's layout of the targets. */
   private static final class Canvas {
      final int w, h, padTop, padBottom;
      final int hudX, diceX, diceY, rollY;

      Canvas(int w, int h, int padTop, int padBottom) {
         this.w = w;
         this.h = h;
         this.padTop = padTop;
         this.padBottom = padBottom;
         hudX = (w - ArtCanvas.DESIGN_W) / 2;
         int diceRow = 6 * UIDice.size + 5 * GameView.DICE_GAP;
         diceX = (w - diceRow) / 2;
         diceY = h - padBottom - GameView.BOTTOM_H + 8;
         rollY = diceY + UIDice.size + 6;
      }

      Rect roll() {
         return new Rect((w - UIDiceRoll.WIDTH) / 2, rollY, UIDiceRoll.WIDTH, UIDiceRoll.HEIGHT);
      }

      /** The roll button grown for a long label, as UIDiceRoll widens it. */
      Rect wideRoll(int width) {
         return new Rect((w - width) / 2, rollY, width, UIDiceRoll.HEIGHT);
      }

      Rect dice() {
         return new Rect(diceX, diceY, 6 * UIDice.size + 5 * GameView.DICE_GAP, UIDice.size);
      }

      Rect die(int i) {
         return new Rect(diceX + i * (UIDice.size + GameView.DICE_GAP), diceY, UIDice.size, UIDice.size);
      }

      Rect costs() {
         TutorialLayout.Rect r = GameView.costsRect(hudX, padTop);
         return new Rect(r.x, r.y, r.w, r.h);
      }

      Rect undo() {
         return new Rect(GameView.undoX(hudX), rollY + (UIDiceRoll.HEIGHT - GameView.UNDO_SIZE) / 2, GameView.UNDO_SIZE, GameView.UNDO_SIZE);
      }

      Rect strip() {
         TutorialLayout.Rect r = GameView.stripRect(hudX, padTop);
         return new Rect(r.x, r.y, r.w, r.h);
      }

      Rect mini(int count) {
         int pitch = GameView.MINI_DIE + GameView.MINI_GAP;
         int inRow = Math.min(GameView.MINI_PER_ROW, count);
         int rows = (count + GameView.MINI_PER_ROW - 1) / GameView.MINI_PER_ROW;
         int x = hudX + GameView.MINI_MARGIN + GameView.MINI_PAD;
         int bottom = h - padBottom - GameView.BOTTOM_H - GameView.MINI_PAD;
         int top = bottom - GameView.MINI_DIE - (rows - 1) * pitch;
         return new Rect(x, top, inRow * pitch - GameView.MINI_GAP, bottom - top);
      }

      /** A village-sized spot at a board position, board origin about the middle of the free space. */
      Rect spot(int dx, int dy) {
         return new Rect(w / 2 + dx - 12, (padTop + GameView.TOP_H + h - GameView.BOTTOM_H - padBottom) / 2 + dy - 15, 24, 30);
      }
   }

   private static Canvas[] canvases() {
      return new Canvas[] {
         new Canvas(216, 480, 0, 0),
         new Canvas(216, 480, 6, 10),
         new Canvas(240, 426, 0, 0),
         new Canvas(240, 426, 6, 10),
      };
   }

   private static Placement place(Canvas c, Rect target, String text) {
      return TutorialLayout.place(target, text, "Got it", "Skip tutorial", c.w, c.h, c.padTop, c.h - c.padBottom, FIVE);
   }

   /** Every rule a placement keeps, whatever the target. */
   private static void assertSound(Canvas c, Rect target, String text, Placement p) {
      String what = "target " + target + " on " + c.w + "x" + c.h + " pads " + c.padTop + "," + c.padBottom + " text '" + text + "'";
      assertNotNull(what, p);
      Rect panel = p.panel;
      // On the canvas, clear of the system bars' padding
      assertTrue(what + " panel " + panel, panel.x >= TutorialLayout.MARGIN);
      assertTrue(what + " panel " + panel, panel.x + panel.w <= c.w - TutorialLayout.MARGIN);
      assertTrue(what + " panel " + panel, panel.y >= c.padTop + TutorialLayout.MARGIN);
      assertTrue(what + " panel " + panel, panel.y + panel.h <= c.h - c.padBottom - TutorialLayout.MARGIN);
      // Never over the target nor its outline
      Rect outlined = new Rect(target.x - 1, target.y - 1, target.w + 2, target.h + 2);
      assertFalse(what + " panel " + panel, panel.intersects(outlined));
      // The pointer's tip is next to the outline, and its base on the panel's edge
      switch(p.side) {
         case ABOVE:
            assertEquals(what, target.y - 2, p.tipY);
            assertEquals(what, panel.y + panel.h, p.baseY);
            assertEquals(what, p.baseY + TutorialLayout.POINTER - 1, p.tipY);
            assertTrue(what, p.tipX >= target.x && p.tipX < target.x + target.w);
            assertTrue(what, p.baseX - TutorialLayout.HALF_BASE >= panel.x && p.baseX + TutorialLayout.HALF_BASE < panel.x + panel.w);
            break;
         case BELOW:
            assertEquals(what, target.y + target.h + 1, p.tipY);
            assertEquals(what, panel.y - 1, p.baseY);
            assertEquals(what, p.baseY - (TutorialLayout.POINTER - 1), p.tipY);
            assertTrue(what, p.tipX >= target.x && p.tipX < target.x + target.w);
            assertTrue(what, p.baseX - TutorialLayout.HALF_BASE >= panel.x && p.baseX + TutorialLayout.HALF_BASE < panel.x + panel.w);
            break;
         case LEFT:
            assertEquals(what, target.x - 2, p.tipX);
            assertEquals(what, panel.x + panel.w, p.baseX);
            assertTrue(what, p.tipY >= target.y && p.tipY < target.y + target.h);
            break;
         default:
            assertEquals(what, target.x + target.w + 1, p.tipX);
            assertEquals(what, panel.x - 1, p.baseX);
            assertTrue(what, p.tipY >= target.y && p.tipY < target.y + target.h);
            break;
      }
      // The text is inside the panel, line by line
      assertTrue(what, p.textX >= panel.x + TutorialLayout.PAD_X);
      for(String line : p.lines) {
         assertTrue(what + " line '" + line + "'", p.textX + FIVE.width(line) <= panel.x + panel.w - TutorialLayout.PAD_X);
      }
      int textBottom = p.textY + (p.lines.size() - 1) * TutorialLayout.LINE_PITCH + TutorialLayout.LINE_INK;
      // The buttons: hit areas at least 14 tall, inside the panel, below the text, apart
      assertTrue(what, p.gotIt.h >= 14 && p.skip.h >= 14);
      for(Rect b : new Rect[] { p.gotIt, p.skip }) {
         assertTrue(what + " button " + b, b.x >= panel.x && b.x + b.w <= panel.x + panel.w);
         assertTrue(what + " button " + b, b.y >= textBottom && b.y + b.h <= panel.y + panel.h);
      }
      assertFalse(what, p.gotIt.intersects(p.skip));
      assertTrue(what, p.gotIt.w >= FIVE.width("Got it"));
   }

   // ---- the nine steps on each canvas ---------------------------------------------------------

   @Test
   public void everyStepFindsItsPlaceOnBothCanvases() {
      for(Canvas c : canvases()) {
         Rect[] targets = {
            c.roll(), c.dice(), c.costs(), c.spot(0, 0), c.undo(), c.roll(), c.strip(), c.die(2), c.mini(1),
         };
         for(int i = 0; i < targets.length; i++) {
            assertSound(c, targets[i], TEXTS[i], place(c, targets[i], TEXTS[i]));
         }
      }
   }

   @Test
   public void everyTextFitsEveryTargetOnBothCanvases() {
      for(Canvas c : canvases()) {
         Rect[] targets = {
            c.roll(), c.wideRoll(130), c.dice(), c.die(0), c.die(5), c.costs(), c.undo(), c.strip(),
            c.mini(1), c.mini(6), c.mini(7), c.spot(0, 0), c.spot(-90, -120), c.spot(90, 120), c.spot(-95, 130), c.spot(95, -120),
         };
         for(Rect target : targets) {
            for(String text : TEXTS) {
               assertSound(c, target, text, place(c, target, text));
            }
         }
      }
   }

   @Test
   public void thePanelSitsAboveTargetsInTheLowerHalfAndBelowTheOnesAtTheTop() {
      for(Canvas c : canvases()) {
         assertEquals(Side.ABOVE, place(c, c.roll(), TEXTS[0]).side);
         assertEquals(Side.ABOVE, place(c, c.dice(), TEXTS[1]).side);
         assertEquals(Side.ABOVE, place(c, c.undo(), TEXTS[4]).side);
         assertEquals(Side.ABOVE, place(c, c.mini(1), TEXTS[8]).side);
         assertEquals(Side.ABOVE, place(c, c.die(3), TEXTS[7]).side);
         assertEquals(Side.BELOW, place(c, c.costs(), TEXTS[2]).side);
         assertEquals(Side.BELOW, place(c, c.strip(), TEXTS[6]).side);
      }
   }

   @Test
   public void aSpotNearTheTopOfTheIslandGetsThePanelBelowAndOneNearTheBottomAbove() {
      Canvas c = new Canvas(216, 480, 0, 0);
      assertEquals(Side.BELOW, place(c, c.spot(0, -100), TEXTS[3]).side);
      assertEquals(Side.ABOVE, place(c, c.spot(0, 100), TEXTS[3]).side);
   }

   // ---- the 216 x 480 canvas, step by step ---------------------------------------------------

   @Test
   public void onTheNarrowCanvasTheRollCalloutIsOneCentredPanelAboveTheButton() {
      Canvas c = new Canvas(216, 480, 0, 0);
      Rect roll = c.roll();
      assertEquals(new Rect(60, 456, 96, 20).toString(), roll.toString());
      Placement p = place(c, roll, TEXTS[0]);
      assertEquals(Side.ABOVE, p.side);
      // Bottom edge: outline and pointer above the button
      assertEquals(roll.y - 1 - TutorialLayout.POINTER, p.panel.y + p.panel.h);
      // Centred on the button, pointer straight down its middle
      assertEquals(roll.x + roll.w / 2, p.baseX);
      assertEquals(roll.x + roll.w / 2, p.panel.x + p.panel.w / 2);
   }

   @Test
   public void theUndoCalloutLeansAwayFromTheRightEdgeButPointsAtTheButton() {
      Canvas c = new Canvas(216, 480, 0, 0);
      Rect undo = c.undo();
      Placement p = place(c, undo, TEXTS[4]);
      assertEquals(Side.ABOVE, p.side);
      assertEquals(c.w - TutorialLayout.MARGIN, p.panel.x + p.panel.w);
      assertTrue(p.tipX >= undo.x && p.tipX < undo.x + undo.w);
   }

   @Test
   public void aPanelNeverNeedsMoreWidthThanTheHud() {
      for(Canvas c : canvases()) {
         for(String text : TEXTS) {
            assertTrue(place(c, c.roll(), text).panel.w <= TutorialLayout.MAX_PANEL_W);
         }
      }
   }

   @Test
   public void bothButtonsShareARowWhenTheyFitAndStackWhenTheyDoNot() {
      Canvas c = new Canvas(216, 480, 0, 0);
      Placement one = TutorialLayout.place(c.roll(), TEXTS[0], "Got it", "Skip tutorial", c.w, c.h, 0, c.h, FIVE);
      assertEquals(one.gotIt.y, one.skip.y);
      assertTrue(one.gotIt.x < one.skip.x);
      Placement two = TutorialLayout.place(c.roll(), TEXTS[0], "Understood completely", "Skip the whole tutorial please", c.w, c.h, 0, c.h, FIVE);
      assertTrue(two.skip.y >= two.gotIt.y + two.gotIt.h);
      assertEquals(two.gotIt.x, two.skip.x);
      assertSound(c, c.roll(), TEXTS[0], two);
   }

   @Test
   public void aTargetWithNoRoomAnywhereGetsNoPlacement() {
      Canvas c = new Canvas(216, 480, 0, 0);
      assertTrue(TutorialLayout.place(new Rect(0, 0, 216, 480), TEXTS[0], "Got it", "Skip", c.w, c.h, 0, c.h, FIVE) == null);
   }
}
