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

import android.graphics.Bitmap;
import android.os.SystemClock;
import android.view.MotionEvent;

public class UIDiceRoll extends UIEntity {
   /** Art pixels. The button grows sideways if a translated label needs the room. */
   static final int WIDTH = 96;
   static final int HEIGHT = 20;
   static final int CORNER = 6;
   static final int LABEL = 0xFF1E140A;
   static final int LABEL_DISABLED = 0xFF4A463E;

   int x;
   int y;
   int width;
   int height;
   Bitmap normal;
   Bitmap pressed;
   Bitmap disabled;
   BitmapFont font;
   boolean down = false;
   static final long LOCKOUT_MS = 500;

   public UIDiceRoll(Game game, Bitmap normal, Bitmap pressed, Bitmap disabled, BitmapFont font) {
      super(game, Type.roll, 0);
      this.normal = normal;
      this.pressed = pressed;
      this.disabled = disabled;
      this.font = font;
   }

   @Override
   public void setBounds(int x, int y, int width, int height) {
      super.setBounds(x, y, width, height);
      this.x = x;
      this.y = y;
      this.width = width;
      this.height = height;
   }

   @Override
   public void touch(int action) {
       if(isLocked()) {
           // Swallow the tap, but let a press that began before the game ended clear.
           down = false;
           return;
       }
       if(action == MotionEvent.ACTION_DOWN) {
           down = true;
           Haptics.tap(game.gameView);
           Sounds.play(Sfx.CLICK);
           game.gameView.postInvalidate();
       }
       else if(down && action == MotionEvent.ACTION_UP) {
           down = false;
           if (game.isGameDone()) {
               // Locked taps never reach here, so this is a deliberate tap after the
               // lockout. No dialog: the board is finished, so nothing is lost.
               game.roll();
           }
           else if (game.shouldConfirmEndTurn()) {
               game.gameView.postInvalidate();
               PixelDialog.confirm(game.gameView.activity, R.string.end_turn_title, R.string.end_turn_text, new Runnable() {
                   @Override
                   public void run() {
                       game.roll();
                   }
               });
           }
           else if (!game.canRoll()) {
               // The button reads "1st Roll" here, so do that rather than ending the
               // turn and making the player tap the same button a second time.
               game.endTurnAndRoll();
               confirmIfRolled();
           }
           else {
               game.roll();
               confirmIfRolled();
           }
       }
       else if(action == MotionEvent.ACTION_CANCEL) {
           down = false;
           game.gameView.postInvalidate();
       }
   }
   
   /**
    * The firmer haptic, when dice were really thrown. Both callers start from a state
    * where the tap is meant to roll, so "a roll has been taken this turn" is the test;
    * it stays quiet when the turn that would have started is the end of the game.
    */
   private void confirmIfRolled() {
      if(game.rolls > 0 && !game.isGameDone()) {
         Haptics.confirm(game.gameView);
      }
   }

   /**
    * True for LOCKOUT_MS after the game ends, so a bump on the button cannot wipe the
    * finished board by starting a new game.
    */
   private boolean isLocked() {
      return game.isGameDone() && SystemClock.uptimeMillis() - game.gameOverAt < LOCKOUT_MS;
   }
   
   /** What the button reads now, which is also what it does next. */
   private String label() {
      String text;
      boolean last = game.turnsTaken + 1 == game.map.turnCap;
      if(game.canRoll()) {
         // turnsTaken counts completed turns, so the one being played is the next.
         String roll = game.getString(R.string.first_roll);
         if (game.rolls == 1) {
             roll = game.getString(R.string.second_roll);
         }
         if (game.rolls == 2) {
             roll = game.getString(R.string.third_roll);
         }
         // The turn is not named here: the King's ship in the top bar shows it
         text = roll;
      }
      else if(game.isGameDone()) {
          text = game.getString(R.string.play_again);
      }
      else if(game.shouldConfirmEndTurn()) {
          // Rolls are spent but something is still affordable, so ending the turn is a
          // real choice and "Done" is the honest label.
          text = last ? game.getString(R.string.finish) : game.getString(R.string.done);
      }
      else if(last) {
         // Ending this turn ends the game: it is the map's last turn.
         text = game.getString(R.string.finish);
      }
      else {
         // Nothing left to build and nothing left to roll, so "Done" described a
         // decision the player does not actually have. This button ends the turn and
         // rolls, so it says what comes next.
         text = game.getString(R.string.next_turn);
      }
      return text;
   }

   /** The button as drawn: widened for a long label, centred where the layout put it. */
   int[] drawnBounds() {
      int w = Math.max(width, font.inkWidth(label()) + 2 * CORNER + 4);
      return new int[] { x - (w - width) / 2, y, w, height };
   }

   @Override
   public void draw(ArtCanvas art) {
      boolean locked = isLocked();
      if(locked) {
         // Redraw when the lockout ends so the button visibly re-enables.
         game.gameView.postInvalidateDelayed(LOCKOUT_MS - (SystemClock.uptimeMillis() - game.gameOverAt));
      }
      String text = label();
      // Widen for a long label, keeping the button centred where the layout put it
      int textWidth = font.inkWidth(text);
      int w = Math.max(width, textWidth + 2 * CORNER + 4);
      int left = x - (w - width) / 2;
      Bitmap skin = locked ? disabled : (down ? pressed : normal);
      art.nineSlice(skin, left, y, w, height, CORNER);
      int textX = left + (w - textWidth) / 2;
      int textY = y + (height - font.height()) / 2 + (down && !locked ? 1 : 0);
      font.draw(art.canvas, text, textX, textY, locked ? LABEL_DISABLED : LABEL);
   }
}
