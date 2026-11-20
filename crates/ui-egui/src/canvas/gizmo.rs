//! The 3D transform's on-canvas gizmo, for the selected layer: the dashed box where the layer
//! would be unturned, and a spherical gyroscope of three coloured rings at the pivot (X pink,
//! Y green, Z blue), each split into a bright near half and a faint far half so the ball reads as
//! 3D, over a faint great-circle sphere. A press on a ring turns the layer about that axis; a drag
//! inside the ball free-rotates it (trackball); the blue dot moves the pivot.

use egui::{Color32, Pos2, Shape, Stroke};
use figcraft_doc::NodeId;
use figcraft_geom::Point;

use super::FigApp;

/// The rings' colours, one per axis (X pink, Y green, Z blue), as the target app's gyroscope.
const AXIS_COLOURS: [Color32; 3] =
    [Color32::from_rgb(0xE5, 0x48, 0x9D), Color32::from_rgb(0x46, 0xA7, 0x58), Color32::from_rgb(0x3B, 0x82, 0xF6)];
/// A press within this many screen points of a ring's path grabs it.
const GRAB_RADIUS: f32 = 12.0;
/// The ring samples along one turn.
const SAMPLES: usize = 72;
/// Trackball sensitivity: degrees of turn per point of drag, about the screen axes.
const TRACKBALL_PER_PT: f64 = 0.5;

/// The gyroscope's radius on screen (a fraction of the layer's projected size, bounded).
fn ball_radius(app: &FigApp, doc: &figcraft_doc::Document, id: NodeId) -> Option<f32> {
    let p = figcraft_doc::transform3d::projector(doc, id)?;
    let n = doc.node(id)?;
    let r = p.radius(n.width, n.height).max(24.0);
    let (px, py) = p.pivot_page();
    let (ex, ey) = p.apply(p.pivot().0 + r, p.pivot().1, 0.0);
    let c = app.to_screen(px, py);
    let e = app.to_screen(ex, ey);
    Some((c.distance(e) * 0.62).clamp(26.0, 120.0))
}

/// One axis's ring as `(screen point, near)` samples: the circle a turn about that axis sweeps,
/// `near` true on the camera-facing half (drawn bright) and false on the far half (faint).
fn ring_samples(app: &FigApp, doc: &figcraft_doc::Document, id: NodeId, axis: usize) -> Option<(Vec<(Pos2, bool)>, Pos2)> {
    let p = figcraft_doc::transform3d::projector(doc, id)?;
    let n = doc.node(id)?;
    let pv = p.pivot();
    let r = p.radius(n.width, n.height).max(24.0);
    let threshold = p.pivot_depth();
    let mut pts = Vec::with_capacity(SAMPLES + 1);
    for i in 0..=SAMPLES {
        let t = (i as f64 / SAMPLES as f64) * std::f64::consts::TAU;
        let (c, s) = (t.cos(), t.sin());
        let (x, y, z) = match axis {
            0 => (pv.0, pv.1 + r * c, r * s),
            1 => (pv.0 + r * c, pv.1, r * s),
            _ => (pv.0 + r * c, pv.1 + r * s, 0.0),
        };
        let near = p.turned(x, y, z).2 >= threshold;
        let (qx, qy) = p.apply(x, y, z);
        pts.push((app.to_screen(qx, qy), near));
    }
    let (px, py) = p.pivot_page();
    Some((pts, app.to_screen(px, py)))
}

/// The faint great-circle sphere behind the rings (the gyroscope's ball outline).
fn sphere_guides(app: &FigApp, doc: &figcraft_doc::Document, id: NodeId) -> Option<(Vec<Vec<Pos2>>, Pos2)> {
    let p = figcraft_doc::transform3d::projector(doc, id)?;
    let n = doc.node(id)?;
    let pv = p.pivot();
    let r = p.radius(n.width, n.height).max(24.0) * 0.9;
    // Three great circles at 45° tilts, so the ball reads from any turn.
    let tilts = [0.0_f64, std::f64::consts::FRAC_PI_4, -std::f64::consts::FRAC_PI_4];
    let mut guides = Vec::new();
    for tilt in tilts {
        let (ct, st) = (tilt.cos(), tilt.sin());
        let mut pts = Vec::with_capacity(SAMPLES + 1);
        for i in 0..=SAMPLES {
            let t = (i as f64 / SAMPLES as f64) * std::f64::consts::TAU;
            let (c, s) = (t.cos(), t.sin());
            // A circle in the plane, tilted about the layer's x axis.
            let (x, y, z) = (pv.0 + r * c, pv.1 + r * s * ct, r * s * st);
            let (qx, qy) = p.apply(x, y, z);
            pts.push(app.to_screen(qx, qy));
        }
        guides.push(pts);
    }
    let (px, py) = p.pivot_page();
    Some((guides, app.to_screen(px, py)))
}

