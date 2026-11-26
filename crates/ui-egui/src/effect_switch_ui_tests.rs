//! The effect popovers' header switcher: a click on the mark, title or chevron lists the effect
//! types with their marks in Figma's order, the type in use lit in its blue, and picking another
//! type replaces the effect in place and opens that type's popover.

use egui::{Event, Pos2, Rect, Shape, Vec2};
use figcraft_doc::{EffectKind, NodeId};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, text_center, texts};

fn rect_with(kind: &str) -> (Ui, NodeId) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"x": 100.0, "y": 100.0, "width": 200.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [r], "type": kind})).unwrap();
    s.execute("select.set", &json!({"ids": [r]})).unwrap();
    (Ui::new(s), r)
}

fn frame_with(kind: &str) -> (Ui, NodeId) {
    let mut s = Session::new();
    let f = NodeId(s.execute("frame.create", &json!({"x": 100.0, "y": 100.0, "width": 300.0, "height": 200.0})).unwrap()["id"].as_u64().unwrap());
    s.execute("effect.add", &json!({"ids": [f], "type": kind})).unwrap();
    s.execute("select.set", &json!({"ids": [f]})).unwrap();
    (Ui::new(s), f)
}

fn press(p: Pos2, pressed: bool) -> Event {
    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

fn click(u: &mut Ui, at: Pos2) {
    u.frame(vec![Event::PointerMoved(at), press(at, true)]);
    u.frame(vec![press(at, false)]);
    u.frame(vec![]);
}

/// Open the effect's popover by clicking its Design panel row.
fn open_row(u: &mut Ui, label: &str, panel_id: &str) -> Rect {
    let at = text_center(&flat(&u.frame_output(vec![]).shapes), label);
    click(u, at);
    for _ in 0..20 {
        u.frame(vec![]);
    }
    u.ctx.memory(|m| m.area_rect(egui::Id::unique(panel_id))).expect("the popover draws")
}

/// Click the popover header's title, then let the type menu's fade finish.
fn open_menu(u: &mut Ui, panel: Rect) {
    click(u, Pos2::new(panel.min.x + 58.0, panel.min.y + 17.0));
    for _ in 0..20 {
        u.frame(vec![]);
    }
}

/// The menu row of `label`: its text sits in the menu's column (the popover's own labels start
/// further left, the header's title a little nearer the mark).
fn menu_row(shapes: &[Shape], label: &str, panel: Rect) -> Pos2 {
    texts(shapes)
        .into_iter()
        .find(|(t, p, _)| t == label && p.x > panel.min.x + 38.0 && p.x < panel.max.x)
        .map(|(_, p, size)| p + size / 2.0)
        .unwrap_or_else(|| panic!("{label:?} is a menu row"))
}

fn selected_blue(shapes: &[Shape]) -> Vec<Rect> {
    const BLUE: egui::Color32 = egui::Color32::from_rgb(0x0D, 0x99, 0xFF);
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.fill == BLUE => Some(r.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn every_type_has_figmas_mark() {
    use crate::icons::Icon;
    use crate::panels::right::effects::type_icon;
    for (ty, icon) in [
        ("innerShadow", Icon::ShadowInner),
        ("dropShadow", Icon::ShadowDrop),
        ("layerBlur", Icon::BlurLayer),
        ("backgroundBlur", Icon::BlurBackground),
        ("noise", Icon::Noise),
        ("texture", Icon::Texture),
        ("glass", Icon::Glass),
        ("shader", Icon::Shader),
        ("transform3d", Icon::Cube),
        ("perspective", Icon::Perspective),
    ] {
        assert_eq!(type_icon(ty), icon, "{ty} keeps its own mark");
    }
}

#[test]
fn the_header_lists_the_types_in_figmas_order_and_lights_the_current_one_blue() {
    let (mut u, _r) = rect_with("innerShadow");
    let panel = open_row(&mut u, "Inner shadow", "inner_shadow_popover");
    open_menu(&mut u, panel);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let order = [
        "Inner shadow",
        "Drop shadow",
        "Layer blur",
        "Background blur",
        "Noise",
        "Texture",
        "Glass",
        "Shader",
        "3D transform",
        "Perspective",
    ];
    let rows: Vec<Pos2> = order.iter().map(|label| menu_row(&shapes, label, panel)).collect();
    for pair in rows.windows(2) {
        assert!(pair[0].y < pair[1].y, "the menu follows Figma's order: {rows:?}");
    }
    let blue = selected_blue(&shapes);
    let current = rows[0];
    assert!(
        blue.iter().any(|r| r.contains(current) && r.contains(Pos2::new(current.x - 20.0, current.y))),
        "the type in use sits on Figma's blue: {blue:?} for {current:?}"
    );
    let tokens = crate::theme::Tokens::get(&u.ctx);
    let shader = shapes.iter().find_map(|s| match s {
        Shape::Text(t) if t.galley.text() == "Shader" && t.pos.x > panel.min.x + 38.0 && t.pos.x < panel.max.x => {
            Some(t.fallback_color)
        }
        _ => None,
    });
    assert_eq!(shader, Some(tokens.menu_dim), "the unimplemented shader entry is dimmed");
}

#[test]
fn picking_a_type_replaces_the_effect_in_place() {
    let (mut u, r) = rect_with("innerShadow");
    let panel = open_row(&mut u, "Inner shadow", "inner_shadow_popover");
    open_menu(&mut u, panel);
    let at = menu_row(&flat(&u.frame_output(vec![]).shapes), "Texture", panel);
    click(&mut u, at);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects[0].kind, EffectKind::Texture(_)), "the row's effect is now a texture");
    assert!(u.app.ui.texture_popover.is_some(), "the texture popover opens in place");
    assert!(u.app.ui.inner_shadow_popover.is_none(), "the shadow popover closed");

    // The texture popover's header switches again.
    for _ in 0..20 {
        u.frame(vec![]);
    }
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("texture_popover"))).expect("the texture popover draws");
    open_menu(&mut u, panel);
    let at = menu_row(&flat(&u.frame_output(vec![]).shapes), "Glass", panel);
    click(&mut u, at);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects[0].kind, EffectKind::Glass(_)), "texture switched to glass");
    assert!(u.app.ui.glass_popover.is_some(), "the glass popover opens in place");
}

