//! The Design panel's effect rows for the 3D transform and the Perspective: the row names the
//! effect, the type menu lists both among the effect types, and the opened settings show the
//! fields (the turns, the depth, the backface; the camera distance).

use egui::{Pos2, Shape};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

fn rect_with_effect(effect: &serde_json::Value) -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": effect["type"]})).unwrap();
    if effect.get("rotateX").is_some() || effect.get("distance").is_some() {
        s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": effect})).unwrap();
    }
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    // The row's settings open (the gear's flag, as the panel keeps it).
    u.ctx.data_mut(|d| d.insert_temp(egui::Id::unique(("fx_open", 0usize)), true));
    u.frame(vec![]);
    (u, r)
}

#[test]
fn the_3d_transform_row_shows_its_settings() {
    let (mut u, _r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 30.0, "rotateY": -40.0, "depth": 60.0, "backface": false}));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "3D transform"), "the row names the effect");
    // The opened settings: the two turns (their degree suffix), the depth and the backface.
    assert!(has_text(&shapes, "Backface"), "the backface setting shows");
    assert!(has_text(&shapes, "Depth"), "the depth field shows");
    // The turned values are there (the fields show them with a degree suffix).
    assert!(has_text(&shapes, "30°"), "the X turn's value");
    assert!(has_text(&shapes, "-40°"), "the Y turn's value");
}

#[test]
fn the_perspective_row_shows_its_distance() {
    let (mut u, _r) = rect_with_effect(&json!({"type": "perspective", "distance": 600.0}));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Perspective"), "the row names the effect");
    assert!(has_text(&shapes, "Distance"), "the camera distance field shows");
}

#[test]
fn the_type_menu_lists_the_new_effects() {
    let (mut u, _r) = rect_with_effect(&json!({"type": "layerBlur"}));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    // The row's type dropdown opens the list of effect types.
    let label = crate::live_tests::text_center(&shapes, "Layer blur");
    let at = Pos2::new(label.x, label.y);
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
    u.frame(vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }]);
    let shapes: Vec<Shape> = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "3D transform"), "the menu lists the 3D transform");
    assert!(has_text(&shapes, "Perspective"), "the menu lists the Perspective");
}
