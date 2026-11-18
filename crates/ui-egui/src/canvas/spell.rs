//! Spell-check underlines while a text layer is being edited: a red squiggle under every word the
//! embedded word list doesn't know ([`figcraft_text::spell`]), while the layer is edited and for
//! it alone. Words added to the session's list ("Add to dictionary", the spelling menu) pass.

use egui::{Color32, Pos2, Shape, Stroke, vec2};

use super::outline::TextOnScreen;
use super::{Document, FigApp, NodeKind};

/// The underline's colour (a plain red that reads on light and dark pages alike).
const COLOUR: Color32 = Color32::from_rgb(0xE8, 0x46, 0x46);
/// The wave's height above and below its line (screen points).
const AMPLITUDE: f32 = 1.2;
/// The wave's half period (screen points), one zig up then one zag down.
const HALF_PERIOD: f32 = 2.5;
/// The wave's line width (screen points).
const WIDTH: f32 = 1.0;
/// The wave's largest sample count (a word's line at an extreme zoom still draws bounded).
const SAMPLES: usize = 500;

/// Draw the squiggles for the words of the text `te` edits (the document its overlay drew, an
/// IME composition included), under their selection rectangles on `text`'s lines.
pub(super) fn underlines(app: &FigApp, painter: &egui::Painter, text: &TextOnScreen, edoc: &Document, te: &figcraft_engine::TextEdit) {
    let Some(n) = edoc.node(te.id) else { return };
    let NodeKind::Text(t) = &n.kind else { return };
    let added = &app.ui.spell_added;
    for m in figcraft_text::spell::misspelled(&t.characters) {
        if added.contains(&m.word) {
            continue;
        }
        for r in text.layout.selection_rects(m.range.0, m.range.1) {
            // The underline sits on the rectangle's bottom edge (the line's bottom), where a
            // descender's room keeps it off the letters.
            let a = text.at(app, r.x0, r.y1);
            let b = text.at(app, r.x1, r.y1);
            squiggle(painter, a, b);
        }
    }
}

/// A zigzag from `a` to `b`, offset below its line so it clears the text's bottom.
fn squiggle(painter: &egui::Painter, a: Pos2, b: Pos2) {
    let line = b - a;
    let len = line.length();
    if len < HALF_PERIOD {
        return;
    }
    let dir = line / len;
    // The line's unit normal: the wave offsets along it, so a rotated line's wave follows it.
    let normal = vec2(dir.y, -dir.x);
    let halves = ((len / HALF_PERIOD) as usize).min(SAMPLES);
    let mut points = Vec::with_capacity(halves + 1);
    for i in 0..=halves {
        let t = (i as f32 * HALF_PERIOD).min(len);
        // The wave's ends sit on its line; between them it swings from side to side, and a
        // little of everything sits below the line so the wave clears the text's bottom.
        let swing = if i == 0 || i == halves { 0.0 } else { if i % 2 == 0 { -AMPLITUDE } else { AMPLITUDE } };
        points.push(a + dir * t + normal * (2.0 + swing));
    }
    painter.add(Shape::line(points, Stroke::new(WIDTH, COLOUR)));
}
