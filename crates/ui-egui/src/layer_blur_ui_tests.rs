//! The Layer blur popover: the row route, its uniform and progressive layouts, the ramp's knobs on
//! the canvas, and its turn with the Background blur panel.

use egui::{Event, Pos2, Rect, Vec2};
use figcraft_doc::{BlurType, EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rect_with_layer_blur() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 100.0, "y": 100.0, "width": 200.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "layerBlur"})).unwrap();
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

fn blur(u: &Ui, r: NodeId) -> figcraft_doc::Blur {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::LayerBlur(b) => b,
        _ => panic!("the effect is Layer blur"),
    }
}

/// The middle of the row's "Layer blur" label.
fn row(u: &mut Ui) -> Pos2 {
    let shapes = flat(&u.frame_output(vec![]).shapes);
    crate::live_tests::text_center(&shapes, "Layer blur")
}

#[test]
fn the_row_opens_the_layer_blur_popover() {
    let (mut u, r) = rect_with_layer_blur();
    let at = row(&mut u);
    click(&mut u, at);
    assert!(u.app.ui.layer_blur_popover.is_some(), "the row opens the popover");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Uniform", "Progressive", "Blur"] {
        assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == label)), "the popover shows {label}");
    }
    assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == "4")), "the radius starts at 4");
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("layer_blur_popover"))).expect("the panel draws");
    assert!((panel.width() - 238.0).abs() < 2.0, "the panel matches the reference's width (got {})", panel.width());
    assert!((panel.height() - 122.0).abs() < 3.0, "the uniform panel matches the reference's height (got {})", panel.height());
    assert!(panel.left() < u.app.ui.right_panel_rect.unwrap().left(), "it opens on the canvas side");

    click(&mut u, at);
    assert!(u.app.ui.layer_blur_popover.is_none(), "the same row toggles it closed");
    let _ = r;
}

#[test]
fn progressive_shows_start_and_end_and_draws_the_ramp() {
    let (mut u, r) = rect_with_layer_blur();
    let at = row(&mut u);
    click(&mut u, at);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    click(&mut u, crate::live_tests::text_center(&shapes, "Progressive"));
    assert_eq!(blur(&u, r).blur_type, BlurType::Progressive, "Progressive is picked");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Start", "End"] {
        assert!(shapes.iter().any(|s| matches!(s, egui::Shape::Text(t) if t.galley.text() == label)), "it shows {label}");
    }
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("layer_blur_popover"))).unwrap();
    assert!((panel.height() - 154.0).abs() < 3.0, "the progressive panel is taller (got {})", panel.height());
    // The ramp's ends are on the canvas: its line joins the layer's top and bottom middles.
    let (a, b) = (page(&u, r, (0.5, 0.0)), page(&u, r, (0.5, 1.0)));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(
        shapes.iter().any(|s| matches!(s, egui::Shape::LineSegment { points, .. } if points[0].distance(a) < 0.6 && points[1].distance(b) < 0.6)),
        "the progressive ramp is drawn across the layer"
    );
}

fn page(u: &Ui, r: NodeId, unit: (f32, f32)) -> Pos2 {
    let doc = u.app.session.doc().unwrap();
    let n = doc.node(r).unwrap().clone();
    let p = doc.world_transform(r) * figcraft_geom::Point::new(f64::from(unit.0) * n.width, f64::from(unit.1) * n.height);
    u.app.to_screen(p.x, p.y)
}

#[test]
fn dragging_a_knob_edits_the_ramp() {
    let (mut u, r) = rect_with_layer_blur();
    let at = row(&mut u);
    click(&mut u, at);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    click(&mut u, crate::live_tests::text_center(&shapes, "Progressive"));
    u.drag_to(page(&u, r, (0.5, 1.0)), page(&u, r, (0.8, 1.4)));
    u.button(page(&u, r, (0.8, 1.4)), false);
    let b = blur(&u, r);
    assert!((b.end.0 - 0.8).abs() < 0.02 && (b.end.1 - 1.4).abs() < 0.02, "the end follows the pointer ({:?})", b.end);
    assert!(u.app.ui.layer_blur_popover.is_some(), "the panel stays open through the drag");
}

#[test]
fn the_two_blur_panels_take_turns() {
    let (mut u, r) = rect_with_layer_blur();
    u.app.session.execute("effect.add", &json!({"ids": [r], "type": "backgroundBlur"})).unwrap();
    u.frame(vec![]);
    u.app.ui.bg_blur_popover = Some((r, 1, Rect::from_min_size(Pos2::new(900.0, 400.0), Vec2::new(150.0, 30.0))));
    u.frame(vec![]);
    u.frame(vec![]);
    assert!(u.app.ui.bg_blur_popover.is_some(), "the Background blur panel is up");
    let at = row(&mut u);
    click(&mut u, at);
    assert!(u.app.ui.layer_blur_popover.is_some(), "the Layer blur row opens its own");
    assert!(u.app.ui.bg_blur_popover.is_none(), "and the other one closes");
}

#[test]
fn removing_the_effect_closes_the_panel() {
    let (mut u, r) = rect_with_layer_blur();
    let at = row(&mut u);
    click(&mut u, at);
    assert!(u.app.ui.layer_blur_popover.is_some());
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    u.frame(vec![]);
    assert!(u.app.ui.layer_blur_popover.is_none(), "the panel closes with its effect");
}
