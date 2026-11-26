//! The Glass popover: its reference layout, the row toggle, the preview's light-angle gesture,
//! the parameter sliders (one undo step), and closing with the effect it edits.

use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2};
use figcraft_doc::{EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

fn rect_with_glass() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "glass"})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 440.0), Vec2::new(150.0, 30.0))
}

fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.glass_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.frame(vec![]);
}

fn press(p: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

fn glass(u: &Ui, r: NodeId) -> figcraft_doc::Glass {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::Glass(g) => g,
        _ => panic!("the effect is Glass"),
    }
}

#[test]
fn the_popover_matches_the_reference_order_beside_the_panel() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Glass", "Light", "Refraction", "Depth", "Dispersion", "Frost", "Splay"] {
        assert!(has_text(&shapes, label), "the popover shows {label}");
    }
    for value in ["-45°", "80", "20", "50", "4", "0"] {
        assert!(has_text(&shapes, value), "the default {value} is shown");
    }
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("glass_popover"))).unwrap();
    assert!(panel.left() < u.app.ui.right_panel_rect.unwrap().left(), "the popover opens on the canvas side");
    let preview = u.app.ui.glass_preview_rect.expect("the preview is drawn");
    assert!(panel.contains(preview.min) && panel.contains(preview.max));
    assert_eq!(u.app.ui.glass_slider_rects.len(), 5, "one well per parameter");
    assert!(u.app.ui.glass_slider_rects.iter().all(|r| panel.contains(r.min) && panel.contains(r.max)));
}

