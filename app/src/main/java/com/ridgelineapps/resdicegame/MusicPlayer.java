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

import java.lang.reflect.Field;
import java.util.Random;
import java.util.TreeMap;

import android.app.Activity;
import android.app.Application;
import android.content.Context;
import android.content.res.AssetFileDescriptor;
import android.content.res.Resources;
import android.media.AudioAttributes;
import android.media.AudioFocusRequest;
import android.media.AudioManager;
import android.media.MediaPlayer;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.util.Log;

/**
 * The background soundtrack: the tracks in res/raw named music_* (any audio format MediaPlayer plays), played one after another
 * in a shuffled order that changes each launch (MusicPlaylist), forever.
 *
 * One instance for the whole process, made by GameApp, so the music runs across the game's
 * screens (the game, Top Scores, About and Rules are separate Activities) and an Activity
 * being recreated cannot leak or restart it: nothing here holds an Activity.
 *
 * <b>When the app as a whole has left the foreground.</b> Moving between the game's own
 * Activities must not interrupt the music, and neither must the brief gap of a recreation,
 * but Home, the app switcher or the screen locking must stop it. Plain Activity lifecycle
 * callbacks answer both: a count of started Activities. Going from one Activity to another
 * starts the new one before the old one stops, so the count never reaches zero in a
 * transition; it reaches zero only when no Activity of this app is visible. Even then a
 * short debounce (BACKGROUND_DEBOUNCE_MS) waits and cancels itself if an Activity starts in
 * the meantime, which covers an Activity finishing just before another opens and an
 * Activity being recreated (the old one stops before the new one starts).
 *
 * <b>Audio focus.</b> Music takes AUDIOFOCUS_GAIN when it starts and gives it back when it
 * stops, at the background, at volume 0 and at the end of the app. A permanent loss (another
 * music app starts, which takes focus) pauses the music and releases the player; it is not
 * resumed on its own, only when the app next comes back to the foreground. A transient loss
 * (a call, a navigation prompt) pauses and resumes with a fade when focus returns. A "may
 * duck" loss lowers the music to DUCK_FACTOR of its gain and restores it on gain. When the
 * game comes to the foreground, or music is wanted again, and some other app is already
 * playing music, nothing starts: a game should not talk over the player's own music. Moving
 * the Music volume slider is the one explicit request for the game's music, and starts it
 * regardless.
 *
 * <b>Loudness.</b> The sound is the system media volume (hardware keys, STREAM_MUSIC) times
 * the MediaPlayer gain, which is MusicGain.gainFor(setting) times the fade level times the
 * duck factor. The app never touches the stream volume itself.
 *
 * At volume 0 there is no player and no focus held. Everything runs on the main thread.
 */
public final class MusicPlayer {

    private static final String TAG = "MusicPlayer";

    /** Every res/raw file whose name starts with this is a track, whatever its extension. */
    static final String TRACK_PREFIX = "music_";

    static final int BACKGROUND_DEBOUNCE_MS = 600;
    static final int FADE_IN_MS = 1000;
    static final int FADE_OUT_MS = 500;
    static final float DUCK_FACTOR = 0.25f;
    private static final int FADE_TICK_MS = 40;

    private static MusicPlayer instance;

    /** Makes the one player, starts following the app's Activities and reads the stored volume. */
    static void install(Application app) {
        instance = new MusicPlayer(app);
        app.registerActivityLifecycleCallbacks(instance.new Lifecycle());
        instance.setVolumeSetting(HighScoresAndSettings.load(app).musicVolume, false);
    }

    /** The player; null before GameApp has run (never in the running app). */
    static MusicPlayer get() {
        return instance;
    }

    private final Context context;
    private final Resources resources;
    private final AudioManager audio;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final AudioAttributes attributes;
    /** The res/raw ids of the tracks this build actually contains, in table order. */
    private final int[] trackIds;
    private final String[] trackNames;
    private final MusicPlaylist playlist;

    private int volume = HighScoresAndSettings.MUSIC_VOLUME_DEFAULT;
    private boolean foreground;
    /** Set by an explicit use of the slider: play even though another app is playing music. */
    private boolean userRequested;
    /** Another app took focus for good; stay quiet until the app next comes to the foreground. */
    private boolean focusLostForGood;

    private MediaPlayer player;
    private boolean prepared;
    private boolean fadeInWhenPrepared;
    private boolean shuttingDown;
    private int currentTrack = -1;
    private int resumePosition;
    private int failures;

    private boolean focusHeld;
    private boolean transientPaused;
    private float duck = 1f;
    private AudioFocusRequest focusRequest;

