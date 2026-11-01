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

import java.util.Arrays;

/**
 * What the player has built and banked, and the rules for building more. Every count,
 * prerequisite and score comes from the map (see MapDef); nothing here names an island.
 *
 * Road, village and city indices are 0 based. Trader ("knight") tiers and bank slots are
 * 1 based, and their arrays carry an unused entry at index 0.
 */
public class Playsheet {
   final MapDef map;
   boolean[] traders; // 1 based, by tier
   boolean[] resourcesAvail; // 1 based, by bank slot
   boolean[] roads;
   boolean[] villages;
   boolean[] cities;
   int turnsNothingBuilt = 0;
   int[] turnScore;
   /**
    * Whether a build may pay with a banked resource by itself. False in trader resource
    * classic mode, where a banked resource is only ever spent by dragging it onto a die
    * (Game.overwriteDie). A rule of play, not of the sheet, so it is not saved; it lives
    * here so the build plan's copies of the sheet carry it.
    */
   boolean bankPaysBuilds = true;

   public Playsheet(MapDef map) {
      this.map = map;
      reset();
   }

   private static boolean anyBuilt(int[] needs, boolean[] roads) {
      for(int need : needs) {
         if(roads[need]) {
            return true;
         }
      }
      return false;
   }

   public boolean canBuildVillage(int i) {
      if(i < 0 || i >= villages.length) {
         return false;
      }

      if(villages[i]) {
         return false;
      }

      if(map.orderedVillages && i > 0 && !villages[i-1]) {
         return false;
      }

      return anyBuilt(map.villageRoads[i], roads);
   }

   public void buildVillage(int i) {
      if(canBuildVillage(i)) {
         villages[i] = true;
      }
   }

   public boolean canBuildCity(int i) {
      if(i < 0 || i >= cities.length) {
         return false;
      }

      if(cities[i]) {
         return false;
      }

      if(map.orderedCities && i > 0 && !cities[i-1]) {
         return false;
      }

      return anyBuilt(map.cityRoads[i], roads);
   }

   public void buildCity(int i) {
      if(canBuildCity(i)) {
         cities[i] = true;
      }
   }

   public boolean canBuildRoad(int i) {
      if(i < 0 || i >= roads.length) {
         return false;
      }

      if(roads[i]) {
         return false;
      }

      // The start road is prebuilt, so this should never be hit, but just in case...
      if(i == 0)
         return true;

      return anyBuilt(map.roadNeeds[i], roads);
   }

   public void buildRoad(int i) {
      if(canBuildRoad(i)) {
         roads[i] = true;
      }
   }

   public boolean canBuildKnight(int i) {
      if(i < 1 || i >= traders.length) {
         return false;
      }

      if(traders[i]) {
         return false;
      }

      if(map.orderedTraders && i > 1 && !traders[i-1]) {
         return false;
      }

      return true;
   }

   public void buildKnight(int i) {
      if(canBuildKnight(i)) {
         traders[i] = true;
      }
   }

   /**
    * Whether hiring this tier would start a bank slot's team of traders while another
    * team is half hired: the slot behind this tier needs more than one trader, none of
    * them stands yet, and some other multi trader slot has at least one but not all of
    * its traders. The player is asked before that, because the "?" of the first team
    * comes only once it is whole. A single trader, a team already begun, and a map with
    * no teams (Tarnholm) never qualify. `hired` is Playsheet.traders (1 based, entry 0
    * unused); nothing here changes it.
    */
   static boolean startsSecondPair(MapDef map, boolean[] hired, int tier) {
      if(tier < 1 || tier > map.traderBank.length || tier >= hired.length || hired[tier]) {
         return false;
      }
      int slot = map.traderBank[tier - 1];
      if(slot <= 0 || map.bankTraders[slot].length < 2 || teamHired(map, hired, slot) > 0) {
         return false;
      }
      for(int other = 1; other < map.bankTraders.length; other++) {
         int size = map.bankTraders[other].length;
         int count = other == slot || size < 2 ? 0 : teamHired(map, hired, other);
         if(count > 0 && count < size) {
            return true;
         }
      }
      return false;
   }

   /**
    * As startsSecondPair for a whole chain of hires in the order they happen: true when any
    * one of them would, counting the hires of the chain before it as done.
    */
   static boolean chainStartsSecondPair(MapDef map, boolean[] hired, int[] tiers) {
      boolean[] sim = hired.clone();
      for(int tier : tiers) {
         if(startsSecondPair(map, sim, tier)) {
            return true;
         }
         if(tier >= 1 && tier < sim.length) {
            sim[tier] = true;
         }
      }
      return false;
   }

   /** How many of a bank slot's traders stand. */
   private static int teamHired(MapDef map, boolean[] hired, int slot) {
      int count = 0;
      for(int tier : map.bankTraders[slot]) {
         if(hired[tier]) {
            count++;
         }
      }
      return count;
   }

   /** Whether the trader of this tier (1 based) stands. */
   public boolean isTraderBuilt(int tier) {
      return tier >= 1 && tier < traders.length && traders[tier];
   }

   public int traderCount() {
      int count = 0;
      for(int tier = 1; tier < traders.length; tier++) {
         if(traders[tier]) {
            count++;
         }
      }
      return count;
   }

   /**
    * Builds tiers 1 to count and clears the rest: how a save or a scenario that records
    * traders as a count (every one written for a map with ordered traders) is applied.
    */
   void setTraderCount(int count) {
      for(int tier = 1; tier < traders.length; tier++) {
         traders[tier] = tier <= count;
      }
   }

