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

import android.graphics.Color;
import android.view.MotionEvent;

public class UIEntity {
   enum Type {
      dice,
      road,
      village,
      city,
      knight,
      resource,
      roll,
//      turnOver,
   }

   // Affordable right now with the dice on the table.
   static final int HIGHLIGHT = Color.argb(100, 35, 180, 15);

   // Buildable, but paying for it spends a trader resource banked from an earlier
   // turn instead of dice sitting in front of the player right now. Kept a deep,
   // saturated blue (rather than the sea's lighter teal) so it reads as a distinct
   // highlight color rather than blending into the board art.
   static final int HIGHLIGHT_KNIGHT = Color.argb(85, 40, 70, 230);

   /**
    * Buildable, but it spends the "?" wildcard from trader slot 6 rather than an exact
    * match. The same blue, heavier, because it is the more expensive way to pay.
    */
   static final int HIGHLIGHT_WILDCARD = Color.argb(141, 40, 70, 230);

   /**
    * The same offer when it is reachable only after building something else first: the
    * same colour at a lighter opacity, so "now" and "after another build" read as one idea
    * at two strengths. One rule for every offer colour.
    */
   static int chained(int argb) {
      return ((argb >>> 24) * 55 / 100) << 24 | (argb & 0xFFFFFF);
   }

   static final int HIGHLIGHT_CHAIN = chained(HIGHLIGHT);
   static final int HIGHLIGHT_KNIGHT_CHAIN = chained(HIGHLIGHT_KNIGHT);
   static final int HIGHLIGHT_WILDCARD_CHAIN = chained(HIGHLIGHT_WILDCARD);

   // Rectangle entities (dice, roll button), in art pixels
   int x1;
   int y1;
   int x2;
   int y2;

   // Board pieces: the anchor in board image pixels, and how far from it a tap still counts
   int anchorX;
   int anchorY;
   int reach;

   Game game;

   Type type;
   int index;

   public UIEntity(Game game, Type type, int index) {
      this.type = type;
      this.game = game;
      this.index = index;
   }

   public UIEntity(Game game, Type type, int index, int anchorX, int anchorY, int reach) {
      this(game, type, index);
      this.anchorX = anchorX;
      this.anchorY = anchorY;
      this.reach = reach;
   }

   public void setBounds(int x, int y, int width, int height) {
      x1 = x;
      y1 = y;
      x2 = x + width;
      y2 = y + height;
   }

   public boolean isWithin(int x, int y) {
      return x >= x1 && x < x2 && y >= y1 && y < y2;
   }

   /**
    * Squared distance from a board point to this piece's anchor, or -1 when the point
    * is beyond its reach.
    */
   public int distanceSquared(int boardX, int boardY) {
      int dx = boardX - anchorX;
      int dy = boardY - anchorY;
      int d = dx * dx + dy * dy;
      return d <= reach * reach ? d : -1;
   }

   public void touch(int action) {
       if(action != MotionEvent.ACTION_DOWN && action != MotionEvent.ACTION_UP) {
           return;
       }

      switch (type) {
         case dice:
            if(action == MotionEvent.ACTION_DOWN) {
                boolean held = game.dice[index].isHeld();
                game.diceTouched(index);
                // Only a die that actually changed: before the first roll a tap does nothing
                if(game.dice[index].isHeld() != held) {
                    Haptics.tap(game.gameView);
                }
            }
            break;
//         case turnOver:
//            game.newTurn(false);
//            break;
         case road:
          case village:
          case city:
          case knight:
              if(action == MotionEvent.ACTION_UP) {
                  // Two questions at most, one after the other: giving up rolls first, then
                  // starting a second team of traders. Declining either builds nothing.
                  confirmEarlyBuildThen(new Runnable() {
                     @Override
                     public void run() {
                        confirmSecondPairThen(new Runnable() {
                           @Override
                           public void run() {
                              build();
                           }
                        });
                     }
                  });
              }
             break;
         case resource:
//            game.consumeKnightResource(index);
            break;
      }
   }

   /** Dice and the roll button draw themselves; board pieces are layered by GameView. */
   public void draw(ArtCanvas art) {
   }

   /**
    * The colour this position is offered in, or 0 when it cannot be built this turn.
    * One path for all four piece types, driven by the build plan.
    */
   public int highlightColor() {
      if(type != Type.road && type != Type.village && type != Type.city && type != Type.knight) {
         return 0;
      }
      BuildPlan.Chain chain = game.getBuildPlan().chainFor(kindOf(type), index);
      if(chain == null) {
         return 0;
      }
      // What paying costs picks the colour; whether another build has to come first
      // picks its strength.
      int now;
      if(chain.spendsWildcard) {
         // Spends the "?", trader slot 6, which stands for any resource and is
         // the scarcest thing on that track. Heavier blue: same idea, higher price.
         now = HIGHLIGHT_WILDCARD;
      } else if(chain.spendsBank) {
         // Spending a banked resource is the only case that costs the player
         // something they saved earlier, so it is blue rather than green.
         now = HIGHLIGHT_KNIGHT;
      } else {
         now = HIGHLIGHT;
      }
      return chain.steps.size() == 1 ? now : chained(now);
   }

