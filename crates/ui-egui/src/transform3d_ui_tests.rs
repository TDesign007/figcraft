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
    // The preview's own rect, as the popover actually placed it (the drawn pass, kept in state).
    let preview = u.app.ui.t3_preview_rect.unwrap_or_else(|| panic!("the popover's preview is drawn"));
    // Drag across the preview: down, move, up in frames.
    let centre = preview.center();
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
fn the_popover_header_reads_on_the_light_panel() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": -29.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    // Past the fade-in, so the title paints in its settled token colour.
    for _ in 0..30 {
        u.frame(vec![]);
    }
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let panel = u.app.ui.t3_popover_pop.as_ref().unwrap().rect;
    let tokens = crate::theme::Tokens::get(&u.ctx);
    let title = shapes
        .iter()
        .find_map(|shape| match shape {
            // The row in the panel draws the same label; the popover's title is the one inside it.
            Shape::Text(t) if t.galley.text() == "3D transform" && panel.contains(t.pos) => Some(t),
            _ => None,
        })
        .expect("the popover's title is drawn");
    assert_eq!(title.fallback_color, tokens.text, "the header's title reads on the light panel");
    // The title and the header's icons share one horizontal centre line (the mark too: it is
    // allocated in the same centred row).
    let icons = u.app.ui.t3_pivot_btn_rect.unwrap().center().y;
    let text = title.pos.y + title.galley.size().y / 2.0;
    assert!((text - icons).abs() < 1.0, "the title is level with the header's icons: {text} vs {icons}");
}

#[test]
fn the_rotation_and_depth_labels_read_and_translate() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0, "depth": 40.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    // Past the fade-in, so the labels paint in their settled token colour.
    for _ in 0..30 {
        u.frame(vec![]);
    }
    let panel = u.app.ui.t3_popover_pop.as_ref().unwrap().rect;
    let tokens = crate::theme::Tokens::get(&u.ctx);
    let label = |u: &mut Ui, want: &str| {
        let shapes = flat(&u.frame_output(vec![]).shapes);
        shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::Text(t) if t.galley.text() == want && panel.contains(t.pos) => Some(t.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{want} is painted in the popover"))
    };
    let rotation = label(&mut u, crate::i18n::tr("Rotation"));
    let depth = label(&mut u, crate::i18n::tr("Depth"));
    assert_eq!(rotation.fallback_color, tokens.text_secondary, "the Rotation label reads on the light panel");
    assert_eq!(depth.fallback_color, tokens.text_secondary, "the Depth label reads on the light panel");
    // The labels sit to the left of their fields (the field column starts 80 points into the row).
    assert!(rotation.pos.x < panel.min.x + 80.0, "Rotation is left of the X field");
    assert!(depth.pos.x < panel.min.x + 80.0, "Depth is left of the depth field");
    // Both are translated keys (the test language is English, where the key is the text; switching
    // the global language here would leak into parallel tests): the Simplified Chinese catalog
    // carries them. Regional catalogs may omit them and fall back to their parent.
    let zh = crate::i18n::languages().iter().find(|l| l.code == "zh-Hans").expect("the Simplified Chinese catalog");
    assert!(zh.ui.iter().any(|(k, _)| *k == "Rotation"), "Rotation translates");
    assert!(zh.ui.iter().any(|(k, _)| *k == "Depth"), "Depth translates");
}

#[test]
fn the_reset_button_is_drawn_at_the_preview_corner() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": -29.0, "rotateY": -45.0, "depth": 80.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    u.frame(vec![]);
    u.frame(vec![]);
    // The reset sits inside the preview's bottom-right corner, and inside the popover panel.
    let reset = u.app.ui.t3_reset_rect.unwrap_or_else(|| panic!("the reset button is drawn"));
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("t3_popover"))).unwrap();
    assert!(panel.contains(reset.center()), "the reset stays inside the popover panel: {reset:?} vs {panel:?}");
    // The whole field column stays inside the panel too (the layout no longer overflows right).
    assert!(panel.width() >= 250.0 - 1.0, "the panel is the popover's width: {:?}", panel.width());
}

