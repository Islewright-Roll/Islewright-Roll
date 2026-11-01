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

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.SystemClock;
import android.util.Log;
import android.view.InputDevice;
import android.view.MotionEvent;

/**
 * Debug builds only: two fingers at once, which `adb shell input` cannot send. It builds
 * real two pointer MotionEvents and hands them to the game view, so the view's own touch
 * code runs exactly as it would under two fingers.
 *
 *   adb shell am broadcast -n io.github.islewright_roll.islewright/com.ridgelineapps.resdicegame.DebugTouchReceiver \
 *       --es a 110,2020 --es b 450,2020
 *
 * a and b are screen pixels. The sequence is: a down, b down, b up, a up, so both fingers
 * are down together and each lifts as a tap. With --ez slide true, finger a slides 300 px
 * to the right before lifting (a swipe) while b still taps.
 *
 * One finger dragging instead, which `adb shell input swipe` sends too fast and too
 * coarsely to watch: down at from, ten moves along the way to to, a lift at to.
 *
 *   adb shell am broadcast -n io.github.islewright_roll.islewright/com.ridgelineapps.resdicegame.DebugTouchReceiver \
 *       --es from 300,2100 --es to 500,1950
 */
public class DebugTouchReceiver extends BroadcastReceiver {
   private static final String TAG = "DebugTouch";

   @Override
   public void onReceive(Context context, Intent intent) {
      final GameView view = GameView.current.get();
      String from = intent.getStringExtra("from");
      String to = intent.getStringExtra("to");
      if(view != null && from != null && to != null) {
         drag(view, point(from, view), point(to, view));
         return;
      }
      String a = intent.getStringExtra("a");
      String b = intent.getStringExtra("b");
      if(view == null || a == null || b == null) {
         Log.w(TAG, "needs a game on screen and --es a x,y --es b x,y");
         return;
      }
      final float[] pa = point(a, view);
      final float[] pb = point(b, view);
      final boolean slide = intent.getBooleanExtra("slide", false);
      view.post(new Runnable() {
         @Override
         public void run() {
            long t = SystemClock.uptimeMillis();
            send(view, t, t, MotionEvent.ACTION_DOWN, new float[][] { pa }, new int[] { 0 });
            send(view, t, t + 20, MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT), new float[][] { pa, pb }, new int[] { 0, 1 });
            float[] end = pa;
            if(slide) {
               end = new float[] { pa[0] + 300, pa[1] };
               send(view, t, t + 40, MotionEvent.ACTION_MOVE, new float[][] { end, pb }, new int[] { 0, 1 });
            }
            send(view, t, t + 60, MotionEvent.ACTION_POINTER_UP | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT), new float[][] { end, pb }, new int[] { 0, 1 });
            send(view, t, t + 80, MotionEvent.ACTION_UP, new float[][] { end }, new int[] { 0 });
            Log.d(TAG, "two fingers: a " + pa[0] + "," + pa[1] + " b " + pb[0] + "," + pb[1] + (slide ? " (a slides)" : ""));
         }
      });
   }

   private static void drag(final GameView view, final float[] from, final float[] to) {
      view.post(new Runnable() {
         @Override
         public void run() {
            long t = SystemClock.uptimeMillis();
            int[] ids = { 0 };
            send(view, t, t, MotionEvent.ACTION_DOWN, new float[][] { from }, ids);
            for(int step = 1; step <= 10; step++) {
               float[] at = { from[0] + (to[0] - from[0]) * step / 10, from[1] + (to[1] - from[1]) * step / 10 };
               send(view, t, t + 20 * step, MotionEvent.ACTION_MOVE, new float[][] { at }, ids);
            }
            send(view, t, t + 240, MotionEvent.ACTION_UP, new float[][] { to }, ids);
            Log.d(TAG, "drag: " + from[0] + "," + from[1] + " to " + to[0] + "," + to[1]);
         }
      });
   }

   /** "x,y" in screen pixels, as view coordinates. */
   private static float[] point(String text, GameView view) {
      String[] parts = text.split(",");
      int[] at = new int[2];
      view.getLocationOnScreen(at);
      return new float[] { Float.parseFloat(parts[0].trim()) - at[0], Float.parseFloat(parts[1].trim()) - at[1] };
   }

   private static void send(GameView view, long down, long when, int action, float[][] points, int[] ids) {
      MotionEvent.PointerProperties[] props = new MotionEvent.PointerProperties[points.length];
      MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[points.length];
      for(int i = 0; i < points.length; i++) {
         props[i] = new MotionEvent.PointerProperties();
         props[i].id = ids[i];
         props[i].toolType = MotionEvent.TOOL_TYPE_FINGER;
         coords[i] = new MotionEvent.PointerCoords();
         coords[i].x = points[i][0];
         coords[i].y = points[i][1];
         coords[i].pressure = 1;
         coords[i].size = 1;
      }
      MotionEvent event = MotionEvent.obtain(down, when, action, points.length, props, coords, 0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
      view.dispatchTouchEvent(event);
      event.recycle();
   }
}
