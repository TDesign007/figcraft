//! The 3D transform and the Perspective effects through the command surface: adding them, setting
//! their fields (unknown keys refused), the projection's quad in page coordinates, and hitting a
//! turned layer where its plane now paints.

use crate::Session;
use figcraft_doc::{EffectKind, geometry};
use figcraft_geom::Point;
use serde_json::json;

fn doc_rect(s: &mut Session) -> u64 {
    let r = s.execute("shape.rectangle", &json!({"x": 40.0, "y": 40.0, "width": 100.0, "height": 100.0})).unwrap();
    r["id"].as_u64().unwrap()
}

/// `effect.add` takes the new types with their defaults, and `effect.set` writes their fields.
#[test]
fn the_new_effects_add_and_set() {
    let mut s = Session::new();
    let r = doc_rect(&mut s);
    s.execute("effect.add", &json!({"ids": [r], "type": "transform3d"})).unwrap();
    let n = s.doc().unwrap().node(figcraft_doc::NodeId(r)).unwrap().clone();
    assert_eq!(n.effects.len(), 1);
    assert!(matches!(&n.effects[0].kind, EffectKind::Transform3d(t) if t.rotate_x == 0.0 && t.rotate_y == 0.0 && t.depth == 0.0 && t.backface));
    s.execute(
        "effect.set",
        &json!({"ids": [r], "index": 0, "effect": {"type": "transform3d", "rotateX": 30.0, "rotateY": 20.0, "depth": 40.0, "backface": false}}),
    )
    .unwrap();
    let n = s.doc().unwrap().node(figcraft_doc::NodeId(r)).unwrap().clone();
    assert!(matches!(&n.effects[0].kind, EffectKind::Transform3d(t) if t.rotate_x == 30.0 && t.rotate_y == 20.0 && t.depth == 40.0 && !t.backface));
    // A field the effect doesn't take is refused, naming what it takes.
    let err = s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "transform3d", "rotateZ": 10.0}})).unwrap_err();
    assert!(err.to_string().contains("rotateX"), "the error names the fields: {err}");
}

/// A frame's Perspective effect carries its camera distance to its descendants, and only them.
#[test]
fn perspective_comes_from_the_parent_frame() {
    let mut s = Session::new();
    let f = s.execute("node.create", &json!({"type": "frame", "x": 0.0, "y": 0.0, "width": 400.0, "height": 400.0})).unwrap()["id"].as_u64().unwrap();
    let r = s.execute("shape.rectangle", &json!({"x": 10.0, "y": 10.0, "width": 50.0, "height": 50.0, "parent": f})).unwrap()["id"].as_u64().unwrap();
    s.execute("effect.add", &json!({"ids": [f], "type": "perspective"})).unwrap();
    s.execute("effect.set", &json!({"ids": [f], "index": 0, "effect": {"type": "perspective", "distance": 600.0}})).unwrap();
    let doc = s.doc().unwrap();
    assert_eq!(figcraft_doc::transform3d::perspective_of(&doc, figcraft_doc::NodeId(r)), Some(600.0));
    assert_eq!(figcraft_doc::transform3d::perspective_of(&doc, figcraft_doc::NodeId(f)), None, "a frame's own camera doesn't apply to itself");
}

/// An unturled layer's projection is its placed box; a 90° turn about X collapses the plane onto
/// the pivot line (every corner at the centre's height).
#[test]
fn the_projection_follows_the_turn() {
    let mut s = Session::new();
    let r = doc_rect(&mut s);
    s.execute("effect.add", &json!({"ids": [r], "type": "transform3d"})).unwrap();
    let id = figcraft_doc::NodeId(r);
    let doc = s.doc().unwrap();
    let q = figcraft_doc::transform3d::projected_quad(&doc, id).unwrap();
    assert_eq!(q[0], Point::new(40.0, 40.0), "no turn: the box as placed");
    drop(doc);
    s.execute(
        "effect.set",
        &json!({"ids": [r], "index": 0, "effect": {"type": "transform3d", "rotateX": 90.0}}),
    )
    .unwrap();
    let doc = s.doc().unwrap();
    let q = figcraft_doc::transform3d::projected_quad(&doc, id).unwrap();
    for c in &q {
        assert!((c.y - 90.0).abs() < 1e-9, "every corner on the pivot line: {c:?}");
    }
    drop(doc);
    // A point maps back through the turn to the same place.
    s.execute(
        "effect.set",
        &json!({"ids": [r], "index": 0, "effect": {"type": "transform3d", "rotateX": 30.0, "rotateY": 20.0, "depth": 50.0}}),
    )
    .unwrap();
    let doc = s.doc().unwrap();
    let pt = Point::new(200.0, 250.0);
    let back = figcraft_doc::transform3d::unwarped(&doc, id, pt).unwrap();
    // `back` is the page point the hit sits at on the flat plane: through the layer's placement
    // it is the plane's local coordinates, and the homography maps them back to the hit.
    let world = doc.world_transform(id);
    let local = world.inverse() * back;
    let h = figcraft_doc::transform3d::node_homography(&doc, id).unwrap();
    let (x, y) = h.apply(local.x, local.y);
    assert!((x - pt.x).abs() < 1e-9 && (y - pt.y).abs() < 1e-9);
}

/// A turned layer is hit where its plane now paints, not where its box was: a 45° turn about X
/// shrinks the plane's span about the centre (90 ± 100·cos 45° / 2… half-height 35.4), so a
/// click near the box's bottom edge misses and a click inside the span hits.
#[test]
fn a_turned_layer_hits_where_its_plane_paints() {
    let mut s = Session::new();
    let r = doc_rect(&mut s);
    let page = s.doc().unwrap().first_page().unwrap();
    s.execute("effect.add", &json!({"ids": [r], "type": "transform3d"})).unwrap();
    s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "transform3d", "rotateX": 45.0}})).unwrap();
    let doc = s.doc().unwrap();
    let result = geometry::BoolResult::default();
    // The projected plane spans y 54.6..125.4 about the centre (90, 90).
    let hit = doc.hit_test(page, Point::new(90.0, 120.0), 0.0, result.clone());
    assert_eq!(hit, Some(figcraft_doc::NodeId(r)), "inside the turned plane's span");
    let hit = doc.hit_test(page, Point::new(90.0, 135.0), 0.0, result);
    assert_eq!(hit, None, "inside the old box, off the turned plane");
}
