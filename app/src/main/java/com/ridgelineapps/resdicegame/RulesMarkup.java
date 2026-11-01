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

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Reads the rules file (res/raw/rules.txt, and res/raw-de/rules.txt for German) into
 * blocks the Rules screen lays out. No Android types here, so it is tested on the JVM.
 * The format is described at the top of Rules.java.
 */
final class RulesMarkup {

    enum Kind {
        /** A section heading, in the display font. */
        HEADING,
        /** Running text. */
        PARAGRAPH,
        /** A picture from drawable-nodpi, centred. text holds its name. */
        IMAGE,
        /** A line with a hanging bullet. */
        BULLET,
        /** A line that starts with a small picture. name is the picture, text the line. */
        ICON
    }

    static final class Block {
        final Kind kind;
        final String name;
        final String text;

        Block(Kind kind, String name, String text) {
            this.kind = kind;
            this.name = name;
            this.text = text;
        }

        @Override
        public String toString() {
            return kind + (name == null ? "" : "[" + name + "]") + (text == null ? "" : " " + text);
        }
    }

    private static final Pattern IMAGE = Pattern.compile("^!\\[([A-Za-z0-9_]+)\\]$");
    private static final Pattern ICON = Pattern.compile("^\\[([A-Za-z0-9_]+)\\]\\s+(.+)$");

    private RulesMarkup() {
    }

    static List<Block> parse(String source) {
        ArrayList<Block> blocks = new ArrayList<Block>();
        StringBuilder paragraph = new StringBuilder();

        String text = source == null ? "" : source;
        if (text.startsWith("﻿")) {
            text = text.substring(1);
        }

        for (String raw : text.split("\r?\n|\r", -1)) {
            String line = raw.trim();

            if (line.length() == 0) {
                flush(blocks, paragraph);
                continue;
            }

            Matcher image = IMAGE.matcher(line);
            Matcher icon = ICON.matcher(line);
            if (line.startsWith("# ")) {
                flush(blocks, paragraph);
                blocks.add(new Block(Kind.HEADING, null, line.substring(2).trim()));
            }
            else if (image.matches()) {
                flush(blocks, paragraph);
                blocks.add(new Block(Kind.IMAGE, image.group(1), null));
            }
            else if (line.startsWith("- ")) {
                flush(blocks, paragraph);
                blocks.add(new Block(Kind.BULLET, null, line.substring(2).trim()));
            }
            else if (icon.matches()) {
                flush(blocks, paragraph);
                blocks.add(new Block(Kind.ICON, icon.group(1), icon.group(2).trim()));
            }
            else {
                // Lines of one paragraph may be wrapped by hand in the file: they run together
                if (paragraph.length() > 0) {
                    paragraph.append(' ');
                }
                paragraph.append(line);
            }
        }
        flush(blocks, paragraph);
        return blocks;
    }

    private static void flush(List<Block> blocks, StringBuilder paragraph) {
        if (paragraph.length() > 0) {
            blocks.add(new Block(Kind.PARAGRAPH, null, paragraph.toString()));
            paragraph.setLength(0);
        }
    }
}
