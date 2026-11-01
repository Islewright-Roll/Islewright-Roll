package com.ridgelineapps.resdicegame;

/**
 * How the glow round the traders still to be hired fades on a map that hires them in
 * order: the next one to hire glows in full, the one after it at half, the third at a
 * fifth, and every one beyond that not at all. Pure, so it can be tested without a view.
 */
final class GlowFade {
   private GlowFade() {
   }

   /** The glow's strength by step: the next trader, the one after, the one after that. */
   static final float[] STRENGTH = { 1.0f, 0.5f, 0.2f };

   /**
    * Which step of the fade this trader is on: 0 for the next one to hire, 1 for the one
    * after it, and so on, counting the traders not yet hired that come before it in the
    * order. -1 for a trader already hired (and for a tier that does not exist).
    * `hired` is Playsheet.traders: 1 based by tier, entry 0 unused.
    */
   static int step(boolean[] hired, int tier) {
      if(tier < 1 || tier >= hired.length || hired[tier]) {
         return -1;
      }
      int step = 0;
      for(int earlier = 1; earlier < tier; earlier++) {
         if(!hired[earlier]) {
            step++;
         }
      }
      return step;
   }

   /** The glow's strength at a step, 0 to 1; none past the last step or for -1. */
   static float strength(int step) {
      return step >= 0 && step < STRENGTH.length ? STRENGTH[step] : 0f;
   }
}
