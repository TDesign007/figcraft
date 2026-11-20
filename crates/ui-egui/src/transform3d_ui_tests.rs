//! The Design panel's effect rows for the 3D transform and the Perspective: the row names the
//! effect, the type menu lists both among the effect types, the opened settings show the fields,
//! and the popover opens beside the row with a live preview (dragged to turn, reset to zero).

use egui::{Pos2, Rect, Shape, Vec2};
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

fn row_rect() -> Rect {
    Rect::from_min_size(Pos2::new(920.0, 470.0), Vec2::new(150.0, 30.0))
}

#[test]
fn the_3d_transform_row_shows_its_settings() {
    let (mut u, _r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 30.0, "rotateY": -40.0, "depth": 60.0, "backface": false}));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "3D transform"), "the row names the effect");
    assert!(has_text(&shapes, "Backface"), "the backface setting shows");
    assert!(has_text(&shapes, "Depth"), "the depth field shows");
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
    let label = crate::live_tests::text_center(&shapes, "Layer blur");
    let at = Pos2::new(label.x, label.y);
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
    u.frame(vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }]);
    let shapes: Vec<Shape> = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "3D transform"), "the menu lists the 3D transform");
    assert!(has_text(&shapes, "Perspective"), "the menu lists the Perspective");
}

#[test]
fn the_popover_opens_beside_the_row_with_the_preview_and_the_fields() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": -29.0, "rotateY": -45.0, "depth": 80.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "3D transform"), "the popover's title");
    assert!(has_text(&shapes, "Rotation"), "the turns' section label");
    assert!(has_text(&shapes, "Depth"), "the depth field");
    // The popover's first frame lays it out invisibly (its placement needs the size): the next
    // one draws it for real.
    u.frame(vec![]);
    // The preview's own rect, as the popover actually placed it.
    let preview = u.ctx.read_response(egui::Id::unique("t3_preview")).map(|r| r.rect).unwrap_or_else(|| panic!("the popover's preview is drawn"));
    // Drag across the preview: down, move, up in frames.
    let centre = preview.center();
    if let Some(resp) = u.ctx.read_response(egui::Id::unique("t3_preview")) {
        eprintln!("preview rect: {:?}", resp.rect);
    } else {
        eprintln!("preview: no response yet");
    }
    let press = |p: Pos2, pressed: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    u.frame(vec![egui::Event::PointerMoved(centre), press(centre, true)]);
    u.frame(vec![egui::Event::PointerMoved(centre + egui::vec2(15.0, 10.0))]);
    u.frame(vec![egui::Event::PointerMoved(centre + egui::vec2(30.0, 20.0))]);
    u.frame(vec![egui::Event::PointerMoved(centre + egui::vec2(30.0, 20.0)), press(centre + egui::vec2(30.0, 20.0), false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert!((t.rotate_y - (-45.0 + 30.0 * 0.55)).abs() < 0.1, "the drag turned Y by the drag's distance: {}", t.rotate_y);
    assert!((t.rotate_x - (-29.0 + 20.0 * 0.55)).abs() < 0.1, "the drag turned X: {}", t.rotate_x);
}

#[test]
fn the_reset_zeroes_the_turns_and_the_depth() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": -29.0, "rotateY": -45.0, "depth": 80.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    // The reset button sits at the preview's bottom-right corner (the preview's rect from the
    // popover's placement beside the row).
    // The preview's own rect, as the popover actually placed it.
    let preview = u.ctx.read_response(egui::Id::unique("t3_preview")).map(|r| r.rect).unwrap_or_else(|| panic!("the popover's preview is drawn"));
    let reset = preview.right_bottom() - egui::vec2(18.0, 18.0);
    u.frame(vec![egui::Event::PointerMoved(reset)]);
    u.frame(vec![egui::Event::PointerButton { pos: reset, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
    u.frame(vec![egui::Event::PointerButton { pos: reset, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert_eq!((t.rotate_x, t.rotate_y, t.depth), (0.0, 0.0, 0.0), "the reset zeroes everything");
}

#[test]
fn clicking_the_row_dropdown_opens_the_popover() {
    let (mut u, _r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0}));
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "3D transform");
    let press = |p: Pos2, pressed: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.t3_popover.is_some(), "the dropdown click opens the popover");
    // A second click on the same row's dropdown closes it again.
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let at = crate::live_tests::text_center(&shapes, "3D transform");
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    assert!(u.app.ui.t3_popover.is_none(), "the second click closes it");
}

#[test]
fn the_settings_popover_shows_the_backface_toggle() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.app.ui.t3_settings_open = true;
    u.frame(vec![]);
    u.frame(vec![]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Settings"), "the settings popover's title");
    assert!(has_text(&shapes, "Backface"), "the backface row");
}

#[test]
fn the_backface_segments_toggle_the_setting() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    u.app.ui.t3_settings_open = true;
    u.frame(vec![]);
    u.frame(vec![]);
    u.frame(vec![]);
    // The two segments: the ✓ (shown, the default) on the right half of the switch, the — on the
    // left. Their rects from the switch's own ids.
    let on = u.ctx.read_response(egui::Id::unique(("t3_backface", true))).map(|r| r.rect).unwrap_or_else(|| panic!("the ✓ segment"));
    let off = u.ctx.read_response(egui::Id::unique(("t3_backface", false))).map(|r| r.rect).unwrap_or_else(|| panic!("the — segment"));
    let press = |p: Pos2, pressed: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    // Picking "—" hides the back.
    let at = off.center();
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert!(!t.backface, "the — segment hides the back");
    // Picking "✓" shows it again.
    let at = on.center();
    u.frame(vec![egui::Event::PointerMoved(at)]);
    u.frame(vec![press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert!(t.backface, "the ✓ segment shows the back");
}

#[test]
fn dragging_the_pivot_dot_moves_the_origin() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 20.0, "rotateY": 30.0}));
    u.frame(vec![]);
    // The pivot's screen point, as the gizmo draws it.
    let doc = u.app.session.doc().unwrap().clone();
    let p = figcraft_doc::transform3d::projector(&doc, r).unwrap();
    let (px, py) = p.pivot_page();
    let pivot = u.app.to_screen(px, py);
    let press = |p: Pos2, pressed: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    // Grab the dot, drag it toward the layer's bottom-right, release.
    u.frame(vec![egui::Event::PointerMoved(pivot), press(pivot, true)]);
    u.frame(vec![egui::Event::PointerMoved(pivot + egui::vec2(20.0, 15.0))]);
    u.frame(vec![egui::Event::PointerMoved(pivot + egui::vec2(40.0, 30.0))]);
    u.frame(vec![egui::Event::PointerMoved(pivot + egui::vec2(40.0, 30.0)), press(pivot + egui::vec2(40.0, 30.0), false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert!(t.origin.0 > 55.0 && t.origin.1 > 55.0, "the origin moved toward the drag: {:?}", t.origin);
    assert_eq!((t.rotate_x, t.rotate_y), (20.0, 30.0), "the turns stay: the drag moved the pivot, not the turn");
}