/// Draw one ring split into near (bright) and far (faint) polylines.
fn draw_ring(painter: &egui::Painter, pts: &[(Pos2, bool)], colour: Color32, width: f32) {
    // Walk the closed loop, emitting a polyline per run of equal `near`.
    let mut run: Vec<Pos2> = Vec::new();
    let mut run_near = pts[0].1;
    let flush = |painter: &egui::Painter, run: &mut Vec<Pos2>, near: bool| {
        if run.len() >= 2 {
            let col = if near { colour } else { colour.gamma_multiply(0.28) };
            let w = if near { width } else { width * 0.8 };
            painter.add(Shape::line(run.clone(), Stroke::new(w, col)));
        }
        run.clear();
    };
    for &(p, near) in pts {
        if near != run_near {
            // Bridge the boundary point into both runs so the halves meet.
            run.push(p);
            flush(painter, &mut run, run_near);
            run_near = near;
        }
        run.push(p);
    }
    flush(painter, &mut run, run_near);
}

/// The gizmo for the selected layer's 3D transform: the dashed original box, the faint sphere and
/// the three split rings at the pivot (the drag lives in [`input`]).
pub(crate) fn draw(app: &FigApp, painter: &egui::Painter, doc: &figcraft_doc::Document, sel: &[NodeId]) {
    let Some(id) = sel.iter().copied().find(|id| figcraft_doc::transform3d::projector(doc, *id).is_some()) else { return };
    let Some(n) = doc.node(id) else { return };
    // The unturned box, dashed (the target app's original-bounds box).
    let w = doc.world_transform(id);
    let corners = [Point::new(0.0, 0.0), Point::new(n.width, 0.0), Point::new(n.width, n.height), Point::new(0.0, n.height)];
    let sp: Vec<Pos2> = corners.map(|c| { let q = w * c; app.to_screen(q.x, q.y) }).into_iter().collect();
    for i in 0..4 {
        let (a, b) = (sp[i], sp[(i + 1) % 4]);
        painter.add(Shape::dashed_line(&[a, b], Stroke::new(1.0, Color32::from_white_alpha(110)), 4.0, 3.0));
    }
    let grabbed = app.canvas.t3_gizmo.as_ref().map(|g| g.axis);
    // The faint sphere behind the rings.
    if let Some((guides, pivot)) = sphere_guides(app, doc, id) {
        for g in &guides {
            painter.add(Shape::line(g.clone(), Stroke::new(1.0, Color32::from_white_alpha(46))));
        }
        // The rings, the grabbed one thicker.
        for axis in 0..3 {
            if let Some((pts, _)) = ring_samples(app, doc, id, axis) {
                let width = if grabbed == Some(axis) { 3.0 } else { 2.0 };
                draw_ring(painter, &pts, AXIS_COLOURS[axis], width);
            }
        }
        painter.circle_filled(pivot, 3.0, Color32::from_rgb(0x0D, 0x99, 0xFF));
    }
}

