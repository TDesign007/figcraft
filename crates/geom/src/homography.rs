//! A 3×3 projective transform: what a flat plane turned in 3D and seen through a camera maps
//! through, where a 2D affine can't hold the perspective.

use kurbo::{Affine, Point, Rect};

/// A 3×3 projective transform (a homography), row-major: a point `(x, y, 1)` maps to
/// `(x', y', w')`, and the screen point is `(x'/w', y'/w')`.
///
/// Everything a flat layer turned in 3D and projected goes through one of these: the layer's
/// rotation about its plane (an affine in 3D) composed with the camera's perspective divide is
/// linear-over-linear in the plane's own coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homography([f64; 9]);

impl Homography {
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

    /// From row-major rows.
    pub const fn from_rows(m: [f64; 9]) -> Self {
        Self(m)
    }

    /// From a 2D affine (kurbo stores `[a, b, c, d, e, f]`: `x' = a·x + c·y + e`,
    /// `y' = b·x + d·y + f`).
    pub fn from_affine(a: Affine) -> Self {
        let [a, b, c, d, e, f] = a.as_coeffs();
        Self([a, c, e, b, d, f, 0.0, 0.0, 1.0])
    }

    /// The matrix product `self · other` (apply `other` first).
    #[allow(clippy::should_implement_trait)] // named for the product, not `*` (call sites stay readable)
    pub fn mul(self, o: Self) -> Self {
        let mut out = [0.0; 9];
        for r in 0..3 {
            for c in 0..3 {
                let mut sum = 0.0;
                for k in 0..3 {
                    sum += self.0[r * 3 + k] * o.0[k * 3 + c];
                }
                out[r * 3 + c] = sum;
            }
        }
        Self(out)
    }

    /// `(x, y)` through the transform, with the perspective divide. A point on the camera plane
    /// (whose `w` comes out at or past zero) maps nowhere: the coordinates come back infinite.
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let m = &self.0;
        let wx = m[0] * x + m[1] * y + m[2];
        let wy = m[3] * x + m[4] * y + m[5];
        let w = m[6] * x + m[7] * y + m[8];
        if w.abs() < 1e-12 {
            (f64::INFINITY, f64::INFINITY)
        } else {
            (wx / w, wy / w)
        }
    }

    /// The inverse, `None` when the transform is singular (a plane seen edge-on).
    pub fn inverse(&self) -> Option<Self> {
        let m = &self.0;
        let (a, b, c, d, e, f, g, h, i) = (m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8]);
        let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        let k = 1.0 / det;
        // The adjugate, transposed.
        Some(Self([
            (e * i - f * h) * k,
            (c * h - b * i) * k,
            (b * f - c * e) * k,
            (f * g - d * i) * k,
            (a * i - c * g) * k,
            (c * d - a * f) * k,
            (d * h - e * g) * k,
            (b * g - a * h) * k,
            (a * e - b * d) * k,
        ]))
    }

    /// Whether the transform mirrors the plane (its winding flips): the layer seen from behind.
    pub fn flips(&self) -> bool {
        let m = &self.0;
        m[0] * m[4] - m[1] * m[3] < 0.0
    }

    /// The corners of rect `r` through the transform, in order (top left, top right, bottom
    /// right, bottom left).
    pub fn quad(&self, r: Rect) -> [Point; 4] {
        let (x0, y0, x1, y1) = (r.x0, r.y0, r.x1, r.y1);
        let at = |x: f64, y: f64| {
            let (qx, qy) = self.apply(x, y);
            Point::new(qx, qy)
        };
        [at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)]
    }
}

#[cfg(test)]
mod tests {
    use super::Homography;
    use kurbo::{Affine, Point, Rect};

    /// Identity applies as itself, inverts to itself.
    #[test]
    fn identity_round_trips() {
        let h = Homography::IDENTITY;
        assert_eq!(h.apply(3.0, -4.0), (3.0, -4.0));
        assert_eq!(h.inverse(), Some(Homography::IDENTITY));
        assert!(!h.flips());
    }

    /// An affine round-trips through the homography form.
    #[test]
    fn affine_round_trips() {
        let a = Affine::translate((30.0, 40.0)) * Affine::rotate(0.5) * Affine::scale_non_uniform(2.0, 3.0);
        let h = Homography::from_affine(a);
        let (x, y) = h.apply(5.0, 7.0);
        let q = a * Point::new(5.0, 7.0);
        assert!((x - q.x).abs() < 1e-9 && (y - q.y).abs() < 1e-9);
        let back = h.inverse().unwrap();
        let (u, v) = back.apply(x, y);
        assert!((u - 5.0).abs() < 1e-9 && (v - 7.0).abs() < 1e-9);
    }

    /// A perspective divide: a plane point pushed toward the camera (w smaller than one) lands
    /// further from the origin than the affine part alone puts it.
    #[test]
    fn perspective_divides_by_w() {
        // w = 0.5 at every point: everything doubles.
        let h = Homography::from_rows([2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(h.apply(10.0, 20.0), (20.0, 40.0));
        // On the camera plane (w = 0) the point maps nowhere.
        let h = Homography::from_rows([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.2, 0.0, -1.0]);
        let (x, y) = h.apply(5.0, 5.0);
        assert!(x.is_infinite() && y.is_infinite());
    }

    /// The product composes in application order: `a.mul(b)` applies `b` first.
    #[test]
    fn mul_composes() {
        let t = Homography::from_affine(Affine::translate((10.0, 0.0)));
        let s = Homography::from_affine(Affine::scale(2.0));
        let ts = t.mul(s);
        assert_eq!(ts.apply(5.0, 0.0), (20.0, 0.0), "scale first, then translate");
        let st = s.mul(t);
        assert_eq!(st.apply(5.0, 0.0), (30.0, 0.0), "translate first, then scale");
    }

    /// A mirrored transform flips the winding.
    #[test]
    fn flips_detects_a_mirror() {
        let m = Homography::from_affine(Affine::scale_non_uniform(-1.0, 1.0));
        assert!(m.flips());
        let r = Homography::from_affine(Affine::rotate(1.0));
        assert!(!r.flips());
    }

    /// A quad's corners come back in order.
    #[test]
    fn quad_corners() {
        let h = Homography::IDENTITY;
        let q = h.quad(Rect::new(0.0, 0.0, 10.0, 20.0));
        assert_eq!(q, [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 20.0), Point::new(0.0, 20.0)]);
    }
}
