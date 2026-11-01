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
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.text.DateFormat;
import java.util.Date;
import java.util.Locale;

import org.json.JSONException;

import android.app.Activity;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.net.Uri;
import android.os.Bundle;
import android.view.View;
import android.view.View.OnClickListener;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.TextView;
import android.widget.Toast;

public class HighScores extends Activity {

    private static final int[][] SCORE_TEXT_VIEWS = {
        { R.id.textView1_1, R.id.textView1_2 },
        { R.id.textView2_1, R.id.textView2_2 },
        { R.id.textView3_1, R.id.textView3_2 },
        { R.id.textView4_1, R.id.textView4_2 },
        { R.id.textView5_1, R.id.textView5_2 },
    };

    /** Intent extra naming the map whose list opens first; absent means the map in the settings. */
    public static final String EXTRA_MAP_ID = "mapId";

    private static final int REQUEST_EXPORT = 1;
    private static final int REQUEST_IMPORT = 2;

    /**
     * A picked file is read fully into memory before it is parsed, so a cap keeps a
     * multi-gigabyte wrong pick (a video, say) from exhausting the heap instead of just
     * failing cleanly. The real document is at most a few kilobytes.
     */
    private static final long MAX_IMPORT_BYTES = 1024 * 1024;

    /** The map whose list is on screen. */
    private MapDef shownMap = MapDef.DEFAULT;

    private View root;

    @Override
    public void onConfigurationChanged(android.content.res.Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        PixelUi.themeChanged(this, root);
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().setFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN, WindowManager.LayoutParams.FLAG_FULLSCREEN);

        super.onCreate(savedInstanceState);
        PixelUi.refreshTheme(this);
        setContentView(R.layout.high_scores);

        String requested = getIntent() == null ? null : getIntent().getStringExtra(EXTRA_MAP_ID);
        shownMap = MapDef.byId(requested != null ? requested : HighScoresAndSettings.load(this).map);
        // Built before skinScreen so the kit skins them with the rest, sizes in art px.
        buildMapToggle();

        int contentArt = PixelUi.skinScreen(this);
        root = ((ViewGroup) findViewById(android.R.id.content)).getChildAt(0);
        int half = (contentArt - 4) / 2;
        findViewById(R.id.high_scores_export).getLayoutParams().width = half * PixelUi.scale(this);
        findViewById(R.id.high_scores_import).getLayoutParams().width = half * PixelUi.scale(this);

        ((Button)findViewById(R.id.high_scores_ok)).setOnClickListener(new OnClickListener() {
            @Override
            public void onClick(View v) {
                finish();
            }
        });

        ((Button)findViewById(R.id.high_scores_export)).setOnClickListener(new OnClickListener() {
            @Override
            public void onClick(View v) {
                startExport();
            }
        });

        ((Button)findViewById(R.id.high_scores_import)).setOnClickListener(new OnClickListener() {
            @Override
            public void onClick(View v) {
                startImport();
            }
        });

