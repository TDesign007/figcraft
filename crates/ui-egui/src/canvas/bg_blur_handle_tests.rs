//! The Background blur popover's progressive ramp shows on the canvas as a two-ended line, and
//! either end drags the ramp's start or end.

use egui::{Pos2, vec2};
use figcraft_doc::{Blur, BlurType, EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;

fn rect_with_progressive_blur() -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 100.0, "y": 100.0, "width": 200.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "backgroundBlur"})).unwrap();
    let mut effect = s.doc().unwrap().node(r).unwrap().effects[0].clone();
    if let EffectKind::BackgroundBlur(b) = &mut effect.kind {
        b.blur_type = BlurType::Progressive;
    }
    s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": effect})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.bg_blur_popover = Some((r, 0, egui::Rect::from_min_size(Pos2::new(900.0, 400.0), vec2(150.0, 30.0))));
    u.frame(vec![]);
    u.frame(vec![]);
}

fn blur(u: &Ui, r: NodeId) -> Blur {
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    match node.effects[0].kind.clone() {
        EffectKind::BackgroundBlur(b) => b,
        _ => panic!("the effect is Background blur"),
    }
}

/// Where the ramp's `u` unit point sits in page coordinates.
fn page(u: &Ui, r: NodeId, unit: (f32, f32)) -> Pos2 {
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let at = figcraft_geom::Point::new(f64::from(unit.0) * n.width, f64::from(unit.1) * n.height);
    let p = u.app.session.doc().unwrap().world_transform(r) * at;
    u.app.to_screen(p.x, p.y)
}

#[test]
fn the_progressive_ramp_shows_a_two_ended_line_while_the_popover_is_open() {
    let (mut u, r) = rect_with_progressive_blur();
    open(&mut u, r);
    let (a, b) = (page(&u, r, (0.5, 0.0)), page(&u, r, (0.5, 1.0)));
    let knobs = |shapes: &[egui::Shape]| {
        shapes
            .iter()
            .filter(|s| {
                matches!(s, egui::Shape::Circle(c) if (c.radius - 3.5).abs() < 0.01
                    && c.fill == egui::Color32::WHITE
                    && (c.center.distance(a) < 1.0 || c.center.distance(b) < 1.0))
            })
            .count()
    };
    let shapes = crate::live_tests::flat(&u.frame_output(vec![]).shapes);
    assert!(knobs(&shapes) >= 2, "the ramp's two ends are drawn");
    assert!(shapes.iter().any(|s| matches!(s, egui::Shape::LineSegment { points, .. } if points[0].distance(a) < 0.6 && points[1].distance(b) < 0.6)), "a line joins the two ends");

    u.app.ui.bg_blur_popover = None;
    u.frame(vec![]);
    let shapes = crate::live_tests::flat(&u.frame_output(vec![]).shapes);
    assert_eq!(knobs(&shapes), 0, "closing the popover takes the line away");
}

#[test]
fn either_end_drags_the_ramp() {
    let (mut u, r) = rect_with_progressive_blur();
    open(&mut u, r);
    // The end knob: drag it out past the layer's bottom right corner.
    u.drag_to(page(&u, r, (0.5, 1.0)), page(&u, r, (0.75, 1.5)));
    u.button(page(&u, r, (0.75, 1.5)), false);
    let b = blur(&u, r);
    assert!((b.end.0 - 0.75).abs() < 0.02 && (b.end.1 - 1.5).abs() < 0.02, "the end follows the pointer ({:?})", b.end);
    assert_eq!((b.start.0, b.start.1), (0.5, 0.0), "the start stays put");
    assert!(u.app.ui.bg_blur_popover.is_some(), "dragging a knob keeps the popover open");

    // The start knob, from where it still is: drag it up out of the layer's top edge.
    u.drag_to(page(&u, r, (0.5, 0.0)), page(&u, r, (0.25, -0.5)));
    u.button(page(&u, r, (0.25, -0.5)), false);
    let b = blur(&u, r);
    assert!((b.start.0 - 0.25).abs() < 0.02 && (b.start.1 + 0.5).abs() < 0.02, "the start follows the pointer ({:?})", b.start);
    assert!((b.end.0 - 0.75).abs() < 0.02, "the end stays where it was left");
    assert!(u.app.ui.bg_blur_popover.is_some(), "the popover is still open after both drags");
}

#[test]
fn the_ends_hide_and_stop_editing_once_the_popover_closes() {
    let (mut u, r) = rect_with_progressive_blur();
    open(&mut u, r);
    let end = page(&u, r, (0.5, 1.0));
    u.app.ui.bg_blur_popover = None;
    u.frame(vec![]);
    u.drag_to(end, end + vec2(60.0, 60.0));
    u.button(end + vec2(60.0, 60.0), false);
    assert_eq!((blur(&u, r).end.0, blur(&u, r).end.1), (0.5, 1.0), "with no popover the point is just canvas");
}
