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
import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.TimeZone;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import android.content.Context;
import android.content.SharedPreferences;
import android.content.SharedPreferences.Editor;
import android.preference.PreferenceManager;
import android.util.Log;

/**
 * The high scores and settings, held in one JSON document rather than SharedPreferences,
 * at a path a power user can read and edit without root (getExternalFilesDir). There is
 * no long-lived instance: every mutation is load, mutate, save.
 */
public final class HighScoresAndSettings {

    public static final String FILE_NAME = "highscores-and-settings.json";
    public static final int SCHEMA_VERSION = 1;
    public static final int MAX_SCORES = 5;

    private static final String TAG = "HighScoresAndSettings";
    private static final String DATE_PATTERN = "yyyy-MM-dd'T'HH:mm:ss'Z'";

    private static final Comparator<HighScores.ScoreAndDate> DESCENDING = new Comparator<HighScores.ScoreAndDate>() {
        @Override
        public int compare(HighScores.ScoreAndDate a, HighScores.ScoreAndDate b) {
            return b.score - a.score;
        }
    };

    private static final Comparator<HighScores.ScoreAndDate> ASCENDING = new Comparator<HighScores.ScoreAndDate>() {
        @Override
        public int compare(HighScores.ScoreAndDate a, HighScores.ScoreAndDate b) {
            return a.score - b.score;
        }
    };

    /**
     * The default map's scores. They stay under the original "highScores" key and are never
     * moved out of it, so a build that knows nothing of maps still finds them. Every other
     * map's list lives in {@link #otherScores}; reach either through {@link #scoresFor}.
     */
    public final ArrayList<HighScores.ScoreAndDate> scores = new ArrayList<HighScores.ScoreAndDate>();

    /**
     * Scores of every other map by map id, written as "highScoresByMap". A map this build
     * does not know is kept exactly as read (not sorted, not trimmed, since its order is
     * not ours to guess) so a file touched by an older build loses nothing.
     */
    private final LinkedHashMap<String, ArrayList<HighScores.ScoreAndDate>> otherScores = new LinkedHashMap<String, ArrayList<HighScores.ScoreAndDate>>();

    /** The maps a score list can belong to: what orders a list and what a result may name. */
    private final MapDef[] maps;

    public int playerColor = PlayerColor.DEFAULT;

    /** The id of the chosen map. A stored field only; the game reads it, this does not act on it. */
    public String map = MapDef.DEFAULT.id;

    /** As playerColorWasSpecified, for map: a missing one is not the same as an explicit default. */
    public boolean mapWasSpecified = false;

    public HighScoresAndSettings() {
        this(MapDef.ALL);
    }

    /** For tests: the same store over a different set of maps. */
    HighScoresAndSettings(MapDef[] maps) {
        this.maps = maps;
    }

    /**
     * When to ask before building early, which forfeits the rolls not yet used.
     *
     * The number is how many dice the turn must be able to spend, counting the whole
     * sequence of builds reachable from the tap and not just the tapped one, before the
     * prompt is skipped as obviously deliberate.
     *
     * SMART settles for five and so asks less often. CONSERVATIVE wants every die and so
     * asks more often, which is what makes it the cautious one. OFF never asks.
     */
    public static final int EARLY_PROMPT_OFF = 0;
    public static final int EARLY_PROMPT_SMART = 5;
    public static final int EARLY_PROMPT_CONSERVATIVE = 6;
    public int earlyBuildPrompt = EARLY_PROMPT_SMART;

    /** Whether to warn when ending a turn with something still affordable from dice. */
    public boolean endTurnPrompt = true;

    /** Whether the game gives haptic feedback at all. See Haptics. */
    public boolean hapticFeedback = true;

    /** As playerColorWasSpecified, for hapticFeedback. */
    public boolean hapticFeedbackWasSpecified = false;

    /** Whether the game plays its sound effects at all. See SoundEffects. */
    public boolean soundEffects = true;

    /** As playerColorWasSpecified, for soundEffects. */
    public boolean soundEffectsWasSpecified = false;

