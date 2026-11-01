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
 * The rules for playing a sound, apart from Android so they can be tested: delayed requests,
 * the machine-gun guard and the retrigger of Sfx.BANKED. The pool sits behind Backend and the
 * clock and timer behind Scheduler.
 *
 * <b>Guard.</b> The same sound requested twice within GUARD_MS plays once: a double delivery
 * of one tap, or two builds landing in the same frame, is one sound.
 *
 * <b>Retrigger.</b> For a sound with Sfx.retriggerMs, a request inside that window of the last
 * play first stops the stream that is still sounding. Several banked resources spent in one
 * build therefore restart the sample each time, stuttering its opening, rather than stacking.
 * The stream id kept for this is only the last one.
 *
 * Everything runs on the main thread.
 */
final class SoundPlayback implements SoundSink {

   /** What actually makes the noise. Returns a stream id, or 0 when nothing played (not loaded yet). */
   interface Backend {
      int play(Sfx sfx);

      void stop(Sfx sfx, int streamId);
   }

   interface Scheduler {
      /** Milliseconds on a monotonic clock. */
      long now();

      void postDelayed(Runnable task, long delayMs);
   }

   static final long GUARD_MS = 40;

   private static final long NEVER = Long.MIN_VALUE / 2;

   private final Backend backend;
   private final Scheduler scheduler;
   private final long[] lastPlayed = new long[Sfx.values().length];
   private final int[] lastStream = new int[Sfx.values().length];
   private boolean closed;

   SoundPlayback(Backend backend, Scheduler scheduler) {
      this.backend = backend;
      this.scheduler = scheduler;
      java.util.Arrays.fill(lastPlayed, NEVER);
   }

   @Override
   public void play(final Sfx sfx, int delayMs) {
      if(closed) {
         return;
      }
      if(delayMs <= 0) {
         playNow(sfx);
      }
      else {
         scheduler.postDelayed(new Runnable() {
            @Override
            public void run() {
               playNow(sfx);
            }
         }, delayMs);
      }
   }

   private void playNow(Sfx sfx) {
      if(closed) {
         return;
      }
      int i = sfx.ordinal();
      long now = scheduler.now();
      if(now - lastPlayed[i] < GUARD_MS) {
         return;
      }
      if(sfx.retriggerMs > 0 && lastStream[i] != 0 && now - lastPlayed[i] < sfx.retriggerMs) {
         backend.stop(sfx, lastStream[i]);
         lastStream[i] = 0;
      }
      int stream = backend.play(sfx);
      if(stream == 0) {
         // Dropped (still loading): it does not count as having played
         return;
      }
      lastPlayed[i] = now;
      lastStream[i] = stream;
   }

   /** After this nothing plays, including requests already waiting on their delay. */
   void close() {
      closed = true;
   }
}