        render();
    }

    /**
     * One radio button per map in a row, named by the map. With a single map there is
     * nothing to choose and the row is hidden.
     */
    private void buildMapToggle() {
        RadioGroup group = (RadioGroup) findViewById(R.id.high_scores_maps);
        if(MapDef.ALL.length < 2) {
            group.setVisibility(View.GONE);
            return;
        }

        for(MapDef map : MapDef.ALL) {
            PixelRadioButton button = new PixelRadioButton(this, null);
            button.setId(View.generateViewId());
            button.setText(mapName(this, map));
            button.setTag(map);
            button.setPadding(0, 2, 0, 2);
            group.addView(button, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1));
            if(map == shownMap) {
                group.check(button.getId());
            }
        }

        group.setOnCheckedChangeListener(new RadioGroup.OnCheckedChangeListener() {
            @Override
            public void onCheckedChanged(RadioGroup g, int checkedId) {
                View checked = g.findViewById(checkedId);
                if(checked != null && checked.getTag() instanceof MapDef) {
                    shownMap = (MapDef) checked.getTag();
                    render();
                }
            }
        });
    }

    /** The map's name as the player reads it; its id until a name string exists. */
    static String mapName(Context context, MapDef map) {
        return map.nameRes != 0 ? context.getString(map.nameRes) : map.id;
    }

    /** Lets the player pick where to write a copy of the current document. */
    private void startExport() {
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("application/json");
        intent.putExtra(Intent.EXTRA_TITLE, HighScoresAndSettings.FILE_NAME);
        startActivityForResult(intent, REQUEST_EXPORT);
    }

    /** Lets the player pick a previously exported document to merge in. */
    private void startImport() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("application/json");
        startActivityForResult(intent, REQUEST_IMPORT);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if(resultCode != RESULT_OK || data == null || data.getData() == null) {
            return;
        }

        Uri uri = data.getData();
        if(requestCode == REQUEST_EXPORT) {
            doExport(uri);
        }
        else if(requestCode == REQUEST_IMPORT) {
            doImport(uri);
        }
    }

    /** Writes the current store, unmodified, to the picked destination. */
    private void doExport(Uri uri) {
        HighScoresAndSettings store = HighScoresAndSettings.load(this);

        ContentResolver resolver = getContentResolver();
        try {
            OutputStream out = resolver.openOutputStream(uri);
            if(out == null) {
                throw new IOException("openOutputStream returned null");
            }
            try {
                out.write(store.toJson().getBytes("UTF-8"));
            }
            finally {
                out.close();
            }

            Toast.makeText(this, getResources().getString(R.string.high_scores_export_success), Toast.LENGTH_SHORT).show();
        }
        catch(IOException e) {
            e.printStackTrace();
            Toast.makeText(this, getResources().getString(R.string.high_scores_export_failed), Toast.LENGTH_LONG).show();
        }
    }

    /**
     * Reads the picked file, parses it with the store's own defensive parser, and merges
     * the scores of every map it contains into the current store, and takes over the
     * colour, haptics and map setting the file actually names (see
     * {@link HighScoresAndSettings#importFrom}). Any failure, at any step, leaves the
     * on-device store exactly as it was.
     */
    private void doImport(Uri uri) {
        String json;
        try {
            json = readUri(uri);
        }
        catch(IOException e) {
            e.printStackTrace();
            showImportFailed();
            return;
        }

        HighScoresAndSettings imported;
        try {
            imported = HighScoresAndSettings.fromJson(json);
        }
        catch(JSONException e) {
            e.printStackTrace();
            showImportFailed();
            return;
        }

        HighScoresAndSettings store = HighScoresAndSettings.load(this);
        int added = store.importFrom(imported);
        store.save(this);
        render();
        // A backup can carry the Theme setting: show it now
        PixelUi.refreshTheme(this);
        PixelUi.restyle(root);

        PixelDialog.message(this, R.string.high_scores_import_title, getResources().getString(R.string.high_scores_import_success, added));
    }

    private void showImportFailed() {
        PixelDialog.message(this, R.string.high_scores_import_failed_title, getResources().getString(R.string.high_scores_import_failed_message));
    }

    /**
     * Reads a picked document fully into a String, refusing anything past
     * MAX_IMPORT_BYTES rather than reading an arbitrarily large file into memory.
     */
    private String readUri(Uri uri) throws IOException {
        ContentResolver resolver = getContentResolver();
        InputStream in = resolver.openInputStream(uri);
        if(in == null) {
            throw new IOException("openInputStream returned null");
        }

        try {
            ByteArrayOutputStream buffer = new ByteArrayOutputStream();
            byte[] chunk = new byte[8192];
            long total = 0;
            int read;
            while((read = in.read(chunk)) >= 0) {
                total += read;
                if(total > MAX_IMPORT_BYTES) {
                    throw new IOException("file too large: over " + MAX_IMPORT_BYTES + " bytes");
                }
                buffer.write(chunk, 0, read);
            }
            return buffer.toString("UTF-8");
        }
        finally {
            in.close();
        }
    }

    /** Fills the five rows from the store, for the map on screen. Rows beyond the stored scores show "-". */
    public void render() {
        HighScoresAndSettings store = HighScoresAndSettings.load(this);
        java.util.List<ScoreAndDate> list = store.scoresFor(shownMap.id);

        for(int i = 0; i < SCORE_TEXT_VIEWS.length; i++) {
            int scoreViewId = SCORE_TEXT_VIEWS[i][0];
            int dateViewId = SCORE_TEXT_VIEWS[i][1];

            if(i < list.size()) {
                ScoreAndDate sad = list.get(i);
                ((TextView)findViewById(scoreViewId)).setText(formatScore(this, shownMap, sad.score));
                ((TextView)findViewById(dateViewId)).setText(formatDate(sad.date));
            }
            else {
                ((TextView)findViewById(scoreViewId)).setText(formatScore(0));
                ((TextView)findViewById(dateViewId)).setText(formatDate(0));
            }
        }
    }

    /** A result as the map's list shows it: turns where fewer is better, points otherwise. */
    public static String formatScore(Context context, MapDef map, int score) {
        if(score == 0 || !map.lowerIsBetter) {
            return formatScore(score);
        }

        return context.getString(R.string.turns_count, score);
    }

    public static String formatScore(int score) {
        if(score == 0) {
            return "-";
        }

        return "" + score;
    }

    public static String formatDate(long date) {
        if(date == 0) {
            return "-";
        }

        return DateFormat.getDateInstance(DateFormat.SHORT, Locale.getDefault()).format(new Date(date));
    }

    /** Records a result on the default map. */
    public static int score(Activity activity, int score) {
        return score(activity, MapDef.DEFAULT.id, score);
    }

    /**
     * Records a result on one map: the points scored where higher is better, the turns
     * taken where lower is. Each map keeps its own list, so a result never lands in another's.
     * Returns the 1-based rank it took in that list, or 0 if it did not make it; rank 1 is the
     * new best, which is what the toast and the fanfare say.
     */
    public static int score(Activity activity, String mapId, int score) {
        HighScoresAndSettings store = HighScoresAndSettings.load(activity);

        int rank = store.addScore(mapId, score);
        if(rank > 0) {
            store.save(activity);
        }

        if(rank == 1) {
            Toast.makeText(activity, activity.getResources().getString(R.string.high_scores_toast), Toast.LENGTH_SHORT).show();
        }
        else if(rank >= 2) {
            Toast.makeText(activity, activity.getResources().getString(R.string.high_scores_top_5_toast), Toast.LENGTH_SHORT).show();
        }
        return rank;
    }

    public static class ScoreAndDate
    {
        public int score;
        public long date;
    }
}
