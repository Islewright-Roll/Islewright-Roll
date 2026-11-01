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

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;

import org.json.JSONException;

import android.content.Context;
import android.widget.Toast;

/**
 * Entry point MainActivity reaches through reflection (see
 * MainActivity.isDebugHarnessAvailable() and applyDebugScenario()). This class lives in
 * the debug source set, not main, so it and everything it does are absent entirely from
 * a release build: there is no class for Class.forName() to find, so the menu item that
 * would call this stays hidden and the reflective call itself would fail if it somehow
 * ran anyway.
 *
 * Reads DebugScenario.FILE_NAME from external files dir and applies it to the live
 * game, showing a Toast that says plainly what happened. A missing file, an unreadable
 * file, and an invalid scenario are three different messages, and in every failure case
 * the live game is left completely untouched: see DebugScenario for why a bad file
 * cannot half-apply. A valid scenario for the island that is not on screen switches island
 * first, through MainActivity.switchMap.
 */
public final class DebugScenarioLoader {

   private DebugScenarioLoader() {
   }

   public static void applyFromFile(Context context, Game game) {
      File file = DebugScenario.file(context);

      if(!file.exists()) {
         Toast.makeText(context, "No scenario file at " + file.getAbsolutePath(), Toast.LENGTH_LONG).show();
         return;
      }

      String json;
      try {
         json = readFile(file);
      }
      catch(IOException e) {
         Toast.makeText(context, "Could not read scenario file: " + e.getMessage(), Toast.LENGTH_LONG).show();
         return;
      }

      DebugScenario scenario;
      try {
         scenario = DebugScenario.fromJson(json);
      }
      catch(JSONException e) {
         Toast.makeText(context, "Scenario invalid, nothing applied: " + e.getMessage(), Toast.LENGTH_LONG).show();
         return;
      }

      // A scenario for the other island switches island first. MainActivity owns that: the
      // game on screen is kept in its island's save, the GameView is rebuilt, and the game
      // now on screen is a different object, so the scenario is applied to the one it returns.
      // The scenario replaces that island's own game, as it would on screen.
      boolean switched = false;
      if(scenario.map != game.map) {
         if(!(context instanceof MainActivity)) {
            Toast.makeText(context, "Scenario is for map \"" + scenario.map.id + "\" but the running game is on \"" + game.map.id
                  + "\" and nothing here can switch it; nothing applied", Toast.LENGTH_LONG).show();
            return;
         }
         game = ((MainActivity) context).switchMap(scenario.map);
         switched = true;
      }

      scenario.applyTo(game);
      Toast.makeText(context, switched ? "Switched to " + scenario.map.id + ", scenario applied" : "Scenario applied", Toast.LENGTH_SHORT).show();
   }

   private static String readFile(File file) throws IOException {
      FileInputStream in = new FileInputStream(file);
      try {
         ByteArrayOutputStream out = new ByteArrayOutputStream();
         byte[] buffer = new byte[4096];
         int read;
         while((read = in.read(buffer)) >= 0) {
            out.write(buffer, 0, read);
         }
         return out.toString("UTF-8");
      }
      finally {
         in.close();
      }
   }
}
