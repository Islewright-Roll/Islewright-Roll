package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import javax.xml.parsers.DocumentBuilderFactory;

import org.junit.Test;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

/**
 * Every translation, checked against the English it stands in for and against what the
 * screen can hold: the same strings as German, the same placeholders as English, the
 * length each fixed-width label has room for, the same rules markup line by line, and
 * no character the baked font atlases lack (BitmapFont draws a missing glyph as '?').
 */
public class LocalizationTest {

   private static final File RES = new File("src/main/res");

   /** The languages that must be there, by resource qualifier. "in" is Indonesian. */
   private static final List<String> LANGUAGES = Arrays.asList("de", "es", "fr", "in", "it", "pt");

   /** The roll button, in the 8 px display font. */
   private static final List<String> ROLL_BUTTON = Arrays.asList(
         "first_roll", "second_roll", "third_roll", "next_turn", "done", "finish", "play_again");
   /** Dialog buttons that share a row with another. */
   private static final List<String> PAIRED_BUTTONS = Arrays.asList(
         "yes", "no", "ok_button", "settings_cancel", "welcome_how_to_play", "welcome_play",
         "high_scores_export_button", "high_scores_import_button");
   private static final List<String> MENU_ENTRIES = Arrays.asList(
         "restart_title", "menu_go_to_island", "settings_menu", "rules_menu", "high_scores_menu",
         "about_menu", "debug_scenario_menu");
   private static final List<String> DIALOG_TITLES = Arrays.asList(
         "restart_title", "build_early_title", "build_second_pair_title", "end_turn_title", "menu_title", "player_color_title",
         "rules_title", "high_scores_title", "high_scores_import_title", "high_scores_import_failed_title",
         "settings_title", "welcome_title", "trader_choose_title");

   private static final int ROLL_BUTTON_MAX = 10;
   private static final int PAIRED_BUTTON_MAX = 10;
   private static final int MENU_ENTRY_MAX = 22;
   private static final int DIALOG_TITLE_MAX = 22;
   private static final int CALLOUT_MAX = 110;
   private static final int GOT_IT_MAX = 12;
   private static final int SKIP_MAX = 22;

   /**
    * German strings that were longer than these limits before the limits were written
    * down, each held at the length it has now. Shorten one and take it out of here.
    */
   private static final Map<String, Integer> GERMAN_AS_SHIPPED = new HashMap<String, Integer>();
   static {
      GERMAN_AS_SHIPPED.put("welcome_how_to_play", 11);
      GERMAN_AS_SHIPPED.put("high_scores_export_button", 11);
      GERMAN_AS_SHIPPED.put("high_scores_import_button", 11);
      GERMAN_AS_SHIPPED.put("welcome_title", 25);
      GERMAN_AS_SHIPPED.put("tutorial_bank", 114);
   }

   private static final Pattern PLACEHOLDER = Pattern.compile("%(?:(\\d+)\\$)?([a-zA-Z])");
   private static final Pattern ICON_LINE = Pattern.compile("\\[([a-z0-9_]+)\\] \\S.*");
   private static final Pattern IMAGE_LINE = Pattern.compile("!\\[([a-z0-9_]+)\\]");
   private static final Pattern UNICODE_ESCAPE = Pattern.compile("\\\\u([0-9a-fA-F]{4})");

   // ---------------------------------------------------------------- reading

