package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;
import java.util.List;

import org.junit.Before;
import org.junit.Test;

/** The repeat guard, the retrigger of the banked resource sound, delays and dropping: the pool behind a fake. */
public class SoundPlaybackTest {

   /** A pool that numbers its streams from 1 and logs every call; "loaded" can be switched off. */
   private static final class FakeBackend implements SoundPlayback.Backend {
      final List<String> log = new ArrayList<String>();
      boolean loaded = true;
      int next = 1;

      @Override
      public int play(Sfx sfx) {
         if(!loaded) {
            return 0;
         }
         int id = next++;
         log.add("play " + sfx + " " + id);
         return id;
      }

      @Override
      public void stop(Sfx sfx, int streamId) {
         log.add("stop " + sfx + " " + streamId);
      }
   }

   private static final class FakeClock implements SoundPlayback.Scheduler {
      long now = 1000;
      final List<Runnable> tasks = new ArrayList<Runnable>();
      final List<Long> due = new ArrayList<Long>();

      @Override
      public long now() {
         return now;
      }

      @Override
      public void postDelayed(Runnable task, long delayMs) {
         tasks.add(task);
         due.add(now + delayMs);
      }

      /** Advances time, running whatever came due on the way, in order. */
      void advance(long ms) {
         long target = now + ms;
         while(true) {
            int best = -1;
            for(int i = 0; i < tasks.size(); i++) {
               if(due.get(i) <= target && (best < 0 || due.get(i) < due.get(best))) {
                  best = i;
               }
            }
            if(best < 0) {
               break;
            }
            now = due.get(best);
            Runnable r = tasks.remove(best);
            due.remove(best);
            r.run();
         }
         now = target;
      }
   }

   private FakeBackend backend;
   private FakeClock clock;
   private SoundPlayback playback;

   @Before
   public void setUp() {
      backend = new FakeBackend();
      clock = new FakeClock();
      playback = new SoundPlayback(backend, clock);
   }

   @Test
   public void aSoundPlaysAndLogsItsStream() {
      playback.play(Sfx.CLICK, 0);
      assertEquals("[play CLICK 1]", backend.log.toString());
   }

   @Test
   public void theSameSoundTwiceWithinTheGuardPlaysOnce() {
      playback.play(Sfx.CLICK, 0);
      clock.advance(SoundPlayback.GUARD_MS - 1);
      playback.play(Sfx.CLICK, 0);
      assertEquals("[play CLICK 1]", backend.log.toString());
      clock.advance(1);
      playback.play(Sfx.CLICK, 0);
      assertEquals("[play CLICK 1, play CLICK 2]", backend.log.toString());
   }

   @Test
   public void differentSoundsAtOnceAllPlay() {
      playback.play(Sfx.CASTLE, 0);
      playback.play(Sfx.BANKED, 0);
      playback.play(Sfx.GOLD, 0);
      assertEquals("[play CASTLE 1, play BANKED 2, play GOLD 3]", backend.log.toString());
   }

   @Test
   public void bankedRetriggersByStoppingTheStreamStillPlaying() {
      playback.play(Sfx.BANKED, 0);
      clock.advance(110);
      playback.play(Sfx.BANKED, 0);
      clock.advance(110);
      playback.play(Sfx.BANKED, 0);
      assertEquals("[play BANKED 1, stop BANKED 1, play BANKED 2, stop BANKED 2, play BANKED 3]", backend.log.toString());
   }

   @Test
   public void bankedAfterItsSampleHasEndedJustPlays() {
      playback.play(Sfx.BANKED, 0);
      clock.advance(Sfx.BANKED.retriggerMs);
      playback.play(Sfx.BANKED, 0);
      assertEquals("[play BANKED 1, play BANKED 2]", backend.log.toString());
   }

   @Test
   public void otherSoundsDoNotRetrigger() {
      playback.play(Sfx.VILLAGE, 0);
      clock.advance(100);
      playback.play(Sfx.VILLAGE, 0);
      assertEquals("[play VILLAGE 1, play VILLAGE 2]", backend.log.toString());
   }

   @Test
   public void aDelayedSoundPlaysWhenItsTimeComes() {
      playback.play(Sfx.SHIP_DOCKS, 300);
      assertTrue(backend.log.isEmpty());
      clock.advance(299);
      assertTrue(backend.log.isEmpty());
      clock.advance(1);
      assertEquals("[play SHIP_DOCKS 1]", backend.log.toString());
   }

   @Test
   public void delayedBankedUsesStutterTheOpening() {
      playback.play(Sfx.BANKED, 0);
      playback.play(Sfx.BANKED, SoundCues.BANKED_GAP_MS);
      playback.play(Sfx.BANKED, 2 * SoundCues.BANKED_GAP_MS);
      clock.advance(1000);
      assertEquals("[play BANKED 1, stop BANKED 1, play BANKED 2, stop BANKED 2, play BANKED 3]", backend.log.toString());
   }

   @Test
   public void aSoundNotLoadedYetIsDroppedAndDoesNotCountAsPlayed() {
      backend.loaded = false;
      playback.play(Sfx.CLICK, 0);
      backend.loaded = true;
      // Straight away, inside the guard: it did not play, so it is not guarded
      playback.play(Sfx.CLICK, 0);
      assertEquals("[play CLICK 1]", backend.log.toString());
   }

   @Test
   public void aDroppedBankedSoundLeavesNothingToStop() {
      backend.loaded = false;
      playback.play(Sfx.BANKED, 0);
      clock.advance(100);
      backend.loaded = true;
      playback.play(Sfx.BANKED, 0);
      assertEquals("[play BANKED 1]", backend.log.toString());
   }

   @Test
   public void closingStopsWaitingSoundsToo() {
      playback.play(Sfx.SHIP_DOCKS, 300);
      playback.close();
      clock.advance(1000);
      playback.play(Sfx.CLICK, 0);
      assertTrue(backend.log.isEmpty());
   }
}