/// Open the main effect popup from its row, then click its settings button.
fn open_settings(u: &mut Ui) -> Pos2 {
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let dropdown = crate::live_tests::text_center(&shapes, "3D transform");
    u.button(dropdown, true);
    u.button(dropdown, false);
    u.frame(vec![]);
    let more = u.app.ui.t3_settings_btn_rect.unwrap().center();
    assert!(!u.app.ui.t3_settings_open);
    u.button(more, true);
    u.button(more, false);
    u.frame(vec![]);
    assert!(u.app.ui.t3_settings_open, "clicking More opens Settings");
    more
}

#[test]
fn the_more_button_opens_a_readable_backface_popover() {
    for theme in ["light", "dark"] {
        let (mut u, _) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0}));
        u.app.session.prefs.theme = theme.into();
        let more = open_settings(&mut u);
        // Past the Area fade-in so title/Backface colours are the settled token colours, not the
        // mid-fade gamma-multiplied ones (same settle the menu contrast tests use).
        for _ in 0..30 {
            u.frame(vec![egui::Event::PointerMoved(Pos2::new(20.0, 20.0))]);
        }
        let shapes = flat(&u.frame_output(vec![]).shapes);
        let main = u.app.ui.t3_popover_pop.as_ref().unwrap().rect;
        let settings = u.app.ui.t3_settings_pop.as_ref().unwrap();
        let panel = settings.rect;
        let close = settings.close.center();
        assert!(panel.right() <= main.left(), "Settings sits to the left of the effect popup");
        assert!((panel.top() - main.top()).abs() < 1.0, "the popups align at the top");
        let text = |value: &str| {
            shapes
                .iter()
                .find_map(|shape| match shape {
                    Shape::Text(t) if t.galley.text() == value && panel.contains(t.pos) => Some(t),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{value} is visible in Settings"))
        };
        let title = text("Settings");
        let label = text("Backface");
        let tokens = crate::theme::Tokens::get(&u.ctx);
        assert_eq!(title.fallback_color, tokens.text, "title uses readable panel text in {theme} mode");
        assert_eq!(label.fallback_color, tokens.text_secondary, "Backface uses readable panel text in {theme} mode");
        assert!((title.pos.x - label.pos.x).abs() < 1.0 && title.pos.y < label.pos.y, "title and setting align on the left");
        u.button(close, true);
        u.button(close, false);
        assert!(u.app.ui.t3_popover.is_some(), "closing Settings leaves the main popup open");
        assert!(!u.app.ui.t3_settings_open);
        u.button(more, true);
        u.button(more, false);
        u.frame(vec![]);
        assert!(u.app.ui.t3_settings_open, "More reopens Settings");
    }
}