    private float level = 1f;
    private float fadeFrom;
    private float fadeTarget;
    private long fadeStartedAt;
    private int fadeMillis;
    private Runnable fadeDone;

    private final AudioManager.OnAudioFocusChangeListener focusListener = new AudioManager.OnAudioFocusChangeListener() {
        @Override
        public void onAudioFocusChange(int change) {
            onFocusChange(change);
        }
    };

    private final Runnable fadeTick = new Runnable() {
        @Override
        public void run() {
            long elapsed = SystemClock.uptimeMillis() - fadeStartedAt;
            float progress = fadeMillis <= 0 ? 1f : Math.min(1f, elapsed / (float) fadeMillis);
            level = fadeFrom + (fadeTarget - fadeFrom) * progress;
            applyVolume();
            if (progress >= 1f) {
                Runnable done = fadeDone;
                fadeDone = null;
                if (done != null) {
                    done.run();
                }
            }
            else {
                handler.postDelayed(this, FADE_TICK_MS);
            }
        }
    };

    private MusicPlayer(Application app) {
        context = app;
        resources = app.getResources();
        audio = (AudioManager) app.getSystemService(Context.AUDIO_SERVICE);
        attributes = new AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build();

        // The tracks are whatever music_* files are in res/raw: found through the generated R
        // class, so adding, dropping or switching the format of a track (.ogg, .flac, .mp3)
        // needs no change here. A build with none simply has no music.
        TreeMap<String, Integer> found = new TreeMap<String, Integer>();
        for (Field field : R.raw.class.getFields()) {
            if (field.getName().startsWith(TRACK_PREFIX) && field.getType() == int.class) {
                try {
                    found.put(field.getName(), field.getInt(null));
                }
                catch (IllegalAccessException e) {
                    Log.w(TAG, "cannot read " + field.getName(), e);
                }
            }
        }
        trackIds = new int[found.size()];
        trackNames = new String[found.size()];
        int n = 0;
        for (java.util.Map.Entry<String, Integer> entry : found.entrySet()) {
            trackNames[n] = entry.getKey();
            trackIds[n++] = entry.getValue();
        }
        playlist = new MusicPlaylist(trackIds.length, new Random());
    }

    // ---------------------------------------------------------------------------------
    // What the rest of the app calls
    // ---------------------------------------------------------------------------------

    /**
     * Sets the Music volume, 0 to 100. userAction is true when the player moved the slider:
     * that starts the music even when another app is playing. 0 stops it and lets go of
     * focus. Cheap to call again with the same value; MainActivity does on every return.
     */
    void setVolumeSetting(int setting, boolean userAction) {
        int before = volume;
        volume = HighScoresAndSettings.clampMusicVolume(setting);
        if (volume != before) {
            Log.i(TAG, "volume setting " + before + " -> " + volume + (userAction ? " (slider)" : ""));
        }
        if (volume == 0) {
            userRequested = false;
        }
        else if (userAction) {
            userRequested = true;
        }
        reconcile();
        applyVolume();
    }

    /** One line saying what is playing and where; for the log, so a transition can be checked. */
    void logState(String why) {
        int pos = player != null && prepared ? safePosition() : resumePosition;
        Log.i(TAG, why + ": track=" + (currentTrack >= 0 ? trackNames[currentTrack] : "none")
                + " pos=" + pos + " player=" + (player != null) + " playing=" + isPlaying()
                + " setting=" + volume + " gain=" + gain() + " focus=" + focusHeld
                + " foreground=" + foreground);
    }

    boolean isPlaying() {
        try {
            return player != null && prepared && player.isPlaying();
        }
        catch (IllegalStateException e) {
            return false;
        }
    }

    // ---------------------------------------------------------------------------------
    // Foreground and background
    // ---------------------------------------------------------------------------------

    private void appForeground() {
        foreground = true;
        focusLostForGood = false;
        logState("foreground");
        reconcile();
        SoundEffects.onAppForeground(true);
    }

    private void appBackground() {
        foreground = false;
        userRequested = false;
        logState("background");
        reconcile();
        SoundEffects.onAppForeground(false);
    }

    /** Follows the count of started Activities; see the class comment. */
    private final class Lifecycle implements Application.ActivityLifecycleCallbacks {

        private int started;

        private final Runnable backgroundCheck = new Runnable() {
            @Override
            public void run() {
                if (started == 0) {
                    appBackground();
                }
            }
        };

        @Override
        public void onActivityCreated(Activity activity, Bundle savedInstanceState) {
            // The hardware volume keys change the media volume, which is the music's, while
            // any screen of the game is up
            activity.setVolumeControlStream(AudioManager.STREAM_MUSIC);
        }

