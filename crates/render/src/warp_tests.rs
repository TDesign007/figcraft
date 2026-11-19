//! The 3D transform's render: an orthographic turn foreshortens the plane (its pixels move where
//! the homography says), depth under a perspective scales it past its box, and a hidden backface
//! draws nothing when the plane faces away. Every case names the rule it pins.

use super::*;
use figcraft_color::{Color, Paint};
use figcraft_doc::{Effect, EffectKind, Transform3d};

fn view(w: u32, h: u32) -> View {
    View { origin: (0.0, 0.0), scale: 1.0, width: w, height: h }
}

fn px(p: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * p.width() + x) * 4) as usize;
    let d = &p.data()[i..i + 4];
    [d[0], d[1], d[2], d[3]]
}

fn rect_with_turn(t3: Transform3d) -> figcraft_doc::Document {
    let mut d = figcraft_doc::Document::new("turn");
    let p = d.first_page().unwrap();
    let r = d.create(p, NodeKind::Rectangle, "R", 0.0, 0.0, 100.0, 100.0).unwrap();
    d.get_mut(r).unwrap().fills = vec![Paint::solid(Color::from_hex("FF0000").unwrap())];
    d.get_mut(r).unwrap().effects = vec![Effect { kind: EffectKind::Transform3d(t3), visible: true }];
    d
}

/// An orthographic turn keeps the centre, shrinks the height (cos 45°), and nothing draws where
/// the plane no longer covers.
#[test]
fn orthographic_turn_foreshortens_the_plane() {
    let d = rect_with_turn(Transform3d { rotate_x: 45.0, ..Transform3d::default() });
    let p = d.first_page().unwrap();
    let pix = render_view(&d, p, &view(200, 200), &Options::default()).unwrap();
    // The centre of the turned plane is still the centre of the box.
    assert_eq!(px(&pix, 50, 50), [255, 0, 0, 255]);
    // The plane's projected height is 100·cos 45° ≈ 70.7 about the centre: the box's corners no
    // longer draw (an affine projection of the turn could not move them there).
    assert_eq!(px(&pix, 50, 5), [245, 245, 245, 255]);
    assert_eq!(px(&pix, 50, 95), [245, 245, 245, 255]);
    // Inside the projected span near its edge, the plane draws.
    assert_eq!(px(&pix, 50, 80), [255, 0, 0, 255]);
}

/// Depth toward the camera under a perspective scales the plane past its box (the projection's
/// 800 / (800 − 400) = 2 about the centre); without a perspective the same depth shows nothing.
#[test]
fn depth_scales_the_plane_past_its_box_under_a_perspective() {
    let d = rect_with_turn(Transform3d { depth: 400.0, ..Transform3d::default() });
    let p = d.first_page().unwrap();
    let pix = render_view(&d, p, &view(400, 400), &Options::default()).unwrap();
    assert_eq!(px(&pix, 50, 50), [255, 0, 0, 255], "no perspective: depth draws in place");
    assert_eq!(px(&pix, 120, 50), [245, 245, 245, 255], "and takes no room past the box");
    // A parent frame with a Perspective effect carries the camera for the layer inside it.
    let mut d = figcraft_doc::Document::new("persp");
    let page = d.first_page().unwrap();
    let f = d.create(page, NodeKind::Frame, "F", 0.0, 0.0, 600.0, 600.0).unwrap();
    let r = d.create(f, NodeKind::Rectangle, "R", 100.0, 100.0, 100.0, 100.0).unwrap();
    d.get_mut(r).unwrap().fills = vec![Paint::solid(Color::from_hex("FF0000").unwrap())];
    d.get_mut(r).unwrap().effects = vec![Effect { kind: EffectKind::Transform3d(Transform3d { depth: 400.0, ..Transform3d::default() }), visible: true }];
    d.get_mut(f).unwrap().effects = vec![Effect { kind: EffectKind::Perspective(figcraft_doc::Perspective { distance: 800.0 }), visible: true }];
    // The turned layer inside the frame: its depth now scales it ×2 about its centre (150, 150),
    // so it paints from 50 to 250 where its box was 100 to 200.
    let pix = render_view(&d, p, &view(600, 600), &Options::default()).unwrap();
    assert_eq!(px(&pix, 150, 150), [255, 0, 0, 255]);
    assert_eq!(px(&pix, 80, 80), [255, 0, 0, 255], "the plane doubled past its box's edge");
    assert_eq!(px(&pix, 230, 150), [255, 0, 0, 255], "the far side doubled too");
    assert_eq!(px(&pix, 30, 30), [255, 255, 255, 255], "the frame's own fill outside the doubled plane");
}

/// With the backface hidden, a plane turned past 90° draws nothing; with it shown (the default),
/// the mirrored front draws.
#[test]
fn a_hidden_backface_draws_nothing_when_the_plane_faces_away() {
    let d = rect_with_turn(Transform3d { rotate_x: 180.0, backface: false, ..Transform3d::default() });
    let p = d.first_page().unwrap();
    let pix = render_view(&d, p, &view(200, 200), &Options::default()).unwrap();
    assert_eq!(px(&pix, 50, 50), [245, 245, 245, 255]);
    let d = rect_with_turn(Transform3d { rotate_x: 180.0, ..Transform3d::default() });
    let p = d.first_page().unwrap();
    let pix = render_view(&d, p, &view(200, 200), &Options::default()).unwrap();
    assert_eq!(px(&pix, 50, 50), [255, 0, 0, 255], "the mirrored front draws");
}
