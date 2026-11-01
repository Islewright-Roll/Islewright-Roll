package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class BoardLayersTest {

    private static int luma(int argb) {
        return Math.round((((argb >> 16) & 0xFF) * 299 + ((argb >> 8) & 0xFF) * 587 + (argb & 0xFF) * 114) / 1000f);
    }

    private static int chroma(int argb) {
        int r = (argb >> 16) & 0xFF, g = (argb >> 8) & 0xFF, b = argb & 0xFF;
        return Math.max(r, Math.max(g, b)) - Math.min(r, Math.min(g, b));
    }

    @Test
    public void factorOneLeavesTheColourAlone() {
        assertEquals(0xFF3A7BD5, BoardLayers.desaturate(0xFF3A7BD5, 1f));
    }

    @Test
    public void factorZeroIsTheGreyOfTheSameBrightness() {
        int grey = BoardLayers.desaturate(0xFF3A7BD5, 0f);
        assertEquals(0, chroma(grey), 1);
        assertEquals(luma(0xFF3A7BD5), luma(grey), 1);
    }

    @Test
    public void desaturatingDrainsColourAndKeepsBrightnessAndAlpha() {
        int in = 0x80E02020;
        int out = BoardLayers.desaturate(in, 0.55f);
        assertEquals(0x80, out >>> 24);
        assertTrue(chroma(out) < chroma(in));
        // Close to the factor: 55 percent of the colour is left
        assertEquals(chroma(in) * 0.55, chroma(out), 2);
        assertEquals(luma(in), luma(out), 1);
    }

    @Test
    public void aGreyHasNothingToDrain() {
        assertEquals(0xFF808080, BoardLayers.desaturate(0xFF808080, 0.55f));
    }

    @Test
    public void channelsStayInRangeForExtremeColours() {
        assertEquals(0xFFFFFFFF, BoardLayers.desaturate(0xFFFFFFFF, 0.3f));
        assertEquals(0xFF000000, BoardLayers.desaturate(0xFF000000, 0.3f));
        int out = BoardLayers.desaturate(0xFF00FF00, 0.55f);
        assertTrue(((out >> 8) & 0xFF) <= 255 && (out & 0xFF) >= 0);
    }

    @Test
    public void aPixelIsInAtMostOneFootprintLayer() {
        boolean[] roads = { true, true, false, true, false, true };
        boolean[] stands = { false, true, true, false, false, true };
        boolean[] roadLayer = BoardLayers.cutOut(roads, stands);
        assertArrayEquals(new boolean[] { true, false, false, true, false, false }, roadLayer);
        for(int p = 0; p < roads.length; p++) {
            assertTrue("pixel " + p, !(roadLayer[p] && stands[p]));
            // and nothing is lost: every pixel either layer would have drawn is still drawn
            assertEquals(roads[p] || stands[p], roadLayer[p] || stands[p]);
        }
    }

    @Test
    public void cutOutLeavesTheInputsAlone() {
        boolean[] roads = { true, true };
        boolean[] stands = { false, true };
        BoardLayers.cutOut(roads, stands);
        assertArrayEquals(new boolean[] { true, true }, roads);
        assertArrayEquals(new boolean[] { false, true }, stands);
    }

    @Test
    public void lightenMovesEachChannelPartWayToWhite() {
        // 45 percent of the way: 0 -> 115, 255 stays, 100 -> 100 + 155 * 0.45 = 170
        assertEquals(0xFF73AAFF, BoardLayers.lighten(0xFF0064FF, 0.45f));
        assertEquals(0x80FFFFFF, BoardLayers.lighten(0x80FFFFFF, 0.45f));
    }

    @Test
    public void lightenZeroLeavesItAloneAndOneIsWhite() {
        assertEquals(0xFF3A7BD5, BoardLayers.lighten(0xFF3A7BD5, 0f));
        assertEquals(0xFFFFFFFF, BoardLayers.lighten(0xFF3A7BD5, 1f));
    }

    @Test
    public void lightenKeepsAlphaAndMakesAColourBrighter() {
        int in = 0xC0E02020;
        int out = BoardLayers.lighten(in, 0.45f);
        assertEquals(0xC0, out >>> 24);
        assertTrue(luma(out) > luma(in));
        // and no more colourful than it was
        assertTrue(chroma(out) < chroma(in));
    }

    @Test
    public void haloIsBlackRoundALightColourAndWhiteRoundADarkOne() {
        assertEquals(0x000000, BoardLayers.haloRgb(0xFFFF80, 110));
        assertEquals(0xFFFFFF, BoardLayers.haloRgb(0x102040, 110));
    }

    /** A 3 by 3 block in the middle of a 9 by 9 canvas: ring one outline, ring two the halo. */
    @Test
    public void outlineIsOnePixelRoundTheMaskAndThenARing() {
        int w = 9;
        boolean[] mask = new boolean[w * w];
        for(int y = 3; y < 6; y++) {
            for(int x = 3; x < 6; x++) {
                mask[y * w + x] = true;
            }
        }
        int[] out = BoardLayers.outline(mask, w, w, 0x112233, 0xFFFFFF, 160, 60, 1f);
        // inside: nothing; straight out: the outline; one further: the ring; beyond: nothing
        assertEquals(0, out[4 * w + 4]);
        assertEquals(0xA0112233, out[4 * w + 2]);
        assertEquals(0xA0112233, out[2 * w + 4]);
        assertEquals(0x3CFFFFFF, out[4 * w + 1]);
        assertEquals(0, out[4 * w + 0]);
        // it grows four way, so the corner beyond the block's corner is not outline
        assertEquals(0x3CFFFFFF, out[2 * w + 2]);
    }

    @Test
    public void outlineStrengthScalesTheOpacityOfBoth() {
        boolean[] mask = new boolean[25];
        mask[12] = true;
        int[] full = BoardLayers.outline(mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 1f);
        int[] half = BoardLayers.outline(mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 0.5f);
        assertEquals(200, full[11] >>> 24);
        assertEquals(100, half[11] >>> 24);
        assertEquals(100, full[10] >>> 24);
        assertEquals(50, half[10] >>> 24);
        assertEquals(0, BoardLayers.outline(mask, 5, 5, 0xFF0000, 0xFFFFFF, 200, 100, 0f)[11] >>> 24);
    }

    /** grow() is what the buildable spots' glow runs: the stronger tier owns a shared pixel. */
    @Test
    public void growGivesASharedPixelToTheStrongerTier() {
        int w = 5;
        int[] dist = new int[w];
        int[] tier = new int[w];
        boolean[] roadOnly = new boolean[w];
        java.util.Arrays.fill(dist, -1);
        dist[0] = 0;
        tier[0] = 4;
        dist[2] = 0;
        tier[2] = 2;
        roadOnly[2] = true;
        BoardLayers.grow(dist, tier, roadOnly, w, 1, 1);
        assertEquals(1, dist[1]);
        assertEquals(4, tier[1]);
        assertEquals(false, roadOnly[1]);
        assertEquals(1, dist[3]);
        assertEquals(2, tier[3]);
        assertEquals(true, roadOnly[3]);
        assertEquals(-1, dist[4]);
    }

    private static double relativeSaturation(int argb) {
        int r = (argb >> 16) & 0xFF, g = (argb >> 8) & 0xFF, b = argb & 0xFF;
        int max = Math.max(r, Math.max(g, b));
        return max == 0 ? 0 : chroma(argb) / (double) max;
    }

}
