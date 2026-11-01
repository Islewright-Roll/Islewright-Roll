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

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.ArrayList;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import android.content.Context;
import android.util.Log;

/**
 * The in-progress games, one JSON document per island, deliberately separate from
 * {@link HighScoresAndSettings}: that file is exported and imported by the player as a
 * backup of scores and colour, and a half finished game does not belong in that document.
 *
 * Each island keeps its own file, so switching islands never loses a game. The default
 * island's file is still {@link #FILE_NAME}, exactly as before there were two islands, so
 * an existing player's save (and an older build) still finds it; any other island uses
 * {@code game-state-<mapId>.json} beside it. Every operation goes through the one code
 * path below and differs only in the file name {@link #fileName} derives from the map.
 *
 * There is no long-lived instance: {@link #save} writes a full snapshot, {@link #load}
 * either returns a fully restored {@link Game} or null, and the caller falls back to
 * {@code new Game(map)} when it gets null.
 */
public final class GameStateStore {

    /** The default island's file. Never renamed: older builds and existing saves use it. */
    public static final String FILE_NAME = "game-state.json";
    public static final int SCHEMA_VERSION = 2;

    /**
     * The format before maps existed: no map id (the game is the default map's) and the
     * traders as a count, "knights". Still read, never written.
     */
    static final int LEGACY_SCHEMA_VERSION = 1;

    private static final String TAG = "GameStateStore";

    private GameStateStore() {
    }

    /**
     * The file name for one island's save: {@link #FILE_NAME} for the default island and
     * {@code game-state-<mapId>.json} for every other.
     */
    static String fileName(MapDef map) {
        if(map.id.equals(MapDef.DEFAULT.id)) {
            return FILE_NAME;
        }
        return "game-state-" + map.id + ".json";
    }

    /**
     * The directory the saves live in. Same directory as {@link HighScoresAndSettings},
     * but different files, so a scores backup never carries a half played game along.
     */
    static File dir(Context context) {
        File dir = context.getExternalFilesDir(null);
        if(dir == null) {
            dir = context.getFilesDir();
        }
        return dir;
    }

    /** One island's store file. */
    public static File file(Context context, MapDef map) {
        return new File(dir(context), fileName(map));
    }

    /**
     * Writes the game if it is still going and clears its island's save if it is over, so a
     * finished game never comes back on relaunch. The one place that decides between the
     * two; {@link Game#changed}, the pause and an island switch all call it.
     */
    public static void persist(Context context, Game game) {
        persist(dir(context), game);
    }

    static void persist(File dir, Game game) {
        if(game.isGameDone()) {
            clear(dir, game.map);
        }
        else {
            save(dir, game);
        }
    }

    public static Game load(Context context, MapDef map) {
        return load(dir(context), map);
    }

    public static void save(Context context, Game game) {
        save(dir(context), game);
    }

    public static void clear(Context context, MapDef map) {
        clear(dir(context), map);
    }

    /**
     * Loads one island's saved game, or returns null when there is nothing to restore: no
     * file, a file that fails to parse or that holds another island's game (quarantined
     * rather than left to crash the next launch), or a save that somehow reached the end
     * of the game. That last case should never happen, since the save is cleared as soon
     * as the game is done, but returning null instead of resurrecting a finished game
     * keeps a stray file from ever surfacing one.
     */
    static Game load(File dir, MapDef map) {
        migrateMisplaced(dir);

        File file = new File(dir, fileName(map));
        if(!file.exists()) {
            return null;
        }

        try {
            Game game = fromJson(readFile(file));
            if(game.map != map) {
                throw new JSONException("file for \"" + map.id + "\" holds a game on \"" + game.map.id + "\"");
            }
            if(game.isGameDone()) {
                clear(dir, map);
                return null;
            }
            return game;
        }
        catch(Exception e) {
            e.printStackTrace();
            quarantine(file);
            return null;
        }
    }

