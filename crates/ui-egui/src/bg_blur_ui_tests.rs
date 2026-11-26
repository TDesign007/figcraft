//! The Background blur popover: uniform and progressive layouts, its reference measurements,
//! row toggling, the add-menu route and its fields.

use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2};
use figcraft_doc::{BlurType, EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rect_with_blur() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "backgroundBlur"})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 440.0), Vec2::new(150.0, 30.0))
}

fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.bg_blur_popover = Some((r, 0, row_rect()));
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

fn blur(u: &Ui, r: NodeId) -> figcraft_doc::Blur {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::BackgroundBlur(blur) => blur,
        _ => panic!("the effect is Background blur"),
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
fn the_uniform_popover_matches_the_reference() {
    let (mut u, r) = rect_with_blur();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Background blur", "Uniform", "Progressive", "Blur"] {
        assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == label)), "the popover shows {label}");
    }
    assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == "4")), "the radius starts at 4");

    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("bg_blur_popover"))).unwrap();
    assert!(panel.left() < u.app.ui.right_panel_rect.unwrap().left(), "the popover opens on the canvas side");
    assert!((panel.width() - 238.0).abs() < 2.0, "the popover matches the reference's width (got {})", panel.width());
    assert!((panel.height() - 122.0).abs() < 3.0, "the uniform popover matches the reference's height (got {})", panel.height());
    let field = u.ctx.read_response(egui::Id::unique(("bg_blur", "radius", 0)).with("field")).expect("the field draws").rect;
    assert!(panel.contains(field.min) && panel.contains(field.max), "the radius field sits inside the popover");
}

#[test]
fn clicking_the_background_blur_row_opens_and_closes_the_popover() {
    let (mut u, r) = rect_with_blur();
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Background blur");
    click(&mut u, at);
    assert!(u.app.ui.bg_blur_popover.is_some(), "the row opens the popover");
    assert!(u.app.ui.bg_blur_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover draws");
    click(&mut u, at);
    assert!(u.app.ui.bg_blur_popover.is_none(), "the same row toggles it closed");

    open(&mut u, r);
    let close = u.ctx.read_response(egui::Id::unique(("bg_blur", "close"))).expect("the close button draws").rect.center();
    click(&mut u, close);
    assert!(u.app.ui.bg_blur_popover.is_none(), "the header X closes it");
}

#[test]
fn the_progressive_tab_switches_to_start_and_end_fields() {
    let (mut u, r) = rect_with_blur();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    click(&mut u, crate::live_tests::text_center(&shapes, "Progressive"));
    assert_eq!(blur(&u, r).blur_type, BlurType::Progressive, "Progressive is picked");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Start", "End"] {
        assert!(shapes.iter().any(|shape| matches!(shape, egui::Shape::Text(text) if text.galley.text() == label)), "Progressive shows {label}");
    }
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("bg_blur_popover"))).unwrap();
    assert!((panel.height() - 154.0).abs() < 3.0, "the progressive popover matches the reference's height (got {})", panel.height());
    assert_eq!((blur(&u, r).start_radius, blur(&u, r).radius), (0.0, 4.0), "the ramp starts at 0 and ends at 4");
    assert!(u.app.ui.bg_blur_popover.is_some(), "the popover stays open");
}

#[test]
fn the_radius_and_ramp_fields_accept_typing() {
    let (mut u, r) = rect_with_blur();
    open(&mut u, r);
    type_into(&mut u, egui::Id::unique(("bg_blur", "radius", 0)), "12");
    assert_eq!(blur(&u, r).radius, 12.0, "the uniform radius accepts a typed value");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    click(&mut u, crate::live_tests::text_center(&shapes, "Progressive"));
    type_into(&mut u, egui::Id::unique(("bg_blur", "start", 0)), "2");
    type_into(&mut u, egui::Id::unique(("bg_blur", "end", 0)), "8");
    assert_eq!((blur(&u, r).start_radius, blur(&u, r).radius), (2.0, 8.0), "start and end are independent");
}

#[test]
fn an_outside_press_and_removing_the_effect_close_it() {
    let (mut u, r) = rect_with_blur();
    open(&mut u, r);
    // Empty canvas, clear of the navigation bar's home button at the top-left corner.
    let outside = Pos2::new(700.0, 780.0);
    click(&mut u, outside);
    assert!(u.app.ui.bg_blur_popover.is_none(), "a press on the canvas closes it");

    open(&mut u, r);
    assert!(u.app.ui.bg_blur_popover_shown, "the popover draws again");
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    u.frame(vec![]);
    let left = u.app.session.doc().unwrap().node(r).unwrap().effects.len();
    assert!(
        u.app.ui.bg_blur_popover.is_none(),
        "the popover closes with its effect ({left} effects left, shown {})",
        u.app.ui.bg_blur_popover_shown
    );
}

#[test]
fn picking_background_blur_from_the_add_menu_opens_the_popover() {
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
    let entry = crate::live_tests::text_center(&shapes, "Background blur");
    u.frame(vec![Event::PointerMoved(entry), press(entry, true), press(entry, false)]);
    u.frame(vec![]);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects.last().map(|effect| &effect.kind), Some(EffectKind::BackgroundBlur(_))), "Background blur is added");
    assert!(u.app.ui.bg_blur_popover.is_some(), "adding Background blur opens its popover");
}