#[test]
fn clicking_the_glass_row_opens_and_closes_the_popover() {
    let (mut u, r) = rect_with_glass();
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Glass");
    u.frame(vec![egui::Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.glass_popover.is_some(), "the row opens the popover");
    assert!(u.app.ui.glass_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover draws");
    u.frame(vec![egui::Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.glass_popover.is_none(), "the same row toggles it closed");

    open(&mut u, r);
    let close = u.ctx.read_response(egui::Id::unique("glass_close")).expect("the close button draws").rect.center();
    u.frame(vec![egui::Event::PointerMoved(close), press(close, true)]);
    u.frame(vec![press(close, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.glass_popover.is_none(), "the header X closes it");
}

#[test]
fn pressing_outside_closes_the_popover() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let outside = Pos2::new(20.0, 20.0);
    u.frame(vec![Event::PointerMoved(outside), press(outside, true)]);
    u.frame(vec![press(outside, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.glass_popover.is_none(), "a press on the canvas closes it");
}

#[test]
fn the_refraction_slider_scrubs_in_one_undo_step() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let well = u.app.ui.glass_slider_rects[0];
    let left = well.left() + 10.0;
    let right = well.right() - 10.0;
    let start = Pos2::new(left + 0.1 * (right - left), well.center().y);
    let end = Pos2::new(left + 0.75 * (right - left), well.center().y);
    u.frame(vec![egui::Event::PointerMoved(start), press(start, true)]);
    u.frame(vec![egui::Event::PointerMoved(end)]);
    u.frame(vec![press(end, false)]);
    u.frame(vec![]);
    assert!((glass(&u, r).refraction - 0.75).abs() < 0.02, "the drag sets refraction");
    u.app.session.undo().unwrap();
    assert!((glass(&u, r).refraction - 0.8).abs() < 1e-9, "the whole drag undoes once");
}

#[test]
fn every_slider_uses_the_brand_green_for_its_progress() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for well in &u.app.ui.glass_slider_rects {
        assert!(
            shapes.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(rect)
                    if well.contains_rect(rect.rect)
                        && rect.rect.height() <= 5.0
                        && rect.fill.g() > rect.fill.r()
                        && rect.fill.g() > rect.fill.b()
                        && rect.fill.a() > 0
            )),
            "the slider progress inside {well:?} is green"
        );
    }
}

#[test]
fn typing_into_a_focused_value_field_updates_the_effect() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let id = egui::Id::unique(("glass", "refraction", 0));
    u.ctx.memory_mut(|m| m.request_focus(id.with("field")));
    u.frame(vec![]);
    u.frame(vec![Event::Text("35".into())]);
    let event = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    u.frame(vec![event(true), event(false)]);
    u.frame(vec![]);
    let refraction = glass(&u, r).refraction;
    assert!((refraction - 0.35).abs() < 1e-9, "the typed percentage sets refraction: {refraction}");
}

#[test]
fn dragging_the_preview_changes_the_light_angle_and_closing_with_the_effect_closes_it() {
    let (mut u, r) = rect_with_glass();
    open(&mut u, r);
    let preview = u.app.ui.glass_preview_rect.expect("the preview is drawn");
    let start = crate::panels::right::glass::flashlight_rect(preview, -45.0).center();
    let end = preview.center() + Vec2::new(-80.0, 80.0);
    u.frame(vec![egui::Event::PointerMoved(start), press(start, true)]);
    u.frame(vec![egui::Event::PointerMoved(end)]);
    u.frame(vec![press(end, false)]);
    u.frame(vec![]);
    let angle = glass(&u, r).light_angle;
    assert!((angle + 135.0).abs() < 1e-9, "dragging lower-left points the light at the lower-left: {angle}");
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    assert!(u.app.ui.glass_popover.is_none(), "the popover closes with its effect");
}

#[test]
fn the_flashlight_turns_a_full_circle_around_the_fixed_object_centre() {
    let preview = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::splat(58.0));
    let centre = preview.center();
    for (pointer, want) in [
        (centre + Vec2::new(0.0, -17.0), 0.0),
        (centre + Vec2::new(17.0, 0.0), 90.0),
        (centre + Vec2::new(0.0, 17.0), 180.0),
        (centre + Vec2::new(-17.0, 0.0), -90.0),
    ] {
        assert_eq!(crate::panels::right::glass::light_angle_at(centre, pointer), want);
        let flashlight = crate::panels::right::glass::flashlight_rect(preview, want);
        assert!((flashlight.center().distance(centre) - 17.0).abs() < 0.001, "the flashlight keeps its orbit");
        assert!((flashlight.center() - pointer).length() < 0.001, "the flashlight follows the pointer for {want}°");
        let beam = crate::panels::right::glass::light_beam_polygon(preview, want);
        assert!(preview.contains(beam[0]) && preview.contains(beam[1]), "the light starts at the flashlight for {want}°");
        assert!((beam[2].distance(beam[3]) - 11.0).abs() < 0.001, "the light ends on the fixed object centre for {want}°");
        let start_mid = beam[0] + (beam[1] - beam[0]) / 2.0;
        let end_mid = beam[2] + (beam[3] - beam[2]) / 2.0;
        let light = start_mid - end_mid;
        let orbit = flashlight.center() - centre;
        assert!((light.x * orbit.y - light.y * orbit.x).abs() < 0.001, "the light follows the flashlight for {want}°");
    }
}

#[test]
fn picking_glass_from_the_add_menu_opens_the_same_popover() {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let header = crate::live_tests::text_center(&shapes, "Effects").y;
    let panel = u.app.ui.right_panel_rect.expect("the Design panel draws");
    let add = Pos2::new(panel.right() - 20.0, header);
    u.frame(vec![egui::Event::PointerMoved(add), press(add, true)]);
    u.frame(vec![press(add, false)]);
    for _ in 0..30 {
        u.frame(vec![egui::Event::PointerMoved(Pos2::new(20.0, 20.0))]);
    }
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let glass = crate::live_tests::text_center(&shapes, "Glass");
    u.frame(vec![egui::Event::PointerMoved(glass), press(glass, true), press(glass, false)]);
    u.frame(vec![]);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects.last().map(|e| &e.kind), Some(EffectKind::Glass(_))), "Glass is added");
    assert!(u.app.ui.glass_popover.is_some(), "adding Glass opens its popover");
}
