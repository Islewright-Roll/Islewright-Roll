package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/** The hire animation's timeline: when each step is, and where the resource is in it. */
public class HireAnimTest {
   static final Pt FROM = new Pt(40, 100);
   static final Pt TO = new Pt(7, 300);

   @Test
   public void theWholeSequenceIsTheSixStepsAdded() {
      assertEquals(180 + 160 + 80 + 450 + 120 + 200, HireAnim.total(true));
      assertEquals(HireAnim.FADE_MS, HireAnim.total(false));
   }

   @Test
   public void stepsComeInOrderAndCoverTheTotal() {
      HireAnim.Phase[] order = { HireAnim.Phase.FADE, HireAnim.Phase.LIFT, HireAnim.Phase.PLATE,
            HireAnim.Phase.TRAVEL, HireAnim.Phase.SETTLE, HireAnim.Phase.GLOW };
      int seen = 0;
      HireAnim.Phase last = null;
      for(int t = 0; t < HireAnim.total(true); t++) {
         HireAnim.Phase p = HireAnim.phase(t, true);
         if(p != last) {
            assertEquals(order[seen], p);
            seen++;
            last = p;
         }
      }
      assertEquals(6, seen);
      assertEquals(HireAnim.Phase.DONE, HireAnim.phase(HireAnim.total(true), true));
      assertEquals(HireAnim.Phase.DONE, HireAnim.phase(HireAnim.total(true) + 5000, true));
      assertEquals(HireAnim.Phase.WAIT, HireAnim.phase(-1, true));
   }

   @Test
   public void aHireThatBanksNothingPlaysTheFadeAlone() {
      assertEquals(HireAnim.Phase.FADE, HireAnim.phase(HireAnim.FADE_MS - 1, false));
      assertEquals(HireAnim.Phase.DONE, HireAnim.phase(HireAnim.FADE_MS, false));
   }

   @Test
   public void progressRunsZeroToOneInEachStep() {
      assertEquals(0f, HireAnim.progress(0, true), 0f);
      assertEquals(0.5f, HireAnim.progress(HireAnim.FADE_MS / 2, true), 0.01f);
      assertEquals(0f, HireAnim.progress(HireAnim.FADE_MS, true), 0f);
      assertEquals(1f, HireAnim.progress(HireAnim.total(true), true), 0f);
      assertEquals(0f, HireAnim.progress(-50, true), 0f);
      float last = -1;
      for(int t = 0; t < HireAnim.FADE_MS; t++) {
         float p = HireAnim.progress(t, true);
         assertTrue(p >= last);
         last = p;
      }
   }

   @Test
   public void theCartFadesInMonotonicallyFromNothingToFull() {
      assertEquals(0, HireAnim.cartAlpha(-10));
      assertEquals(0, HireAnim.cartAlpha(0));
      assertEquals(255, HireAnim.cartAlpha(HireAnim.FADE_MS));
      assertEquals(255, HireAnim.cartAlpha(5000));
      int last = 0;
      for(int t = 0; t <= HireAnim.FADE_MS + 20; t++) {
         int a = HireAnim.cartAlpha(t);
         assertTrue(a >= last);
         assertTrue(a >= 0 && a <= 255);
         last = a;
      }
   }

   @Test
   public void aResourceThatFliesIsSolidAndOneThatDoesNotTradesPlacesWithTheCart() {
      for(int t = -20; t < 400; t += 10) {
         assertEquals(255, HireAnim.iconAlpha(t, true));
         assertEquals(255, HireAnim.iconAlpha(t, false) + HireAnim.cartAlpha(t));
      }
      assertEquals(0, HireAnim.iconAlpha(HireAnim.FADE_MS, false));
   }

   @Test
   public void theLiftIsThreePixelsAndSettlesBackToNothing() {
      assertEquals(3, HireAnim.LIFT_PX);
      int lift = 0;
      int top = 0;
      for(int t = 0; t < HireAnim.total(true); t++) {
         int l = HireAnim.lift(t);
         assertTrue(l >= 0 && l <= 3);
         top = Math.max(top, l);
         lift = l;
      }
      assertEquals(3, top);
      assertEquals(0, HireAnim.lift(HireAnim.FADE_MS - 1));
      assertEquals(0, HireAnim.lift(HireAnim.FADE_MS + 0));
      assertEquals(3, HireAnim.lift(HireAnim.FADE_MS + HireAnim.LIFT_MS));
      assertEquals(3, HireAnim.lift(HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS + HireAnim.TRAVEL_MS));
      assertEquals(0, HireAnim.lift(HireAnim.total(true) - HireAnim.GLOW_MS));
      assertEquals(0, lift);
   }

   @Test
   public void liftRisesThenSettlesWithoutWobbling() {
      int settleStart = HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS + HireAnim.TRAVEL_MS;
      int last = 0;
      for(int t = 0; t < settleStart; t++) {
         int l = HireAnim.lift(t);
         assertTrue("rising at " + t, l >= last);
         last = l;
      }
      for(int t = settleStart; t < HireAnim.total(true); t++) {
         int l = HireAnim.lift(t);
         assertTrue("falling at " + t, l <= last);
         last = l;
      }
   }

