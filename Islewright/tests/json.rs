// SPDX-License-Identifier: GPL-3.0-or-later
//! The small JSON reader and writer the stores use.

use dayapp::rules::json::Json;

#[test]
fn a_document_round_trips_in_key_order() {
    let text = "{\"b\":1,\"a\":[true,false,null],\"c\":{\"d\":\"x\\\"y\"},\"e\":-2.5}";
    let doc = Json::parse(text).unwrap();
    assert_eq!(text, doc.to_compact());
    assert_eq!(vec!["b", "a", "c", "e"], doc.keys());
    assert_eq!(doc, Json::parse(&doc.to_pretty(2)).unwrap());
}

#[test]
fn numbers_keep_their_kind() {
    let doc = Json::parse("[1, 1.0, 1e3, \"1\", 1700000000000]").unwrap();
    let items = doc.as_array().unwrap();
    assert_eq!(Json::Int(1), items[0]);
    assert_eq!(Json::Float(1.0), items[1]);
    assert_eq!(Some(1000), items[2].as_i64_lenient());
    assert_eq!(
        Some(1),
        items[3].as_i64_lenient(),
        "a numeric string reads as a number where org.json let it"
    );
    assert_eq!(None, items[3].as_number_i64(), "but is not a number");
    assert_eq!(Some(1_700_000_000_000), items[4].as_i64_lenient());
    assert_ne!(items[0].canonical(), items[3].canonical());
}

#[test]
fn escapes_and_unicode_are_read() {
    let doc = Json::parse("\"a\\u00e9\\n\\ud83c\\udfb2\"").unwrap();
    assert_eq!(Some("a\u{e9}\n\u{1F3B2}"), doc.as_str());
    assert_eq!("\"a\u{e9}\\n\u{1F3B2}\"", doc.to_compact());
}

#[test]
fn broken_documents_are_refused() {
    for bad in [
        "",
        "{",
        "{\"a\"}",
        "[1,]",
        "{\"a\":1} x",
        "tru",
        "\"open",
        "{a:1}",
    ] {
        assert!(Json::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn put_replaces_and_remove_takes_out() {
    let mut doc = Json::object();
    doc.put("x", 1);
    doc.put("y", "two");
    doc.put("x", 3);
    assert_eq!("{\"x\":3,\"y\":\"two\"}", doc.to_compact());
    assert_eq!(Some(Json::Int(3)), doc.remove("x"));
    assert!(!doc.has("x"));
    assert_eq!("{}", Json::object().to_compact());
}
