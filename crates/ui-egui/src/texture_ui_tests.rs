//! The Texture popover: its reference layout, row toggling, independent size fields, corner
//! radius, clipping, and the add-menu route into the same panel.

use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2};
use figcraft_doc::{EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rect_with_texture() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "texture"})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 440.0), Vec2::new(150.0, 30.0))
}

fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.texture_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.frame(vec![]);
}

fn press(p: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

fn texture(u: &Ui, r: NodeId) -> figcraft_doc::Texture {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::Texture(texture) => texture,
        _ => panic!("the effect is Texture"),
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

#[test]
fn the_popover_matches_the_reference_order_and_sits_beside_the_panel() {
    let (mut u, r) = rect_with_texture();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Texture", "Size", "X", "Y", "Corner", "Clip to shape"] {
        assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == label)), "the popover shows {label}");
    }
    assert_eq!(
        shapes.iter().filter(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "0.5")).count(),
        2,
        "X and Y start at 0.5"
    );
    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "4")), "the radius starts at 4");

    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("texture_popover"))).unwrap();
    assert!(panel.left() < u.app.ui.right_panel_rect.unwrap().left(), "the popover opens on the canvas side");
    assert!((panel.height() - 203.0).abs() < 2.0, "the popover matches the reference's 203 pt height (got {})", panel.height());
    for (name, index) in [("sizeX", 0), ("sizeY", 0), ("radius", 0)] {
        let field = u.ctx.read_response(egui::Id::unique(("texture", name, index)).with("field")).expect("the field draws").rect;
        assert!(panel.contains(field.min) && panel.contains(field.max), "{name} sits inside the popover");
        assert!((field.height() - 30.0).abs() < 1.0, "{name} is 30 pt tall");
    }
}

#[test]
fn clicking_the_texture_row_opens_and_closes_the_popover() {
    let (mut u, r) = rect_with_texture();
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Texture");
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.texture_popover.is_some(), "the row opens the popover");
    assert!(u.app.ui.texture_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover draws");
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.texture_popover.is_none(), "the same row toggles it closed");

    open(&mut u, r);
    let close = u.ctx.read_response(egui::Id::unique("texture_close")).expect("the close button draws").rect.center();
    u.frame(vec![Event::PointerMoved(close), press(close, true)]);
    u.frame(vec![press(close, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.texture_popover.is_none(), "the header X closes it");
}

#[test]
fn the_size_fields_are_independent_and_radius_accepts_typing() {
    let (mut u, r) = rect_with_texture();
    open(&mut u, r);
    type_into(&mut u, egui::Id::unique(("texture", "sizeX", 0)), "2");
    let after_x = texture(&u, r);
    assert_eq!((after_x.size, after_x.size_y), (2.0, Some(0.5)), "X changes without moving Y");
    type_into(&mut u, egui::Id::unique(("texture", "sizeY", 0)), "3");
    assert_eq!(texture(&u, r).size_y, Some(3.0), "Y changes on its own");
    type_into(&mut u, egui::Id::unique(("texture", "radius", 0)), "12");
    assert_eq!(texture(&u, r).radius, 12.0, "the corner radius accepts a typed value");
}

#[test]
fn the_clip_checkbox_updates_the_effect() {
    let (mut u, r) = rect_with_texture();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Clip to shape");
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(texture(&u, r).clip_to_shape, "clicking the checkbox turns clipping on");
}

#[test]
fn an_outside_press_closes_the_popover() {
    let (mut u, r) = rect_with_texture();
    open(&mut u, r);
    let outside = Pos2::new(20.0, 20.0);
    u.frame(vec![Event::PointerMoved(outside), press(outside, true)]);
    u.frame(vec![press(outside, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.texture_popover.is_none(), "a press on the canvas closes it");
}

#[test]
fn removing_the_effect_closes_its_popover() {
    let (mut u, r) = rect_with_texture();
    open(&mut u, r);
    assert!(u.app.ui.texture_popover.is_some(), "the popover is open before removing its effect");
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    u.frame(vec![]);
    let effects = u.app.session.doc().unwrap().node(r).unwrap().effects.len();
    let shown = u.app.ui.texture_popover_shown;
    assert!(u.app.ui.texture_popover.is_none(), "the popover closes with its effect ({effects} effects left, shown {shown})");
}

#[test]
fn picking_texture_from_the_add_menu_opens_the_same_popover() {
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
    let texture = crate::live_tests::text_center(&shapes, "Texture");
    u.frame(vec![Event::PointerMoved(texture), press(texture, true), press(texture, false)]);
    u.frame(vec![]);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects.last().map(|effect| &effect.kind), Some(EffectKind::Texture(_))), "Texture is added");
    assert!(u.app.ui.texture_popover.is_some(), "adding Texture opens its popover");
}
