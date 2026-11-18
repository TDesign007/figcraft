//! The spell checker's behaviour: which words flag against the embedded list, where their
//! character ranges land, and what the suggestions say. Every case names the rule it pins.

use crate::spell::{known, misspelled, suggestions};
use crate::spell::Misspelled;

fn words(text: &str) -> Vec<String> {
    misspelled(text).into_iter().map(|m| m.word).collect()
}

fn ranges(text: &str) -> Vec<(usize, usize)> {
    misspelled(text).into_iter().map(|m| m.range).collect()
}

/// Unknown words flag, known words pass.
#[test]
fn known_and_unknown_words() {
    assert_eq!(words("The quik brwon fox"), vec!["quik", "brwon"]);
    assert!(words("The quick brown fox jumps over the lazy dog").is_empty());
}

/// Ranges are character indices over the word's letters only.
#[test]
fn ranges_cover_the_word() {
    assert_eq!(ranges("a quik fox"), vec![(2, 6)]);
    // Unicode before the word: the range counts characters, not bytes.
    assert_eq!(ranges("héllo quik"), vec![(6, 10)]);
    assert_eq!(misspelled("héllo quik")[0].word, "quik");
}

/// Single letters never flag ("a", "I", a stray "x" between words).
#[test]
fn single_letters_pass() {
    assert!(words("a b c I x").is_empty());
}

/// Everything uppercase passes: acronyms the list doesn't keep (a layer's own "KPI") don't flag.
#[test]
fn acronyms_pass() {
    assert!(words("USB CSS HTML QWRTY").is_empty());
    // Mixed case still checks: "Docnent" is not an acronym.
    assert_eq!(words("DoCNENT DoCnENT"), vec!["DoCnENT"]);
}

/// A capitalized word checks through its lowercase form; a proper noun the list keeps passes as
/// written.
#[test]
fn capitalized_words() {
    assert!(known("Monday"));
    assert!(known("monday"));
    assert!(known("London"));
    // A capitalized unknown word still flags.
    assert_eq!(words("Hello Mnday"), vec!["Mnday"]);
}

/// Apostrophes join a word: contractions the list keeps pass, unknown ones flag, curly quotes
/// normalize, and a possessive plural checks as its plain form.
#[test]
fn apostrophes() {
    assert!(words("don't isn't o'clock").is_empty());
    assert!(words("don’t isn’t").is_empty());
    assert_eq!(words("do'nt ca'ts"), vec!["do'nt", "ca'ts"]);
    // Possessive plural: the trailing apostrophe is looked up without it.
    assert!(words("the cats' bowls").is_empty());
    assert!(words("the cats’ bowls").is_empty());
    // A quote before a word isn't part of it; the range covers the letters only.
    assert_eq!(misspelled("'quik'")[0].range, (1, 5));
}

/// Digits, symbols and words that aren't letters don't reach the list.
#[test]
fn non_words_pass() {
    assert!(words("2 42 3.14 100% $5 v1.2").is_empty());
    // Letters joined to digits check as the letters' word ("v2" as "v" passes as one letter).
    assert!(words("v2").is_empty());
}

/// Text in another language flags (the list is en_US only), but its ranges stay on characters.
#[test]
fn other_languages_flag() {
    assert_eq!(words("привет мир"), vec!["привет", "мир"]);
    assert_eq!(words("中文设计"), Vec::<String>::new());
    // CJK next to English: only the English word's range.
    assert_eq!(ranges("设计 quik 设计"), vec![(3, 7)]);
}

/// An empty or whitespace-only text has nothing to flag.
#[test]
fn empty_text() {
    assert!(misspelled("").is_empty());
    assert!(misspelled("   \n\t").is_empty());
}

/// Suggestions come from the list, nearest first, within two edits.
#[test]
fn suggestions_rank_by_distance() {
    let s = suggestions("quik", 5);
    assert_eq!(s.first().map(String::as_str), Some("quick"));
    // A typo two edits away is found ("stat" → "start" is one insert).
    assert!(suggestions("stat", 10).iter().any(|w| w == "start"));
}

/// Suggestions respect the word's capitalization ("Helo" at the start of a sentence suggests
/// "Hello").
#[test]
fn suggestions_keep_capitalization() {
    let s = suggestions("Helo", 5);
    assert_eq!(s.first().map(String::as_str), Some("Hello"));
    assert_eq!(suggestions("helo", 5).first().map(String::as_str), Some("hello"));
}

/// A capitalized proper noun is suggested for its typo.
#[test]
fn suggestions_include_proper_nouns() {
    assert!(suggestions("Mnday", 10).iter().any(|w| w == "Monday"));
}

/// A possessive plural suggests the plain word's forms ("catss" → "cats").
#[test]
fn suggestions_for_apostrophes() {
    let s = suggestions("catss", 5);
    assert!(s.iter().any(|w| w == "cats"));
    // A curly apostrophe normalizes before the scan.
    assert!(suggestions("don’", 5).iter().any(|w| w == "don't"));
}

/// No suggestions beyond what the list has: an empty result, never a panic, on gibberish.
#[test]
fn gibberish_has_no_suggestions() {
    assert!(suggestions("xqzjwv", 5).is_empty());
    assert!(suggestions("", 5).is_empty());
    // A cap keeps the scan bounded even for a long word.
    assert!(suggestions(&"a".repeat(100), 5).is_empty());
}

/// The check is stable over a whole paragraph: every unknown word once, in order.
#[test]
fn paragraph_order() {
    let text = "The desinr draws a fram, then a teext layer underneth it.";
    assert_eq!(words(text), vec!["desinr", "fram", "teext", "underneth"]);
    let r = ranges(text);
    assert_eq!(r.len(), 4);
    // Ranges are ascending and inside the text.
    for pair in r.windows(2) {
        assert!(pair[0].1 <= pair[1].0);
    }
}

/// The `Misspelled` struct carries both the range and the word.
#[test]
fn misspelled_struct() {
    let m = misspelled("a quik fox");
    assert_eq!(
        m,
        vec![Misspelled { range: (2, 6), word: "quik".to_string() }]
    );
}