#[test]
fn the_add_button_lists_the_types_and_opens_the_3d_popover() {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    let mut u = Ui::new(s);
    // The + at the Effects header's right end: find it along the header row (its x depends on the
// panel's inner margins), right to left.
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let header = crate::live_tests::text_center(&shapes, "Effects").y;
    let panel = u.app.ui.right_panel_rect.expect("the Design panel draws");
    // The + sits at the header's right end (8 + 24 in from the panel's edge).
    let at = Pos2::new(panel.right() - 20.0, header);
    u.button(at, true);
    u.button(at, false);
    u.frame(vec![]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Drop shadow"), "the + opens the effects menu");
    let shapes = flat(&u.frame_output(vec![]).shapes);
    for label in ["Drop shadow", "Layer blur", "Background blur", "Noise", "Texture", "Glass", "3D transform"] {
        assert!(has_text(&shapes, label), "the add menu lists {label}");
    }
    // Picking 3D transform adds the effect and opens its popover in the same interaction. The
    // press and release share a frame here (the quickest click): the opening frame must not read
    // that press as an outside press and close the new popover.
    let at = crate::live_tests::text_center(&shapes, "3D transform");
    let press = |pressed: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    u.frame(vec![egui::Event::PointerMoved(at), press(true), press(false)]);
    u.frame(vec![]);
    assert!(u.app.ui.t3_popover.is_some(), "picking 3D transform opens its popover");
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(
        matches!(n.effects.last().map(|e| &e.kind), Some(figcraft_doc::EffectKind::Transform3d(_))),
        "the 3D transform effect is added"
    );
    assert!(u.app.ui.t3_popover_pop.as_ref().is_some_and(|p| p.rect.is_positive()), "the popover is drawn");
}

#[test]
fn the_pivot_picker_is_its_own_popover() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 10.0, "rotateY": 20.0}));
    u.app.ui.t3_popover = Some((r, 0, row_rect()));
    for _ in 0..3 {
        u.frame(vec![]);
    }
    let main_h = u.app.ui.t3_popover_pop.as_ref().unwrap().rect.height();
    // The header's crosshair (⊕).
    let button = u.app.ui.t3_pivot_btn_rect.unwrap().center();
    u.button(button, true);
    u.button(button, false);
    for _ in 0..3 {
        u.frame(vec![]);
    }
    assert!(u.app.ui.t3_origin_open, "the crosshair opens the picker");
    let main = u.app.ui.t3_popover_pop.as_ref().unwrap().rect;
    let pivot = u.app.ui.t3_pivot_pop.as_ref().unwrap().rect;
    assert!(pivot.is_positive(), "the picker is drawn: {pivot:?}");
    assert!(pivot.bottom() <= main.top(), "its own popover, above the main one: {pivot:?} vs {main:?}");
    assert!((pivot.center().x - main.center().x).abs() < 1.0, "centred over the main popover");
    assert!((main.height() - main_h).abs() < 1.0, "the main popover does not expand for it");
    // The nine points: the top-left one sets the origin to the layer's 0%, 0%.
    let grid = Rect::from_min_size(pivot.min + egui::vec2(10.0, 10.0), egui::vec2(pivot.width() - 20.0, 96.0));
    let at = Pos2::new(grid.min.x + 24.0, grid.min.y + 18.0);
    u.button(at, true);
    u.button(at, false);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    assert_eq!(t.origin, (0.0, 0.0), "the top-left point is picked");
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

#[test]
fn dragging_inside_the_ball_free_rotates() {
    let (mut u, r) = rect_with_effect(&json!({"type": "transform3d", "rotateX": 0.0, "rotateY": 0.0}));
    u.frame(vec![]);
    let doc = u.app.session.doc().unwrap().clone();
    let p = figcraft_doc::transform3d::projector(&doc, r).unwrap();
    let (px, py) = p.pivot_page();
    let pivot = u.app.to_screen(px, py);
    // A point inside the ball but off the rings and the dot: free rotation. (With no turn the
    // Y ring projects onto the pivot's horizontal line and X's onto its vertical one, so the
    // quadrant diagonal clears both by more than the grab radius.)
    let inside = pivot + egui::vec2(14.0, 14.0);
    let press = |p: Pos2, pressed: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    u.frame(vec![egui::Event::PointerMoved(inside), press(inside, true)]);
    u.frame(vec![egui::Event::PointerMoved(inside + egui::vec2(20.0, 10.0))]);
    u.frame(vec![egui::Event::PointerMoved(inside + egui::vec2(40.0, 20.0))]);
    u.frame(vec![egui::Event::PointerMoved(inside + egui::vec2(40.0, 20.0)), press(inside + egui::vec2(40.0, 20.0), false)]);
    u.frame(vec![]);
    let n = u.app.session.doc().unwrap().node(r).unwrap().clone();
    let figcraft_doc::EffectKind::Transform3d(t) = &n.effects[0].kind else { panic!("no turn") };
    // The drag's total offset from the press: 40 pt right turns Y by 20°, 20 pt down turns X
    // by 10° (the accumulation makes a missed frame harmless).
    assert!((t.rotate_y - 20.0).abs() < 1.0, "horizontal drag turned Y: {}", t.rotate_y);
    assert!((t.rotate_x - 10.0).abs() < 1.0, "vertical drag turned X: {}", t.rotate_x);
}