        @Override
        public void onActivityStarted(Activity activity) {
            started++;
            handler.removeCallbacks(backgroundCheck);
            if (started == 1) {
                appForeground();
            }
        }

        @Override
        public void onActivityResumed(Activity activity) {
            logState("resumed " + activity.getClass().getSimpleName());
        }

        @Override
        public void onActivityPaused(Activity activity) {
        }

        @Override
        public void onActivityStopped(Activity activity) {
            started = Math.max(0, started - 1);
            if (started == 0) {
                handler.postDelayed(backgroundCheck, BACKGROUND_DEBOUNCE_MS);
            }
        }

        @Override
        public void onActivitySaveInstanceState(Activity activity, Bundle outState) {
        }

        @Override
        public void onActivityDestroyed(Activity activity) {
        }
    }

    // ---------------------------------------------------------------------------------
    // Starting and stopping
    // ---------------------------------------------------------------------------------

    /** Brings the player in line with what is wanted: in the foreground, volume above 0, tracks to play. */
    private void reconcile() {
        boolean want = foreground && volume > 0 && trackIds.length > 0 && !focusLostForGood;
        if (want) {
            if (player == null) {
                tryStart();
            }
            else if (shuttingDown) {
                // Back in the foreground, or the slider raised, during the fade out
                shuttingDown = false;
                fadeTo(1f, FADE_IN_MS, null);
            }
        }
        else if (player != null && !shuttingDown) {
            shutDown();
        }
    }

    private void tryStart() {
        if (!userRequested && audio.isMusicActive()) {
            // Another app is playing: a game does not talk over it
            logState("not starting, another app is playing");
            return;
        }
        if (!requestFocus()) {
            logState("not starting, focus refused");
            return;
        }
        failures = 0;
        level = 0f;
        duck = 1f;
        int track = currentTrack >= 0 ? currentTrack : playlist.next();
        openTrack(track, resumePosition, true);
    }

    private void openTrack(int track, int position, boolean fadeIn) {
        MediaPlayer p = new MediaPlayer();
        try {
            p.setAudioAttributes(attributes);
            AssetFileDescriptor fd = resources.openRawResourceFd(trackIds[track]);
            try {
                p.setDataSource(fd.getFileDescriptor(), fd.getStartOffset(), fd.getLength());
            }
            finally {
                fd.close();
            }
            final MediaPlayer opened = p;
            p.setOnPreparedListener(new MediaPlayer.OnPreparedListener() {
                @Override
                public void onPrepared(MediaPlayer mp) {
                    if (opened == player) {
                        onTrackPrepared();
                    }
                }
            });
            p.setOnCompletionListener(new MediaPlayer.OnCompletionListener() {
                @Override
                public void onCompletion(MediaPlayer mp) {
                    if (opened == player) {
                        onTrackCompleted();
                    }
                }
            });
            p.setOnErrorListener(new MediaPlayer.OnErrorListener() {
                @Override
                public boolean onError(MediaPlayer mp, int what, int extra) {
                    if (opened == player) {
                        onTrackError(what, extra);
                    }
                    return true;
                }
            });
            player = p;
            prepared = false;
            currentTrack = track;
            resumePosition = position;
            fadeInWhenPrepared = fadeIn;
            p.prepareAsync();
            logState("opening");
        }
        catch (Exception e) {
            Log.w(TAG, "could not open " + trackNames[track], e);
            p.release();
            player = null;
            prepared = false;
            onOpenFailed();
        }
    }

    private void onTrackPrepared() {
        prepared = true;
        failures = 0;
        if (resumePosition > 0) {
            player.seekTo(resumePosition);
        }
        applyVolume();
        if (!transientPaused) {
            player.start();
            if (fadeInWhenPrepared) {
                level = 0f;
                applyVolume();
                fadeTo(1f, FADE_IN_MS, null);
            }
        }
        logState("started");
    }

    /** A track played to its end: the next one, straight away. */
    private void onTrackCompleted() {
        logState("completed");
        releasePlayer(false);
        currentTrack = -1;
        resumePosition = 0;
        if (foreground && volume > 0 && !shuttingDown) {
            openTrack(playlist.next(), 0, false);
        }
        else {
            abandonFocus();
        }
    }

    private void onTrackError(int what, int extra) {
        Log.w(TAG, "playback error " + what + "/" + extra + " in track " + (currentTrack >= 0 ? trackNames[currentTrack] : "none"));
        releasePlayer(false);
        currentTrack = -1;
        resumePosition = 0;
        onOpenFailed();
    }

