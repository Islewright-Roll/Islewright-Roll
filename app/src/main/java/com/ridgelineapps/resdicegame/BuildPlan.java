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

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;

/**
 * A breadth-first search over copies of the live dice and playsheet, answering "what can
 * this position be built by, and in how many steps": a chain of one step is affordable
 * right now (the existing green highlight); a chain of two or more steps needs an earlier
 * build first (the new yellow-green tier). Each step runs the real
 * {@link Game#useResources(Dice.Value[], Dice[], Playsheet, ArrayList)} and the real
 * {@code Playsheet.buildX} on a copy, so board position gates (a village needing a
 * road beside it, say) apply for free with no separate "position tier" logic.
 *
 * <p>Never mutates the live {@link Game}. The search runs entirely on copies obtained
 * from {@link Dice#copy()} and {@link Playsheet#copy()}.
 */
public class BuildPlan {

    public enum Kind { ROAD, VILLAGE, CITY, KNIGHT }

    public static final class Step {
        public final Kind kind;
        public final int index;

        Step(Kind kind, int index) {
            this.kind = kind;
            this.index = index;
        }
    }

    public static final class Chain {
        public final ArrayList<Step> steps;
        public final boolean spendsBank;

        /**
         * Whether the chain spends a bank slot holding the "?" that stands for any
         * resource. It is the scarcest thing in the bank, so spending it costs more than
         * spending an exact match and the board says so.
         */
        public final boolean spendsWildcard;

        /**
         * How many rolled dice the whole chain consumes. Counted by diffing the usable
         * dice before and after, so resources paid from the bank correctly do not add
         * to it: those are not dice.
         */
        public final int diceUsed;

        Chain(ArrayList<Step> steps, boolean spendsBank, boolean spendsWildcard, int diceUsed) {
            this.steps = steps;
            this.spendsBank = spendsBank;
            this.spendsWildcard = spendsWildcard;
            this.diceUsed = diceUsed;
        }
    }

    // Defensive only (see the epic's cost table): a bug in the search can never hang the
    // UI thread. On the cap, positions past it simply stay unhighlighted, which fails safe.
    static final int MAX_STATES = 512;

    private final HashMap<Integer, Chain> chains = new HashMap<Integer, Chain>();

    /** Key = kind.ordinal() * 32 + index; 32 is above the largest index (MapDef allows at most 32 roads). */
    private static int key(Kind kind, int index) {
        return kind.ordinal() * 32 + index;
    }

    public Chain chainFor(Kind kind, int index) {
        return chains.get(key(kind, index));
    }

    /**
     * The most dice the turn can still spend if the player builds this position now,
     * counting the chain that reaches it plus everything reachable afterwards.
     *
     * This is the question that matters for the early-build prompt. A village costing
     * four dice looks modest alone, but if the two it leaves behind are exactly a road,
     * the player can spend all six and plainly does not want another roll. Asking there
     * is the prompt being wrong, not the player being careless.
     *
     * It cannot be read off the recorded chains: the search stores only the shortest
     * chain to each position, so "village, then road" is never recorded when the road
     * is already reachable on its own. This walks forward from the state the build leaves
     * behind. It runs on a tap, not on every draw, so the extra search is affordable.
     */
    public static int maxDiceSpendableThrough(Dice[] liveDice, Playsheet livePlaysheet, Chain chain) {
        Dice[] dice = copyDice(liveDice);
        Playsheet playsheet = livePlaysheet.copy();
        int rootUsable = usableCount(dice);

        for(Step step : chain.steps) {
            Game.useResources(resourcesFor(step.kind), dice, playsheet, new ArrayList<Dice.Value>());
            applyBuild(step.kind, step.index, playsheet);
        }

        return (rootUsable - usableCount(dice)) + spendableFrom(dice, playsheet, new HashSet<String>());
    }

