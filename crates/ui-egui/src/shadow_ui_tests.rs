//! The Drop shadow and Inner shadow popovers: their row routes, fields, colour wells, the
//! droplet's blend menu, and their turns with the other effect panels.

use egui::{Event, Key, Modifiers, Pos2, Rect};
use figcraft_color::BlendMode;
use figcraft_doc::{EffectKind, NodeId, Shadow};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rect_with(kind: &str) -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 100.0, "y": 100.0, "width": 200.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": kind})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn press(p: Pos2, pressed: bool) -> Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

fn click(u: &mut Ui, at: Pos2) {
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
}

fn shadow(u: &Ui, r: NodeId) -> Shadow {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::DropShadow(s) | EffectKind::InnerShadow(s) => s,
        _ => panic!("the effect is a shadow"),
    }
}

/// The middle of the row's `label`.
fn row(u: &mut Ui, label: &str) -> Pos2 {
    let shapes = flat(&u.frame_output(vec![]).shapes);
    crate::live_tests::text_center(&shapes, label)
}

fn panel(u: &Ui, id: &str) -> Rect {
    u.ctx.memory(|m| m.area_rect(egui::Id::unique(id))).expect("the panel draws")
}

fn key(key: Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }
}

fn type_into(u: &mut Ui, id: egui::Id, clears: usize, text: &str) {
    u.ctx.memory_mut(|m| m.request_focus(id));
    u.frame(vec![]);
    for _ in 0..clears {
        u.frame(vec![key(Key::Backspace, Modifiers::NONE, true), key(Key::Backspace, Modifiers::NONE, false)]);
    }
    u.frame(vec![Event::Text(text.into())]);
    u.frame(vec![key(Key::Enter, Modifiers::NONE, true), key(Key::Enter, Modifiers::NONE, false)]);
    u.frame(vec![]);
}

#[test]
fn both_rows_open_their_own_popover() {
    for (label, kind, panel_id, open) in [
        ("Drop shadow", "dropShadow", "shadow_popover", (|u: &Ui| u.app.ui.shadow_popover.is_some()) as fn(&Ui) -> bool),
        ("Inner shadow", "innerShadow", "inner_shadow_popover", (|u: &Ui| u.app.ui.inner_shadow_popover.is_some()) as fn(&Ui) -> bool),
    ] {
        let (mut u, _r) = rect_with(kind);
        let at = row(&mut u, label);
        click(&mut u, at);
        assert!(open(&u), "{label}: the row opens the popover");
        let shapes = flat(&u.frame_output(vec![]).shapes);
        assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == label)), "{label}: the panel's title");
        for name in ["Position", "Blur", "Spread", "Color"] {
            assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == name)), "{label}: the popover shows {name}");
        }
        for value in ["0", "4", "000000", "25%"] {
            assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == value)), "{label}: the popover shows {value}");
        }
        let p = panel(&u, panel_id);
        assert!((p.width() - 238.0).abs() < 2.0, "{label}: the reference's width (got {})", p.width());
        assert!((p.height() - 218.0).abs() < 4.0, "{label}: the reference's height (got {})", p.height());
        assert!(p.left() < u.app.ui.right_panel_rect.unwrap().left(), "{label}: it opens on the canvas side");

        click(&mut u, at);
        assert!(!open(&u), "{label}: the same row toggles it closed");
    }
}

#[test]
fn the_fields_edit_either_shadow() {
    for (label, kind, salt) in [("Drop shadow", "dropShadow", "shadow"), ("Inner shadow", "innerShadow", "inner_shadow")] {
        let (mut u, r) = rect_with(kind);
        let at = row(&mut u, label);
        click(&mut u, at);
        type_into(&mut u, egui::Id::unique((salt, "y", 0)).with("field"), 1, "12");
        assert!((shadow(&u, r).offset.1 - 12.0).abs() < 1e-9, "{label}: the typed Y moves the shadow: {:?}", shadow(&u, r).offset);
        type_into(&mut u, egui::Id::unique((salt, "radius", 0)).with("field"), 1, "9");
        assert!((shadow(&u, r).radius - 9.0).abs() < 1e-9, "{label}: the typed blur sets the radius");
        type_into(&mut u, egui::Id::unique((salt, "spread", 0)).with("field"), 1, "3");
        assert!((shadow(&u, r).spread - 3.0).abs() < 1e-9, "{label}: the typed spread sets it");
    }
}

