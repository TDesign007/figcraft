//! The context menu's spelling section, while a text layer is being edited: suggestions for a
//! misspelled word (the one under the pointer, else the caret's own) and the word added to the
//! session's list so it passes the check afterwards. English (US) only: the embedded word list
//! ([`figcraft_text::spell`]) is the checker.

use super::tree::{indented_item, separator};
use super::{FigApp, Tokens, Ui, tr};
use crate::canvas::TextOnScreen;

/// The largest number of suggestions the menu lists.
const SUGGESTIONS: usize = 6;

/// The context menu's spelling rows, before the layer rows. Returns whether the menu closes (a
/// suggestion replaced its word, or the word was added to the list).
pub(super) fn spell_menu(app: &mut FigApp, ui: &mut Ui, pos: egui::Pos2, t: &Tokens) -> bool {
    // What the section shows, decided before the menu draws: the edited text, the word under the
    // pointer (the caret's own when the pointer is off the text) and the suggestions for it.
    let state = spelling(app, pos);
    let Some((range, word, sugs)) = state else { return false };
    let mut close = false;
    let mut replace = None;
    // The section's title, a dim row like the other menus' section names.
    indented_item(ui, tr("Spelling"), "", false, false, false, 0.0, t);
    if sugs.is_empty() {
        indented_item(ui, tr("No suggestions"), "", false, false, false, 0.0, t);
    }
    for sug in &sugs {
        if indented_item(ui, sug, "", true, false, false, 0.0, t) {
            close = true;
            replace = Some((range, sug.clone()));
        }
    }
    separator(ui);
    if indented_item(ui, tr("Add to dictionary"), "", true, false, false, 0.0, t) {
        app.ui.spell_added.insert(word.clone());
        close = true;
    }
    separator(ui);
    if let Some(((a, b), sug)) = replace {
        // Select the word's range and type the suggestion over it (the edit continues).
        figcraft_engine::textedit::set_caret(&mut app.session, b, false).ok();
        figcraft_engine::textedit::set_caret(&mut app.session, a, true).ok();
        if let Err(e) = figcraft_engine::textedit::insert(&mut app.session, &sug) {
            app.toast(e.to_string());
        }
    }
    close
}

/// The misspelled word the spelling menu acts on: its char range in the text, the word and the
/// suggestions for it. `None` when no text is being edited or the pointer (and the caret) sit on
/// words the list knows.
fn spelling(app: &FigApp, pos: egui::Pos2) -> Option<((usize, usize), String, Vec<String>)> {
    let ds = app.session.ds().ok()?;
    let te = ds.text_edit.as_ref()?.clone();
    // The composition view, so a word beside an IME composition still checks against the text.
    let doc = app.preedit_view().map(|p| p.doc.clone()).unwrap_or_else(|| ds.doc.clone());
    let n = doc.node(te.id)?;
    let figcraft_doc::NodeKind::Text(td) = &n.kind else { return None };
    let text = TextOnScreen::of(&doc, te.id)?;
    // The word the pointer is on (a right-click moves nothing, so the caret stays where it was);
    // when the pointer is off the text's box, or the text is laid out on a path (where the
    // pointer's inverse mapping doesn't hold), the caret's own word stands in.
    let page = app.to_page(pos);
    let on_box = text.box_contains(page, n.width, n.height);
    let index = if text.layout.on_path.is_some() || !on_box {
        te.caret
    } else {
        let (x, y) = text.layout_at(page);
        text.layout.hit(x, y)
    };
    let list = figcraft_text::spell::misspelled(&td.characters);
    // The word under the index, or one ending there (the caret sits just after its last letter).
    let m = list.into_iter().find(|m| (m.range.0..m.range.1).contains(&index) || m.range.1 == index)?;
    // A word the session added ("Add to dictionary") passes the check everywhere: no section.
    if app.ui.spell_added.contains(&m.word) {
        return None;
    }
    let sugs = figcraft_text::spell::suggestions(&m.word, SUGGESTIONS);
    Some((m.range, m.word, sugs))
}
