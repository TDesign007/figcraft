//! Ruler guide drags released off the canvas, driven through egui headless: the gesture ends there,
//! and a later click on the canvas neither adds a guide nor moves one. Also the drag preview: a
//! dashed line across the canvas (past a frame's edges), gone on release, dashed again on hover.

use egui::{Color32, Event, Pos2, Shape, vec2};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn rulers() -> Ui {
    let mut s = Session::new();
    s.execute("view.rulers", &json!({"on": true})).unwrap();
    Ui::new(s)
}

fn offsets(u: &Ui) -> Vec<f64> {
    let ds = u.app.session.ds().unwrap();
    ds.doc.node(ds.page).unwrap().guides().iter().map(|g| g.offset).collect()
}

fn drag(u: &mut Ui, from: Pos2, to: Pos2) {
    u.frame(vec![Event::PointerMoved(from)]);
    u.button(from, true);
    u.frame(vec![Event::PointerMoved(to)]);
    u.button(to, false);
}

fn click(u: &mut Ui, p: Pos2) {
    u.frame(vec![Event::PointerMoved(p)]);
    u.button(p, true);
    u.button(p, false);
}

#[test]
fn a_ruler_drag_released_off_the_canvas_adds_no_guide_on_a_later_click() {
    for left in [false, true] {
        let mut u = rulers();
        let r = u.app.canvas.rect;
        let start = if left { Pos2::new(r.min.x + 5.0, r.min.y + 60.0) } else { Pos2::new(r.min.x + 60.0, r.min.y + 5.0) };
        drag(&mut u, start, Pos2::new(r.min.x - 100.0, r.center().y));
        let released = offsets(&u);
        click(&mut u, r.center() + vec2(30.0, 60.0));
        assert_eq!(offsets(&u), released, "left ruler: {left}");
    }
}

#[test]
fn a_ruler_drag_released_off_the_canvas_ends_even_when_the_next_frame_presses() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    let start = Pos2::new(r.min.x + 60.0, r.min.y + 5.0);
    let off = Pos2::new(r.min.x - 100.0, r.center().y);
    u.frame(vec![Event::PointerMoved(start)]);
    u.button(start, true);
    u.frame(vec![Event::PointerMoved(off)]);
    u.button(off, false);
    let p = r.center() + vec2(30.0, 60.0);
    u.button(p, true);
    u.button(p, false);
    assert!(offsets(&u).is_empty());
}

#[test]
fn a_guide_moved_off_the_canvas_doesnt_follow_a_later_click() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    drag(&mut u, Pos2::new(r.min.x + 60.0, r.min.y + 5.0), r.center());
    assert_eq!(offsets(&u).len(), 1);
    drag(&mut u, r.center(), Pos2::new(r.min.x - 100.0, r.center().y));
    let released = offsets(&u);
    click(&mut u, r.center() + vec2(30.0, 60.0));
    assert_eq!(offsets(&u), released);
}

/// The guide-coloured line segments in `shapes`.
fn guide_segments(shapes: &[egui::Shape]) -> Vec<[Pos2; 2]> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } if stroke.color == Color32::from_rgb(0xF2, 0x48, 0x22) => Some(*points),
            _ => None,
        })
        .collect()
}

#[test]
fn a_guide_dragged_into_a_frame_previews_dashed_across_the_canvas_and_dashes_on_hover() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    let c = u.app.to_page(r.center());
    let f = u
        .app
        .session
        .execute("frame.create", &json!({"x": c.x - 150.0, "y": c.y - 100.0, "width": 300.0, "height": 200.0}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    u.frame(vec![]);
    let inside = u.app.to_screen(c.x, c.y);
    let (fl, fr) = (u.app.to_screen(c.x - 150.0, c.y).x, u.app.to_screen(c.x + 150.0, c.y).x);
    // Drag from the top ruler into the frame and hold: the preview is dashed and runs across the
    // whole canvas (the frame guide's own line would stop at the frame's edges).
    let start = Pos2::new(inside.x, r.min.y + 5.0);
    u.frame(vec![Event::PointerMoved(start)]);
    u.button(start, true);
    u.frame(vec![Event::PointerMoved(inside)]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at_y: Vec<[Pos2; 2]> = guide_segments(&shapes)
        .into_iter()
        .filter(|[a, b]| (a.y - b.y).abs() < 0.5 && (a.y - inside.y).abs() < 1.5)
        .collect();
    assert!(at_y.len() > 4, "the preview is a dashed line: {at_y:?}");
    let left = at_y.iter().map(|[a, b]| a.x.min(b.x)).fold(f32::INFINITY, f32::min);
    let right = at_y.iter().map(|[a, b]| a.x.max(b.x)).fold(f32::NEG_INFINITY, f32::max);
    assert!(left <= r.min.x + 1.0 && right >= r.max.x - 1.0, "past the frame to the canvas' edges: {left}..{right} (frame {fl}..{fr})");
    // Release: the dashed preview goes and the guide is kept, spanning only its frame.
    u.button(inside, false);
    u.frame(vec![]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let kept: Vec<[Pos2; 2]> = guide_segments(&shapes).into_iter().filter(|[a, b]| (a.y - b.y).abs() < 0.5).collect();
    assert!(kept.iter().all(|[a, b]| a.x.min(b.x) >= fl - 1.0 && a.x.max(b.x) <= fr + 1.0), "the guide spans its frame only: {kept:?}");
    assert!(!kept.is_empty(), "the guide is drawn");
    // Hover the guide: dashed again (the hover state), inside the frame.
    u.frame(vec![Event::PointerMoved(Pos2::new(inside.x, inside.y + 1.0))]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let hovered = guide_segments(&shapes).into_iter().filter(|[a, b]| (a.y - b.y).abs() < 0.5 && (a.y - inside.y).abs() < 2.0).count();
    assert!(hovered > 4, "hovering shows the dashed line again ({hovered} dashes)");
    let _ = f;
}

#[test]
fn a_ruler_drag_released_on_the_canvas_adds_one_guide() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    drag(&mut u, Pos2::new(r.min.x + 60.0, r.min.y + 5.0), r.center());
    assert_eq!(offsets(&u).len(), 1);
    click(&mut u, r.center() + vec2(30.0, 60.0));
    assert_eq!(offsets(&u).len(), 1);
    u.app.session.execute("edit.undo", &json!({})).unwrap();
    assert!(offsets(&u).is_empty());
}
