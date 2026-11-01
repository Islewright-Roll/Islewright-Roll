// FROZEN ORACLE. The static rules of Game.java as they stood before the map-driven
// refactor (commit 56673ab): the recipes, useResources, canBuildStatus and
// getKnightResourceIndex, copied verbatim and retargeted at LegacyPlaysheet. Never edit it.
package com.ridgelineapps.resdicegame;

import java.util.ArrayList;

final class LegacyRules {
   public static Dice.Value[] RESOURCES_ROAD = new Dice.Value[]{ Dice.Value.Brick, Dice.Value.Lumber };
   public static Dice.Value[] RESOURCES_VILLAGE = new Dice.Value[]{ Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Wool, Dice.Value.Grain };
   public static Dice.Value[] RESOURCES_KNIGHT = new Dice.Value[]{ Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore };
   public static Dice.Value[] RESOURCES_CITY = new Dice.Value[]{ Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore };

   static void useResources(Dice.Value[] resources, Dice[] dice, LegacyPlaysheet playsheet, ArrayList<Dice.Value> knightResources) {
      // TODO: Cleanup how knightResources are tracked
      knightResources.clear();
      for(int i=1; i <= 6; i++) {
          if(playsheet.canUseKnightResource(i)) {
              knightResources.add(playsheet.getKnightResource(i));
          }
      }

      int gold = 0;
      for(Dice die : dice) {
         if(die.isUsable() && die.getValue().equals(Dice.Value.Gold)) {
            gold++;
         }
      }

      if(gold % 2 != 0) {
         gold--;
      }

      ArrayList<Dice.Value> resList = new ArrayList<Dice.Value>();
      for(Dice.Value val : resources) {
          resList.add(val);
      }

      for(Dice.Value val : resources) {
         for(int i=0; i < dice.length; i++) {
            if(dice[i].isUsable() && dice[i].value.equals(val)) {
               dice[i].use();
               resList.remove(val);
               break;
            }
         }
      }

      ArrayList<Dice.Value> resListFound = new ArrayList<Dice.Value>();

      for(Dice.Value val : resList) {
          for(int i = knightResources.size() - 1; i >=0 ; i--) {
             if(knightResources.get(i).equals(val)) {
                resListFound.add(val);
                break;
             }
          }
      }

      for(Dice.Value val : resListFound) {
         resList.remove(val);
      }

      int goldUsed = 0;
      boolean useWildcardResource = false;

      if(gold / 2 == resList.size()) {
         goldUsed = gold;
      }
      else if(gold / 2 > resList.size()) {
         goldUsed = resList.size() * 2;
         for(int i=0; i < gold / 2 - resList.size() && resListFound.size() > 0; i++) {
            resListFound.remove(0);
            goldUsed += 2;
         }
      }
      else if(gold / 2 < resList.size()) {
         useWildcardResource = true;
         goldUsed = gold;
      }


      for(int i=0; i < dice.length && goldUsed > 0; i++) {
         if(dice[i].isUsable() && dice[i].value.equals(Dice.Value.Gold)) {
            dice[i].use();
            goldUsed--;
         }
      }

      for(Dice.Value val : resListFound) {
         for(int i = knightResources.size() - 1; i >=0 ; i--) {
            if(knightResources.get(i).equals(val)) {
               int index = getKnightResourceIndex(knightResources.get(i));
               playsheet.useKnightResource(index);
               knightResources.remove(i);
               break;
            }
         }
      }
      if(useWildcardResource) {
          for(int i = knightResources.size() - 1; i >=0 ; i--) {
              if(knightResources.get(i).equals(Dice.Value.Any)) {
                  int index = getKnightResourceIndex(knightResources.get(i));
                  playsheet.useKnightResource(index);
                  knightResources.remove(i);
                  break;
              }
          }
      }
   }

   // Build status returned by canBuildStatus(): whether a recipe is affordable at all,
   // and if so, whether paying for it would spend a banked knight resource rather than
   // just the dice sitting in front of the player this turn.
   public static final int BUILD_NONE = 0;
   public static final int BUILD_WITH_DICE = 1;
   public static final int BUILD_WITH_KNIGHT = 2;


   static int canBuildStatus(Dice.Value[] resourcesRequired, Dice[] dice, LegacyPlaysheet playsheet, ArrayList<Dice.Value> knightResources) {
      boolean[] diceUsed = new boolean[dice.length];

      // TODO: Cleanup how knightResources are tracked
      knightResources.clear();
      for(int i=1; i <= 6; i++) {
          if(playsheet.canUseKnightResource(i)) {
              knightResources.add(playsheet.getKnightResource(i));
          }
      }

      boolean[] knightResourcesUsed = new boolean[knightResources.size()];
      boolean usedKnightResource = false;
      int gold = 0;
      for(Dice die : dice) {
         if(die.isUsable() && die.getValue().equals(Dice.Value.Gold)) {
            gold++;
         }
      }
      int unfound = 0;
      for(Dice.Value val : resourcesRequired) {
         boolean found = false;
         for(int i=0; i < dice.length; i++) {
            if(dice[i].isUsable() && !diceUsed[i] && dice[i].value.equals(val)) {
               diceUsed[i] = true;
               found = true;
               break;
            }
         }
         if(!found) {
            for(int i=0; i < knightResources.size(); i++) {
               if(!knightResourcesUsed[i] && knightResources.get(i).equals(val)) {
                  knightResourcesUsed[i] = true;
                  found = true;
                  usedKnightResource = true;
                  break;
               }
            }
         }
         if(!found) {
             for(int i=0; i < knightResources.size(); i++) {
                if(!knightResourcesUsed[i] && knightResources.get(i).equals(Dice.Value.Any)) {
                   knightResourcesUsed[i] = true;
                   found = true;
                   usedKnightResource = true;
                   break;
                }
             }
          }

         if(!found) {
            unfound++;
         }
      }

      if(unfound > gold / 2) {
          return BUILD_NONE;
      }
      return usedKnightResource ? BUILD_WITH_KNIGHT : BUILD_WITH_DICE;
   }

   
   public static int getKnightResourceIndex(Dice.Value val) {
       int index = 0;
       switch(val) {
           case Ore:
               index = 1;
               break;
           case Grain:
               index = 2;
               break;
           case Wool:
               index = 3;
               break;
           case Lumber:
               index = 4;
               break;
           case Brick:
               index = 5;
               break;
           case Any:
               index = 6;
               break;
        }
       return index;
   }
}
