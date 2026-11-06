//! Checks for the frontend translations in `dist/locales`: every listed language
//! has a file, the same keys as English, and the same `{placeholders}`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn locales_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist/locales")
}

/// The JSON value inside `NAME(...)`: from the first `open` to the last `close`.
fn json_inside(src: &str, open: char, close: char) -> serde_json::Value {
    let start = src.find(open).expect("no JSON start");
    let end = src.rfind(close).expect("no JSON end");
    serde_json::from_str(&src[start..=end]).expect("locale data must be strict JSON")
}

fn load(code: &str) -> BTreeMap<String, String> {
    let path = locales_dir().join(format!("{code}.js"));
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(src.contains(&format!("I18N_REGISTER(\"{code}\"")), "{code}.js registers another code");
    let obj = json_inside(&src, '{', '}');
    obj.as_object()
        .expect("object")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().expect("string value").to_string()))
        .collect()
}

fn placeholders(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        match rest[i..].find('}') {
            Some(j) => {
                out.insert(rest[i + 1..i + j].to_string());
                rest = &rest[i + j + 1..];
            }
            None => break,
        }
    }
    out
}

fn languages() -> Vec<String> {
    let src = std::fs::read_to_string(locales_dir().join("index.js")).unwrap();
    json_inside(&src, '[', ']')
        .as_array()
        .expect("array")
        .iter()
        .map(|l| l["code"].as_str().expect("code").to_string())
        .collect()
}

#[test]
fn english_is_listed() {
    assert!(languages().iter().any(|c| c == "en"));
}

#[test]
fn every_language_matches_english() {
    let en = load("en");
    for code in languages() {
        let dict = load(&code);
        let missing: Vec<_> = en.keys().filter(|k| !dict.contains_key(*k)).collect();
        let extra: Vec<_> = dict.keys().filter(|k| !en.contains_key(*k)).collect();
        assert!(missing.is_empty() && extra.is_empty(), "{code}: missing {missing:?}, extra {extra:?}");
        for (k, v) in &en {
            assert_eq!(placeholders(v), placeholders(&dict[k]), "{code}: placeholders differ in {k}");
        }
    }
}
