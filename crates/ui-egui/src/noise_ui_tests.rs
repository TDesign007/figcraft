//! The Noise popover: its reference layout, row toggling, the three modes, independent size
//! fields, density and the colour row.

use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2};
use figcraft_doc::{EffectKind, NodeId, Noise, NoiseMode};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rect_with_noise() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "noise"})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 440.0), Vec2::new(150.0, 30.0))
}

fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.noise_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.frame(vec![]);
}

fn press(p: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

fn click(u: &mut Ui, at: Pos2) {
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
}

fn noise(u: &Ui, r: NodeId) -> Noise {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::Noise(noise) => noise,
        _ => panic!("the effect is Noise"),
    }
}

fn type_into(u: &mut Ui, id: egui::Id, text: &str) {
    u.ctx.memory_mut(|m| m.request_focus(id.with("field")));
    u.frame(vec![]);
    u.frame(vec![Event::Text(text.into())]);
    let key = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    u.frame(vec![key(true), key(false)]);
    u.frame(vec![]);
}

fn type_at(u: &mut Ui, at: Pos2, text: &str) {
    click(u, at);
    u.frame(vec![Event::Text(text.into())]);
    let key = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    u.frame(vec![key(true), key(false)]);
    u.frame(vec![]);
}

#[test]
fn the_popover_matches_the_reference_order_and_sits_beside_the_panel() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Noise", "Mono", "Duo", "Multi", "Noise size", "X", "Y", "Density", "Color"] {
        assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == label)), "the popover shows {label}");
    }
    assert_eq!(
        shapes.iter().filter(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "0.5")).count(),
        2,
        "X and Y start at 0.5"
    );
    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text().contains("100%"))), "density starts at 100%");
    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text().contains("25%"))), "the colour starts at 25%");

    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("noise_popover"))).unwrap();
    assert!(panel.left() < u.app.ui.right_panel_rect.unwrap().left(), "the popover opens on the canvas side");
    assert!((panel.width() - 235.0).abs() < 2.0, "the popover matches the reference's width (got {})", panel.width());
    assert!((panel.height() - 222.0).abs() < 3.0, "the popover matches the reference's height (got {})", panel.height());
    for (name, id) in [("sizeX", ("noise", "sizeX", 0)), ("sizeY", ("noise", "sizeY", 0)), ("density", ("noise", "density", 0))] {
        let field = u.ctx.read_response(egui::Id::unique(id).with("field")).expect("the field draws").rect;
        assert!(panel.contains(field.min) && panel.contains(field.max), "{name} sits inside the popover");
        assert!((field.height() - 24.0).abs() < 1.0, "{name} is 24 pt tall");
    }
}

#[test]
fn clicking_the_noise_row_opens_and_closes_the_popover() {
    let (mut u, r) = rect_with_noise();
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Noise");
    click(&mut u, at);
    assert!(u.app.ui.noise_popover.is_some(), "the row opens the popover");
    assert!(u.app.ui.noise_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover draws");
    click(&mut u, at);
    assert!(u.app.ui.noise_popover.is_none(), "the same row toggles it closed");

    open(&mut u, r);
    let close = u.ctx.read_response(egui::Id::unique("noise_close")).expect("the close button draws").rect.center();
    click(&mut u, close);
    assert!(u.app.ui.noise_popover.is_none(), "the header X closes it");
}

#[test]
fn the_tabs_switch_the_noise_mode() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    click(&mut u, crate::live_tests::text_center(&shapes, "Duo"));
    assert_eq!(noise(&u, r).mode, NoiseMode::Duo, "Duo is picked");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert_eq!(
        shapes.iter().filter(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "Color")).count(),
        1,
        "Duo labels its two colour rows once"
    );
    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "FFFFFF")), "Duo shows its second colour");
    click(&mut u, crate::live_tests::text_center(&shapes, "Multi"));
    assert_eq!(noise(&u, r).mode, NoiseMode::Multi, "Multi is picked");
    assert!(u.ctx.read_response(egui::Id::unique(("noise", "opacity", 0)).with("field")).is_some(), "Multi shows the opacity field");
    assert!(u.app.ui.noise_popover.is_some(), "the popover stays open");
}

