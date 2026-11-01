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

import android.app.Application;
import android.content.Context;
import android.media.AudioAttributes;
import android.media.SoundPool;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.util.Log;

/**
 * The game's sound effects: sixteen short sounds in res/raw (sfx_*), played through one
 * SoundPool. One instance for the whole process, made by GameApp, like MusicPlayer, and
 * following the same signal for whether the app is on screen.
 *
 * <b>When it is loaded.</b> The pool exists only while the "Sound effects" setting is On and
 * the app is in the foreground (MusicPlayer tells us, with its own debounce for moving between
 * the game's screens). It is released when the app goes to the background or the setting goes
 * Off, and loaded again on the way back: sixteen files, about 250 KB, a fraction of a second.
 * With the setting Off nothing is loaded and nothing is played. A sound requested before it has
 * finished loading is dropped silently.
 *
 * <b>Audio.</b> USAGE_GAME on the media stream, so the hardware volume keys that move the
 * music move these too, and silent or do-not-disturb apply as they do to any media. No audio
 * focus is requested: effects never interrupt or duck the music. At most MAX_STREAMS sounds at
 * once; the rules of retriggering and the repeat guard are in SoundPlayback.
 *
 * Each request that reaches the pool logs one debug line with the moment and the stream id.
 * Everything runs on the main thread.
 */
final class SoundEffects implements SoundSink {

   private static final String TAG = "SoundEffects";
   static final int MAX_STREAMS = 6;

   private static SoundEffects instance;

   /** Makes the one instance, reads the stored setting and becomes the process-wide sink. */
   static void install(Application app) {
      instance = new SoundEffects(app);
      Sounds.sink = instance;
      instance.setEnabled(HighScoresAndSettings.load(app).soundEffects);
   }

   /** The effects; null before GameApp has run (never in the running app). */
   static SoundEffects get() {
      return instance;
   }

   /** MusicPlayer's word that the app as a whole came to the foreground or left it. */
   static void onAppForeground(boolean foreground) {
      if(instance != null) {
         instance.setForeground(foreground);
      }
   }

   /** The res/raw id of a moment's file. */
   static int resourceFor(Sfx sfx) {
      switch(sfx) {
         case CLICK: return R.raw.sfx_click;
         case DIE: return R.raw.sfx_die;
         case ROLL: return R.raw.sfx_roll;
         case ROLL_B: return R.raw.sfx_roll_b;
         case ROAD: return R.raw.sfx_road;
         case VILLAGE: return R.raw.sfx_village;
         case CASTLE: return R.raw.sfx_castle;
         case TRADER: return R.raw.sfx_trader;
         case BANKED: return R.raw.sfx_banked;
         case GOLD: return R.raw.sfx_gold;
         case TURN_END: return R.raw.sfx_turn_end;
         case EMPTY_TURN: return R.raw.sfx_empty_turn;
         case SHIP_APPEARS: return R.raw.sfx_ship_appears;
         case SHIP_DOCKS: return R.raw.sfx_ship_docks;
         case GAME_OVER: return R.raw.sfx_game_over;
         case TOP_SCORE: return R.raw.sfx_top_score;
         default: return R.raw.sfx_bonus;
      }
   }

   private final Context context;
   private final Handler handler = new Handler(Looper.getMainLooper());
   private final SoundPlayback.Scheduler scheduler = new SoundPlayback.Scheduler() {
      @Override
      public long now() {
         return SystemClock.uptimeMillis();
      }

      @Override
      public void postDelayed(Runnable task, long delayMs) {
         handler.postDelayed(task, delayMs);
      }
   };

   private boolean enabled;
   private boolean foreground;

   private SoundPool pool;
   private SoundPlayback playback;
   private final java.util.Random random = new java.util.Random();
   private final int[] soundIds = new int[Sfx.values().length];
   private final boolean[] loaded = new boolean[Sfx.values().length];

   private SoundEffects(Context context) {
      this.context = context.getApplicationContext();
   }

   /** The "Sound effects" setting. */
   void setEnabled(boolean enabled) {
      this.enabled = enabled;
      reconcile();
   }

   private void setForeground(boolean foreground) {
      this.foreground = foreground;
      reconcile();
   }

   /** True while the pool exists (for the debug harness and the logs). */
   boolean isLoaded() {
      return pool != null;
   }

   @Override
   public void play(Sfx sfx, int delayMs) {
      if(playback != null) {
         // There are two roll sounds, a rattle and a throw: each roll plays one of them
         if(sfx == Sfx.ROLL && random.nextBoolean()) {
            sfx = Sfx.ROLL_B;
         }
         playback.play(sfx, delayMs);
      }
   }

   private void reconcile() {
      boolean want = enabled && foreground;
      if(want && pool == null) {
         create();
      }
      else if(!want && pool != null) {
         release();
      }
   }

   private void create() {
      AudioAttributes attributes = new AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_GAME)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build();
      final SoundPool p = new SoundPool.Builder().setMaxStreams(MAX_STREAMS).setAudioAttributes(attributes).build();
      pool = p;
      final Sfx[] all = Sfx.values();
      java.util.Arrays.fill(loaded, false);
      p.setOnLoadCompleteListener(new SoundPool.OnLoadCompleteListener() {
         @Override
         public void onLoadComplete(SoundPool soundPool, int sampleId, int status) {
            for(Sfx sfx : all) {
               if(soundIds[sfx.ordinal()] == sampleId && pool == soundPool) {
                  loaded[sfx.ordinal()] = status == 0;
                  Log.d(TAG, "loaded sfx_" + sfx.file + " status=" + status);
               }
            }
         }
      });
      for(Sfx sfx : all) {
         soundIds[sfx.ordinal()] = p.load(context, resourceFor(sfx), 1);
      }
      playback = new SoundPlayback(new SoundPlayback.Backend() {
         @Override
         public int play(Sfx sfx) {
            int i = sfx.ordinal();
            if(!loaded[i]) {
               Log.d(TAG, "drop " + sfx + " (not loaded)");
               return 0;
            }
            int stream = p.play(soundIds[i], 1f, 1f, 1, 0, 1f);
            Log.d(TAG, "play " + sfx + " stream=" + stream);
            return stream;
         }

         @Override
         public void stop(Sfx sfx, int streamId) {
            Log.d(TAG, "stop " + sfx + " stream=" + streamId);
            p.stop(streamId);
         }
      }, scheduler);
      Log.d(TAG, "pool created, loading " + all.length + " sounds");
   }

   private void release() {
      if(playback != null) {
         playback.close();
         playback = null;
      }
      handler.removeCallbacksAndMessages(null);
      SoundPool p = pool;
      pool = null;
      java.util.Arrays.fill(loaded, false);
      p.release();
      Log.d(TAG, "pool released");
   }
}
