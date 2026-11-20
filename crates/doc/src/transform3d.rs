//! The 3D transform's math: a flat layer turned about its centre and seen through a camera is a
//! homography from the layer's own plane to the page, where an affine can't hold the perspective.
//!
//! The conventions match the CSS values the feature maps to: `rotateX(a) rotateY(b)` (so `b`
//! turns first), `translateZ(depth)` along the turned plane's normal (positive toward the
//! camera), y-down coordinates, and a perspective distance dividing by `d / (d − z)`.

use crate::{Document, EffectKind, Node, Transform3d};
use figcraft_geom::homography::Homography;
use figcraft_geom::{Affine, Point, Rect};

/// The plane's homography about `pivot` (the layer's own coordinates, before its placement): the
/// turn about the X and Y axes, the depth along the turned normal, and the camera's perspective
/// divide (`None`: no perspective — the projection is orthographic, where depth shows nothing).
pub fn plane_homography(t3: &Transform3d, pivot: (f64, f64), perspective: Option<f64>) -> Homography {
    let t3 = t3.sanitized();
    let (ax, ay, az) = (t3.rotate_x.to_radians(), t3.rotate_y.to_radians(), t3.rotate_z.to_radians());
    let (cx, sx, cy, sy, cz, sz) = (ax.cos(), ax.sin(), ay.cos(), ay.sin(), az.cos(), az.sin());
    // The affine part in 3D, restricted to the plane (z = 0): a plane point (x, y) goes through
    // `translateZ` after the turns, so its depth is linear in x and y.
    let turn = |x: f64, y: f64| -> (f64, f64, f64) {
        // The plane spins in itself first (CSS composes `rotateX(a) rotateY(b) rotateZ(c)` as
        // Rx·Ry·Rz), then rotateY, then rotateX.
        let (x0, y0) = (x * cz - y * sz, x * sz + y * cz);
        // rotateY.
        let x1 = x0 * cy;
        let z1 = -x0 * sy;
        let y1 = y0;
        // Then rotateX.
        let y2 = y1 * cx - z1 * sx;
        let z2 = y1 * sx + z1 * cx;
        (x1, y2, z2 + t3.depth)
    };
    let (px, py) = pivot;
    let q0 = turn(-px, -py);
    let qx = turn(-px + 1.0, -py);
    let qy = turn(-px, -py + 1.0);
    // Each 3D coordinate is affine in (x, y): q = q0 + x·(qx − q0) + y·(qy − q0).
    let lin = |q: (f64, f64, f64), q0: (f64, f64, f64)| (q.0 - q0.0, q.1 - q0.1, q.2 - q0.2);
    let (a1, a2, a3) = lin(qx, q0);
    let (b1, b2, b3) = lin(qy, q0);
    let (c1, c2, c3) = q0;
    // The rows describe the turn about the pivot (the basis is evaluated at `p − pivot`): the
    // turned plane lands back into place with the pivot added afterwards.
    let rel = match perspective {
        // The perspective divide turns linear-over-linear into a homography: with the camera at
        // distance d, a point at depth z scales by d / (d − z).
        Some(d) if d.is_finite() && d > 0.0 => {
            Homography::from_rows([
                a1 * d, b1 * d, c1 * d, //
                a2 * d, b2 * d, c2 * d, //
                -a3, -b3, d - c3, //
            ])
        }
        // No camera (or one at infinity): the projection is orthographic, an affine.
        _ => Homography::from_rows([a1, b1, c1, a2, b2, c2, 0.0, 0.0, 1.0]),
    };
    Homography::from_affine(Affine::translate((px, py))).mul(rel)
}

/// The first visible 3D transform of `n`, sanitised (`None`: none).
pub fn of(n: &Node) -> Option<Transform3d> {
    n.effects
        .iter()
        .filter(|e| e.visible)
        .find_map(|e| match &e.kind {
            EffectKind::Transform3d(t) => Some(t.sanitized()),
            _ => None,
        })
}

/// The nearest ancestor frame's Perspective distance for `id` (its own effect doesn't count: a
/// camera sits on a parent). `None`: orthographic.
pub fn perspective_of(doc: &Document, id: crate::NodeId) -> Option<f64> {
    let mut parent = doc.node(id).and_then(|n| n.parent);
    while let Some(pid) = parent {
        let n = doc.node(pid)?;
        if n.kind.is_frame_like()
            && let Some(crate::EffectKind::Perspective(p)) = n.effects.iter().find(|e| e.visible).map(|e| &e.kind)
        {
            return Some(p.sanitized().distance);
        }
        parent = n.parent;
    }
    None
}