    /** Depth first, returning the most additional dice any sequence of builds consumes. */
    private static int spendableFrom(Dice[] dice, Playsheet playsheet, HashSet<String> seen) {
        if(seen.size() >= MAX_STATES || !seen.add(fingerprint(dice, playsheet))) {
            return 0;
        }

        int before = usableCount(dice);
        int best = 0;
        // One bound for every kind: an index a kind does not have is simply not legal
        MapDef map = playsheet.map;
        int top = Math.max(Math.max(map.roads.length, map.villages.length), Math.max(map.cities.length, map.traders.length));
        for(Kind kind : Kind.values()) {
            for(int i = 0; i <= top; i++) {
                if(!isLegal(kind, i, dice, playsheet)) {
                    continue;
                }
                Dice[] childDice = copyDice(dice);
                Playsheet childPlaysheet = playsheet.copy();
                Game.useResources(resourcesFor(kind), childDice, childPlaysheet, new ArrayList<Dice.Value>());
                applyBuild(kind, i, childPlaysheet);

                int spent = (before - usableCount(childDice))
                      + spendableFrom(childDice, childPlaysheet, seen);
                if(spent > best) {
                    best = spent;
                }
            }
        }
        return best;
    }

    /** True when some position is buildable right now without dipping into the bank. */
    public boolean hasDiceOnlyBuild() {
        for(Chain chain : chains.values()) {
            if(chain.steps.size() == 1 && !chain.spendsBank) {
                return true;
            }
        }
        return false;
    }

    /** Dice still available to spend this turn. */
    private static int usableCount(Dice[] dice) {
        int n = 0;
        for(Dice die : dice) {
            if(die.isUsable()) {
                n++;
            }
        }
        return n;
    }

    private static final class Node {
        final Dice[] dice;
        final Playsheet playsheet;
        final ArrayList<Step> chain;
        final boolean spendsBank;
        final boolean spendsWildcard;

        Node(Dice[] dice, Playsheet playsheet, ArrayList<Step> chain, boolean spendsBank, boolean spendsWildcard) {
            this.dice = dice;
            this.playsheet = playsheet;
            this.chain = chain;
            this.spendsBank = spendsBank;
            this.spendsWildcard = spendsWildcard;
        }
    }

    public static BuildPlan compute(Dice[] liveDice, Playsheet livePlaysheet, int rolls) {
        BuildPlan plan = new BuildPlan();
        if(rolls == 0) {
            return plan;
        }

        HashSet<String> visited = new HashSet<String>();
        ArrayDeque<Node> queue = new ArrayDeque<Node>();

        Dice[] rootDice = copyDice(liveDice);
        Playsheet rootPlaysheet = livePlaysheet.copy();
        int rootUsable = usableCount(rootDice);
        visited.add(fingerprint(rootDice, rootPlaysheet));
        queue.add(new Node(rootDice, rootPlaysheet, new ArrayList<Step>(), false, false));

        MapDef map = livePlaysheet.map;
        while(!queue.isEmpty()) {
            Node node = queue.poll();
            // Road 0 is the start road, never built; knight tiers are 1 based
            for(int i = 1; i < map.roads.length; i++) {
                tryStep(plan, Kind.ROAD, i, node, visited, queue, rootUsable);
            }
            for(int i = 0; i < map.villages.length; i++) {
                tryStep(plan, Kind.VILLAGE, i, node, visited, queue, rootUsable);
            }
            for(int i = 0; i < map.cities.length; i++) {
                tryStep(plan, Kind.CITY, i, node, visited, queue, rootUsable);
            }
            for(int i = 1; i <= map.traders.length; i++) {
                tryStep(plan, Kind.KNIGHT, i, node, visited, queue, rootUsable);
            }
        }

        return plan;
    }

    private static void tryStep(BuildPlan plan, Kind kind, int index, Node node, HashSet<String> visited, ArrayDeque<Node> queue, int rootUsable) {
        if(!isLegal(kind, index, node.dice, node.playsheet)) {
            return;
        }

        Dice[] childDice = copyDice(node.dice);
        Playsheet childPlaysheet = node.playsheet.copy();
        Game.useResources(resourcesFor(kind), childDice, childPlaysheet, new ArrayList<Dice.Value>());
        boolean spendsBankStep = spendsBank(node.playsheet, childPlaysheet);
        boolean spendsWildcardStep = spendsWildcard(node.playsheet, childPlaysheet);
        applyBuild(kind, index, childPlaysheet);

        boolean chainSpendsBank = node.spendsBank || spendsBankStep;
        boolean chainSpendsWildcard = node.spendsWildcard || spendsWildcardStep;
        int key = key(kind, index);
        if(!plan.chains.containsKey(key)) {
            ArrayList<Step> steps = new ArrayList<Step>(node.chain);
            steps.add(new Step(kind, index));
            plan.chains.put(key, new Chain(steps, chainSpendsBank, chainSpendsWildcard, rootUsable - usableCount(childDice)));
        }

        if(visited.size() >= MAX_STATES) {
            return;
        }
        String fingerprint = fingerprint(childDice, childPlaysheet);
        if(visited.contains(fingerprint)) {
            return;
        }
        visited.add(fingerprint);

        ArrayList<Step> chain = new ArrayList<Step>(node.chain);
        chain.add(new Step(kind, index));
        queue.add(new Node(childDice, childPlaysheet, chain, chainSpendsBank, chainSpendsWildcard));
    }