    /**
     * Trader resource classic mode: a banked resource is not spent by itself but dragged
     * onto a rolled die, which it overwrites. Off by default. See Playsheet.bankPaysBuilds.
     */
    public boolean traderClassic = false;

    /** As playerColorWasSpecified, for traderClassic. */
    public boolean traderClassicWasSpecified = false;

    /** The Theme setting: follow the OS night mode, or always light, or always dark. */
    public static final int THEME_SYSTEM = 0;
    public static final int THEME_LIGHT = 1;
    public static final int THEME_DARK = 2;
    public int theme = THEME_SYSTEM;

    /** As playerColorWasSpecified, for theme. */
    public boolean themeWasSpecified = false;

    /**
     * Whether the welcome dialog is shown when the app starts. On for a new install and for
     * a file written before this setting existed, so an upgrading player sees it once.
     */
    public boolean showWelcome = true;

    /** As playerColorWasSpecified, for showWelcome. */
    public boolean showWelcomeWasSpecified = false;

    /**
     * Whether the in-context tutorial's callouts show during play. On for a new install and
     * for a file written before this setting existed; Skip tutorial, or seeing every
     * callout, turns it off.
     */
    public boolean tutorial = true;

    /** As playerColorWasSpecified, for tutorial. */
    public boolean tutorialWasSpecified = false;

    /**
     * Which tutorial steps have been seen, one bit each (Tutorial.Step.bit), so the tutorial
     * resumes where it left off after the app is closed. 0 for none.
     */
    public int tutorialSeen = 0;

    /** As playerColorWasSpecified, for tutorialSeen. */
    public boolean tutorialSeenWasSpecified = false;

    /**
     * The Music volume setting, 0 to 100. 0 is music off. How loud that is on top of the
     * device's media volume is MusicPlayer.gainFor's business.
     */
    public static final int MUSIC_VOLUME_MIN = 0;
    public static final int MUSIC_VOLUME_MAX = 100;
    public static final int MUSIC_VOLUME_DEFAULT = 60;
    public int musicVolume = MUSIC_VOLUME_DEFAULT;

    /** As playerColorWasSpecified, for musicVolume. */
    public boolean musicVolumeWasSpecified = false;

    /**
     * The Dunholm target setting: the score that wins the race, 1 to the most the island
     * can give (MapDef.maxScore). Read when a race starts and kept by the game, so a saved
     * game goes on with the target it was started with. Top Scores record only a game
     * played to the island's own target, since fewer turns to another target is not the
     * same feat.
     */
    // A race is never shorter than the island's own target: the setting only lengthens it
    public static final int RACE_TARGET_MIN = MapDef.DUNHOLM.targetScore;
    public static final int RACE_TARGET_DEFAULT = MapDef.DUNHOLM.targetScore;
    public int raceTarget = RACE_TARGET_DEFAULT;

    /** As playerColorWasSpecified, for raceTarget. */
    public boolean raceTargetWasSpecified = false;

    /**
     * Whether the parsed document actually carried a playerColor, as opposed to falling
     * back to the default. Import needs the distinction: applying a defaulted colour
     * would silently repaint the board red whenever an imported file simply lacked the
     * field.
     */
    public boolean playerColorWasSpecified = false;

    /**
     * The store file. External storage needs no runtime permission at any supported API
     * level and is readable from a computer with "adb pull" or USB file transfer without
     * root. Falls back to internal storage on the rare device with none mounted.
     */
    public static File file(Context context) {
        File dir = context.getExternalFilesDir(null);
        if(dir == null) {
            dir = context.getFilesDir();
        }
        return new File(dir, FILE_NAME);
    }

    /**
     * Loads the store. The first time the file is missing, this migrates the old
     * SharedPreferences scores into it and removes those keys, so there is one source of
     * truth from then on. A file that exists but fails to parse is quarantined rather
     * than overwritten, since a power user editing by hand should get their text back,
     * not lose it to a silent rewrite.
     */
    public static HighScoresAndSettings load(Context context) {
        File file = file(context);

        if(file.exists()) {
            try {
                return fromJson(readFile(file));
            }
            catch(Exception e) {
                e.printStackTrace();
                quarantine(file);
            }
        }

        return migrate(context);
    }