    /**
     * A build from the days when there was one file for both islands wrote whichever game
     * was on screen to {@link #FILE_NAME}. If that file holds another island's game, it is
     * moved to that island's file, never discarded: this runs on every load, so the game is
     * found whichever island the launch opens on. When that island already has a file of its
     * own (an older build was run after a newer one), the newer of the two keeps the file and
     * the other is set aside under a {@code .misplaced-} name rather than deleted. A default
     * file that does not parse is left for {@link #load} to quarantine.
     */
    private static void migrateMisplaced(File dir) {
        File misplaced = new File(dir, FILE_NAME);
        if(!misplaced.exists()) {
            return;
        }

        MapDef owner;
        try {
            owner = fromJson(readFile(misplaced)).map;
        }
        catch(Exception e) {
            return;
        }
        if(owner.id.equals(MapDef.DEFAULT.id)) {
            return;
        }

        File target = new File(dir, fileName(owner));
        if(target.exists()) {
            if(misplaced.lastModified() > target.lastModified()) {
                setAside(target);
            }
            else {
                setAside(misplaced);
                return;
            }
        }
        if(!misplaced.renameTo(target)) {
            Log.w(TAG, "could not move " + misplaced + " to " + target);
        }
    }

    /** Atomic write: the temp file is written in full, then renamed over the real one. */
    static void save(File dir, Game game) {
        File file = new File(dir, fileName(game.map));
        File tmp = new File(dir, file.getName() + ".tmp");

        try {
            dir.mkdirs();

            FileOutputStream out = new FileOutputStream(tmp);
            try {
                out.write(toJson(game).getBytes("UTF-8"));
            }
            finally {
                out.close();
            }

            if(!tmp.renameTo(file)) {
                Log.w(TAG, "could not rename " + tmp + " to " + file);
            }
        }
        catch(IOException e) {
            e.printStackTrace();
        }
    }

    /** Deletes one island's saved game, if any. The other island's is not touched. */
    static void clear(File dir, MapDef map) {
        File file = new File(dir, fileName(map));
        if(file.exists() && !file.delete()) {
            Log.w(TAG, "could not delete " + file);
        }
    }

    /**
     * put() only throws for a NaN or infinite double, which this class never writes, so
     * the JSONException it declares is unreachable in practice and is wrapped here rather
     * than declared, matching {@link HighScoresAndSettings#toJson}.
     */
    static String toJson(Game game) {
        try {
            return buildJson(game);
        }
        catch(JSONException e) {
            throw new RuntimeException(e);
        }
    }

    static String buildJson(Game game) throws JSONException {
        JSONObject root = new JSONObject();
        root.put("schemaVersion", SCHEMA_VERSION);

        JSONObject gameJson = new JSONObject();
        gameJson.put("mapId", game.map.id);
        gameJson.put("rolls", game.rolls);
        gameJson.put("turnsTaken", game.turnsTaken);
        gameJson.put("builtResourceThisTurn", game.builtResourceThisTurn);
        gameJson.put("scored", game.scored);
        // Only a race has a target, and it is the game's own: the setting may have moved
        // since the game began. A points map writes the same document as before.
        if(game.isRace()) {
            gameJson.put("targetScore", game.targetScore);
        }

        JSONArray knightResources = new JSONArray();
        for(Dice.Value val : game.knightResources) {
            knightResources.put(val.name());
        }
        gameJson.put("knightResources", knightResources);

        JSONArray dice = new JSONArray();
        for(Dice die : game.dice) {
            JSONObject dieJson = new JSONObject();
            dieJson.put("value", die.value.name());
            dieJson.put("held", die.held);
            dieJson.put("used", die.used);
            dieJson.put("locked", die.locked);
            dice.put(dieJson);
        }
        gameJson.put("dice", dice);

        JSONObject playsheet = new JSONObject();
        // The array itself, not a count: a map whose traders can be built in any order has
        // states a count cannot say. Index 0 is unused, as in the sheet.
        playsheet.put("traders", toBooleanArray(game.playsheet.traders));
        playsheet.put("resourcesAvail", toBooleanArray(game.playsheet.resourcesAvail));
        playsheet.put("roads", toBooleanArray(game.playsheet.roads));
        playsheet.put("villages", toBooleanArray(game.playsheet.villages));
        playsheet.put("cities", toBooleanArray(game.playsheet.cities));
        playsheet.put("turnsNothingBuilt", game.playsheet.turnsNothingBuilt);
        playsheet.put("turnScore", toIntArray(game.playsheet.turnScore));
        gameJson.put("playsheet", playsheet);

        root.put("game", gameJson);

        // Not pretty printed. This is written after every action and nobody reads it by
            // hand: the indentation was 869 of its 1601 bytes, more than half the file.
            return root.toString();
    }

