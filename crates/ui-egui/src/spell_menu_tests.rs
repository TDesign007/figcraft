//! The spelling section of the context menu while a text layer is edited, driven through egui
//! headless: a misspelled word under the pointer offers the list's nearest words and replaces the
//! word on pick (the edit continues), a known word shows no section, and "Add to dictionary"
//! passes the word for the rest of the session.

use egui::{Event, Key, Modifiers, Pos2, Shape};
use figcraft_doc::{NodeId, NodeKind};
use figcraft_engine::Session;
use serde_json::json;

use crate::canvas::TextOnScreen;
use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text, text_center};

fn press(u: &mut Ui, key: Key) {
    u.frame(vec![
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
    ]);
}

fn characters(u: &Ui, id: NodeId) -> String {
    match &u.app.session.doc().unwrap().node(id).unwrap().kind {
        NodeKind::Text(t) => t.characters.clone(),
        _ => panic!("not text"),
    }
}

/// A text layer `chars` being edited with its caret inside characters `at..at`, and a screen
/// point off the text's box: the menu acts on the caret's own word (a right-click beside the
/// text stands in for the pointer's word, which the harness's fontless layout can't place).
fn editing_with_caret_at(chars: &str, at: usize) -> (Ui, NodeId, Pos2) {
    let mut s = Session::new();
    let t = NodeId(s.execute("text.create", &json!({"text": chars, "x": 20, "y": 20})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [t]})).unwrap();
    let mut u = Ui::new(s);
    press(&mut u, Key::Enter);
    figcraft_engine::textedit::set_caret(&mut u.app.session, at, false).unwrap();
    (u, t, Pos2::new(900.0, 700.0))
}

fn open_menu(u: &mut Ui, at: Pos2) -> Vec<Shape> {
    u.app.ui.context_menu = Some((at, Vec::new()));
    // The menu remembers how wide its widest row asked to be from the frame before, so a second
    // frame draws its rows.
    u.frame(vec![]);
    flat(&u.frame_output(vec![]).shapes)
}

#[test]
fn the_spelling_menu_suggests_and_replaces_a_misspelled_word() {
    // "quik" sits at characters 6..10 of the text; the pointer lands in its middle.
    let (mut u, t, at) = editing_with_caret_at("Hello quik world", 8);
    let shapes = open_menu(&mut u, at);
    assert!(has_text(&shapes, "Spelling"), "the section shows for a misspelled word");
    assert!(has_text(&shapes, "quick"), "the list's nearest word is suggested");
    // Picking the suggestion replaces the word and the menu closes; the edit continues.
    let row = text_center(&shapes, "quick");
    u.button(row, true);
    u.button(row, false);
    u.frame(vec![]);
    assert_eq!(characters(&u, t), "Hello quick world", "the suggestion replaced the word");
    let ds = u.app.session.ds().unwrap();
    assert!(ds.text_edit.is_some(), "the edit continues after the replacement");
}

#[test]
fn a_known_word_shows_no_spelling_section() {
    let (mut u, _t, at) = editing_with_caret_at("Hello quik world", 2);
    let shapes = open_menu(&mut u, at);
    assert!(!has_text(&shapes, "Spelling"), "'Hello' is the list's own");
    assert!(!has_text(&shapes, "Add to dictionary"), "no word, no section");
}

#[test]
fn a_word_added_to_the_dictionary_passes_for_the_session() {
    let (mut u, t, at) = editing_with_caret_at("Hello quik world", 8);
    let shapes = open_menu(&mut u, at);
    assert!(has_text(&shapes, "Spelling"), "the word flags before it is added");
    let row = text_center(&shapes, "Add to dictionary");
    u.button(row, true);
    u.button(row, false);
    u.frame(vec![]);
    assert!(u.app.ui.spell_added.contains("quik"), "the word joined the session's list");
    // The same word offers no spelling section any more (and no suggestion replaced the text).
    let shapes = open_menu(&mut u, at);
    assert!(!has_text(&shapes, "Spelling"), "the word passes once it is added");
    assert_eq!(characters(&u, t), "Hello quik world", "adding changes no text");
}