#[test]
fn the_3d_and_perspective_headers_switch_too() {
    let (mut u, r) = rect_with("transform3d");
    u.app.ui.t3_popover = Some((r, 0, Rect::from_min_size(Pos2::new(900.0, 420.0), Vec2::new(160.0, 30.0))));
    u.frame(vec![]);
    u.frame(vec![]);
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("t3_popover"))).expect("the 3D popover draws");
    open_menu(&mut u, panel);
    let at = menu_row(&flat(&u.frame_output(vec![]).shapes), "Layer blur", panel);
    click(&mut u, at);
    let node = u.app.session.doc().unwrap().node(r).unwrap().clone();
    assert!(matches!(node.effects[0].kind, EffectKind::LayerBlur(_)), "3D switched to a layer blur");
    assert!(u.app.ui.layer_blur_popover.is_some(), "the blur popover opens");
    assert!(u.app.ui.t3_popover.is_none(), "the 3D popover closed");

    let (mut u, f) = frame_with("perspective");
    u.app.ui.persp_popover = Some((f, 0, Rect::from_min_size(Pos2::new(900.0, 420.0), Vec2::new(160.0, 30.0))));
    u.frame(vec![]);
    u.frame(vec![]);
    let panel = u.ctx.memory(|m| m.area_rect(egui::Id::unique("persp_popover"))).expect("the Perspective popover draws");
    open_menu(&mut u, panel);
    let at = menu_row(&flat(&u.frame_output(vec![]).shapes), "3D transform", panel);
    click(&mut u, at);
    let node = u.app.session.doc().unwrap().node(f).unwrap().clone();
    assert!(matches!(node.effects[0].kind, EffectKind::Transform3d(_)), "Perspective switched to 3D");
    assert!(u.app.ui.t3_popover.is_some(), "the 3D popover opens");
}