    /**
     * Parses and fully applies the save in one pass. Reads the map first, then starts from
     * a plain {@code new Game(map)} (whose constructor already leaves dice, playsheet and
     * turn counters at their fresh-game defaults) and overwrites every field this class
     * persists, the same
     * way {@link com.ridgelineapps.resdicegame.DebugScenario#applyTo} mutates a live game.
     * Nothing from the constructor's own {@code newTurn(true)} survives: every field it
     * touches is listed below and gets overwritten here.
     *
     * A version 1 save has no map id and is the default map's. A version 2 save names its
     * map, and a name nobody knows throws rather than falling back to the default: the
     * file is quarantined, where loading it as the wrong map would mangle a game that a
     * newer build could still read.
     *
     * Throws on anything that does not match the expected shape, so the caller can
     * quarantine the file and fall back to a fresh game rather than restore something
     * half wrong.
     */
    static Game fromJson(String json) throws JSONException {
        return fromJson(json, MapDef.ALL);
    }

    /** As {@link #fromJson(String)}, with the maps a save may name given by the caller. */
    static Game fromJson(String json, MapDef[] maps) throws JSONException {
        JSONObject root = new JSONObject(json);

        int schemaVersion = root.optInt("schemaVersion", -1);
        if(schemaVersion != SCHEMA_VERSION && schemaVersion != LEGACY_SCHEMA_VERSION) {
            throw new JSONException("unsupported schemaVersion " + schemaVersion);
        }

        JSONObject gameJson = root.getJSONObject("game");

        MapDef map = MapDef.DEFAULT;
        if(schemaVersion >= SCHEMA_VERSION) {
            map = findMap(gameJson.getString("mapId"), maps);
        }
        Game game = new Game(map);

        game.rolls = gameJson.getInt("rolls");
        game.turnsTaken = gameJson.getInt("turnsTaken");
        game.builtResourceThisTurn = gameJson.getBoolean("builtResourceThisTurn");
        game.scored = gameJson.getBoolean("scored");
        // A save from before the target could be set raced to the island's own. A stray
        // value from a hand edit is pulled into the range a game can reach.
        if(game.isRace()) {
            int target = gameJson.optInt("targetScore", map.targetScore);
            game.targetScore = Math.max(1, Math.min(map.maxScore(), target));
        }

        game.knightResources = new ArrayList<Dice.Value>();
        JSONArray knightResources = gameJson.getJSONArray("knightResources");
        for(int i = 0; i < knightResources.length(); i++) {
            game.knightResources.add(parseDiceValue(knightResources.getString(i)));
        }

        JSONArray dice = gameJson.getJSONArray("dice");
        if(dice.length() != game.dice.length) {
            throw new JSONException("dice must have exactly " + game.dice.length + " entries, had " + dice.length());
        }
        for(int i = 0; i < dice.length(); i++) {
            JSONObject dieJson = dice.getJSONObject(i);
            Dice die = game.dice[i];
            die.value = parseDiceValue(dieJson.getString("value"));
            die.held = dieJson.getBoolean("held");
            die.used = dieJson.getBoolean("used");
            // Not in a save from before trader resource classic mode, where nothing was locked
            die.locked = dieJson.optBoolean("locked", false);
        }

        JSONObject playsheet = gameJson.getJSONObject("playsheet");
        if(schemaVersion >= SCHEMA_VERSION) {
            game.playsheet.traders = parseBooleanArray(playsheet.getJSONArray("traders"), game.playsheet.traders.length, "traders");
            game.playsheet.traders[0] = false;
        }
        else {
            int knights = playsheet.getInt("knights");
            if(knights < 0 || knights > game.map.traders.length) {
                throw new JSONException("knights must be between 0 and " + game.map.traders.length + ", was " + knights);
            }
            game.playsheet.setTraderCount(knights);
        }
        game.playsheet.resourcesAvail = parseBooleanArray(playsheet.getJSONArray("resourcesAvail"), game.playsheet.resourcesAvail.length, "resourcesAvail");
        game.playsheet.roads = parseBooleanArray(playsheet.getJSONArray("roads"), game.playsheet.roads.length, "roads");
        game.playsheet.villages = parseBooleanArray(playsheet.getJSONArray("villages"), game.playsheet.villages.length, "villages");
        game.playsheet.cities = parseBooleanArray(playsheet.getJSONArray("cities"), game.playsheet.cities.length, "cities");
        game.playsheet.turnsNothingBuilt = playsheet.getInt("turnsNothingBuilt");
        // A map with no turn cap grows its turn scores as the game goes on, so any length
        // is right for it. That includes a save from when Dunholm had a cap of twenty.
        game.playsheet.turnScore = game.map.turnCap == 0
                ? parseIntArray(playsheet.getJSONArray("turnScore"))
                : parseIntArray(playsheet.getJSONArray("turnScore"), game.playsheet.turnScore.length, "turnScore");

        return game;
    }