   /** Builds this position, including any earlier builds its chain needs. */
   private void build() {
      game.build(kindOf(type), index);
      if(alreadyBuilt()) {
         Haptics.confirm(game.gameView);
      }
   }

   /**
    * Building ends the roll phase: canRoll() goes false the moment anything is built.
    * On the first or second roll that silently throws away rolls the player has not
    * used, so ask first. On the third roll there is nothing left to forfeit and the
    * build runs straight through.
    */
   private void confirmEarlyBuildThen(final Runnable build) {
      if(!game.wouldForfeitRolls()) {
         build.run();
         return;
      }

      if(game.earlyBuildPrompt == HighScoresAndSettings.EARLY_PROMPT_OFF) {
         build.run();
         return;
      }

      // Measured across the whole sequence of builds this tap opens up, not just the
      // tapped one. A village costing four dice is modest alone, but when the two it
      // leaves are exactly a road the player can spend all six, and plainly does not
      // want another roll. Bank paid resources do not count: they are not dice.
      BuildPlan.Chain plannedChain = game.getBuildPlan().chainFor(kindOf(type), index);
      // A build that wins the race ends the game, so the rolls it gives up are worth nothing
      if(game.chainReachesTarget(plannedChain)) {
         build.run();
         return;
      }
      // No ceiling on the threshold. An earlier version capped it by what the turn could
      // currently spend, so a stranded gold silenced the warning. That was wrong: this
      // prompt only ever fires while rolls remain, and any die left unspent can be
      // rerolled, a lone gold very much included, since another gold would pair it.
      // "I cannot spend it right now" never means "there is nothing better to do".
      if(plannedChain != null
            && BuildPlan.maxDiceSpendableThrough(game.dice, game.playsheet, plannedChain)
                  >= game.earlyBuildPrompt) {
         build.run();
         return;
      }

      int message = game.rolls == 1 ? R.string.build_early_text_two : R.string.build_early_text_one;
      PixelDialog.confirm(game.gameView.activity, R.string.build_early_title, message, build);
   }


   /**
    * A trader that shares its bank slot with another (Dunholm's two desert hexes) earns the
    * slot's resource only when both stand. Hiring the first of one team while another team
    * has one of its two hired spreads the player thin, so ask first. Measured over the whole
    * chain this tap would run, the traders it hires in order, so a chain that reaches such
    * a trader at any step asks too. Not a setting.
    */
   private void confirmSecondPairThen(Runnable build) {
      BuildPlan.Chain chain = game.getBuildPlan().chainFor(kindOf(type), index);
      // A build that wins the race ends the game, so there is no later turn to spread thin.
      // The third trader's bonus can be the winning build, which is why this asks too.
      if(chain == null || game.chainReachesTarget(chain)) {
         build.run();
         return;
      }
      int count = 0;
      for(BuildPlan.Step step : chain.steps) {
         if(step.kind == BuildPlan.Kind.KNIGHT) {
            count++;
         }
      }
      int[] tiers = new int[count];
      count = 0;
      for(BuildPlan.Step step : chain.steps) {
         if(step.kind == BuildPlan.Kind.KNIGHT) {
            tiers[count++] = step.index;
         }
      }
      if(!Playsheet.chainStartsSecondPair(game.map, game.playsheet.traders, tiers)) {
         build.run();
         return;
      }
      PixelDialog.confirm(game.gameView.activity, R.string.build_second_pair_title, R.string.build_second_pair_text, build);
   }

   /** Maps this entity's type onto the build plan's own vocabulary. */
   private static BuildPlan.Kind kindOf(Type type) {
      switch (type) {
         case road:
            return BuildPlan.Kind.ROAD;
         case village:
            return BuildPlan.Kind.VILLAGE;
         case city:
            return BuildPlan.Kind.CITY;
         default:
            return BuildPlan.Kind.KNIGHT;
      }
   }

   /** Whether this position already holds something the player built. */
   boolean alreadyBuilt() {
      switch (type) {
         case road:
            return game.playsheet.roads[index];
         case village:
            return game.playsheet.villages[index];
         case city:
            return game.playsheet.cities[index];
         case knight:
            return game.playsheet.isTraderBuilt(index);
         default:
            return false;
      }
   }

}
