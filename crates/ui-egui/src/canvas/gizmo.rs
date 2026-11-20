//! The 3D transform's on-canvas gizmo, for the selected layer: the dashed box where the layer
//! would be unturned, and the three coloured rings at the pivot — one per axis — dragged to turn
//! the layer around that axis in real time (Figma's rotation rings).

use egui::{Color32, Pos2, Shape, Stroke};
use figcraft_doc::NodeId;
use figcraft_geom::Point;

use super::FigApp;

/// The rings' colours, one per axis (X, Y, Z).
const AXIS_COLOURS: [Color32; 3] =
    [Color32::from_rgb(0xE5, 0x48, 0x4D), Color32::from_rgb(0x46, 0xA7, 0x58), Color32::from_rgb(0x0D, 0x99, 0xFF)];
/// A press within this many screen points of a ring's path grabs it.
const GRAB_RADIUS: f32 = 12.0;
/// The ring samples along one turn.
const SAMPLES: usize = 48;

/// One axis's ring: the sampled screen points of the circle a turn about that axis sweeps, and
/// the pivot's screen point.
fn ring_points(app: &FigApp, doc: &figcraft_doc::Document, id: NodeId, axis: usize) -> Option<(Vec<Pos2>, Pos2)> {
    let p = figcraft_doc::transform3d::projector(doc, id)?;
    let n = doc.node(id)?;
    let pv = p.pivot();
    let r = p.radius(n.width, n.height).max(24.0);
    let mut pts = Vec::with_capacity(SAMPLES + 1);
    for i in 0..=SAMPLES {
        let t = (i as f64 / SAMPLES as f64) * std::f64::consts::TAU;
        let (c, s) = (t.cos(), t.sin());
        // The circle a turn about the axis sweeps, in the layer's own space (the ring for Z
        // sweeps the plane itself; X's and Y's stand out of it).
        let (x, y, z) = match axis {
            0 => (pv.0, pv.1 + r * c, r * s),
            1 => (pv.0 + r * c, pv.1, r * s),
            _ => (pv.0 + r * c, pv.1 + r * s, 0.0),
        };
        let (qx, qy) = p.apply(x, y, z);
        pts.push(app.to_screen(qx, qy));
    }
    let (px, py) = p.pivot_page();
    Some((pts, app.to_screen(px, py)))
}

/// The gizmo for the selected layer's 3D transform: the dashed box of the unturned layer and the
/// three rings at the pivot (the drag lives in [`input`]).
pub(crate) fn draw(app: &FigApp, painter: &egui::Painter, doc: &figcraft_doc::Document, sel: &[NodeId]) {
    let Some(id) = sel.iter().copied().find(|id| figcraft_doc::transform3d::projector(doc, *id).is_some()) else { return };
    let Some(n) = doc.node(id) else { return };
    // The unturned box, dashed (Figma's original-bounds box): where the layer sits before the
    // turn — the projected quad beside it is what paints.
    let w = doc.world_transform(id);
    let corners = [Point::new(0.0, 0.0), Point::new(n.width, 0.0), Point::new(n.width, n.height), Point::new(0.0, n.height)];
    let sp: Vec<Pos2> = corners.map(|c| { let q = w * c; app.to_screen(q.x, q.y) }).into_iter().collect();
    for i in 0..4 {
        let (a, b) = (sp[i], sp[(i + 1) % 4]);
        painter.add(Shape::dashed_line(&[a, b], Stroke::new(1.0, Color32::from_white_alpha(110)), 4.0, 3.0));
    }
    // The rings, the grabbed one thicker; the pivot dot under them.
    let grabbed = app.canvas.t3_gizmo.as_ref().map(|(_, axis, _, _)| *axis);
    let mut pivot = Pos2::ZERO;
    for axis in 0..3 {
        if let Some((pts, at)) = ring_points(app, doc, id, axis) {
            pivot = at;
            let colour = AXIS_COLOURS[axis];
            let width = if grabbed == Some(axis) { 3.0 } else { 2.0 };
            painter.add(Shape::line(pts, Stroke::new(width, colour)));
        }
    }
    if pivot != Pos2::ZERO {
        painter.circle_filled(pivot, 3.0, Color32::from_rgb(0x0D, 0x99, 0xFF));
    }
}