    private static boolean isLegal(Kind kind, int index, Dice[] dice, Playsheet playsheet) {
        boolean positionOk;
        switch(kind) {
            case ROAD:
                positionOk = playsheet.canBuildRoad(index);
                break;
            case VILLAGE:
                positionOk = playsheet.canBuildVillage(index);
                break;
            case CITY:
                positionOk = playsheet.canBuildCity(index);
                break;
            case KNIGHT:
                positionOk = playsheet.canBuildKnight(index);
                break;
            default:
                positionOk = false;
        }
        if(!positionOk) {
            return false;
        }
        return Game.canBuildStatus(resourcesFor(kind), dice, playsheet, new ArrayList<Dice.Value>()) != Game.BUILD_NONE;
    }

    private static void applyBuild(Kind kind, int index, Playsheet playsheet) {
        switch(kind) {
            case ROAD:
                playsheet.buildRoad(index);
                break;
            case VILLAGE:
                playsheet.buildVillage(index);
                break;
            case CITY:
                playsheet.buildCity(index);
                break;
            case KNIGHT:
                playsheet.buildKnight(index);
                break;
        }
    }

    private static Dice.Value[] resourcesFor(Kind kind) {
        switch(kind) {
            case ROAD:
                return Game.RESOURCES_ROAD;
            case VILLAGE:
                return Game.RESOURCES_VILLAGE;
            case CITY:
                return Game.RESOURCES_CITY;
            case KNIGHT:
                return Game.RESOURCES_KNIGHT;
            default:
                return new Dice.Value[0];
        }
    }

    /** True when building the step took at least one resource out of the bank. */
    private static boolean spendsBank(Playsheet before, Playsheet after) {
        for(int i = 1; i <= before.bankSize(); i++) {
            if(before.resourcesAvail[i] && !after.resourcesAvail[i]) {
                return true;
            }
        }
        return false;
    }

    /**
     * True when paying for the step took a "?" out of the bank. Only paying is compared
     * (the build itself is applied afterwards), and a slot can only be spent while it is
     * usable, so a slot that was available and no longer is was usable and got spent.
     */
    private static boolean spendsWildcard(Playsheet before, Playsheet after) {
        for(int i = 1; i <= before.bankSize(); i++) {
            if(before.map.bankResource[i] == Dice.Value.Any && before.resourcesAvail[i] && !after.resourcesAvail[i]) {
                return true;
            }
        }
        return false;
    }

    private static Dice[] copyDice(Dice[] dice) {
        Dice[] copy = new Dice[dice.length];
        for(int i = 0; i < dice.length; i++) {
            copy[i] = dice[i].copy();
        }
        return copy;
    }

    /**
     * A textual key for a (dice, playsheet) state: each die's value ordinal and used flag,
     * then the bank's resourcesAvail, roads, villages, cities and knight tiers. Held is
     * excluded because it does not affect affordability. Field order and every length
     * are fixed for a map, so no separators are needed between fields.
     */
    static String fingerprint(Dice[] dice, Playsheet playsheet) {
        StringBuilder sb = new StringBuilder();
        for(Dice die : dice) {
            sb.append(die.value.ordinal());
            sb.append(die.used ? '1' : '0');
        }
        for(int i = 1; i <= playsheet.bankSize(); i++) {
            sb.append(playsheet.resourcesAvail[i] ? '1' : '0');
        }
        for(boolean b : playsheet.roads) {
            sb.append(b ? '1' : '0');
        }
        for(boolean b : playsheet.villages) {
            sb.append(b ? '1' : '0');
        }
        for(boolean b : playsheet.cities) {
            sb.append(b ? '1' : '0');
        }
        for(int i = 1; i < playsheet.traders.length; i++) {
            sb.append(playsheet.traders[i] ? '1' : '0');
        }
        return sb.toString();
    }
}
