package com.ridgelineapps.resdicegame;

/**
 * The pure timeline of hiring a trader, free of Android so a unit test can reach it. GameView
 * holds only when each hire began and draws what this says.
 *
 * Six steps, in art pixels and whole pixel positions only:
 *  1. FADE: the hired cart fades in.
 *  2. LIFT: a glow comes up round the resource on the cart's bed and it lifts LIFT_PX.
 *  3. PLATE: the plate of a mini die appears behind it (a snap, not a fade: it is on screen
 *     for PLATE_MS before it sets off, which is the moment the eye needs).
 *  4. TRAVEL: it moves in a straight line, eased, to LIFT_PX above its slot on the tab.
 *  5. SETTLE: it sets down those LIFT_PX into the slot.
 *  6. GLOW: the glow fades; after it the normal drawing takes over (DONE).
 * A hire that banks nothing (the first trader of a pair, or one whose resource was spent in
 * the same tap) plays step 1 alone ("fades only").
 *
 * Every function takes the milliseconds elapsed since the hire began. Negative means it has
 * not begun (a chain of hires staggers them), and anything past the total means it is over.
 */
final class HireAnim {
   private HireAnim() {
   }

   static final int FADE_MS = 180;
   static final int LIFT_MS = 160;
   static final int PLATE_MS = 80;
   static final int TRAVEL_MS = 450;
   static final int SETTLE_MS = 120;
   static final int GLOW_MS = 200;
   /** How far the resource lifts off the cart, and sets down again into its slot. */
   static final int LIFT_PX = 3;
   /** A second hire in the same tap begins this long after the one before it. */
   static final int CHAIN_STAGGER_MS = 150;
   /** How far toward white the player's colour is taken for the banking glow. */
   static final float GLOW_LIGHTEN = 0.45f;

   enum Phase { WAIT, FADE, LIFT, PLATE, TRAVEL, SETTLE, GLOW, DONE }

   /** The whole sequence, or step 1 alone. */
   static int total(boolean flies) {
      return flies ? FADE_MS + LIFT_MS + PLATE_MS + TRAVEL_MS + SETTLE_MS + GLOW_MS : FADE_MS;
   }

   /** Smoothstep, the easing of Voyage.sailAt. */
   static float smooth(float t) {
      return t * t * (3 - 2 * t);
   }

   private static final Phase[] ORDER = { Phase.FADE, Phase.LIFT, Phase.PLATE, Phase.TRAVEL, Phase.SETTLE, Phase.GLOW };
   private static final int[] LENGTH = { FADE_MS, LIFT_MS, PLATE_MS, TRAVEL_MS, SETTLE_MS, GLOW_MS };

   /** The step under way. Without flying there is only FADE, then DONE. */
   static Phase phase(int elapsed, boolean flies) {
      if(elapsed < 0) {
         return Phase.WAIT;
      }
      int left = elapsed;
      int steps = flies ? ORDER.length : 1;
      for(int i = 0; i < steps; i++) {
         if(left < LENGTH[i]) {
            return ORDER[i];
         }
         left -= LENGTH[i];
      }
      return Phase.DONE;
   }

   /** How far through its step it is, 0 to 1 (0 while waiting, 1 once done). */
   static float progress(int elapsed, boolean flies) {
      if(elapsed < 0) {
         return 0f;
      }
      int left = elapsed;
      int steps = flies ? ORDER.length : 1;
      for(int i = 0; i < steps; i++) {
         if(left < LENGTH[i]) {
            return (float)left / LENGTH[i];
         }
         left -= LENGTH[i];
      }
      return 1f;
   }

   /** The cart's opacity, 0 to 255: nothing until it begins, full once step 1 is over. */
   static int cartAlpha(int elapsed) {
      if(elapsed <= 0) {
         return 0;
      }
      if(elapsed >= FADE_MS) {
         return 255;
      }
      return Math.round(255 * smooth((float)elapsed / FADE_MS));
   }

   /**
    * The resource's opacity while it stays on the cart. A resource that flies is solid
    * throughout (this is only asked of one that does not fly); one that does not trades places
    * with the cart as it fades in, and is gone when the cart is whole.
    */
   static int iconAlpha(int elapsed, boolean flies) {
      return flies ? 255 : 255 - cartAlpha(elapsed);
   }

   /** The glow's strength, 0 to 1, in the flying sequence. */
   static float glow(int elapsed) {
      switch(phase(elapsed, true)) {
         case LIFT:
            return smooth(progress(elapsed, true));
         case PLATE:
         case TRAVEL:
         case SETTLE:
            return 1f;
         case GLOW:
            return 1f - smooth(progress(elapsed, true));
         default:
            return 0f;
      }
   }

   /** How many pixels the resource stands above its resting place, 0 to LIFT_PX. */
   static int lift(int elapsed) {
      switch(phase(elapsed, true)) {
         case LIFT:
            return Math.round(LIFT_PX * smooth(progress(elapsed, true)));
         case PLATE:
         case TRAVEL:
            return LIFT_PX;
         case SETTLE:
            return LIFT_PX - Math.round(LIFT_PX * smooth(progress(elapsed, true)));
         default:
            return 0;
      }
   }

   /** Whether the mini die's plate is drawn behind the resource: from step 3 to the end. */
   static boolean plateShown(int elapsed) {
      switch(phase(elapsed, true)) {
         case PLATE:
         case TRAVEL:
         case SETTLE:
         case GLOW:
            return true;
         default:
            return false;
      }
   }

   /**
    * Where the resource's top left is, in art pixels, given where it rests on the cart's bed
    * (from) and where it rests in its slot (to). It starts on the cart, lifts straight up,
    * travels in a straight line to LIFT_PX above the slot, and sets down onto it. Whole pixels.
    */
   static Pt iconAt(Pt from, Pt to, int elapsed) {
      Phase phase = phase(elapsed, true);
      if(phase == Phase.TRAVEL) {
         float e = smooth(progress(elapsed, true));
         return new Pt(from.x + Math.round((to.x - from.x) * e),
               from.y - LIFT_PX + Math.round((to.y - from.y) * e));
      }
      if(phase == Phase.SETTLE || phase == Phase.GLOW || phase == Phase.DONE) {
         return new Pt(to.x, to.y - lift(elapsed));
      }
      return new Pt(from.x, from.y - lift(elapsed));
   }

   /**
    * Whether a pixel of a d by d mini die plate is solid: the square with its four corner
    * pixels cut, as ArtCanvas.plate draws it. The glow's outline follows this shape.
    */
   static boolean plateHas(int x, int y, int d) {
      if(x < 0 || y < 0 || x >= d || y >= d) {
         return false;
      }
      boolean cornerX = x == 0 || x == d - 1;
      boolean cornerY = y == 0 || y == d - 1;
      return !(cornerX && cornerY);
   }
}
