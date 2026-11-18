//! Spell checking for edited text: which words the embedded word list doesn't know, and what to
//! suggest instead.
//!
//! The checker is a plain word list: the hunspell en_US dictionary (SCOWL, permissively licensed;
//! see `assets/spell_en_us_generation.py` and `ATTRIBUTION.md`) expanded to its inflected forms,
//! embedded as text and looked up in a set. Checking a word is a membership test, so the editor
//! can check the whole layer on every frame; suggestions are the list's nearest words by edit
//! distance, computed only when the spelling menu opens.
//!
//! English (US) only: text in other languages checks against the same list, so words in any other
//! language flag as unknown; callers decide where that shows (the editor underlines a word only
//! while it is being typed).

use std::collections::HashSet;
use std::sync::OnceLock;

/// The embedded word list (one word per line, its original case kept: the list carries proper
/// nouns, so a capitalized unknown word still flags while a known one passes as written).
static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn words() -> &'static HashSet<&'static str> {
    WORDS.get_or_init(|| include_str!("../assets/spell_en_us.txt").lines().collect())
}

/// A word the list doesn't know: where it sits in the checked text and what it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misspelled {
    /// The word's character range in the checked text.
    pub range: (usize, usize),
    /// The word as written (its apostrophes normalized, a trailing possessive kept).
    pub word: String,
}

/// Is `word` in the list? Both sides of case pass through each other: a capitalized word checks
/// through its lowercase form ("Monday" typed mid-sentence), and a lowercase one through the
/// list's capitalized proper nouns ("monday" typed at the start of a sentence).
pub fn known(word: &str) -> bool {
    let list = words();
    if list.contains(word) {
        return true;
    }
    if word.chars().any(|c| c.is_uppercase()) {
        let lower = word.to_lowercase();
        if list.contains(lower.as_str()) {
            return true;
        }
    }
    let mut chars = word.chars();
    match chars.next() {
        Some(c) if c.is_lowercase() => {
            let cap = c.to_uppercase().collect::<String>() + chars.as_str();
            list.contains(cap.as_str())
        }
        _ => false,
    }
}

/// The words of `text` the list doesn't know, in order: every maximal run of letters and word
/// apostrophes (single letters, all-uppercase acronyms and known words pass; a trailing possessive
/// apostrophe is looked up without it). Ranges are character indices, the editor's own currency.
pub fn misspelled(text: &str) -> Vec<Misspelled> {
    let mut out = Vec::new();
    let mut start = None;
    let mut index = 0usize;
    let mut start_byte = 0usize;
    for (i, c) in text.char_indices() {
        if is_word_char(c) {
            if start.is_none() {
                start = Some(index);
                start_byte = i;
            }
        } else if let Some(from) = start.take() {
            check_word(text, from, index, start_byte, i, &mut out);
        }
        index += 1;
    }
    if let Some(from) = start {
        check_word(text, from, index, start_byte, text.len(), &mut out);
    }
    out
}

/// A letter or an apostrophe inside a word (`'` and `’` both count; the run's ends are trimmed
/// later, so a quote opening before a word doesn't join it).
fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || c == '\'' || c == '’'
}

/// Look up the word at char range `from..to` (its bytes `from_byte..to_byte`) and, unknown, push
/// it onto `out`.
fn check_word(text: &str, from: usize, to: usize, from_byte: usize, to_byte: usize, out: &mut Vec<Misspelled>) {
    let Some(slice) = text.get(from_byte..to_byte) else { return };
    let normalized = slice.replace('’', "'");
    // A possessive plural checks as its plain form; a quote before a word isn't part of it. The
    // underline's range is trimmed to match, so it covers the word's letters only.
    let lead = normalized.chars().take_while(|c| *c == '\'').count();
    let trail = normalized.chars().rev().take_while(|c| *c == '\'').count();
    let chars: Vec<char> = normalized.chars().collect();
    if lead + trail >= chars.len() {
        return;
    }
    let word: String = chars[lead..chars.len() - trail].iter().collect();
    // One letter is never wrong ("a", "I", and every letter a vector's path spells out).
    if word.chars().count() < 2 {
        return;
    }
    // Words with letters outside ASCII pass: the list is English (US) only, and another script's
    // words flagging under it (Russian, Arabic, CJK) would be noise, as would accented borrowings.
    if !word.chars().all(|c| c.is_ascii_alphabetic() || c == '\'') {
        return;
    }
    // An identifier or acronym reads as one and passes: USB, DON'T, and the camelCase code a
    // design's annotations carry (getElementById, fontSize); only a uniformly cased word checks.
    if word[1..].chars().any(|c| c.is_uppercase()) {
        return;
    }
    if known(&word) {
        return;
    }
    out.push(Misspelled { range: (from + lead, to - trail), word });
}

/// The list's nearest words to `word`, best first: edit distance 2 at most, ties preferring the
/// longer word, then alphabetically, compared case-insensitively. A capitalized word gets
/// capitalized suggestions ("Helo" at the start of a sentence suggests "Hello"), and proper nouns
/// come along in their own ("Mnday" suggests "Monday").
pub fn suggestions(word: &str, max: usize) -> Vec<String> {
    let target = word.replace('’', "'").trim_matches('\'').to_lowercase();
    if target.is_empty() || max == 0 {
        return Vec::new();
    }
    let len = target.chars().count();
    // The distance compares case-insensitively (a proper noun's capital must not count against
    // it); the list's word as written comes back, so "Mnday" suggests "Monday" in its own case.
    let mut scored: Vec<(usize, usize, String)> = words()
        .iter()
        .filter(|w| {
            let l = w.chars().count();
            l + 2 >= len && l <= len + 2
        })
        .filter_map(|w| {
            let lower = w.to_lowercase();
            let d = edit_distance(&target, &lower, 2)?;
            let l = lower.chars().count();
            Some((d, l, (*w).to_string()))
        })
        .collect();
    // Distance first; equal distances prefer the longer word (the typo is more often a missing
    // letter than a stray one: "quik" reads as "quick", "Helo" as "Hello"); then alphabetical.
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    let capitalized = word.chars().next().is_some_and(|c| c.is_uppercase());
    scored
        .into_iter()
        .take(max)
        .map(|(_, _, w)| {
            if capitalized {
                capitalize(&w)
            } else {
                w
            }
        })
        .collect()
}

/// `word` with its first letter uppercased (the rest as it was).
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The edit distance between two words, capped: `None` when it passes `max` (an early bail keeps
/// the menu's scan over the whole list cheap). Neither word is empty by the caller's filters.
fn edit_distance(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    // One row of the distance table at a time; the band's cells only.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut next = vec![i + 1];
        let mut best = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let d = (row[j] + cost).min(row[j + 1] + 1).min(next[j] + 1);
            next.push(d);
            best = best.min(d);
        }
        if best > max {
            return None;
        }
        row = next;
    }
    let d = *row.last()?;
    (d <= max).then_some(d)
}
