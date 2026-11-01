// FROZEN ORACLE. The turn and build flow of Game.java as it stood before the map-driven
// refactor (commit 56673ab): the constructor, newTurn, canBuildX, buildX, build and
// isGameDone, transcribed line for line onto LegacyPlaysheet, LegacyRules and
// LegacyBuildPlan. Left out, because none of it is rules: the view, saving, the top
// score toast, the game over clock and the plan cache (the plan is computed afresh, which
// gives the same plan). Never edit it.
package com.ridgelineapps.resdicegame;

import java.util.ArrayList;

final class LegacyGame {
   LegacyPlaysheet playsheet;
   Dice[] dice;
   int rolls;
   ArrayList<Dice.Value> knightResources;
   boolean builtResourceThisTurn;
   int turnsTaken;

   LegacyGame() {
      playsheet = new LegacyPlaysheet();
      knightResources = new ArrayList<Dice.Value>();
      dice = new Dice[6];
      for(int i=0; i < dice.length; i++) {
         dice[i] = new Dice();
      }

      turnsTaken = 0;
      newTurn(true);
   }

   void newTurn(boolean firstTurn) {
      if(isGameDone()) {
         return;
      }

      rolls = 0;

      //Return unused knightResources
      for(int i=0; i < knightResources.size(); i++) {
         int index = LegacyRules.getKnightResourceIndex(knightResources.get(i));
         if(index > 0) {
            playsheet.resourcesAvail[index] = true;
         }
      }

      knightResources.clear();
      for(int i=0; i < dice.length; i++) {
         dice[i].reset();
      }
      if(!firstTurn && !builtResourceThisTurn) {
         playsheet.turnsNothingBuilt++;
      }
      builtResourceThisTurn = false;

      if(!firstTurn) {
         turnsTaken++;
         playsheet.scoreTurn(turnsTaken);
      }
   }

   boolean isGameDone() {
      return (turnsTaken == 15);
   }

   boolean canBuild(Dice.Value[] resourcesRequired) {
      return canBuildStatus(resourcesRequired) != LegacyRules.BUILD_NONE;
   }

   int canBuildStatus(Dice.Value[] resourcesRequired) {
      return LegacyRules.canBuildStatus(resourcesRequired, dice, playsheet, knightResources);
   }

   void useResources(Dice.Value[] resources) {
      LegacyRules.useResources(resources, dice, playsheet, knightResources);
   }

   void buildVillage(int i) {
      if(!canBuildVillage(i)) {
         return;
      }

      useResources(LegacyRules.RESOURCES_VILLAGE);
      playsheet.buildVillage(i);
      builtResourceThisTurn = true;
   }

   boolean canBuildVillage(int i) {
      if(rolls == 0 || !canBuild(LegacyRules.RESOURCES_VILLAGE)) {
         return false;
      }

      return playsheet.canBuildVillage(i);
   }

   void buildKnight(int i) {
      if(!canBuildKnight(i)) {
         return;
      }

      useResources(LegacyRules.RESOURCES_KNIGHT);
      playsheet.buildKnight(i);
      builtResourceThisTurn = true;
   }

   boolean canBuildKnight(int i) {
      if(rolls == 0 || !canBuild(LegacyRules.RESOURCES_KNIGHT)) {
         return false;
      }

      return playsheet.canBuildKnight(i);
   }

   void buildRoad(int i) {
      if(!canBuildRoad(i)) {
         return;
      }

      useResources(LegacyRules.RESOURCES_ROAD);
      playsheet.buildRoad(i);
      builtResourceThisTurn = true;
   }

   boolean canBuildRoad(int i) {
      if(rolls == 0 || !canBuild(LegacyRules.RESOURCES_ROAD)) {
         return false;
      }

      return playsheet.canBuildRoad(i);
   }

   void buildCity(int i) {
      if(!canBuildCity(i)) {
         return;
      }

      useResources(LegacyRules.RESOURCES_CITY);
      playsheet.buildCity(i);
      builtResourceThisTurn = true;
   }

   boolean canBuildCity(int i) {
      if(rolls == 0 || !canBuild(LegacyRules.RESOURCES_CITY)) {
         return false;
      }

      return playsheet.canBuildCity(i);
   }

   LegacyBuildPlan getBuildPlan() {
      return LegacyBuildPlan.compute(dice, playsheet, rolls);
   }

   void build(LegacyBuildPlan.Kind kind, int index) {
      switch(kind) {
         case ROAD:
            if(canBuildRoad(index)) {
               buildRoad(index);
               return;
            }
            break;
         case VILLAGE:
            if(canBuildVillage(index)) {
               buildVillage(index);
               return;
            }
            break;
         case CITY:
            if(canBuildCity(index)) {
               buildCity(index);
               return;
            }
            break;
         case KNIGHT:
            if(canBuildKnight(index)) {
               buildKnight(index);
               return;
            }
            break;
      }

      LegacyBuildPlan.Chain chain = getBuildPlan().chainFor(kind, index);
      if(chain == null) {
         return;
      }

      for(LegacyBuildPlan.Step step : chain.steps) {
         switch(step.kind) {
            case ROAD:
               buildRoad(step.index);
               if(!playsheet.roads[step.index]) {
                  return;
               }
               break;
            case VILLAGE:
               buildVillage(step.index);
               if(!playsheet.villages[step.index]) {
                  return;
               }
               break;
            case CITY:
               buildCity(step.index);
               if(!playsheet.cities[step.index]) {
                  return;
               }
               break;
            case KNIGHT:
               buildKnight(step.index);
               if(playsheet.knights < step.index) {
                  return;
               }
               break;
         }
      }
   }
}
