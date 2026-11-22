//! The Perspective popover: the header (the title, the frame's name badge, the close), the
//! depth-cards preview drawn beside the row, the adaptive distance slider (scrubbed, its
//! double-click reset, one undo step per gesture) with the vanishing-point toggle, and the
//! add menu opening it in the same click.

use egui::{Pos2, Rect, Shape, Vec2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

/// A 300 × 200 rectangle with the Perspective effect at `distance`, selected, its row's
/// settings open (the panel state the rows keep).
fn rect_with_perspective(distance: f64) -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": "perspective"})).unwrap();
    s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "perspective", "distance": distance}})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    u.ctx.data_mut(|d| d.insert_temp(egui::Id::unique(("fx_open", 0usize)), true));
    u.frame(vec![]);
    (u, r)
}

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 470.0), Vec2::new(150.0, 30.0))
}

/// Open the popover on the layer's effect row and settle it (the first frame lays it out
/// invisibly; the second draws it).
fn open(u: &mut Ui, r: NodeId) {
    u.app.ui.persp_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.frame(vec![]);
}

fn press(p: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

#[test]
fn the_popover_opens_beside_the_row_with_the_preview_and_the_distance() {
    let (mut u, r) = rect_with_perspective(500.0);
    open(&mut u, r);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Perspective"), "the popover's title");
    assert!(has_text(&shapes, "500"), "the precise distance field");
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("persp_popover"))).unwrap();
    let preview = u.app.ui.persp_preview_rect.unwrap_or_else(|| panic!("the preview is drawn"));
    assert!(panel.contains(preview.min) && panel.contains(preview.max), "the preview sits inside the panel: {preview:?} vs {panel:?}");
    let slider = u.app.ui.persp_slider_rect.unwrap_or_else(|| panic!("the slider is drawn"));
    assert!(panel.contains(slider.min) && panel.contains(slider.max), "the slider sits inside the panel");
    assert!(u.app.ui.persp_preview.is_some(), "the depth cards' texture is built");
}