#[test]
fn the_colour_well_edits_the_hex_and_the_alpha() {
    for (label, kind, salt) in [("Drop shadow", "dropShadow", "shadow"), ("Inner shadow", "innerShadow", "inner_shadow")] {
        let (mut u, r) = rect_with(kind);
        let at = row(&mut u, label);
        click(&mut u, at);
        let shapes = flat(&u.frame_output(vec![]).shapes);
        let hex = crate::live_tests::text_center(&shapes, "000000");
        click(&mut u, hex);
        u.frame(vec![Event::Text("FF0000".into())]);
        u.frame(vec![key(Key::Enter, Modifiers::NONE, true), key(Key::Enter, Modifiers::NONE, false)]);
        u.frame(vec![]);
        let c = shadow(&u, r).color;
        assert!(c.r > 0.99 && c.g < 0.01 && c.b < 0.01, "{label}: the typed hex is the colour: {c:?}");
        assert!((f64::from(c.a) - 0.25).abs() < 1e-6, "{label}: the hex keeps the opacity");
        type_into(&mut u, egui::Id::unique((salt, "alpha", 0)).with("field"), 2, "50");
        assert!((f64::from(shadow(&u, r).color.a) - 0.5).abs() < 1e-6, "{label}: the typed alpha is 50 %");
    }
}

#[test]
fn the_droplet_opens_the_blend_menu_and_the_x_closes() {
    for (label, kind, panel_id, open) in [
        ("Drop shadow", "dropShadow", "shadow_popover", (|u: &Ui| u.app.ui.shadow_popover.is_some()) as fn(&Ui) -> bool),
        ("Inner shadow", "innerShadow", "inner_shadow_popover", (|u: &Ui| u.app.ui.inner_shadow_popover.is_some()) as fn(&Ui) -> bool),
    ] {
        let (mut u, r) = rect_with(kind);
        let at = row(&mut u, label);
        click(&mut u, at);
        let p = panel(&u, panel_id);
        click(&mut u, Pos2::new(p.max.x - 32.0 - 30.0 + 13.0, p.min.y + 17.0));
        let shapes = flat(&u.frame_output(vec![]).shapes);
        assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == "Multiply")), "{label}: the droplet lists the blend modes");
        click(&mut u, crate::live_tests::text_center(&shapes, "Multiply"));
        assert_eq!(shadow(&u, r).blend, BlendMode::Multiply, "{label}: picking a mode sets it");
        click(&mut u, Pos2::new(p.max.x - 32.0 + 13.0, p.min.y + 17.0));
        assert!(!open(&u), "{label}: the X closes the panel");
    }
}

#[test]
fn a_press_outside_closes_the_popover() {
    let (mut u, _r) = rect_with("dropShadow");
    let at = row(&mut u, "Drop shadow");
    click(&mut u, at);
    u.frame(vec![]);
    click(&mut u, Pos2::new(400.0, 560.0));
    assert!(u.app.ui.shadow_popover.is_none(), "a press on the canvas closes the panel");
}

#[test]
fn the_shadow_panels_take_turns_and_go_with_their_effects() {
    let (mut u, r) = rect_with("dropShadow");
    let drop = row(&mut u, "Drop shadow");
    click(&mut u, drop);
    assert!(u.app.ui.shadow_popover.is_some(), "the Drop shadow panel is up");
    u.app.session.execute("effect.add", &json!({"ids": [r], "type": "innerShadow"})).unwrap();
    u.frame(vec![]);
    let inner = row(&mut u, "Inner shadow");
    click(&mut u, inner);
    assert!(u.app.ui.inner_shadow_popover.is_some(), "the Inner shadow row opens its own");
    assert!(u.app.ui.shadow_popover.is_none(), "and the Drop shadow panel closes");
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 1})).unwrap();
    for _ in 0..2 {
        u.frame(vec![]);
    }
    assert!(u.app.ui.inner_shadow_popover.is_none(), "the panel closes with its effect");
}

#[test]
fn the_other_effect_panels_take_their_turn() {
    let (mut u, r) = rect_with("innerShadow");
    u.app.session.execute("effect.add", &json!({"ids": [r], "type": "layerBlur"})).unwrap();
    u.app.ui.layer_blur_popover = Some((r, 1, Rect::from_min_size(Pos2::new(900.0, 400.0), egui::vec2(150.0, 30.0))));
    for _ in 0..2 {
        u.frame(vec![]);
    }
    let at = row(&mut u, "Inner shadow");
    click(&mut u, at);
    assert!(u.app.ui.inner_shadow_popover.is_some(), "the Inner shadow row opens its panel");
    assert!(u.app.ui.layer_blur_popover.is_none(), "and the Layer blur panel closes");
}