/// The gizmo's pointer input, before the canvas's own: a press on a ring grabs that axis, the
/// drag turns the layer around it in real time (one batched undo step), the release commits.
/// Returns whether the gesture was consumed.
pub(crate) fn input(app: &mut FigApp, ctx: &egui::Context, pos: Pos2) -> bool {
    // An active gesture runs to its end.
    if let Some((id, axis, start_angle, start_value)) = app.canvas.t3_gizmo {
        let (released, hover) = ctx.input(|i| (i.pointer.any_released(), i.pointer.hover_pos()));
        if released {
            app.session.commit_interaction().ok();
            app.canvas.t3_gizmo = None;
            return true;
        }
        let pivot = screen_pivot(app, id);
        if axis == 3 {
            // The pivot's drag: the origin moves to the pointer's place on the layer, in the
            // layer's own percent.
            if let Some(pos) = hover {
                apply_origin_at(app, id, pos);
            }
            return true;
        }
        if let (Some(_), Some(d)) = (hover, hover.map(|p| wrap_deg(angle(p, pivot) - start_angle))) {
            let cmd = match axis { 0 => "rotateX", 1 => "rotateY", _ => "rotateZ" };
            apply_turn(app, id, cmd, (start_value + d).clamp(-360.0, 360.0));
        }
        return true;
    }
    // A press on a ring grabs it.
    if !ctx.input(|i| i.pointer.primary_pressed()) {
        return false;
    }
    let Ok(doc) = app.session.doc() else { return false };
    let Some(id) = app.session.selection().iter().copied().find(|id| figcraft_doc::transform3d::projector(&doc, *id).is_some()) else { return false };
    // The nearest ring's axis, by the distance to its sampled points.
    let mut best: Option<(usize, f32)> = None;
    let mut pivot = Pos2::ZERO;
    for axis in 0..3 {
        if let Some((pts, at)) = ring_points(app, &doc, id, axis) {
            pivot = at;
            for p in &pts {
                let d = p.distance(pos);
                if d < GRAB_RADIUS && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((axis, d));
                }
            }
        }
    }
    // The pivot dot first: a press on it grabs the origin (dragged to a new place on the layer).
    if pivot.distance(pos) < 10.0 {
        app.session.begin_interaction("3D gizmo").ok();
        app.canvas.t3_gizmo = Some((id, 3, 0.0, 0.0));
        return true;
    }
    let Some((axis, _)) = best else { return false };
    app.session.begin_interaction("3D gizmo").ok();
    app.canvas.t3_gizmo = Some((id, axis, angle(pos, pivot), current_turn(app, id, axis)));
    true
}

fn screen_pivot(app: &FigApp, id: NodeId) -> Pos2 {
    if let Ok(doc) = app.session.doc()
        && let Some(p) = figcraft_doc::transform3d::projector(&doc, id)
    {
        let (px, py) = p.pivot_page();
        return app.to_screen(px, py);
    }
    Pos2::ZERO
}

fn angle(p: Pos2, centre: Pos2) -> f64 {
    f64::from(p.y - centre.y).atan2(f64::from(p.x - centre.x))
}

/// The shortest way from `a` to `b`, in ±π.
fn wrap_deg(d: f64) -> f64 {
    let d = d.to_degrees();
    if d > 180.0 {
        d - 360.0
    } else if d < -180.0 {
        d + 360.0
    } else {
        d
    }
}

fn current_turn(app: &FigApp, id: NodeId, axis: usize) -> f64 {
    app.session
        .doc()
        .ok()
        .and_then(|doc| doc.node(id).cloned())
        .and_then(|n| {
            n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
                figcraft_doc::EffectKind::Transform3d(t) => Some(t.sanitized()),
                _ => None,
            })
        })
        .map(|t| match axis { 0 => t.rotate_x, 1 => t.rotate_y, _ => t.rotate_z })
        .unwrap_or(0.0)
}

/// The origin moves to the pointer's place on the layer: the pointer through the layer's own
/// placement into its coordinates, as the pivot's percent (one batched gesture).
fn apply_origin_at(app: &mut FigApp, id: NodeId, pos: Pos2) {
    let Some(mut e) = app
        .session
        .doc()
        .ok()
        .and_then(|doc| doc.node(id).cloned())
        .and_then(|n| {
            n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
                figcraft_doc::EffectKind::Transform3d(_) => Some(e.clone()),
                _ => None,
            })
        })
    else {
        return;
    };
    let Some((w, h)) = app.session.doc().ok().and_then(|doc| doc.node(id)).map(|n| (n.width, n.height)) else { return };
    let page = app.to_page(pos);
    let Some(local) = app.session.doc().ok().map(|doc| doc.world_transform(id).inverse() * figcraft_geom::Point::new(page.x, page.y)) else { return };
    if let figcraft_doc::EffectKind::Transform3d(t) = &mut e.kind {
        t.origin = (
            (local.x / w.max(1e-6) * 100.0).clamp(0.0, 100.0),
            (local.y / h.max(1e-6) * 100.0).clamp(0.0, 100.0),
        );
    }
    app.session.execute("effect.set", &serde_json::json!({"ids": [id.0], "index": 0, "effect": e})).ok();
}

/// One turn's new value, through `effect.set` (the gesture batches it).
fn apply_turn(app: &mut FigApp, id: NodeId, prop: &str, value: f64) {
    let Some(mut e) = app
        .session
        .doc()
        .ok()
        .and_then(|doc| doc.node(id).cloned())
        .and_then(|n| {
            n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
                figcraft_doc::EffectKind::Transform3d(_) => Some(e.clone()),
                _ => None,
            })
        })
    else {
        return;
    };
    if let figcraft_doc::EffectKind::Transform3d(t) = &mut e.kind {
        match prop {
            "rotateX" => t.rotate_x = value,
            "rotateY" => t.rotate_y = value,
            _ => t.rotate_z = value,
        }
    }
    app.session.execute("effect.set", &serde_json::json!({"ids": [id.0], "index": 0, "effect": e})).ok();
}
