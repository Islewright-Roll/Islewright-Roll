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

import java.lang.reflect.Method;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.content.res.Configuration;
import android.os.Bundle;
import android.view.KeyEvent;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

public class MainActivity extends Activity {

    // The class name of the debug-only scenario loader, referenced by string rather
    // than by class literal so this file compiles unchanged whether or not that loader
    // is in the build: it lives in app/src/debug/java and is absent from a release
    // build, along with the debug-scenario.json format it reads. See DebugScenario and
    // DebugScenarioLoader for what it does.
    private static final String DEBUG_SCENARIO_LOADER = "com.ridgelineapps.resdicegame.DebugScenarioLoader";

    /**
     * Whether the welcome dialog has already been offered since this app was launched.
     * Static, so an Activity recreated in the same process (the OS can do that for reasons
     * of its own) does not show it again; cleared when the Activity really finishes, so
     * the next launch is a new start. A process that was killed and restored gets a saved
     * state Bundle instead, which onCreate also treats as "not a launch".
     */
    private static boolean welcomeOffered = false;

    /**
     * True from the moment the welcome dialog is scheduled until it is on screen, so the
     * tutorial's first callout does not show for a frame under it.
     */
    static boolean welcomePending = false;

    Game game;

    /**
     * The in-context tutorial's progress, shared by both islands. Its settings are written
     * back whenever it changes (saveTutorial) and read again from the file when they may
     * have changed elsewhere (applySettings).
     */
    Tutorial tutorial = new Tutorial(true, 0);


    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (android.os.Build.VERSION.SDK_INT < 11) {
            this.requestWindowFeature(Window.FEATURE_NO_TITLE);
        }
        setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        this.getWindow().setFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN, WindowManager.LayoutParams.FLAG_FULLSCREEN);
        // Each island keeps its own saved game, so the Map setting decides which one opens.
        HighScoresAndSettings settings = HighScoresAndSettings.load(this);
        tutorial = new Tutorial(settings.tutorial, settings.tutorialSeen);
        MapDef map = MapDef.forLaunch(settings.map);
        game = gameFor(map);
        rememberMap(map);
        applySettings();
        PixelUi.refreshTheme(this);
        showGame();
        offerWelcome(savedInstanceState, settings);
    }

    /** Shows the welcome dialog once per launch, when the setting is on. */
    private void offerWelcome(Bundle savedInstanceState, HighScoresAndSettings settings) {
        if (savedInstanceState != null || welcomeOffered || !settings.showWelcome) {
            return;
        }
        welcomeOffered = true;
        welcomePending = true;
        // Once the first frame is up, so the dialog opens over the game and not over a blank window
        getWindow().getDecorView().post(new Runnable() {
            @Override
            public void run() {
                if (!isFinishing() && !isDestroyed()) {
                    WelcomeDialog.show(MainActivity.this, game);
                }
                // The dialog is shown by now (and counted by PixelDialog.isOpen), or never will be
                welcomePending = false;
            }
        });
    }

    @Override
    protected void onDestroy() {
        if (isFinishing()) {
            welcomeOffered = false;
            welcomePending = false;
        }
        super.onDestroy();
    }

    /**
     * The OS night mode is the only configuration change this Activity handles itself
     * (see the manifest), so that Theme "System" can follow it without the game being
     * rebuilt: the game, the view and any open dialog stay as they are, and the dialogs
     * are repainted in place.
     */
    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        PixelUi.themeChanged(this, null);
    }

    /** Builds the view for the game as it now is and puts it on screen. */
    private void showGame() {
        GameView gameView = new GameView(this, game);
        game.setGameView(gameView);
        setContentView(gameView);
        // A view that replaces another has to ask for the insets the first one was given
        gameView.requestApplyInsets();
    }

    /** Writes the Map setting, only when it would change, so it always names the live game's map. */
    private void rememberMap(MapDef map) {
        HighScoresAndSettings settings = HighScoresAndSettings.load(this);
        if (!map.id.equals(settings.map)) {
            settings.map = map.id;
            settings.mapWasSpecified = true;
            settings.save(this);
        }
    }

    /** The saved game of this island, or a fresh one when it has none. */
    private Game gameFor(MapDef map) {
        Game saved = GameStateStore.load(this, map);
        return saved != null ? saved : new Game(map);
    }

    /**
     * Moves to the given island and shows it, keeping the game that was on screen. That game
     * is written to its island's file first (Game.changed already does after every action;
     * this makes sure nothing is pending), then the other island's saved game is loaded, or
     * a fresh one started when it has none, and the Map setting follows. The GameView is
     * built again rather than patched: its piece lists and cached layers are sized for one
     * map at construction. Returns the game now on screen, which is a different object from
     * the one before, so a caller must not go on using the old one.
     *
     * Public for the debug scenario loader, which lives in the debug source set.
     */
    public Game switchMap(MapDef map) {
        if (map == game.map) {
            return game;
        }
        GameStateStore.persist(this, game);
        game = gameFor(map);
        applySettings();
        showGame();
        rememberMap(map);
        return game;
    }

    @Override
    protected void onResume() {
        super.onResume();
        // Settings can change somewhere other than this screen, notably by importing a
        // backup from Top Scores, so re-read them whenever this returns to the front
        // rather than only when a picker reports back.
        applySettings();
        // The Theme can change behind this screen too: an import on Top Scores, or the OS
        PixelUi.refreshTheme(this);
        followImportedMap();
        if (game.gameView != null) {
            game.gameView.postInvalidate();
        }
    }

    /**
     * Importing a backup on Top Scores can bring a Map setting with it. The setting and
     * the live game must not disagree, so the game moves to the imported island the same
     * way the menu's island entry does it: this island's game is kept and the other's restored.
     */
    private void followImportedMap() {
        MapDef wanted = MapDef.byId(HighScoresAndSettings.load(this).map);
        if (wanted != game.map) {
            switchMap(wanted);
        }
    }

    /**
     * The last callback guaranteed to run before Android can kill this process in the
     * background, so this is where the in-progress game is saved rather than onStop or
     * onDestroy, neither of which is guaranteed to run at all. A finished game clears its
     * own island's save instead of writing it, so relaunching after the game-over screen
     * starts a fresh game rather than resurrecting the one that just ended.
     */
    @Override
    protected void onPause() {
        super.onPause();
        if (game.gameView != null) {
            game.gameView.endAnimations();
        }
        GameStateStore.persist(this, game);
    }

    /** Writes the tutorial's progress, only when it has changed. Called by the game screen. */
    void saveTutorial() {
        HighScoresAndSettings settings = HighScoresAndSettings.load(this);
        settings.tutorial = tutorial.isActive();
        settings.tutorialSeen = tutorial.seenMask();
        settings.save(this);
    }

    /** Copies the persisted settings onto the live game. */
    private void applySettings() {
        HighScoresAndSettings settings = HighScoresAndSettings.load(this);
        // Also picks up the Tutorial setting from the Settings dialog or an imported backup
        tutorial.configure(settings.tutorial, settings.tutorialSeen);
        game.playerColor = settings.playerColor;
        game.earlyBuildPrompt = settings.earlyBuildPrompt;
        game.endTurnPrompt = settings.endTurnPrompt;
        game.setTraderClassic(settings.traderClassic);
        // Also picks up a target imported from a backup; it moves a game in progress only
        // as far as Game.setRaceTarget allows
        game.setRaceTarget(settings.raceTarget);
        Haptics.enabled = settings.hapticFeedback;
        // Also picks up a sound effects choice imported from a backup on Top Scores
        SoundEffects effects = SoundEffects.get();
        if (effects != null) {
            effects.setEnabled(settings.soundEffects);
        }
        // Also picks up a volume imported from a backup on Top Scores, and gives the music
        // another chance to start if it was held back because another app was playing
        MusicPlayer music = MusicPlayer.get();
        if (music != null) {
            music.setVolumeSetting(settings.musicVolume, false);
        }
    }

    /**
     * Opens the menu: a PixelDialog listing the same entries the action bar menu had, with
     * the way to the other island second, named for that island. Debug Scenario is listed
     * only when isDebugHarnessAvailable(). Called by the in-canvas menu button, and by the
     * hardware menu key.
     */
    public void showMenu() {
        int[] items;
        if (isDebugHarnessAvailable()) {
            items = new int[] { R.string.restart_title, R.string.menu_go_to_island, R.string.settings_menu, R.string.rules_menu,
                    R.string.high_scores_menu, R.string.about_menu, R.string.debug_scenario_menu };
        }
        else {
            items = new int[] { R.string.restart_title, R.string.menu_go_to_island, R.string.settings_menu, R.string.rules_menu,
                    R.string.high_scores_menu, R.string.about_menu };
        }
        final int[] menuItems = items;
        CharSequence[] labels = new CharSequence[items.length];
        for (int i = 0; i < items.length; i++) {
            labels[i] = items[i] == R.string.menu_go_to_island
                    ? getString(R.string.menu_go_to_island, getString(WelcomeDialog.otherMap(game.map).nameRes))
                    : getText(items[i]);
        }
        PixelDialog.choose(this, R.string.menu_title, labels, new PixelDialog.OnChoice() {
            @Override
            public void onChoice(int index) {
                onMenuChoice(menuItems[index]);
            }
        });
    }

    @Override
    public boolean onKeyUp(int keyCode, KeyEvent event) {
        if (keyCode == KeyEvent.KEYCODE_MENU) {
            showMenu();
            return true;
        }
        return super.onKeyUp(keyCode, event);
    }

    /**
     * True only when DebugScenarioLoader was actually compiled into this build. A
     * release build does not have the class at all (it lives in the debug source set),
     * so this is always false there, and the menu item stays hidden with nothing
     * behind it.
     */
    private static boolean isDebugHarnessAvailable() {
        try {
            Class.forName(DEBUG_SCENARIO_LOADER);
            return true;
        }
        catch (ClassNotFoundException e) {
            return false;
        }
    }

    /**
     * Reached only through the Debug Scenario menu entry, which is only ever listed when
     * isDebugHarnessAvailable() is true, so this reflection normally cannot fail. The
     * catch is defence in depth, not an expected path: DEBUG_SCENARIO_LOADER not
     * resolving here would mean either a release build (menu item already hidden) or a
     * broken debug build, and either way there is nothing better to do than say so.
     */
    private void applyDebugScenario() {
        try {
            Class<?> loaderClass = Class.forName(DEBUG_SCENARIO_LOADER);
            Method method = loaderClass.getMethod("applyFromFile", Context.class, Game.class);
            method.invoke(null, this, game);
        }
        catch (Exception e) {
            Toast.makeText(this, "Debug scenario harness unavailable: " + e.getMessage(), Toast.LENGTH_LONG).show();
        }
    }

    /** Routes one menu entry, named by its string resource. Same branches as the old options menu. */
    private void onMenuChoice(int item) {
        // Resource IDs are not compile-time constants under modern AGP, so these cannot be
        // switch labels any more. Same branches, same order.
        if (item == R.string.restart_title) {
            PixelDialog.confirm(this, R.string.restart_title, R.string.restart_text, new Runnable() {
                @Override
                public void run() {
                    game.reset();
                }
            });
        }
        else if (item == R.string.menu_go_to_island) {
            // Each island keeps its own game, so nothing is lost and nothing needs confirming
            switchMap(WelcomeDialog.otherMap(game.map));
        }
        else if (item == R.string.settings_menu) {
            HighScoresAndSettings settings = HighScoresAndSettings.load(this);
            SettingsDialog.show(this, settings.earlyBuildPrompt, settings.endTurnPrompt, settings.hapticFeedback, new SettingsDialog.OnSettingsChosen() {
                @Override
                public void onSettingsChosen(int earlyBuildPrompt, boolean endTurnPrompt, boolean hapticFeedback, int raceTarget) {
                    game.earlyBuildPrompt = earlyBuildPrompt;
                    game.endTurnPrompt = endTurnPrompt;
                    Haptics.enabled = hapticFeedback;
                    game.setRaceTarget(raceTarget);
                    // The dialog may have switched the tutorial on or off, or started it over
                    HighScoresAndSettings saved = HighScoresAndSettings.load(MainActivity.this);
                    game.setTraderClassic(saved.traderClassic);
                    tutorial.configure(saved.tutorial, saved.tutorialSeen);
                    // Nothing else redraws the game once the dialog is gone, and a callout may be due
                    if (game.gameView != null) {
                        game.gameView.postInvalidate();
                    }
                }
            }, new PlayerColorPickerDialog.OnColorChosen() {
                @Override
                public void onColorChosen(int color) {
                    game.playerColor = color;
                    if (game.gameView != null) {
                        game.gameView.postInvalidate();
                    }
                }
            });
        }
        else if (item == R.string.about_menu) {
            startActivity(new Intent(MainActivity.this, About.class));
        }
        else if (item == R.string.rules_menu) {
            startActivity(new Intent(MainActivity.this, Rules.class));
        }
        else if (item == R.string.high_scores_menu) {
            // Top Scores opens on the list of the map being played
            Intent scores = new Intent(MainActivity.this, HighScores.class);
            scores.putExtra(HighScores.EXTRA_MAP_ID, game.map.id);
            startActivity(scores);
        }
        else if (item == R.string.debug_scenario_menu) {
            applyDebugScenario();
        }
    }

}