   /** The number of bank slots; they are numbered from 1 to this. */
   public int bankSize() {
      return resourcesAvail.length - 1;
   }

   /** Whether every trader behind this bank slot stands. */
   private boolean bankOpen(int slot) {
      for(int tier : map.bankTraders[slot]) {
         if(!traders[tier]) {
            return false;
         }
      }
      return true;
   }

   public boolean canUseKnightResource(int i) {
      if(i < 1 || i > bankSize()) {
         return false;
      }

      return bankOpen(i) && resourcesAvail[i];
   }

   public boolean isKnightResourceUsed(int i) {
      if(i < 1 || i > bankSize()) {
         return false;
      }

      return bankOpen(i) && !resourcesAvail[i];
   }

   /**
    * Spends a bank slot if it can be used. The return value is None either way and
    * always has been (the slot is spent before it is read back, and a spent slot reads
    * as None); nothing uses it. Ask getKnightResource first for what the slot holds.
    */
   public Dice.Value useKnightResource(int i) {
      if(canUseKnightResource(i)) {
         resourcesAvail[i] = false;
         return getKnightResource(i);
      }

      return Dice.Value.None;
   }

   public Dice.Value getKnightResource(int i) {
      if(canUseKnightResource(i)) {
         return map.bankResource[i];
      }

      return Dice.Value.None;
   }

   /** The lowest bank slot holding this resource that can be used right now, or 0. */
   public int firstAvailableSlot(Dice.Value v) {
      for(int slot = 1; slot <= bankSize(); slot++) {
         if(map.bankResource[slot] == v && canUseKnightResource(slot)) {
            return slot;
         }
      }
      return 0;
   }

   /** Whether a slot that stands for any resource can be used right now. */
   public boolean hasWildcard() {
      return firstAvailableSlot(Dice.Value.Any) != 0;
   }

   /**
    * Hands a resource the turn set aside but did not spend back to the bank: the lowest
    * spent slot holding it becomes available again. Nothing happens if no slot holding
    * it is spent.
    */
   void returnToBank(Dice.Value v) {
      for(int slot = 1; slot <= bankSize(); slot++) {
         if(map.bankResource[slot] == v && !resourcesAvail[slot]) {
            resourcesAvail[slot] = true;
            return;
         }
      }
   }

   public void reset() {
      traders = new boolean[map.traders.length + 1];
      resourcesAvail = new boolean[map.bankResource.length];
      for(int i=1; i < resourcesAvail.length; i++) {
         resourcesAvail[i] = true;
      }
      roads = new boolean[map.roads.length];
      roads[0] = true;
      villages = new boolean[map.villages.length];
      cities = new boolean[map.cities.length];
      turnsNothingBuilt = 0;
      // A map with no turn cap grows the array as turns are scored (see scoreTurn)
      turnScore = new int[map.turnCap + 1];
   }

   public Playsheet copy() {
      Playsheet copy = new Playsheet(map);
      copy.traders = traders.clone();
      copy.resourcesAvail = resourcesAvail.clone();
      copy.roads = roads.clone();
      copy.villages = villages.clone();
      copy.cities = cities.clone();
      copy.turnsNothingBuilt = turnsNothingBuilt;
      copy.turnScore = turnScore.clone();
      copy.bankPaysBuilds = bankPaysBuilds;
      return copy;
   }

   public void scoreTurn(int turn) {
      if(turn < 1 || (map.turnCap > 0 && turn > map.turnCap)) {
         return;
      }
      if(turn >= turnScore.length) {
         turnScore = Arrays.copyOf(turnScore, turn + 1);
      }
      turnScore[turn] = getScore();
   }

   public int getTurnScore(int turn) {
      if(turn < 1 || turn >= turnScore.length) {
         return 0;
      }
      return turnScore[turn] - turnScore[turn - 1];
   }

   /**
    * Which of the island's bonuses stand, one bit each: 1 the long road, 2 the trader bonus.
    * The score is derived from the tables, so the sound for earning a bonus compares this
    * before and after a build (see SoundCues.bonusGained).
    */
   int bonusState() {
      int state = 0;
      if(map.longRoad >= 0 && roads[map.longRoad]) {
         state |= 1;
      }
      if(map.traderBonusAt > 0 && traderCount() >= map.traderBonusAt) {
         state |= 2;
      }
      return state;
   }

   /**
    * The bonuses are worked out from what stands, never stored, so nothing that happens
    * to the bank can take one away.
    */
   public int getScore() {
      int score = 0;
      for(int i=1; i < roads.length; i++) {
         if(roads[i]) {
            score += map.roadPoints;
         }
      }

      for(int tier=1; tier < traders.length; tier++) {
         if(traders[tier]) {
            score += map.traderPoints[tier-1];
         }
      }

      for(int i=0; i < villages.length; i++) {
         if(villages[i]) {
            score += map.villagePoints[i];
         }
      }

      for(int i=0; i < cities.length; i++) {
         if(cities[i]) {
            score += map.cityPoints[i];
         }
      }

      if(map.longRoad >= 0 && roads[map.longRoad]) {
         score += map.longRoadBonus;
      }

      if(map.traderBonusAt > 0 && traderCount() >= map.traderBonusAt) {
         score += map.traderBonus;
      }

      score -= turnsNothingBuilt * map.emptyTurnPenalty;

      return score;
   }
}