   /** A strings.xml as name to text, with Android's escapes resolved to the characters drawn. */
   private static Map<String, String> strings(String qualifier) throws Exception {
      File file = new File(RES, (qualifier == null ? "values" : "values-" + qualifier) + "/strings.xml");
      assertTrue(file + " is missing", file.isFile());
      NodeList nodes = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file).getElementsByTagName("string");
      Map<String, String> out = new LinkedHashMap<String, String>();
      for(int i = 0; i < nodes.getLength(); i++) {
         Element e = (Element) nodes.item(i);
         out.put(e.getAttribute("name"), unescape(e.getTextContent()));
      }
      return out;
   }

   private static String unescape(String s) {
      Matcher m = UNICODE_ESCAPE.matcher(s);
      StringBuffer sb = new StringBuffer();
      while(m.find()) {
         m.appendReplacement(sb, Matcher.quoteReplacement(String.valueOf((char) Integer.parseInt(m.group(1), 16))));
      }
      m.appendTail(sb);
      return sb.toString().replace("\\'", "'").replace("\\\"", "\"").replace("\\n", "\n");
   }

   private static List<String> rules(String qualifier) throws Exception {
      File file = new File(RES, (qualifier == null ? "raw" : "raw-" + qualifier) + "/rules.txt");
      assertTrue(file + " is missing", file.isFile());
      String text = new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
      assertFalse(file + " has carriage returns", text.contains("\r"));
      return Arrays.asList(text.split("\n", -1));
   }

   /** The code points a baked atlas holds: the first field of each line after the first. */
   private static Set<Integer> glyphs(String name) throws Exception {
      List<String> lines = Files.readAllLines(new File(RES, "raw/" + name + ".txt").toPath(), StandardCharsets.UTF_8);
      Set<Integer> out = new HashSet<Integer>();
      for(String line : lines.subList(1, lines.size())) {
         if(!line.trim().isEmpty()) {
            out.add(Integer.parseInt(line.trim().split(" ")[0]));
         }
      }
      return out;
   }

   /** The values-XX directories that hold a strings.xml, by qualifier. */
   private static List<String> translated(String prefix, String file) {
      List<String> out = new ArrayList<String>();
      File[] dirs = RES.listFiles();
      assertTrue("run from the app module directory: " + RES.getAbsolutePath(), dirs != null);
      for(File dir : dirs) {
         if(dir.getName().startsWith(prefix + "-") && new File(dir, file).isFile()) {
            out.add(dir.getName().substring(prefix.length() + 1));
         }
      }
      Collections.sort(out);
      return out;
   }

   // ---------------------------------------------------------------- the checks

   @Test
   public void everyLanguageHasItsStringsAndItsRules() {
      assertEquals(LANGUAGES, translated("values", "strings.xml"));
      assertEquals(LANGUAGES, translated("raw", "rules.txt"));
   }

   @Test
   public void aboutIsEnglishOnly() {
      assertEquals("about.txt carries legal text and is not translated",
            Collections.emptyList(), translated("raw", "about.txt"));
   }

   @Test
   public void everyLanguageHasTheStringsGermanHas() throws Exception {
      Set<String> german = strings("de").keySet();
      assertEquals(strings(null).keySet(), german);
      for(String lang : LANGUAGES) {
         assertEquals(lang + " string names", new TreeSet<String>(german), new TreeSet<String>(strings(lang).keySet()));
      }
   }

   @Test
   public void namesThatAreNotTranslatedStayAsTheyAre() throws Exception {
      Map<String, String> english = strings(null);
      for(String lang : LANGUAGES) {
         Map<String, String> s = strings(lang);
         for(String name : Arrays.asList("app_name", "map_tarnholm", "map_dunholm", "high_scores_default")) {
            assertEquals(lang + " " + name, english.get(name), s.get(name));
         }
      }
   }

   private static List<String> placeholders(String text) {
      List<String> out = new ArrayList<String>();
      Matcher m = PLACEHOLDER.matcher(text);
      int next = 1;
      while(m.find()) {
         // An unnumbered placeholder takes the next argument, as String.format does
         int index = m.group(1) != null ? Integer.parseInt(m.group(1)) : next++;
         out.add(index + "$" + m.group(2));
      }
      Collections.sort(out);
      return out;
   }

   @Test
   public void placeholdersKeepTheirIndexAndType() throws Exception {
      Map<String, String> english = strings(null);
      for(String lang : LANGUAGES) {
         for(Map.Entry<String, String> e : strings(lang).entrySet()) {
            assertEquals(lang + " " + e.getKey(), placeholders(english.get(e.getKey())), placeholders(e.getValue()));
            // What is left once the placeholders are out must not hold a stray %
            assertFalse(lang + " " + e.getKey() + " has a stray %", PLACEHOLDER.matcher(e.getValue()).replaceAll("").contains("%"));
         }
      }
   }

   @Test
   public void theFormattedStringsFormat() throws Exception {
      for(String lang : LANGUAGES) {
         Map<String, String> s = strings(lang);
         assertTrue(String.format(Locale.ROOT, s.get("welcome_other_race"), "Dunholm", 10).contains("Dunholm"));
         assertTrue(String.format(Locale.ROOT, s.get("welcome_other_turns"), "Tarnholm", 15).contains("15"));
      }
   }

   private static List<String> everyText(String lang) throws Exception {
      List<String> out = new ArrayList<String>(strings(lang).values());
      out.addAll(rules(lang));
      return out;
   }

   @Test
   public void noDashesAndNoOtherGamesName() throws Exception {
      List<String> all = new ArrayList<String>(LANGUAGES);
      all.add(null);
      for(String lang : all) {
         for(String text : everyText(lang)) {
            assertFalse(lang + ": em dash in \"" + text + "\"", text.indexOf(0x2014) >= 0);
            assertFalse(lang + ": en dash in \"" + text + "\"", text.indexOf(0x2013) >= 0);
            String lower = text.toLowerCase(Locale.ROOT);
            // The two words of another game's name, spelt in pieces so this file does not hold them
            assertFalse(lang + ": \"" + text + "\"", lower.contains("cat" + "an") || lower.contains("sett" + "lers"));
         }
      }
   }

   private static void assertFits(String lang, Map<String, String> s, List<String> names, int max) {
      for(String name : names) {
         int limit = max;
         if("de".equals(lang) && GERMAN_AS_SHIPPED.containsKey(name)) {
            limit = Math.max(max, GERMAN_AS_SHIPPED.get(name));
         }
         String text = s.get(name);
         assertTrue(lang + " " + name + " \"" + text + "\" is " + text.length() + " characters, over " + limit,
               text.length() <= limit);
         assertFalse(lang + " " + name + " is empty", text.trim().isEmpty());
      }
   }

   @Test
   public void labelsFitTheRoomTheyHave() throws Exception {
      for(String lang : LANGUAGES) {
         Map<String, String> s = strings(lang);
         assertFits(lang, s, ROLL_BUTTON, ROLL_BUTTON_MAX);
         assertFits(lang, s, PAIRED_BUTTONS, PAIRED_BUTTON_MAX);
         assertFits(lang, s, MENU_ENTRIES, MENU_ENTRY_MAX);
         // The island entry is a template: it has to fit with each island's name in it
         Map<String, String> filled = new HashMap<String, String>();
         for(String island : Arrays.asList("map_tarnholm", "map_dunholm")) {
            filled.put(island, String.format(s.get("menu_go_to_island"), s.get(island)));
         }
         assertFits(lang, filled, new ArrayList<String>(filled.keySet()), MENU_ENTRY_MAX);
         assertFits(lang, s, DIALOG_TITLES, DIALOG_TITLE_MAX);
         assertFits(lang, s, Arrays.asList("tutorial_got_it"), GOT_IT_MAX);
         assertFits(lang, s, Arrays.asList("tutorial_skip"), SKIP_MAX);
         List<String> callouts = new ArrayList<String>();
         for(String name : s.keySet()) {
            if(name.startsWith("tutorial_") && !name.equals("tutorial_got_it") && !name.equals("tutorial_skip")) {
               callouts.add(name);
            }
         }
         assertEquals(11, callouts.size());
         assertFits(lang, s, callouts, CALLOUT_MAX);
      }
   }

   @Test
   public void shortLabelsAreNotAbbreviatedWithAFullStop() throws Exception {
      List<String> names = new ArrayList<String>(ROLL_BUTTON);
      names.addAll(PAIRED_BUTTONS);
      names.addAll(MENU_ENTRIES);
      names.addAll(Arrays.asList("score", "tutorial_got_it", "tutorial_skip"));
      for(String lang : LANGUAGES) {
         if("de".equals(lang)) {
            continue; // German writes its ordinals "1. Wurf", which is spelling, not abbreviation
         }
         Map<String, String> s = strings(lang);
         for(String name : names) {
            assertFalse(lang + " " + name + " \"" + s.get(name) + "\"", s.get(name).contains("."));
         }
      }
   }

   /** A rules line as its kind, with the icon or image it names: the part a translation must keep. */
   private static String kind(String line) {
      if(line.trim().isEmpty()) {
         return "blank";
      }
      if(line.startsWith("# ")) {
         return "heading";
      }
      if(line.startsWith("- ")) {
         return "bullet";
      }
      Matcher image = IMAGE_LINE.matcher(line);
      if(image.matches()) {
         return "image " + image.group(1);
      }
      Matcher icon = ICON_LINE.matcher(line);
      if(icon.matches()) {
         return "icon " + icon.group(1);
      }
      return "text";
   }

   @Test
   public void rulesKeepTheEnglishMarkupLineForLine() throws Exception {
      List<String> english = new ArrayList<String>();
      for(String line : rules(null)) {
         english.add(kind(line));
      }
      assertTrue(english.contains("heading") && english.contains("text") && english.contains("icon icon_gold_12")
            && english.contains("image rules_highlight_now"));
      for(String lang : LANGUAGES) {
         List<String> kinds = new ArrayList<String>();
         for(String line : rules(lang)) {
            kinds.add(kind(line));
            // A line that only looks like markup would be drawn as plain text
            assertFalse(lang + ": \"" + line + "\"", kind(line).equals("text") && (line.startsWith("#") || line.startsWith("[") || line.startsWith("!")));
            assertEquals(lang + ": stray space around \"" + line + "\"", line.trim(), line);
         }
         assertEquals(lang + " rules", english, kinds);
      }
   }

   private static void assertDrawable(String where, String text, Set<Integer> glyphs, String font) {
      for(int i = 0; i < text.length(); i++) {
         char c = text.charAt(i);
         assertTrue(where + ": " + font + " has no glyph for '" + c + "' (U+" + Integer.toHexString(c).toUpperCase(Locale.ROOT)
               + ") in \"" + text + "\"; add it to CODEPOINTS in tools/bake_font.py and rebake", glyphs.contains((int) c));
      }
   }

   @Test
   public void everyCharacterIsInTheBodyFontAtlas() throws Exception {
      Set<Integer> body = glyphs("font_body");
      List<String> all = new ArrayList<String>(LANGUAGES);
      all.add(null);
      for(String lang : all) {
         for(String text : everyText(lang)) {
            assertDrawable(String.valueOf(lang), text, body, "font_body");
         }
      }
   }

   @Test
   public void everyDisplayFontStringIsInTheDisplayFontAtlas() throws Exception {
      Set<Integer> display = glyphs("font_display");
      // What the game canvas draws in the display font, then what the dialogs title and
      // label with the same face
      Set<String> names = new TreeSet<String>(ROLL_BUTTON);
      names.add("score");
      names.addAll(PAIRED_BUTTONS);
      names.addAll(MENU_ENTRIES);
      names.addAll(DIALOG_TITLES);
      List<String> all = new ArrayList<String>(LANGUAGES);
      all.add(null);
      for(String lang : all) {
         Map<String, String> s = strings(lang);
         for(String name : names) {
            assertDrawable(lang + " " + name, s.get(name), display, "font_display");
         }
      }
   }

   @Test
   public void theAtlasesHoldTheSameGlyphsAndAQuestionMarkToFallBackOn() throws Exception {
      assertEquals(glyphs("font_body"), glyphs("font_display"));
      assertTrue(glyphs("font_body").contains((int) '?'));
   }

   @Test
   public void theNumeralsAtlasHoldsTheDigitsAndThePlusTheFlagsDraw() throws Exception {
      Set<Integer> numerals = glyphs("font_numerals");
      assertDrawable("flags", "0123456789+", numerals, "font_numerals");
   }
}
