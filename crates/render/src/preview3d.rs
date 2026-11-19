//! The 3D transform popover's preview: the layer as an extruded slab, turned by the effect's own
//! values about its centre, under a fixed camera. The front face is the layer's own render; the
//! back is the mirrored, darkened render; the four sides are the silhouette's colour, shaded by
//! their face; a dot marks the pivot. Transparent background: the popover's panel shows through.

use figcraft_doc::{Document, Node, NodeId, Transform3d};
use figcraft_geom::{Affine, Homography, Point, Rect};
use tiny_skia::{Pixmap, Transform};

use crate::paint::composite;
use crate::warp::warp;

/// The camera distance of the preview itself (px): far enough to read as a mild perspective.
const CAMERA: f64 = 1400.0;
/// The slab's illustrative thickness when the effect's depth is 0 (the preview always shows a
/// body, so the third dimension reads even before a depth is set).
const MIN_THICKNESS: f64 = 70.0;

/// The turned slab of layer `id` drawn into a transparent pixmap of `size` (px, the preview
/// panel's own pixels).
pub fn cube(doc: &Document, id: NodeId, size: (u32, u32)) -> Option<Pixmap> {
    let n = doc.node(id)?;
    let t3 = turned(n)?;
    let (pw, ph) = (f64::from(size.0), f64::from(size.1));
    if pw < 8.0 || ph < 8.0 {
        return None;
    }
    let (w, h) = (n.width.max(1.0), n.height.max(1.0));
    let pivot = t3.pivot(w, h);
    // The thickness: the effect's own depth, at least the illustrative minimum (signed the same
    // way, so a negative depth's body reaches forward).
    let thickness = if t3.depth < 0.0 { -(-t3.depth).max(MIN_THICKNESS) } else { t3.depth.max(MIN_THICKNESS) };
    // The plane's homography, about the pivot, in the layer's own coordinates.
    let front = figcraft_doc::transform3d::plane_homography(&t3, pivot, Some(CAMERA));
    let back_t3 = Transform3d { depth: t3.depth - thickness, ..t3.clone() };
    let back = figcraft_doc::transform3d::plane_homography(&back_t3, pivot, Some(CAMERA));
    // The local box's extent through the two planes (local units, about the pivot): how much
    // room the slab takes, for the fit.
    let rect = Rect::new(0.0, 0.0, w, h);
    let (fx0, fx1) = extent(&front.quad(rect), |p| p.x);
    let (fy0, fy1) = extent(&front.quad(rect), |p| p.y);
    let (bx0, bx1) = extent(&back.quad(rect), |p| p.x);
    let (by0, by1) = extent(&back.quad(rect), |p| p.y);
    let span_x = (fx1 - fx0).max(bx1 - bx0).max(1.0);
    let span_y = (fy1 - fy0).max(by1 - by0).max(1.0);
    let fit = ((pw * 0.72) / span_x).min((ph * 0.72) / span_y).min(8.0);
    if !(fit.is_finite() && fit > 0.0) {
        return None;
    }
    // Turned local → preview: the pivot sits at the panel's centre.
    let place = |h: Homography| {
        Homography::from_affine(Affine::translate((pw / 2.0, ph / 2.0)))
            .mul(Homography::from_affine(Affine::scale(fit)))
            .mul(h)
            .mul(Homography::from_affine(Affine::translate((-pivot.0, -pivot.1))))
    };
    let (front, back) = (place(front), place(back));
    // The layer's flat render, in page coordinates.
    let bounds = crate::raster_bounds(doc, id)?;
    let content_w = ((bounds.width() * fit * 2.0).ceil().min(1024.0) as u32).max(1);
    let content_h = ((bounds.height() * fit * 2.0).ceil().min(1024.0) as u32).max(1);
    let view = crate::View { origin: (bounds.x0, bounds.y0), scale: (fit * 2.0).min(8.0), width: content_w, height: content_h };
    let content = crate::render_alone(doc, id, &view, crate::Alone::Plain)?;
    let rs = f64::from(content.width()) / bounds.width().max(1e-6);
    // The page → preview homography: through the layer's placement, the turn, and the fit.
    let page_to_preview = |plane: Homography| {
        Homography::from_affine(place_affine(pw, ph, fit, pivot))
            .mul(plane)
            .mul(Homography::from_affine(doc.world_transform(id).inverse()))
    };
    let front_h = page_to_preview(front);
    let back_h = page_to_preview(back);
    let Some(mut out) = Pixmap::new(size.0, size.1) else { return None };
    // The back face first: the mirrored, darkened render.
    if let Some((mut face, at)) = warp(&content, bounds, rs, &back_h, size.0, size.1) {
        dim(&mut face, 0.35);
        composite(&mut out, at, &face, 1.0, figcraft_color::BlendMode::Normal);
    }
    // The four sides: the silhouette's colour, shaded per face (the light from the top left).
    let average = average_colour(&content);
    let world = doc.world_transform(id);
    let page_corner = |local: (f64, f64)| world * Point::new(local.0, local.1);
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let pa = page_corner(a);
        let pb = page_corner(b);
        let (fa, fb) = (front_h.apply(pa.x, pa.y), front_h.apply(pb.x, pb.y));
        let (ba, bb) = (back_h.apply(pa.x, pa.y), back_h.apply(pb.x, pb.y));
        let shade = match i {
            0 => 0.86, // top
            1 => 0.62, // right
            2 => 0.42, // bottom
            _ => 0.55, // left
        };
        fill_quad(&mut out, [fa, fb, bb, ba], average, shade);
    }
    // The front face: the layer's own render, turned.
    if let Some((face, at)) = warp(&content, bounds, rs, &front_h, size.0, size.1) {
        composite(&mut out, at, &face, 1.0, figcraft_color::BlendMode::Normal);
    }
    // The pivot dot.
    let (dx, dy) = front_h.apply(page_corner(pivot).x, page_corner(pivot).y);
    if let Some(dot) = tiny_skia::PathBuilder::from_circle(dx as f32, dy as f32, 3.5) {
        out.fill_path(&dot, &tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(css(0x0D, 0x99, 0xFF)), anti_alias: true, ..Default::default() }, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    Some(out)
}

