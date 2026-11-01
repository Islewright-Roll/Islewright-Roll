// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from RulesMarkupTest: the rules text's small markup, and the whole-factor shrink of
//! its pictures.

use dayapp::rules::rules_markup::{Kind, parse, shrink, shrink_factor};

#[test]
fn every_construct_is_recognised() {
    let blocks = parse(Some(
        "# Turn\nRoll the dice.\n\n![piece_village]\n- A bullet\n[icon_gold_12] Gold line\n",
    ));
    assert_eq!(5, blocks.len());
    assert_eq!(Kind::Heading, blocks[0].kind);
    assert_eq!(Some("Turn"), blocks[0].text.as_deref());
    assert_eq!(Kind::Paragraph, blocks[1].kind);
    assert_eq!(Some("Roll the dice."), blocks[1].text.as_deref());
    assert_eq!(Kind::Image, blocks[2].kind);
    assert_eq!(Some("piece_village"), blocks[2].name.as_deref());
    assert_eq!(Kind::Bullet, blocks[3].kind);
    assert_eq!(Some("A bullet"), blocks[3].text.as_deref());
    assert_eq!(Kind::Icon, blocks[4].kind);
    assert_eq!(Some("icon_gold_12"), blocks[4].name.as_deref());
    assert_eq!(Some("Gold line"), blocks[4].text.as_deref());
}

#[test]
fn consecutive_plain_lines_run_together_and_a_blank_line_ends_the_paragraph() {
    let blocks = parse(Some("one\ntwo\n\nthree"));
    assert_eq!(2, blocks.len());
    assert_eq!(Some("one two"), blocks[0].text.as_deref());
    assert_eq!(Some("three"), blocks[1].text.as_deref());
}

#[test]
fn a_construct_ends_the_paragraph_before_it() {
    let blocks = parse(Some("text\n# Head\nmore"));
    assert_eq!(3, blocks.len());
    assert_eq!(Kind::Paragraph, blocks[0].kind);
    assert_eq!(Kind::Heading, blocks[1].kind);
    assert_eq!(Kind::Paragraph, blocks[2].kind);
}

#[test]
fn windows_line_ends_and_a_bom_are_handled() {
    let blocks = parse(Some("\u{FEFF}# Head\r\nbody\r\n"));
    assert_eq!(2, blocks.len());
    assert_eq!(Some("Head"), blocks[0].text.as_deref());
    assert_eq!(Some("body"), blocks[1].text.as_deref());
}

#[test]
fn things_that_are_not_constructs_stay_plain_text() {
    // No space after the hash, no text after an icon, a name with a dot, an image with text after it
    let blocks = parse(Some(
        "#nospace\n\n[icon_gold_12]\n\n![a.b]\n\n![piece_village] caption",
    ));
    assert_eq!(4, blocks.len());
    for block in blocks {
        assert_eq!(Kind::Paragraph, block.kind);
    }
}

#[test]
fn empty_and_none_give_nothing() {
    assert!(parse(Some("")).is_empty());
    assert!(parse(None).is_empty());
    assert!(parse(Some("\n\n  \n")).is_empty());
}

#[test]
fn a_picture_is_only_ever_shrunk_by_a_whole_factor() {
    assert_eq!(1, shrink_factor(20, 200));
    assert_eq!(1, shrink_factor(200, 200));
    assert_eq!(2, shrink_factor(201, 200));
    assert_eq!(2, shrink_factor(400, 200));
    assert_eq!(3, shrink_factor(401, 200));
}

#[test]
fn shrinking_keeps_the_centre_pixel_of_every_block_and_crops_the_rest() {
    // 5 x 2 picture, factor 2: 2 x 1 result from the centre of each 2 x 2 block
    let pixels = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(vec![6, 8], shrink(&pixels, 5, 2, 2));
    // factor 1 returns the picture untouched
    assert_eq!(pixels.to_vec(), shrink(&pixels, 5, 2, 1));
}
