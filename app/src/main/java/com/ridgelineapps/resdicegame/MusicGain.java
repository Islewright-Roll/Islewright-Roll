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

/**
 * How loud the Music volume setting is. The setting is a whole number from 0 to 100; the
 * gain handed to MediaPlayer.setVolume is that fraction squared. Loudness is heard roughly
 * logarithmically, so a straight fraction would feel as if the slider did nothing over its
 * top half and everything in the bottom; squaring spreads the steps more evenly to the ear.
 *
 * The gain applies on top of the device's media volume, which the hardware keys and the
 * system own: this app never reads or changes the media stream's volume.
 */
public final class MusicGain {

    private MusicGain() {
    }

    /** The MediaPlayer gain, 0 to 1, for a setting; out-of-range settings are pulled into range. */
    public static float gainFor(int setting) {
        float fraction = Math.max(0, Math.min(100, setting)) / 100f;
        return fraction * fraction;
    }
}