   @Test
   public void theGlowComesUpWithTheLiftHoldsAndFadesAtTheEnd() {
      assertEquals(0f, HireAnim.glow(0), 0f);
      assertEquals(0f, HireAnim.glow(HireAnim.FADE_MS - 1), 0f);
      assertEquals(1f, HireAnim.glow(HireAnim.FADE_MS + HireAnim.LIFT_MS), 0f);
      int settleEnd = HireAnim.total(true) - HireAnim.GLOW_MS;
      assertEquals(1f, HireAnim.glow(settleEnd - 1), 0f);
      assertEquals(0f, HireAnim.glow(HireAnim.total(true)), 0f);
      float up = 0;
      for(int t = HireAnim.FADE_MS; t <= HireAnim.FADE_MS + HireAnim.LIFT_MS; t++) {
         float g = HireAnim.glow(t);
         assertTrue(g >= up);
         up = g;
      }
      float down = 1;
      for(int t = settleEnd; t < HireAnim.total(true); t++) {
         float g = HireAnim.glow(t);
         assertTrue(g <= down);
         assertTrue(g >= 0 && g <= 1);
         down = g;
      }
   }

   @Test
   public void thePlateIsThereFromStepThreeToTheEnd() {
      int plateAt = HireAnim.FADE_MS + HireAnim.LIFT_MS;
      assertFalse(HireAnim.plateShown(plateAt - 1));
      assertTrue(HireAnim.plateShown(plateAt));
      assertTrue(HireAnim.plateShown(HireAnim.total(true) - 1));
      assertFalse(HireAnim.plateShown(HireAnim.total(true)));
   }

   @Test
   public void theResourceStartsOnTheCartAndEndsExactlyOnTheSlot() {
      assertEquals(FROM, HireAnim.iconAt(FROM, TO, -30));
      assertEquals(FROM, HireAnim.iconAt(FROM, TO, 0));
      assertEquals(FROM, HireAnim.iconAt(FROM, TO, HireAnim.FADE_MS - 1));
      assertEquals(TO, HireAnim.iconAt(FROM, TO, HireAnim.total(true) - HireAnim.GLOW_MS));
      assertEquals(TO, HireAnim.iconAt(FROM, TO, HireAnim.total(true) - 1));
      assertEquals(TO, HireAnim.iconAt(FROM, TO, HireAnim.total(true)));
      assertEquals(TO, HireAnim.iconAt(FROM, TO, 99999));
   }

   @Test
   public void theTravelEndsThreePixelsAboveTheSlot() {
      int travelEnd = HireAnim.total(true) - HireAnim.GLOW_MS - HireAnim.SETTLE_MS;
      Pt just = HireAnim.iconAt(FROM, TO, travelEnd - 1);
      assertTrue(just.y <= TO.y - 3 + 1);
      assertEquals(new Pt(TO.x, TO.y - 3), HireAnim.iconAt(FROM, TO, travelEnd));
      int travelStart = HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS;
      assertEquals(new Pt(FROM.x, FROM.y - 3), HireAnim.iconAt(FROM, TO, travelStart));
   }

   @Test
   public void theTravelIsMonotonicAndNeverLeavesTheStraightLinesBounds() {
      int travelStart = HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS;
      int travelEnd = travelStart + HireAnim.TRAVEL_MS;
      Pt last = HireAnim.iconAt(FROM, TO, travelStart);
      for(int t = travelStart; t <= travelEnd; t++) {
         Pt p = HireAnim.iconAt(FROM, TO, t);
         // from (40, 97) to (7, 297): x never rises, y never falls
         assertTrue("x at " + t, p.x <= last.x);
         assertTrue("y at " + t, p.y >= last.y);
         assertTrue(p.x >= TO.x && p.x <= FROM.x);
         last = p;
      }
   }

   @Test
   public void theTravelIsEasedSoItIsSlowAtBothEnds() {
      int travelStart = HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS;
      int early = HireAnim.iconAt(FROM, TO, travelStart + 50).y - HireAnim.iconAt(FROM, TO, travelStart).y;
      int middle = HireAnim.iconAt(FROM, TO, travelStart + 275).y - HireAnim.iconAt(FROM, TO, travelStart + 225).y;
      assertTrue(early < middle);
   }

   @Test
   public void aSlotLeftOfAndAboveTheCartAlsoWorks() {
      Pt from = new Pt(20, 300);
      Pt to = new Pt(200, 120);
      Pt last = HireAnim.iconAt(from, to, HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS);
      int travelEnd = HireAnim.total(true) - HireAnim.GLOW_MS - HireAnim.SETTLE_MS;
      for(int t = HireAnim.FADE_MS + HireAnim.LIFT_MS + HireAnim.PLATE_MS; t <= travelEnd; t++) {
         Pt p = HireAnim.iconAt(from, to, t);
         assertTrue(p.x >= last.x);
         assertTrue(p.y <= last.y);
         last = p;
      }
      assertEquals(new Pt(200, 117), last);
   }

   @Test
   public void aStaggeredHireWaitsBeforeItBegins() {
      assertEquals(HireAnim.Phase.WAIT, HireAnim.phase(-HireAnim.CHAIN_STAGGER_MS, true));
      assertEquals(0, HireAnim.cartAlpha(-HireAnim.CHAIN_STAGGER_MS));
      assertEquals(150, HireAnim.CHAIN_STAGGER_MS);
   }

   @Test
   public void thePlateIsTheSquareWithItsFourCornerPixelsCut() {
      int d = 16;
      int solid = 0;
      for(int y = -1; y <= d; y++) {
         for(int x = -1; x <= d; x++) {
            if(HireAnim.plateHas(x, y, d)) {
               solid++;
            }
         }
      }
      assertEquals(d * d - 4, solid);
      assertFalse(HireAnim.plateHas(0, 0, d));
      assertFalse(HireAnim.plateHas(d - 1, d - 1, d));
      assertTrue(HireAnim.plateHas(1, 0, d));
      assertTrue(HireAnim.plateHas(0, 1, d));
   }
}