/// The gizmo's pointer input, before the canvas's own: a press on a ring grabs that axis, a drag
/// inside the ball free-rotates (trackball), the blue dot moves the pivot; the release commits one
/// batched undo step. Returns whether the gesture was consumed.
pub(crate) fn input(app: &mut FigApp, ctx: &egui::Context, pos: Pos2) -> bool {
    // An active gesture runs to its end.
    if let Some(g) = app.canvas.t3_gizmo {
        let (id, axis, start_angle, start_value) = (g.id, g.axis, g.start_angle, g.start_value);
        let (released, hover) = ctx.input(|i| (i.pointer.any_released(), i.pointer.hover_pos()));
        if released {
            app.session.commit_interaction().ok();
            app.canvas.t3_gizmo = None;
            return true;
        }
        let pivot = screen_pivot(app, id);
        match axis {
            3 => {
                if let Some(pos) = hover {
                    apply_origin_at(app, id, pos);
                }
            }
            4 => {
                // Trackball: the drag's total offset from the press turns Y (horizontal) and X
                // (vertical), accumulated so a missed frame can't drop part of the gesture.
                if let Some(pos) = hover {
                    let dx = f64::from(pos.x - g.press.x) * TRACKBALL_PER_PT;
                    let dy = f64::from(pos.y - g.press.y) * TRACKBALL_PER_PT;
                    apply_trackball(app, id, g.start_value_b + dx, start_value + dy);
                }
            }
            _ => {
                if let (Some(_), Some(d)) = (hover, hover.map(|p| wrap_deg(angle(p, pivot) - start_angle))) {
                    let cmd = match axis { 0 => "rotateX", 1 => "rotateY", _ => "rotateZ" };
                    apply_turn(app, id, cmd, (start_value + d).clamp(-360.0, 360.0));
                }
            }
        }
        return true;
    }
    // A press grabs: the pivot dot, a ring, or the ball's interior (trackball).
    if !ctx.input(|i| i.pointer.primary_pressed()) {
        return false;
    }
    let Ok(doc) = app.session.doc() else { return false };
    let Some(id) = app.session.selection().iter().copied().find(|id| figcraft_doc::transform3d::projector(&doc, *id).is_some()) else { return false };
    let pivot = screen_pivot(app, id);
    // The pivot dot first.
    if pivot.distance(pos) < 10.0 {
        app.session.begin_interaction("3D gizmo").ok();
        app.canvas.t3_gizmo = Some(super::GizmoGrab { id, axis: 3, start_angle: 0.0, start_value: 0.0, press: pos, start_value_b: 0.0 });
        return true;
    }
    // The nearest ring's axis, by the distance to its sampled points.
    let mut best: Option<(usize, f32)> = None;
    for axis in 0..3 {
        if let Some((pts, _)) = ring_samples(app, &doc, id, axis) {
            for (p, _) in &pts {
                let d = p.distance(pos);
                if d < GRAB_RADIUS && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((axis, d));
                }
            }
        }
    }
    if let Some((axis, _)) = best {
        app.session.begin_interaction("3D gizmo").ok();
        app.canvas.t3_gizmo = Some(super::GizmoGrab {
            id,
            axis,
            start_angle: angle(pos, pivot),
            start_value: current_turn(app, id, axis),
            press: pos,
            start_value_b: 0.0,
        });
        return true;
    }
    // Inside the ball: free rotation.
    if let Some(r) = ball_radius(app, &doc, id)
        && pivot.distance(pos) < r
    {
        app.session.begin_interaction("3D gizmo").ok();
        app.canvas.t3_gizmo = Some(super::GizmoGrab {
            id,
            axis: 4,
            start_angle: 0.0,
            start_value: current_turn(app, id, 0),
            press: pos,
            start_value_b: current_turn(app, id, 1),
        });
        return true;
    }
    false
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

/// The visible 3D transform effect of `id`, cloned (`None` without one).
fn effect_of(app: &FigApp, id: NodeId) -> Option<figcraft_doc::Effect> {
    app.session.doc().ok().and_then(|doc| doc.node(id).cloned()).and_then(|n| {
        n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
            figcraft_doc::EffectKind::Transform3d(_) => Some(e.clone()),
            _ => None,
        })
    })
}

/// The origin moves to the pointer's place on the layer, in the layer's own percent.
fn apply_origin_at(app: &mut FigApp, id: NodeId, pos: Pos2) {
    let Some(mut e) = effect_of(app, id) else { return };
    let Some((w, h)) = app.session.doc().ok().and_then(|doc| doc.node(id)).map(|n| (n.width, n.height)) else { return };
    let page = app.to_page(pos);
    let Some(local) = app.session.doc().ok().map(|doc| doc.world_transform(id).inverse() * Point::new(page.x, page.y)) else { return };
    if let figcraft_doc::EffectKind::Transform3d(t) = &mut e.kind {
        t.origin = ((local.x / w.max(1e-6) * 100.0).clamp(0.0, 100.0), (local.y / h.max(1e-6) * 100.0).clamp(0.0, 100.0));
    }
    app.session.execute("effect.set", &serde_json::json!({"ids": [id.0], "index": 0, "effect": e})).ok();
}

/// One turn's new value, through `effect.set` (the gesture batches it).
fn apply_turn(app: &mut FigApp, id: NodeId, prop: &str, value: f64) {
    let Some(mut e) = effect_of(app, id) else { return };
    if let figcraft_doc::EffectKind::Transform3d(t) = &mut e.kind {
        match prop {
            "rotateX" => t.rotate_x = value,
            "rotateY" => t.rotate_y = value,
            _ => t.rotate_z = value,
        }
    }
    app.session.execute("effect.set", &serde_json::json!({"ids": [id.0], "index": 0, "effect": e})).ok();
}

/// The trackball's turns: `y` and `x` are the absolute values for this drag (batched).
fn apply_trackball(app: &mut FigApp, id: NodeId, y: f64, x: f64) {
    let Some(mut e) = effect_of(app, id) else { return };
    if let figcraft_doc::EffectKind::Transform3d(t) = &mut e.kind {
        t.rotate_y = y.clamp(-360.0, 360.0);
        t.rotate_x = x.clamp(-360.0, 360.0);
    }
    app.session.execute("effect.set", &serde_json::json!({"ids": [id.0], "index": 0, "effect": e})).ok();
}