    private static MapDef findMap(String id, MapDef[] maps) throws JSONException {
        for(MapDef map : maps) {
            if(map.id.equals(id)) {
                return map;
            }
        }
        throw new JSONException("unknown mapId \"" + id + "\"");
    }

    private static JSONArray toBooleanArray(boolean[] values) {
        JSONArray array = new JSONArray();
        for(boolean value : values) {
            array.put(value);
        }
        return array;
    }

    private static JSONArray toIntArray(int[] values) {
        JSONArray array = new JSONArray();
        for(int value : values) {
            array.put(value);
        }
        return array;
    }

    private static boolean[] parseBooleanArray(JSONArray array, int length, String field) throws JSONException {
        if(array.length() != length) {
            throw new JSONException(field + " must have exactly " + length + " entries, had " + array.length());
        }
        boolean[] result = new boolean[length];
        for(int i = 0; i < length; i++) {
            result[i] = array.getBoolean(i);
        }
        return result;
    }

    private static int[] parseIntArray(JSONArray array, int length, String field) throws JSONException {
        if(array.length() != length) {
            throw new JSONException(field + " must have exactly " + length + " entries, had " + array.length());
        }
        return parseIntArray(array);
    }

    /** Any length at all, with at least the unused entry at index 0 in what it returns. */
    private static int[] parseIntArray(JSONArray array) throws JSONException {
        int length = array.length();
        int[] result = new int[Math.max(1, length)];
        for(int i = 0; i < length; i++) {
            result[i] = array.getInt(i);
        }
        return result;
    }

    private static Dice.Value parseDiceValue(String name) throws JSONException {
        try {
            return Dice.Value.valueOf(name);
        }
        catch(IllegalArgumentException e) {
            throw new JSONException("unrecognised dice value \"" + name + "\"");
        }
    }

    private static void quarantine(File file) {
        rename(file, ".corrupt-");
    }

    private static void setAside(File file) {
        rename(file, ".misplaced-");
    }

    private static void rename(File file, String infix) {
        File renamed = new File(file.getParentFile(), file.getName() + infix + System.currentTimeMillis());
        if(!file.renameTo(renamed)) {
            Log.w(TAG, "could not rename save file " + file);
        }
    }

    private static String readFile(File file) throws IOException {
        byte[] bytes = new byte[(int) file.length()];
        FileInputStream in = new FileInputStream(file);
        try {
            int offset = 0;
            while(offset < bytes.length) {
                int read = in.read(bytes, offset, bytes.length - offset);
                if(read < 0) {
                    break;
                }
                offset += read;
            }
        }
        finally {
            in.close();
        }
        return new String(bytes, "UTF-8");
    }
}