    /** One track would not play; try the next, and give up once every track has failed in a row. */
    private void onOpenFailed() {
        failures++;
        if (failures <= trackIds.length && foreground && volume > 0) {
            openTrack(playlist.next(), 0, false);
        }
        else {
            Log.w(TAG, "giving up, no track would play");
            abandonFocus();
        }
    }

    /** Fades out, then stops, keeps the place and lets go of focus. */
    private void shutDown() {
        shuttingDown = true;
        if (!prepared || transientPaused || gain() == 0f || level == 0f) {
            finishShutdown();
        }
        else {
            fadeTo(0f, FADE_OUT_MS, new Runnable() {
                @Override
                public void run() {
                    finishShutdown();
                }
            });
        }
    }

    private void finishShutdown() {
        if (player != null && prepared) {
            resumePosition = safePosition();
        }
        releasePlayer(true);
        logState("stopped");
    }

    private void releasePlayer(boolean abandon) {
        handler.removeCallbacks(fadeTick);
        fadeDone = null;
        MediaPlayer p = player;
        player = null;
        prepared = false;
        shuttingDown = false;
        transientPaused = false;
        if (p != null) {
            try {
                p.setOnPreparedListener(null);
                p.setOnCompletionListener(null);
                p.setOnErrorListener(null);
                p.release();
            }
            catch (RuntimeException e) {
                Log.w(TAG, "release", e);
            }
        }
        if (abandon) {
            abandonFocus();
        }
    }

    private int safePosition() {
        try {
            return player.getCurrentPosition();
        }
        catch (IllegalStateException e) {
            return resumePosition;
        }
    }

    // ---------------------------------------------------------------------------------
    // Audio focus
    // ---------------------------------------------------------------------------------

    private boolean requestFocus() {
        if (focusHeld) {
            return true;
        }
        int result;
        if (Build.VERSION.SDK_INT >= 26) {
            focusRequest = new AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                    .setAudioAttributes(attributes)
                    .setOnAudioFocusChangeListener(focusListener, handler)
                    .build();
            result = audio.requestAudioFocus(focusRequest);
        }
        else {
            result = audio.requestAudioFocus(focusListener, AudioManager.STREAM_MUSIC, AudioManager.AUDIOFOCUS_GAIN);
        }
        focusHeld = result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED;
        return focusHeld;
    }

    private void abandonFocus() {
        if (!focusHeld) {
            return;
        }
        focusHeld = false;
        if (Build.VERSION.SDK_INT >= 26) {
            if (focusRequest != null) {
                audio.abandonAudioFocusRequest(focusRequest);
            }
        }
        else {
            audio.abandonAudioFocus(focusListener);
        }
    }

    private void onFocusChange(int change) {
        Log.i(TAG, "focus change " + change);
        if (change == AudioManager.AUDIOFOCUS_GAIN) {
            duck = 1f;
            if (transientPaused && player != null) {
                transientPaused = false;
                if (prepared) {
                    level = 0f;
                    applyVolume();
                    player.start();
                    fadeTo(1f, FADE_IN_MS, null);
                }
            }
            applyVolume();
        }
        else if (change == AudioManager.AUDIOFOCUS_LOSS) {
            // Someone else is the music now. Let go, keep the place, and wait for the app to
            // come back to the foreground before trying again.
            focusLostForGood = true;
            focusHeld = false;
            if (player != null) {
                if (prepared) {
                    resumePosition = safePosition();
                }
                releasePlayer(false);
            }
        }
        else if (change == AudioManager.AUDIOFOCUS_LOSS_TRANSIENT) {
            if (player != null) {
                transientPaused = true;
                if (prepared && isPlaying()) {
                    player.pause();
                }
            }
        }
        else if (change == AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK) {
            duck = DUCK_FACTOR;
            applyVolume();
        }
        logState("focus " + change);
    }

    // ---------------------------------------------------------------------------------
    // Loudness
    // ---------------------------------------------------------------------------------

    private float gain() {
        return MusicGain.gainFor(volume);
    }

    /** The MediaPlayer gain: the setting's curve, times the fade, times the duck. */
    private void applyVolume() {
        if (player != null && prepared) {
            float v = gain() * level * duck;
            try {
                player.setVolume(v, v);
            }
            catch (IllegalStateException e) {
                Log.w(TAG, "setVolume", e);
            }
        }
    }

    private void fadeTo(float target, int millis, Runnable done) {
        handler.removeCallbacks(fadeTick);
        fadeFrom = level;
        fadeTarget = target;
        fadeStartedAt = SystemClock.uptimeMillis();
        fadeMillis = millis;
        fadeDone = done;
        fadeTick.run();
    }
}