#[test]
fn the_slider_scrubs_the_distance_in_one_undo_step() {
    let (mut u, r) = rect_with_perspective(800.0);
    open(&mut u, r);
    let slider = u.app.ui.persp_slider_rect.expect("the slider is drawn");
    // The track inside the well: 10 pt in from the left, ending short of the toggle (16 pt
    // centred at the right end, 8 pt of gap).
    let track_min = slider.min.x + 10.0;
    let track_max = slider.max.x - 32.0;
    let at = |m: f32| Pos2::new(track_min + m * (track_max - track_min), slider.center().y);
    // Pressing the track's left third and dragging right scrubs toward the far end (a larger
    // distance than the default mid-track camera).
    let right = at(0.35);
    u.frame(vec![egui::Event::PointerMoved(right), press(right, true)]);
    u.frame(vec![egui::Event::PointerMoved(right + egui::vec2(30.0, 0.0))]);
    u.frame(vec![press(right + egui::vec2(30.0, 0.0), false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Perspective(p) = &n.effects[0].kind else { panic!("no perspective") };
    assert!(p.distance > 800.0, "the right side of the track scrubs the camera away: {}", p.distance);
    // The gesture was one undo step.
    u.app.session.undo().unwrap();
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Perspective(p) = &n.effects[0].kind else { panic!("no perspective") };
    assert_eq!(p.distance, 800.0, "the scrub undoes as one step");
}

#[test]
fn a_double_click_on_the_track_falls_back_to_the_default_camera() {
    let (mut u, r) = rect_with_perspective(5000.0);
    open(&mut u, r);
    let slider = u.app.ui.persp_slider_rect.expect("the slider is drawn");
    let track_min = slider.min.x + 10.0;
    let track_max = slider.max.x - 32.0;
    let at = Pos2::new(track_min + 0.3 * (track_max - track_min), slider.center().y);
    u.frame(vec![egui::Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Perspective(p) = &n.effects[0].kind else { panic!("no perspective") };
    assert_eq!(p.distance, 800.0, "the double-click restores the default camera");
}

#[test]
fn the_square_toggle_shows_the_vanishing_point_on_the_canvas() {
    let (mut u, r) = rect_with_perspective(500.0);
    open(&mut u, r);
    assert!(!u.app.ui.persp_vp, "the marker starts hidden");
    let slider = u.app.ui.persp_slider_rect.expect("the slider is drawn");
    let toggle = Pos2::new(slider.max.x - 16.0, slider.center().y);
    u.frame(vec![egui::Event::PointerMoved(toggle), press(toggle, true)]);
    u.frame(vec![press(toggle, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.persp_vp, "the toggle turns the marker on");
    // The marker: a small ring at the frame's centre, on the canvas.
    let doc = u.app.session.doc().unwrap().clone();
    let c = doc.world_transform(r) * figcraft_geom::Point::new(150.0, 100.0);
    let centre = u.app.to_screen(c.x, c.y);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let ring = shapes.iter().any(|s| matches!(s, Shape::Circle(g) if (g.center - centre).length() < 1.0 && g.radius > 4.0 && g.radius < 6.0));
    assert!(ring, "the vanishing-point ring is drawn at the frame's centre");
    // Toggling again hides it.
    u.frame(vec![egui::Event::PointerMoved(toggle), press(toggle, true)]);
    u.frame(vec![press(toggle, false)]);
    u.frame(vec![]);
    assert!(!u.app.ui.persp_vp, "the toggle turns the marker off");
}

#[test]
fn the_slider_range_adapts_to_the_frame_size() {
    // The reference's adaptive range: a safe nearest camera scales with the frame's diagonal,
    // the far end is where the perspective barely reads.
    let diag = 300.0f64.hypot(200.0);
    let (near, far) = crate::panels::right::perspective::range(300.0, 200.0);
    assert!((near - diag * 0.12).abs() < 0.1, "the nearest camera is a share of the diagonal: {near}");
    assert!((far - diag * 50.0).abs() < 1.0, "the far end is the diagonal's multiple: {far}");
    let (small_near, small_far) = crate::panels::right::perspective::range(40.0, 40.0);
    assert!(small_near < near && small_far < far, "a smaller frame reads a nearer range: {small_near} {small_far}");
    // The slider maps logarithmically, so the default camera lands mid-track whatever the size.
    for (w, h) in [(300.0, 200.0), (40.0, 40.0), (2000.0, 1500.0)] {
        let (min, max) = crate::panels::right::perspective::range(w, h);
        let m = ((800.0f64.ln() - min.ln()) / (max.ln() - min.ln())).clamp(0.0, 1.0);
        assert!(m > 0.15 && m < 0.85, "the default sits inside the track for {w}×{h}: {m}");
    }
}

#[test]
fn the_row_dropdown_opens_the_popover_not_the_type_menu() {
    let (mut u, _r) = rect_with_perspective(600.0);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let label = crate::live_tests::text_center(&shapes, "Perspective");
    u.button(label, true);
    u.button(label, false);
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover.is_some(), "the row's dropdown opens the popover");
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover is drawn");
    // A second click closes it again (the toggle).
    u.button(label, true);
    u.button(label, false);
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover.is_none(), "the second click closes the popover");
}

#[test]
fn the_add_menu_opens_the_perspective_popover_in_the_same_click() {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let header = crate::live_tests::text_center(&shapes, "Effects").y;
    let panel = u.app.ui.right_panel_rect.expect("the Design panel draws");
    let at = Pos2::new(panel.right() - 20.0, header);
    u.button(at, true);
    u.button(at, false);
    u.frame(vec![]);
    // Picking Perspective adds the effect and opens its popover in the same interaction (the
    // press and release share a frame: the opening frame must not read that press as an
    // outside press and close the new popover).
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "Perspective");
    u.frame(vec![egui::Event::PointerMoved(at), press(at, true), press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover.is_some(), "picking Perspective opens its popover");
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(n.effects.last().map(|e| &e.kind), Some(figcraft_doc::EffectKind::Perspective(_))), "the perspective effect is added");
    assert!(u.app.ui.persp_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover is drawn");
    // The header's close closes it.
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let panel = u.app.ui.persp_popover_pop.as_ref().unwrap().rect;
    let close = shapes
        .iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == "Perspective" && panel.contains(t.pos) => Some(t.pos),
            _ => None,
        })
        .map(|title| Pos2::new(panel.right() - 21.0, title.y + 5.0))
        .expect("the popover's title is drawn");
    u.frame(vec![egui::Event::PointerMoved(close), press(close, true)]);
    u.frame(vec![press(close, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover.is_none(), "the header's X closes the popover");
}

#[test]
fn the_popover_closes_with_what_it_edits() {
    let (mut u, r) = rect_with_perspective(600.0);
    open(&mut u, r);
    assert!(u.app.ui.persp_popover.is_some(), "the popover is open");
    // Removing the effect closes the popover on the next frame (nothing left to edit).
    u.app.session.execute("effect.remove", &json!({"ids": [r], "index": 0})).unwrap();
    u.frame(vec![]);
    assert!(u.app.ui.persp_popover.is_none(), "the popover closed with its effect");
    assert!(!u.app.ui.persp_popover_shown);
}