    /** Atomic write: the temp file is written in full, then renamed over the real one. */
    public void save(Context context) {
        File file = file(context);
        File tmp = new File(file.getParentFile(), file.getName() + ".tmp");

        try {
            File parent = tmp.getParentFile();
            if(parent != null) {
                parent.mkdirs();
            }

            FileOutputStream out = new FileOutputStream(tmp);
            try {
                out.write(toJson().getBytes("UTF-8"));
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

    /** {@link #addScore(String, int)} on the default map. */
    public int addScore(int score) {
        return addScore(MapDef.DEFAULT.id, score);
    }

    /**
     * Inserts a score into one map's list at the first position whose entry it beats or
     * ties (matching the original SharedPreferences semantics: a tie enters at or above the
     * equal score), trims to MAX_SCORES, and returns the 1-based rank, or 0 if it did not
     * make the table. "Beats" follows the map: more points on a map where higher is better,
     * fewer turns where lower is. A map this store does not know is a bug in the caller, not
     * a score to file under some other map, so it throws.
     */
    public int addScore(String mapId, int score) {
        MapDef def = knownMap(mapId);
        if(def == null) {
            throw new IllegalArgumentException("unknown map \"" + mapId + "\"");
        }

        ArrayList<HighScores.ScoreAndDate> list = listFor(mapId, true);
        int insertAt = list.size();
        for(int i = 0; i < list.size(); i++) {
            int existing = list.get(i).score;
            if(def.lowerIsBetter ? score <= existing : score >= existing) {
                insertAt = i;
                break;
            }
        }

        if(insertAt >= MAX_SCORES) {
            return 0;
        }

        HighScores.ScoreAndDate sad = new HighScores.ScoreAndDate();
        sad.score = score;
        sad.date = System.currentTimeMillis();
        list.add(insertAt, sad);

        while(list.size() > MAX_SCORES) {
            list.remove(list.size() - 1);
        }

        return insertAt + 1;
    }

    /**
     * One map's results, best first: highest score first where higher is better, fewest
     * turns first where lower is. The default map's list is {@link #scores} itself; the
     * others are lists kept by this store. The list is live, so a caller that changes it
     * changes the store. An id this store has no list for gives a fresh empty list.
     */
    public ArrayList<HighScores.ScoreAndDate> scoresFor(String mapId) {
        ArrayList<HighScores.ScoreAndDate> list = listFor(mapId, knownMap(mapId) != null);
        return list != null ? list : new ArrayList<HighScores.ScoreAndDate>();
    }

    /** The ids of every map this store holds a list for, the default map first. */
    public List<String> mapIdsWithScores() {
        ArrayList<String> ids = new ArrayList<String>();
        ids.add(MapDef.DEFAULT.id);
        for(String id : otherScores.keySet()) {
            ids.add(id);
        }
        return ids;
    }

    /** {@link #mergeScores(String, List)} on the default map. */
    public int mergeScores(List<HighScores.ScoreAndDate> other) {
        return mergeScores(MapDef.DEFAULT.id, other);
    }

    /**
     * Unions another list of one map's results into that map's list, deduping on (score,
     * date), keeping the best MAX_SCORES in the map's own order. Returns how many of the
     * given scores were new. A map this store does not know has the new entries appended
     * and nothing else done to them.
     */
    public int mergeScores(String mapId, List<HighScores.ScoreAndDate> other) {
        ArrayList<HighScores.ScoreAndDate> list = listFor(mapId, true);
        int added = 0;
        for(HighScores.ScoreAndDate sad : new ArrayList<HighScores.ScoreAndDate>(other)) {
            if(!containsScoreAndDate(list, sad)) {
                list.add(sad);
                added++;
            }
        }

        if(added > 0) {
            order(mapId, list);
        }

        return added;
    }

    /**
     * What importing a backup does: every map's results in the file are merged into this
     * store, and the player's colour, haptics, sound effects, theme, welcome, tutorial and map are taken over only where the file
     * actually carried one (the parser defaults a missing value, and applying that would
     * overwrite a real choice with a guess). Returns how many scores were new.
     */
    public int importFrom(HighScoresAndSettings imported) {
        int added = 0;
        for(String mapId : imported.mapIdsWithScores()) {
            added += mergeScores(mapId, imported.scoresFor(mapId));
        }
        // A backup carries the player's colour and restoring one should bring it back.
        if(imported.playerColorWasSpecified) {
            playerColor = imported.playerColor;
        }
        if(imported.hapticFeedbackWasSpecified) {
            hapticFeedback = imported.hapticFeedback;
        }
        if(imported.soundEffectsWasSpecified) {
            soundEffects = imported.soundEffects;
            soundEffectsWasSpecified = true;
        }
        if(imported.traderClassicWasSpecified) {
            traderClassic = imported.traderClassic;
            traderClassicWasSpecified = true;
        }
        if(imported.themeWasSpecified) {
            theme = imported.theme;
            themeWasSpecified = true;
        }
        if(imported.showWelcomeWasSpecified) {
            showWelcome = imported.showWelcome;
            showWelcomeWasSpecified = true;
        }
        if(imported.tutorialWasSpecified) {
            tutorial = imported.tutorial;
            tutorialWasSpecified = true;
        }
        if(imported.tutorialSeenWasSpecified) {
            tutorialSeen = imported.tutorialSeen;
            tutorialSeenWasSpecified = true;
        }
        if(imported.musicVolumeWasSpecified) {
            musicVolume = imported.musicVolume;
            musicVolumeWasSpecified = true;
        }
        if(imported.raceTargetWasSpecified) {
            raceTarget = imported.raceTarget;
            raceTargetWasSpecified = true;
        }
        if(imported.mapWasSpecified) {
            map = imported.map;
            mapWasSpecified = true;
        }
        return added;
    }

    /** This store's map with the given id, or null when it has none. */
    private MapDef knownMap(String mapId) {
        for(MapDef def : maps) {
            if(def.id.equals(mapId)) {
                return def;
            }
        }
        return null;
    }

    /** The list for a map id; null when there is none and create is false. */
    private ArrayList<HighScores.ScoreAndDate> listFor(String mapId, boolean create) {
        if(MapDef.DEFAULT.id.equals(mapId)) {
            return scores;
        }

        ArrayList<HighScores.ScoreAndDate> list = otherScores.get(mapId);
        if(list == null && create) {
            list = new ArrayList<HighScores.ScoreAndDate>();
            otherScores.put(mapId, list);
        }
        return list;
    }

    /** Sorts a list best first for its map and cuts it to MAX_SCORES. A map nobody knows is left alone. */
    private void order(String mapId, ArrayList<HighScores.ScoreAndDate> list) {
        MapDef def = knownMap(mapId);
        if(def == null) {
            return;
        }

        Collections.sort(list, def.lowerIsBetter ? ASCENDING : DESCENDING);
        while(list.size() > MAX_SCORES) {
            list.remove(list.size() - 1);
        }
    }

    private static boolean containsScoreAndDate(List<HighScores.ScoreAndDate> list, HighScores.ScoreAndDate sad) {
        for(HighScores.ScoreAndDate existing : list) {
            if(existing.score == sad.score && existing.date == sad.date) {
                return true;
            }
        }
        return false;
    }

    public String toJson() {
        try {
            JSONObject root = new JSONObject();
            root.put("schemaVersion", SCHEMA_VERSION);

            root.put("highScores", scoresToJson(scores));

            // Only for the maps that have results, so a player who never leaves the first
            // island has a file no different from the one the previous build wrote.
            JSONObject byMap = new JSONObject();
            for(String mapId : otherScores.keySet()) {
                ArrayList<HighScores.ScoreAndDate> list = otherScores.get(mapId);
                if(!list.isEmpty()) {
                    byMap.put(mapId, scoresToJson(list));
                }
            }
            if(byMap.length() > 0) {
                root.put("highScoresByMap", byMap);
            }

            JSONObject settings = new JSONObject();
            settings.put("playerColor", PlayerColor.toHex(playerColor));
            settings.put("earlyBuildPrompt", earlyBuildPrompt);
            settings.put("endTurnPrompt", endTurnPrompt);
            settings.put("hapticFeedback", hapticFeedback);
            // Left out while it is the default and was never in the file, so a file from the
            // previous build rewrites as the same document
            if(soundEffectsWasSpecified || !soundEffects) {
                settings.put("soundEffects", soundEffects);
            }
            if(traderClassicWasSpecified || traderClassic) {
                settings.put("traderClassic", traderClassic);
            }
            // Like the map, left out while it is the default and was never in the file, so a
            // file from the previous build rewrites as the same document
            if(themeWasSpecified || theme != THEME_SYSTEM) {
                settings.put("theme", themeName(theme));
            }
            if(showWelcomeWasSpecified || !showWelcome) {
                settings.put("showWelcome", showWelcome);
            }
            if(tutorialWasSpecified || !tutorial) {
                settings.put("tutorial", tutorial);
            }
            if(tutorialSeenWasSpecified || tutorialSeen != 0) {
                settings.put("tutorialSeen", tutorialSeen);
            }
            if(musicVolumeWasSpecified || musicVolume != MUSIC_VOLUME_DEFAULT) {
                settings.put("musicVolume", musicVolume);
            }
            if(raceTargetWasSpecified || raceTarget != RACE_TARGET_DEFAULT) {
                settings.put("raceTarget", raceTarget);
            }
            // Left out while it is the default and was never in the file, for the same reason.
            if(mapWasSpecified || !MapDef.DEFAULT.id.equals(map)) {
                settings.put("map", map);
            }
            root.put("settings", settings);

            return root.toString(2);
        }
        catch(JSONException e) {
            // put() only throws for a NaN or infinite double, which this class never
            // writes, so this is unreachable in practice.
            throw new RuntimeException(e);
        }
    }

    private static JSONArray scoresToJson(List<HighScores.ScoreAndDate> list) throws JSONException {
        JSONArray array = new JSONArray();
        for(HighScores.ScoreAndDate sad : list) {
            JSONObject entry = new JSONObject();
            entry.put("score", sad.score);
            entry.put("playedAt", formatDate(sad.date));
            array.put(entry);
        }
        return array;
    }

    /** Reads a list of {score, playedAt} entries, skipping any that is not one. */
    private static ArrayList<HighScores.ScoreAndDate> scoresFromJson(JSONArray array) {
        ArrayList<HighScores.ScoreAndDate> list = new ArrayList<HighScores.ScoreAndDate>();
        for(int i = 0; i < array.length(); i++) {
            JSONObject entry = array.optJSONObject(i);
            if(entry == null || !entry.has("score")) {
                continue;
            }

            int score;
            try {
                score = entry.getInt("score");
            }
            catch(JSONException e) {
                continue;
            }

            HighScores.ScoreAndDate sad = new HighScores.ScoreAndDate();
            sad.score = score;
            sad.date = parseDate(entry.opt("playedAt"));
            list.add(sad);
        }
        return list;
    }

    /**
     * Parses the store. A schemaVersion other than the one this code understands is
     * treated as corrupt, since a newer or unknown shape cannot be read safely. Missing
     * "settings" or "playerColor" fall back to the default preset; a missing "highScores"
     * is an empty list; a score entry with a non-integer "score" is skipped rather than
     * failing the whole file, so one bad hand-edit does not cost the rest of the table.
     */
    public static HighScoresAndSettings fromJson(String json) throws JSONException {
        return fromJson(json, MapDef.ALL);
    }

    /**
     * As {@link #fromJson(String)} over the given maps. Each list is ordered for its own map
     * on load. The optional "highScoresByMap" holds the other maps' lists; a "tarnholm" key
     * in it is ignored, since that list lives in "highScores" and nowhere else.
     */
    static HighScoresAndSettings fromJson(String json, MapDef[] maps) throws JSONException {
        JSONObject root = new JSONObject(json);

        int schemaVersion = root.optInt("schemaVersion", -1);
        if(schemaVersion != SCHEMA_VERSION) {
            throw new JSONException("unsupported schemaVersion " + schemaVersion);
        }

        HighScoresAndSettings store = new HighScoresAndSettings(maps);

        JSONArray array = root.optJSONArray("highScores");
        if(array != null) {
            store.scores.addAll(scoresFromJson(array));
        }
        Collections.sort(store.scores, DESCENDING);
        while(store.scores.size() > MAX_SCORES) {
            store.scores.remove(store.scores.size() - 1);
        }

        JSONObject byMap = root.optJSONObject("highScoresByMap");
        if(byMap != null) {
            for(Iterator<String> ids = byMap.keys(); ids.hasNext();) {
                String mapId = ids.next();
                JSONArray list = byMap.optJSONArray(mapId);
                if(list == null || MapDef.DEFAULT.id.equals(mapId)) {
                    continue;
                }
                ArrayList<HighScores.ScoreAndDate> mapScores = store.listFor(mapId, true);
                mapScores.addAll(scoresFromJson(list));
                store.order(mapId, mapScores);
            }
        }

        JSONObject settings = root.optJSONObject("settings");
        if(settings != null && settings.has("playerColor")) {
            try {
                store.playerColor = PlayerColor.parseHex(settings.optString("playerColor"));
                    store.playerColorWasSpecified = true;
            }
            catch(IllegalArgumentException e) {
                store.playerColor = PlayerColor.DEFAULT;
            }
            store.earlyBuildPrompt = settings.optInt("earlyBuildPrompt", EARLY_PROMPT_SMART);
            store.endTurnPrompt = settings.optBoolean("endTurnPrompt", true);
        }
        if(settings != null && settings.has("map")) {
            // An id this build has no map for is left unspecified, so the default stands.
            String mapId = settings.optString("map");
            if(store.knownMap(mapId) != null) {
                store.map = mapId;
                store.mapWasSpecified = true;
            }
        }
        if(settings != null && settings.has("hapticFeedback")) {
            store.hapticFeedback = settings.optBoolean("hapticFeedback", true);
            store.hapticFeedbackWasSpecified = true;
        }
        if(settings != null && settings.has("soundEffects")) {
            store.soundEffects = settings.optBoolean("soundEffects", true);
            store.soundEffectsWasSpecified = true;
        }
        if(settings != null && settings.has("traderClassic")) {
            store.traderClassic = settings.optBoolean("traderClassic", false);
            store.traderClassicWasSpecified = true;
        }

        if(settings != null && settings.has("theme")) {
            int parsed = themeFromName(settings.optString("theme"));
            if(parsed >= 0) {
                store.theme = parsed;
                store.themeWasSpecified = true;
            }
        }
        if(settings != null && settings.has("showWelcome")) {
            store.showWelcome = settings.optBoolean("showWelcome", true);
            store.showWelcomeWasSpecified = true;
        }
        if(settings != null && settings.has("tutorial")) {
            store.tutorial = settings.optBoolean("tutorial", true);
            store.tutorialWasSpecified = true;
        }
        if(settings != null && settings.has("tutorialSeen")) {
            // A stray value from a hand edit is cut down to the bits there are steps for
            Object raw = settings.opt("tutorialSeen");
            if(raw instanceof Number) {
                store.tutorialSeen = ((Number) raw).intValue() & Tutorial.ALL;
                store.tutorialSeenWasSpecified = true;
            }
        }
        if(settings != null && settings.has("musicVolume")) {
            // A stray value from a hand edit is pulled into range rather than refused
            Object raw = settings.opt("musicVolume");
            if(raw instanceof Number) {
                store.musicVolume = clampMusicVolume(((Number) raw).intValue());
                store.musicVolumeWasSpecified = true;
            }
        }
        if(settings != null && settings.has("raceTarget")) {
            // As the volume: a stray value from a hand edit is pulled into range
            Object raw = settings.opt("raceTarget");
            if(raw instanceof Number) {
                store.raceTarget = clampRaceTarget(((Number) raw).intValue());
                store.raceTargetWasSpecified = true;
            }
        }

        return store;
    }

    static int clampMusicVolume(int volume) {
        return Math.max(MUSIC_VOLUME_MIN, Math.min(MUSIC_VOLUME_MAX, volume));
    }

    /** The highest Dunholm target there is: everything the island has to build, built. */
    static int raceTargetMax() {
        return MapDef.DUNHOLM.maxScore();
    }

    static int clampRaceTarget(int target) {
        return Math.max(RACE_TARGET_MIN, Math.min(raceTargetMax(), target));
    }

    /** The name a Theme value is stored under: plain words, so a hand edit is easy. */
    static String themeName(int theme) {
        if(theme == THEME_LIGHT) {
            return "light";
        }
        if(theme == THEME_DARK) {
            return "dark";
        }
        return "system";
    }

    /** The Theme value for a stored name, or -1 for one this build does not know. */
    static int themeFromName(String name) {
        if("system".equals(name)) {
            return THEME_SYSTEM;
        }
        if("light".equals(name)) {
            return THEME_LIGHT;
        }
        if("dark".equals(name)) {
            return THEME_DARK;
        }
        return -1;
    }

    /**
     * Reads the old hs_score_i / hs_date_i SharedPreferences keys, keeps only real
     * entries (score > 0, no "0" placeholders), writes them into the new store, and
     * removes the old keys so they are never read again.
     */
    private static HighScoresAndSettings migrate(Context context) {
        HighScoresAndSettings store = new HighScoresAndSettings();

        SharedPreferences prefs = PreferenceManager.getDefaultSharedPreferences(context);
        for(int i = 1; i <= MAX_SCORES; i++) {
            int score = 0;
            try {
                score = Integer.parseInt(prefs.getString("hs_score_" + i, "0"));
            }
            catch(Exception e) {
                e.printStackTrace();
            }

            long date = 0;
            try {
                date = Long.parseLong(prefs.getString("hs_date_" + i, "0"));
            }
            catch(Exception e) {
                e.printStackTrace();
            }

            if(score > 0) {
                HighScores.ScoreAndDate sad = new HighScores.ScoreAndDate();
                sad.score = score;
                sad.date = date;
                store.scores.add(sad);
            }
        }

        Collections.sort(store.scores, DESCENDING);
        store.save(context);

        Editor editor = prefs.edit();
        for(int i = 1; i <= MAX_SCORES; i++) {
            editor.remove("hs_score_" + i);
            editor.remove("hs_date_" + i);
        }
        editor.commit();

        return store;
    }

    private static void quarantine(File file) {
        File corrupt = new File(file.getParentFile(), file.getName() + ".corrupt-" + System.currentTimeMillis());
        if(!file.renameTo(corrupt)) {
            Log.w(TAG, "could not rename corrupt store file " + file);
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

    private static String formatDate(long date) {
        SimpleDateFormat format = new SimpleDateFormat(DATE_PATTERN, Locale.US);
        format.setTimeZone(TimeZone.getTimeZone("UTC"));
        return format.format(new Date(date));
    }

    // Also accepts a numeric playedAt (epoch millis), so a hand-edited file still loads.
    private static long parseDate(Object value) {
        if(value instanceof Number) {
            return ((Number) value).longValue();
        }

        if(value instanceof String) {
            try {
                SimpleDateFormat format = new SimpleDateFormat(DATE_PATTERN, Locale.US);
                format.setTimeZone(TimeZone.getTimeZone("UTC"));
                return format.parse((String) value).getTime();
            }
            catch(ParseException e) {
                e.printStackTrace();
            }
        }

        return 0;
    }
}
