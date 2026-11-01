package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotEquals;

import org.junit.Test;

public class PaletteRecolourTest {

    @Test
    public void lightLeavesArtAlone() {
        assertEquals(0xFF7A5A2C, Palette.LIGHT.recolourField(0xFF7A5A2C, false));
        assertEquals(0xFFA0783C, Palette.LIGHT.recolourThumb(0xFFA0783C));
    }

    @Test
    public void darkFieldMapsEachArtColour() {
        Palette d = Palette.DARK;
        assertEquals(0x00000000, d.recolourField(0x00000000, false));
        assertEquals(d.fieldShadow, d.recolourField(0xFF1E140A, false));
        assertEquals(d.fieldFace, d.recolourField(0xFF7A5A2C, true));
        assertEquals(d.fieldInner, d.recolourField(0xFF5A3C14, false));
        assertEquals(d.fieldLight, d.recolourField(0xFF5A3C14, true));
    }

    @Test
    public void darkThumbKeepsItsBevelTones() {
        Palette d = Palette.DARK;
        assertEquals(d.thumbFace, d.recolourThumb(0xFFA0783C));
        assertNotEquals(d.recolourThumb(0xFFA0783C), d.recolourThumb(0xFFB48C4C));
        assertEquals(d.thumbOutline, d.recolourThumb(0xFF1E140A));
    }
}