/// The layer's homography in page coordinates: the plane's turn about its centre composed with
/// its placement (`None`: no visible 3D transform).
pub fn node_homography(doc: &Document, id: crate::NodeId) -> Option<Homography> {
    let n = doc.node(id)?;
    let t3 = of(n)?;
    let plane = plane_homography(&t3, t3.pivot(n.width, n.height), perspective_of(doc, id));
    Some(Homography::from_affine(doc.world_transform(id)).mul(plane))
}

/// The page point that `pt` hits on the turned layer's flat plane: the hit point taken out of the
/// turn (through the plane homography's inverse), then back into the layer's un-turned
/// placement — so the hit test and everything inside it reads the flat plane as if the turn
/// weren't there. `None` without a visible 3D transform, or on a plane seen edge-on.
pub fn unwarped(doc: &Document, id: crate::NodeId, pt: Point) -> Option<Point> {
    let n = doc.node(id)?;
    let t3 = of(n)?;
    let world = doc.world_transform(id);
    let pivot = t3.pivot(n.width, n.height);
    let hp = plane_homography(&t3, pivot, perspective_of(doc, id));
    let inv = hp.inverse()?;
    // Into the layer's own coordinates, out of the turn, back onto the un-turned placement.
    let local = world.inverse() * pt;
    let (x, y) = inv.apply(local.x, local.y);
    (x.is_finite() && y.is_finite()).then_some(world * Point::new(x, y))
}

/// The layer's 3D → page projector: any point of the layer's own space (x, y in the plane, z
/// out of it, both in px about the pivot) through the turns, the depth and the camera, for the
/// gizmo's rings and anything else that lives off the flat plane.
#[derive(Clone, Copy, Debug)]
pub struct Projector {
    world: Affine,
    pivot: (f64, f64),
    depth: f64,
    d: Option<f64>,
    cx: f64,
    sx: f64,
    cy: f64,
    sy: f64,
    cz: f64,
    sz: f64,
}

impl Projector {
    /// The page point of the layer-space point `(x, y, z)`.
    pub fn apply(&self, x: f64, y: f64, z: f64) -> (f64, f64) {
        let (px, py) = self.pivot;
        // The plane out of the pivot, spun in itself first (as CSS's rotateZ), then turned.
        let (x0, y0) = ((x - px) * self.cz - (y - py) * self.sz, (x - px) * self.sz + (y - py) * self.cz);
        let x1 = x0 * self.cy;
        let z1 = -x0 * self.sy;
        let y1 = y0;
        let y2 = y1 * self.cx - z1 * self.sx;
        let z2 = y1 * self.sx + z1 * self.cx;
        let z3 = z2 + z + self.depth;
        // The camera's divide (orthographic without one).
        let (x, y) = match self.d {
            Some(d) if d > 0.0 && (d - z3).abs() > 1e-9 => {
                let k = d / (d - z3);
                (x1 * k + px, y2 * k + py)
            }
            _ => (x1 + px, y2 + py),
        };
        let q = self.world * Point::new(x, y);
        (q.x, q.y)
    }

    /// The pivot in the layer's own coordinates.
    pub fn pivot(&self) -> (f64, f64) {
        self.pivot
    }

    /// The pivot in page coordinates (the gizmo's centre).
    pub fn pivot_page(&self) -> (f64, f64) {
        let q = self.apply(self.pivot.0, self.pivot.1, 0.0);
        (q.0, q.1)
    }

    /// The pivot's distance from a point of the layer, in the layer's own px.
    pub fn radius(&self, w: f64, h: f64) -> f64 {
        ((self.pivot.0.abs() + (w - self.pivot.0).abs()).max(self.pivot.1.abs() + (h - self.pivot.1).abs())) * 0.45
    }
}

/// The projector of layer `id`'s visible 3D transform (`None`: none).
pub fn projector(doc: &Document, id: crate::NodeId) -> Option<Projector> {
    let n = doc.node(id)?;
    let t3 = of(n)?;
    let (cx, sx) = (t3.rotate_x.to_radians().cos(), t3.rotate_x.to_radians().sin());
    let (cy, sy) = (t3.rotate_y.to_radians().cos(), t3.rotate_y.to_radians().sin());
    let (cz, sz) = (t3.rotate_z.to_radians().cos(), t3.rotate_z.to_radians().sin());
    Some(Projector { world: doc.world_transform(id), pivot: t3.pivot(n.width, n.height), depth: t3.depth, d: perspective_of(doc, id), cx, sx, cy, sy, cz, sz })
}

/// The nearest ancestor frame's Perspective distance for `n`'s subtree (its own effect doesn't
/// count), for code that holds the node but not its id.
pub fn perspective_of_node(doc: &Document, n: &Node) -> Option<f64> {
    let mut parent = n.parent;
    while let Some(pid) = parent {
        let p = doc.node(pid)?;
        if p.kind.is_frame_like()
            && let Some(crate::EffectKind::Perspective(persp)) = p.effects.iter().filter(|e| e.visible).find_map(|e| Some(&e.kind))
        {
            return Some(persp.sanitized().distance);
        }
        parent = p.parent;
    }
    None
}

