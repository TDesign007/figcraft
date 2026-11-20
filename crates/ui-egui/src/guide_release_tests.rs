//! Ruler guide drags released off the canvas, driven through egui headless: the gesture ends there,
//! and a later click on the canvas neither adds a guide nor moves one. Also the drag preview: a
//! dashed line across the canvas (past a frame's edges), gone on release, dashed again on hover.

use egui::{Color32, Event, Pos2, Shape, vec2};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

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
    guide_segments_width(shapes).into_iter().map(|(p, _)| p).collect()
}

/// The guide-coloured line segments with their stroke widths.
fn guide_segments_width(shapes: &[egui::Shape]) -> Vec<([Pos2; 2], f32)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } if stroke.color == Color32::from_rgb(0xF2, 0x48, 0x22) => Some((*points, stroke.width)),
            _ => None,
        })
        .collect()
}

/// The page's guides' orientations (true: vertical lines, from the left ruler).
fn orientations(u: &Ui) -> Vec<bool> {
    let ds = u.app.session.ds().unwrap();
    ds.doc.node(ds.page).unwrap().guides().iter().map(|g| g.vertical).collect()
}

/// A secondary click at `at`.
fn right_click(u: &mut Ui, at: Pos2) {
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed, modifiers: egui::Modifiers::NONE };
    u.frame(vec![Event::PointerMoved(at)]);
    u.frame(vec![button(true)]);
    u.frame(vec![button(false)]);
    u.frame(vec![]);
}

#[test]
fn right_clicking_a_ruler_offers_removing_its_guides_and_hiding_the_rulers() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    // One guide from each ruler: a horizontal line (top) and a vertical one (left).
    drag(&mut u, Pos2::new(r.min.x + 100.0, r.min.y + 5.0), Pos2::new(r.min.x + 140.0, r.center().y));
    drag(&mut u, Pos2::new(r.min.x + 5.0, r.min.y + 120.0), Pos2::new(r.center().x, r.min.y + 160.0));
    assert_eq!(orientations(&u).iter().filter(|v| **v).count(), 1, "{:?}", orientations(&u));
    assert_eq!(orientations(&u).iter().filter(|v| !**v).count(), 1, "{:?}", orientations(&u));
    // The top ruler's menu: remove its (horizontal) guides, or hide the rulers.
    right_click(&mut u, Pos2::new(r.min.x + 100.0, r.min.y + 8.0));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Remove all horizontal guides"), "the top ruler removes horizontal guides");
    assert!(has_text(&shapes, "Hide rulers"));
    let item = crate::live_tests::text_center(&shapes, "Remove all horizontal guides");
    u.button(item, true);
    u.button(item, false);
    u.frame(vec![]);
    assert_eq!(orientations(&u), vec![true], "the horizontal guide went, the vertical stays");
    // The left ruler's menu names the vertical guides, and Hide rulers turns them off.
    right_click(&mut u, Pos2::new(r.min.x + 8.0, r.min.y + 120.0));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Remove all vertical guides"), "the left ruler removes vertical guides");
    let hide = crate::live_tests::text_center(&shapes, "Hide rulers");
    u.button(hide, true);
    u.button(hide, false);
    u.frame(vec![]);
    assert!(!u.app.session.ds().unwrap().view.rulers, "Hide rulers turns the rulers off");
}

#[test]
fn a_guide_dragged_into_a_frame_is_solid_inside_and_dashed_past_it() {
    let mut u = rulers();
    let r = u.app.canvas.rect;
    let c = u.app.to_page(r.center());
    u.app
        .session
        .execute("frame.create", &json!({"x": c.x - 150.0, "y": c.y - 100.0, "width": 300.0, "height": 200.0}))
        .unwrap();
    u.frame(vec![]);
    let inside = u.app.to_screen(c.x, c.y);
    let (fl, fr) = (u.app.to_screen(c.x - 150.0, c.y).x, u.app.to_screen(c.x + 150.0, c.y).x);
    let row = |shapes: &[egui::Shape]| -> Vec<[Pos2; 2]> {
        guide_segments(shapes).into_iter().filter(|[a, b]| (a.y - b.y).abs() < 0.5 && (a.y - inside.y).abs() < 1.5).collect()
    };
    // Drag from the top ruler into the frame and hold: solid inside the frame, dashed past it.
    let start = Pos2::new(inside.x, r.min.y + 5.0);
    u.frame(vec![Event::PointerMoved(start)]);
    u.button(start, true);
    u.frame(vec![Event::PointerMoved(inside)]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let dragged = row(&shapes);
    let solid: Vec<&[Pos2; 2]> = dragged.iter().filter(|[a, b]| (a.x - b.x).abs() > 50.0).collect();
    assert_eq!(solid.len(), 1, "one solid line inside the frame: {dragged:?}");
    let [a, b] = *solid[0];
    assert!((a.x.min(b.x) - fl).abs() < 1.5 && (a.x.max(b.x) - fr).abs() < 1.5, "it spans the frame: {a:?}-{b:?} (frame {fl}..{fr})");
    let left = dragged.iter().filter(|[a, b]| a.x.max(b.x) < fl - 1.0).count();
    let right = dragged.iter().filter(|[a, b]| a.x.min(b.x) > fr + 1.0).count();
    assert!(left > 2 && right > 2, "dashed past both frame edges: left {left}, right {right}");
    // Release and move off: no dashes outside the frame; the guide is the frame's solid line.
    u.button(inside, false);
    u.frame(vec![Event::PointerMoved(inside + vec2(0.0, -40.0))]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let kept = row(&shapes);
    assert!(kept.iter().all(|[a, b]| a.x.min(b.x) >= fl - 1.0 && a.x.max(b.x) <= fr + 1.0), "nothing outside the frame: {kept:?}");
    assert!(kept.iter().any(|[a, b]| (a.x - b.x).abs() > 50.0), "the solid guide is drawn: {kept:?}");
    // Hover: the inside stays solid — same width, not thickened — and the outside turns dashed.
    u.frame(vec![Event::PointerMoved(Pos2::new(inside.x, inside.y + 1.0))]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let hovered = row(&shapes);
    assert!(hovered.iter().any(|[a, b]| (a.x - b.x).abs() > 50.0), "the solid part stays: {hovered:?}");
    let widths: Vec<f32> = guide_segments_width(&shapes)
        .into_iter()
        .filter(|([a, b], _)| (a.y - b.y).abs() < 0.5 && (a.y - inside.y).abs() < 1.5 && (a.x - b.x).abs() > 50.0)
        .map(|(_, w)| w)
        .collect();
    assert!(widths.iter().all(|w| (*w - 1.0).abs() < 0.01), "the inside line keeps its 1 pt width on hover: {widths:?}");
    let left = hovered.iter().filter(|[a, b]| a.x.max(b.x) < fl - 1.0).count();
    let right = hovered.iter().filter(|[a, b]| a.x.min(b.x) > fr + 1.0).count();
    assert!(left > 2 && right > 2, "hover dashes the parts outside the frame: left {left}, right {right}");
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