#[test]
fn the_size_fields_are_independent_and_density_accepts_typing() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    type_into(&mut u, egui::Id::unique(("noise", "sizeX", 0)), "2");
    let after_x = noise(&u, r);
    assert_eq!((after_x.size, after_x.size_y), (2.0, Some(0.5)), "X changes without moving Y");
    type_into(&mut u, egui::Id::unique(("noise", "sizeY", 0)), "3");
    assert_eq!(noise(&u, r).size_y, Some(3.0), "Y changes on its own");
    type_into(&mut u, egui::Id::unique(("noise", "density", 0)), "80");
    assert_eq!(noise(&u, r).density, 0.8, "density accepts a typed percentage");
}

#[test]
fn the_colour_row_edits_the_hex_and_opacity() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    let panel = u.app.ui.noise_popover_pop.as_ref().expect("the popover draws").rect;
    type_at(&mut u, Pos2::new(panel.left() + 135.5, panel.top() + 192.0), "ff0000");
    let after_hex = noise(&u, r);
    assert!((after_hex.color.r - 1.0).abs() < 1e-6 && after_hex.color.g.abs() < 1e-6, "the hex sets the colour (got {})", after_hex.color.to_hex());
    type_at(&mut u, Pos2::new(panel.left() + 194.0, panel.top() + 192.0), "60");
    assert!((noise(&u, r).color.a - 0.6).abs() < 1e-3, "the percentage sets the colour's opacity");
}

#[test]
fn an_outside_press_closes_the_popover() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    let outside = Pos2::new(20.0, 20.0);
    u.frame(vec![Event::PointerMoved(outside), press(outside, true)]);
    u.frame(vec![press(outside, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.noise_popover.is_none(), "a press on the canvas closes it");
}

#[test]
fn the_droplet_switches_the_effects_blend_mode() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    assert_eq!(noise(&u, r).blend.label(), "Normal", "a new noise starts on Normal");
    let panel = u.app.ui.noise_popover_pop.as_ref().expect("the popover draws").rect;
    click(&mut u, Pos2::new(panel.right() - 49.0, panel.top() + 16.5));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Normal", "Multiply", "Saturation", "Color", "Luminosity"] {
        assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == label)), "the droplet menu shows {label}");
    }
    assert!(
        !shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "Pass through")),
        "an effect has no pass-through mode"
    );
    click(&mut u, crate::live_tests::text_center(&shapes, "Multiply"));
    assert_eq!(noise(&u, r).blend.label(), "Multiply", "picking a row sets the effect's blend");
}

#[test]
fn removing_the_effect_closes_its_popover() {
    let (mut u, r) = rect_with_noise();
    open(&mut u, r);
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    u.frame(vec![]);
    assert!(u.app.ui.noise_popover.is_none(), "the popover closes with its effect");
}

#[test]
fn picking_noise_from_the_add_menu_opens_the_same_popover() {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let header = crate::live_tests::text_center(&shapes, "Effects").y;
    let panel = u.app.ui.right_panel_rect.expect("the Design panel draws");
    let add = Pos2::new(panel.right() - 20.0, header);
    u.frame(vec![Event::PointerMoved(add), press(add, true)]);
    u.frame(vec![press(add, false)]);
    for _ in 0..30 {
        u.frame(vec![Event::PointerMoved(Pos2::new(20.0, 20.0))]);
    }
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let noise = crate::live_tests::text_center(&shapes, "Noise");
    u.frame(vec![Event::PointerMoved(noise), press(noise, true), press(noise, false)]);
    u.frame(vec![]);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects.last().map(|effect| &effect.kind), Some(EffectKind::Noise(_))), "Noise is added");
    assert!(u.app.ui.noise_popover.is_some(), "adding Noise opens its popover");
}