/// The layer's projected corners in page coordinates (its local box through the homography),
/// for the selection outline and the warped render's extent. `None` without a visible 3D
/// transform.
pub fn projected_quad(doc: &Document, id: crate::NodeId) -> Option<[Point; 4]> {
    let n = doc.node(id)?;
    if of(n).is_none() {
        return None;
    }
    let h = node_homography(doc, id)?;
    Some(h.quad(Rect::new(0.0, 0.0, n.width, n.height)))
}

#[cfg(test)]
mod tests {
    use super::plane_homography;
    use crate::Transform3d;
    use figcraft_geom::Rect;

    /// An orthographic turn only foreshortens: a 100 × 100 plane turned 45° about X is 100 wide
    /// and 100·cos 45° tall, centred where it was.
    #[test]
    fn orthographic_turn_foreshortens() {
        let t3 = Transform3d { rotate_x: 45.0, ..Transform3d::default() };
        let h = plane_homography(&t3, (50.0, 50.0), None);
        let q = h.quad(Rect::new(0.0, 0.0, 100.0, 100.0));
        let ys: Vec<f64> = q.iter().map(|p| p.y).collect();
        let height = ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
        let xs: Vec<f64> = q.iter().map(|p| p.x).collect();
        let width = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
        assert!((width - 100.0).abs() < 1e-9, "width {width}");
        assert!((height - 100.0 * 0.5f64.sqrt()).abs() < 1e-9, "height {height}");
        // The centre stays at the pivot.
        let (cx, cy) = h.apply(50.0, 50.0);
        assert!((cx - 50.0).abs() < 1e-9 && (cy - 50.0).abs() < 1e-9);
    }

    /// Depth toward the camera scales the plane up under a perspective, and does nothing
    /// without one.
    #[test]
    fn depth_scales_only_with_a_perspective() {
        let t3 = Transform3d { depth: 400.0, ..Transform3d::default() };
        let ortho = plane_homography(&t3, (50.0, 50.0), None);
        assert_eq!(ortho.apply(50.0, 50.0), (50.0, 50.0), "orthographic depth is invisible");
        let p = plane_homography(&t3, (50.0, 50.0), Some(800.0));
        // The plane's centre sits at z = 400: it scales by 800 / (800 − 400) = 2 about the pivot.
        let (x, y) = p.apply(50.0, 50.0);
        assert!((x - 50.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9, "the pivot stays");
        let (x, y) = p.apply(0.0, 0.0);
        assert!((x - (-50.0)).abs() < 1e-9 && (y - (-50.0)).abs() < 1e-9, "a corner doubles away from the pivot");
    }

    /// A perspective turn is a true homography: parallel edges converge (the far edge is
    /// shorter than the near one).
    #[test]
    fn perspective_turn_converges_edges() {
        let t3 = Transform3d { rotate_x: 45.0, ..Transform3d::default() };
        let h = plane_homography(&t3, (50.0, 50.0), Some(800.0));
        let q = h.quad(Rect::new(0.0, 0.0, 100.0, 100.0));
        let top = q[0].distance(q[1]);
        let bottom = q[3].distance(q[2]);
        assert!((top - bottom).abs() > 1.0, "the far edge differs from the near one: {top} vs {bottom}");
    }

    /// A pivot at a corner keeps that corner exactly where it was: the turn happens about it.
    #[test]
    fn pivot_at_a_corner_stays_put() {
        let t3 = Transform3d { rotate_x: 40.0, origin: (0.0, 0.0), ..Transform3d::default() };
        let h = plane_homography(&t3, (0.0, 0.0), Some(800.0));
        let (x, y) = h.apply(0.0, 0.0);
        assert!((x - 0.0).abs() < 1e-9 && (y - 0.0).abs() < 1e-9, "the origin corner stays: ({x}, {y})");
    }

    /// A point maps back to itself through the turn and its inverse.
    #[test]
    fn plane_round_trips_through_the_inverse() {
        let t3 = Transform3d { rotate_x: 30.0, rotate_y: 20.0, rotate_z: 0.0, depth: 50.0, origin: (50.0, 50.0), backface: true };
        let h = plane_homography(&t3, (50.0, 50.0), Some(800.0));
        let inv = h.inverse().unwrap();
        for (x, y) in [(0.0, 0.0), (100.0, 0.0), (50.0, 50.0), (13.0, 87.0)] {
            let (qx, qy) = h.apply(x, y);
            let (bx, by) = inv.apply(qx, qy);
            assert!((bx - x).abs() < 1e-9 && (by - y).abs() < 1e-9, "({x}, {y}) -> ({bx}, {by})");
        }
    }
}
