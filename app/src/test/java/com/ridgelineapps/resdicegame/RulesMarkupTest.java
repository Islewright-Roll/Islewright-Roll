package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;

import java.util.List;

import org.junit.Test;

/** The rules file format (see Rules.java) and the whole-factor picture fitting. */
public class RulesMarkupTest {

   private static List<RulesMarkup.Block> parse(String text) {
      return RulesMarkup.parse(text);
   }

   @Test
   public void everyConstructIsRecognised() {
      List<RulesMarkup.Block> blocks = parse("# Turn\nRoll the dice.\n\n![piece_village]\n- A bullet\n[icon_gold_12] Gold line\n");
      assertEquals(5, blocks.size());
      assertEquals(RulesMarkup.Kind.HEADING, blocks.get(0).kind);
      assertEquals("Turn", blocks.get(0).text);
      assertEquals(RulesMarkup.Kind.PARAGRAPH, blocks.get(1).kind);
      assertEquals("Roll the dice.", blocks.get(1).text);
      assertEquals(RulesMarkup.Kind.IMAGE, blocks.get(2).kind);
      assertEquals("piece_village", blocks.get(2).name);
      assertEquals(RulesMarkup.Kind.BULLET, blocks.get(3).kind);
      assertEquals("A bullet", blocks.get(3).text);
      assertEquals(RulesMarkup.Kind.ICON, blocks.get(4).kind);
      assertEquals("icon_gold_12", blocks.get(4).name);
      assertEquals("Gold line", blocks.get(4).text);
   }

   @Test
   public void consecutivePlainLinesRunTogetherAndABlankLineEndsTheParagraph() {
      List<RulesMarkup.Block> blocks = parse("one\ntwo\n\nthree");
      assertEquals(2, blocks.size());
      assertEquals("one two", blocks.get(0).text);
      assertEquals("three", blocks.get(1).text);
   }

   @Test
   public void aConstructEndsTheParagraphBeforeIt() {
      List<RulesMarkup.Block> blocks = parse("text\n# Head\nmore");
      assertEquals(3, blocks.size());
      assertEquals(RulesMarkup.Kind.PARAGRAPH, blocks.get(0).kind);
      assertEquals(RulesMarkup.Kind.HEADING, blocks.get(1).kind);
      assertEquals(RulesMarkup.Kind.PARAGRAPH, blocks.get(2).kind);
   }

   @Test
   public void windowsLineEndsAndABomAreHandled() {
      List<RulesMarkup.Block> blocks = parse("\uFEFF# Head\r\nbody\r\n");
      assertEquals(2, blocks.size());
      assertEquals("Head", blocks.get(0).text);
      assertEquals("body", blocks.get(1).text);
   }

   @Test
   public void thingsThatAreNotConstructsStayPlainText() {
      // No space after the hash, no text after an icon, a name with a dot, an image with text after it
      List<RulesMarkup.Block> blocks = parse("#nospace\n\n[icon_gold_12]\n\n![a.b]\n\n![piece_village] caption");
      assertEquals(4, blocks.size());
      for(RulesMarkup.Block block : blocks) {
         assertEquals(RulesMarkup.Kind.PARAGRAPH, block.kind);
      }
   }

   @Test
   public void emptyAndNullGiveNothing() {
      assertEquals(0, parse("").size());
      assertEquals(0, parse(null).size());
      assertEquals(0, parse("\n\n  \n").size());
   }

   @Test
   public void aPictureIsOnlyEverShrunkByAWholeFactor() {
      assertEquals(1, PixelImage.shrinkFactor(20, 200));
      assertEquals(1, PixelImage.shrinkFactor(200, 200));
      assertEquals(2, PixelImage.shrinkFactor(201, 200));
      assertEquals(2, PixelImage.shrinkFactor(400, 200));
      assertEquals(3, PixelImage.shrinkFactor(401, 200));
   }

   @Test
   public void shrinkingKeepsTheCentrePixelOfEveryBlockAndCropsTheRest() {
      // 5 x 2 picture, factor 2: 2 x 1 result from the centre of each 2 x 2 block.
      int[] pixels = {
            0, 1, 2, 3, 4,
            5, 6, 7, 8, 9 };
      // offX = (5 - 4) / 2 = 0, offY = 0; sample (x*2 + 1, y*2 + 1)
      assertArrayEquals(new int[] { 6, 8 }, PixelImage.shrink(pixels, 5, 2, 2));
      // factor 1 returns the picture untouched
      assertArrayEquals(pixels, PixelImage.shrink(pixels, 5, 2, 1));
   }
}
