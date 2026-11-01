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

import android.content.res.Resources;
import android.graphics.Bitmap;

public class UIDice extends UIEntity {
   /** A die is a square of this many art pixels. */
   public static final int size = 28;
   static final int ICON = 16;

   Dice dice;
   int x;
   int y;
   static Bitmap face;
   static Bitmap faceHeld;
   static Bitmap slotEmpty;
   static Bitmap wool;
   static Bitmap grain;
   static Bitmap brick;
   static Bitmap ore;
   static Bitmap lumber;
   static Bitmap gold;
   static Bitmap any;

   public UIDice(Game game, int index) {
      super(game, Type.dice, index);
      dice = game.dice[index];
   }

   public void setPosition(int x, int y) {
      this.x = x;
      this.y = y;
      setBounds(x, y, size, size);
   }

   public static void loadBitmaps(Resources resources) {
      face = ArtCanvas.load(resources, R.drawable.die_face);
      faceHeld = ArtCanvas.load(resources, R.drawable.die_face_held);
      slotEmpty = ArtCanvas.load(resources, R.drawable.die_slot_empty);
      wool = ArtCanvas.load(resources, R.drawable.icon_wool_16);
      grain = ArtCanvas.load(resources, R.drawable.icon_grain_16);
      brick = ArtCanvas.load(resources, R.drawable.icon_brick_16);
      ore = ArtCanvas.load(resources, R.drawable.icon_ore_16);
      lumber = ArtCanvas.load(resources, R.drawable.icon_lumber_16);
      gold = ArtCanvas.load(resources, R.drawable.icon_gold_16);
      any = ArtCanvas.load(resources, Art.ICON_ANY_16);
   }

   @Override
   public void draw(ArtCanvas art) {
      Bitmap bitmap = null;
      if(dice.isUsable()) {
         switch(dice.getValue()) {
            case Wool:
               bitmap = wool;
               break;
            case Grain:
               bitmap = grain;
               break;
            case Brick:
               bitmap = brick;
               break;
            case Ore:
               bitmap = ore;
               break;
            case Lumber:
               bitmap = lumber;
               break;
            case Gold:
               bitmap = gold;
               break;
            case Any:
               bitmap = any;
               break;
            default:
               break;
         }
      }
      if(bitmap == null) {
         // Spent on a build, or not rolled yet
         art.sprite(slotEmpty, x, y);
         return;
      }
      // The frame in the player's colour means "this die is staying": held by the player,
      // or locked in because the rolls are over.
      boolean staying = dice.isHeld() || !game.canRoll();
      art.sprite(staying ? TeamColor.recolor(faceHeld, TeamColor.MAGENTA, game.playerColor) : face, x, y);
      art.sprite(bitmap, x + (size - ICON) / 2, y + (size - ICON) / 2);
   }
}
