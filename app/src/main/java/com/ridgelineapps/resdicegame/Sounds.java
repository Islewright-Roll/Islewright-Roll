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
 * The process-wide sound sink: SoundEffects puts itself here when GameApp installs it, and
 * until then (a unit test, or a view with no app) it is the silent one. The input classes
 * ask for sounds here, beside their haptic calls.
 */
final class Sounds {
   static volatile SoundSink sink = SoundSink.NONE;

   private Sounds() {
   }

   static void play(Sfx sfx) {
      sink.play(sfx, 0);
   }
}
