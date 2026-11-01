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

import java.util.Random;

/**
 * The order the tracks play in: every track once in a shuffled order, then a new shuffle,
 * forever. A fresh shuffle never starts with the track that just ended, so there is no
 * immediate repeat across the loop boundary (unless there is only one track, when a repeat
 * is all there is). Pure Java, so the order can be tested without a device.
 */
public final class MusicPlaylist {

    private final int count;
    private final Random random;
    private int[] order;
    private int position;
    private int last = -1;

    /** A playlist over the indexes 0 to count - 1. */
    public MusicPlaylist(int count, Random random) {
        this.count = count;
        this.random = random;
        this.position = 0;
        this.order = new int[0];
    }

    public int size() {
        return count;
    }

    /** The index of the next track to play, or -1 when there are no tracks. */
    public int next() {
        if (count <= 0) {
            return -1;
        }
        if (position >= order.length) {
            reshuffle();
        }
        last = order[position++];
        return last;
    }

    private void reshuffle() {
        order = new int[count];
        for (int i = 0; i < count; i++) {
            order[i] = i;
        }
        for (int i = count - 1; i > 0; i--) {
            int j = random.nextInt(i + 1);
            int t = order[i];
            order[i] = order[j];
            order[j] = t;
        }
        if (count > 1 && order[0] == last) {
            // Swap the track that just played away from the front, with a random later one
            int j = 1 + random.nextInt(count - 1);
            int t = order[0];
            order[0] = order[j];
            order[j] = t;
        }
        position = 0;
    }
}