fn turned(n: &Node) -> Option<Transform3d> {
    n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
        figcraft_doc::EffectKind::Transform3d(t) => Some(t.sanitized()),
        _ => None,
    })
}

fn place_affine(pw: f64, ph: f64, fit: f64, pivot: (f64, f64)) -> Affine {
    Affine::translate((pw / 2.0, ph / 2.0)) * Affine::scale(fit) * Affine::translate((-pivot.0, -pivot.1))
}

/// The smallest and largest coordinate along `at`.
fn extent(q: &[Point; 4], at: impl Fn(&Point) -> f64) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in q {
        let v = at(p);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

/// Fill a projected quad with `rgb` scaled by `shade`, anti-aliased.
fn fill_quad(pix: &mut Pixmap, q: [(f64, f64); 4], rgb: (u8, u8, u8), shade: f64) {
    let mut path = tiny_skia::PathBuilder::new();
    path.move_to(q[0].0 as f32, q[0].1 as f32);
    for corner in &q[1..] {
        path.line_to(corner.0 as f32, corner.1 as f32);
    }
    path.close();
    let Some(path) = path.finish() else { return };
    let (r, g, b) = rgb;
    let scale = |v: u8| ((f64::from(v) * shade).round() as u8).min(255);
    let paint = tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(css(scale(r), scale(g), scale(b))), anti_alias: true, ..Default::default() };
    pix.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
}

/// The average premultiplied colour of the raster, demultiplied (its silhouette's tone).
fn average_colour(pix: &Pixmap) -> (u8, u8, u8) {
    let pixels = pix.pixels();
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for (i, c) in pixels.iter().enumerate() {
        if i % 13 != 0 || c.alpha() == 0 {
            continue;
        }
        let a = f64::from(c.alpha());
        r += (f64::from(c.red()) * 255.0 / a) as u64;
        g += (f64::from(c.green()) * 255.0 / a) as u64;
        b += (f64::from(c.blue()) * 255.0 / a) as u64;
        n += 1;
    }
    if n == 0 {
        return (0x88, 0x88, 0x88);
    }
    ((r / n).min(255) as u8, (g / n).min(255) as u8, (b / n).min(255) as u8)
}

/// Darken a premultiplied face in place (its RGB towards black, alpha kept).
fn dim(pix: &mut Pixmap, k: f64) {
    for c in pix.pixels_mut() {
        let d = |v: u8| ((f64::from(v) * k).round() as u8).min(c.alpha());
        *c = tiny_skia::PremultipliedColorU8::from_rgba(d(c.red()), d(c.green()), d(c.blue()), c.alpha()).unwrap_or(*c);
    }
}

fn css(r: u8, g: u8, b: u8) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(r, g, b, 255)
}
