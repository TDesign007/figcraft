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
/// The slab's illustrative thickness floor when the effect's depth is 0 (the preview always
/// shows a body, so the third dimension reads even before a depth is set).
const MIN_THICKNESS: f64 = 48.0;

/// The turned slab of layer `id` drawn into a transparent pixmap of `size` (px, the preview
/// panel's own pixels).
pub fn cube(doc: &Document, id: NodeId, size: (u32, u32)) -> Option<(Pixmap, Option<(f32, f32)>)> {
    let n = doc.node(id)?;
    let t3 = turned(n)?;
    let (pw, ph) = (f64::from(size.0), f64::from(size.1));
    if pw < 8.0 || ph < 8.0 {
        return None;
    }
    let (w, h) = (n.width.max(1.0), n.height.max(1.0));
    let pivot = t3.pivot(w, h);
    // The thickness: none while the plane is still flat (the preview starts as the layer's own
    // plane); once it is turned or given a depth it is a slab — the effect's depth, else a share
    // of the layer's smaller side so the 3D body reads (a cube for a square layer).
    let thickness = slab_thickness(&t3, w, h);
    let slab = thickness.abs() > 0.01;
    // The plane's homography, about the pivot, in the layer's own coordinates.
    let front = figcraft_doc::transform3d::plane_homography(&t3, pivot, Some(CAMERA));
    let back = slab.then(|| {
        let back_t3 = Transform3d { depth: t3.depth - thickness, ..t3.clone() };
        figcraft_doc::transform3d::plane_homography(&back_t3, pivot, Some(CAMERA))
    });
    // The local box's extent through the plane(s) (local units, about the pivot): how much room
    // the slab takes, for the fit.
    let rect = Rect::new(0.0, 0.0, w, h);
    let (fx0, fx1) = extent(&front.quad(rect), |p| p.x);
    let (fy0, fy1) = extent(&front.quad(rect), |p| p.y);
    let (mut span_x, mut span_y) = ((fx1 - fx0).max(1.0), (fy1 - fy0).max(1.0));
    if let Some(back) = &back {
        let (bx0, bx1) = extent(&back.quad(rect), |p| p.x);
        let (by0, by1) = extent(&back.quad(rect), |p| p.y);
        span_x = span_x.max(bx1 - bx0);
        span_y = span_y.max(by1 - by0);
    }
    let fit = ((pw * 0.72) / span_x).min((ph * 0.72) / span_y).min(8.0);
    if !(fit.is_finite() && fit > 0.0) {
        return None;
    }
    // The layer's flat render, in page coordinates.
    let bounds = crate::raster_bounds(doc, id)?;
    let content_w = ((bounds.width() * fit * 2.0).ceil().min(1024.0) as u32).max(1);
    let content_h = ((bounds.height() * fit * 2.0).ceil().min(1024.0) as u32).max(1);
    let view = crate::View { origin: (bounds.x0, bounds.y0), scale: (fit * 2.0).min(8.0), width: content_w, height: content_h };
    let content = crate::render_alone(doc, id, &view, crate::Alone::Plain)?;
    let rs = f64::from(content.width()) / bounds.width().max(1e-6);
    // The page → preview homography: through the layer's placement, the turn, and the fit.
    let page_to_preview = |plane: Homography| {
        place_homography(pw, ph, fit, pivot).mul(plane).mul(Homography::from_affine(doc.world_transform(id).inverse()))
    };
    // The page → preview homography goes through the *unplaced* turn: the placement is already
    // inside `page_to_preview` (and `place` is for local pixels only).
    let front_h = page_to_preview(front);
    let mut out = Pixmap::new(size.0, size.1)?;
    let mut back_h = None;
    if let Some(back) = back {
        let h = page_to_preview(back);
        // The back face first: the mirrored, darkened render.
        if let Some((mut face, at)) = warp(&content, bounds, rs, &h, size.0, size.1) {
            dim(&mut face, 0.35);
            composite(&mut out, at, &face, 1.0, figcraft_color::BlendMode::Normal);
        }
        back_h = Some(h);
    }
    // The four sides (only a slab has them): the silhouette's colour, shaded per face (the
    // light from the top left).
    let world = doc.world_transform(id);
    let page_corner = |local: (f64, f64)| world * Point::new(local.0, local.1);
    if let Some(back_h) = &back_h {
        let average = average_colour(&content);
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
    }
    // The front face: the layer's own render, turned.
    if let Some((face, at)) = warp(&content, bounds, rs, &front_h, size.0, size.1) {
        composite(&mut out, at, &face, 1.0, figcraft_color::BlendMode::Normal);
    }
    // The pivot dot, and where it landed in the pixels (the panel puts its grab handle exactly
    // there). The centroid of what was drawn is the truth, even where the projection rounds.
    let (dx, dy) = front_h.apply(page_corner(pivot).x, page_corner(pivot).y);
    let mut dot = None;
    if let Some(path) = tiny_skia::PathBuilder::from_circle(dx as f32, dy as f32, 3.5) {
        out.fill_path(&path, &tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(css(0x0D, 0x99, 0xFF)), anti_alias: true, ..Default::default() }, tiny_skia::FillRule::Winding, Transform::identity(), None);
        let (mut sx, mut sy, mut n) = (0u64, 0u64, 0u64);
        for (i, c) in out.pixels().iter().enumerate() {
            if c.red() == 13 && c.green() == 153 && c.blue() == 255 {
                sx += (i % out.width() as usize) as u64;
                sy += (i / out.width() as usize) as u64;
                n += 1;
            }
        }
        if n > 0 {
            dot = Some((sx as f32 / n as f32 / pw as f32, sy as f32 / n as f32 / ph as f32));
        }
    }
    Some((out, dot))
}

/// The preview slab's thickness for `t3` on a `w` × `h` layer: 0 while the plane is still flat
/// (no X/Y turn and no depth — the preview shows the layer's own plane), else the effect's depth
/// with the illustrative floor that makes the 3D body read.
fn slab_thickness(t3: &Transform3d, w: f64, h: f64) -> f64 {
    let t3 = t3.sanitized();
    if t3.rotate_x.abs() < 0.01 && t3.rotate_y.abs() < 0.01 && t3.depth.abs() < 0.01 {
        return 0.0;
    }
    // The body grows with the turn: a 1° tilt is a plane barely off the page, and by about 30°
    // the preview reads as the full 3D model (still a plane at rest: that is the flat starting
    // point the drag begins from).
    let illustrative = MIN_THICKNESS.max(w.min(h) * 0.6);
    let grow = (t3.rotate_x.abs().max(t3.rotate_y.abs()) / 30.0).clamp(0.2, 1.0);
    if t3.depth < 0.0 { -(-t3.depth).max(illustrative * grow) } else { t3.depth.max(illustrative * grow) }
}

fn turned(n: &Node) -> Option<Transform3d> {
    n.effects.iter().filter(|e| e.visible).find_map(|e| match &e.kind {
        figcraft_doc::EffectKind::Transform3d(t) => Some(t.sanitized()),
        _ => None,
    })
}

/// The generic card the preview illustrates with (the reference's own artwork, not the layer's
/// content): a portrait rounded rectangle, stroked, no fill.
const CARD_W: f64 = 44.0;
const CARD_H: f64 = 62.0;
const CARD_RADIUS: f64 = 9.0;
const CARD_STROKE: f64 = 1.6;
/// The card outline's ink (a light grey, brighter than the dashed bounds').
const CARD_INK: (u8, u8, u8) = (0xC9, 0xC9, 0xC9);
/// The flanks' turn into the scene (the reference's fan: the middle card faces the camera, the
/// outer two angle inward, keystoned by the perspective).
const CARD_TILT: f64 = 24.0;
/// How far the flanks sit behind the plane: shares of the card's own diagonal, in card units.
/// Fixed depths (not shares of the camera distance) are what keep the slider meaningful: the
/// same scene through a nearer camera spreads more, and at the default camera the spread reads
/// exactly like the reference's illustration.
const CARD_DEPTHS: [f64; 3] = [-3.7, 0.0, -7.3];
/// The gap between neighbouring cards, as a share of the card's width.
const CARD_GAP: f64 = 0.42;
/// The dashed bounds' ink (a mid grey that reads on the light and the dark panel alike).
const DASH_INK: (u8, u8, u8) = (0x96, 0x96, 0x96);

/// The perspective popover's preview, the reference's illustration 1:1: three of the generic
/// cards receding in depth under the camera `distance` — the middle one facing the camera (the
/// largest), the outer two further away and turned inward (smaller, keystoned by the turn), a
/// dashed bounds round the three. A nearer camera shrinks the far cards harder, so scrubbing the
/// slider reads as the camera moving through the scene.
pub fn cards(size: (u32, u32), distance: f64) -> Option<Pixmap> {
    let (pw, ph) = (f64::from(size.0), f64::from(size.1));
    if pw < 8.0 || ph < 8.0 {
        return None;
    }
    let d = if distance.is_finite() { distance.clamp(1.0, 100_000.0) } else { 800.0 };
    let (w, h) = (CARD_W, CARD_H);
    let pivot = (w / 2.0, h / 2.0);
    // Each depth's projected scale: d / (d − z), the same divide the canvas draws with. The
    // middle card sits on the plane itself (scale 1), the outer two further away (smaller).
    let depths = CARD_DEPTHS.map(|share| share * w.hypot(h));
    let scales: [f64; 3] = std::array::from_fn(|i| d / (d - depths[i]));
    // The sideways offsets, laid out in projected units around the middle card (a gap of about
    // a third of the card's width), converted back into the planes' own units: applied ahead of
    // the projection, so the off-centre cards pick up the keystone.
    let side = |i: usize| (w * scales[1] + w * scales[i]) / 2.0 + CARD_GAP * w;
    let offsets = [-side(0), 0.0, side(2)];
    let rect = Rect::new(0.0, 0.0, w, h);
    // Left to right: the turned copy nearer the plane, the plane itself, the turned copy furthest.
    let planes: [Transform3d; 3] = [
        Transform3d { depth: depths[0], rotate_y: -CARD_TILT, ..Default::default() },
        Transform3d { depth: depths[1], ..Default::default() },
        Transform3d { depth: depths[2], rotate_y: CARD_TILT, ..Default::default() },
    ];
    let cards: [Homography; 3] = std::array::from_fn(|i| {
        figcraft_doc::transform3d::plane_homography(&planes[i], pivot, Some(d))
            .mul(Homography::from_affine(Affine::translate((offsets[i] / scales[i], 0.0))))
    });
    // The fit: the three cards' projected extent, 72% of the panel like the slab preview.
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for card in &cards {
        let q = card.quad(rect);
        let (a, b) = extent(&q, |p| p.x);
        let (c, e) = extent(&q, |p| p.y);
        x0 = x0.min(a);
        x1 = x1.max(b);
        y0 = y0.min(c);
        y1 = y1.max(e);
    }
    let (span_x, span_y) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let fit = ((pw * 0.72) / span_x).min((ph * 0.72) / span_y).min(8.0);
    if !(fit.is_finite() && fit > 0.0) {
        return None;
    }
    // The card's outline as its own raster (4 px per card unit, crisp under the warp), warped
    // per card like the layer's render was.
    let (card, card_rect) = card_texture(4.0)?;
    let rs = 4.0;
    let mut out = Pixmap::new(size.0, size.1)?;
    for card_h in &cards {
        let full = place_homography(pw, ph, fit, pivot).mul(*card_h);
        let q = full.quad(rect);
        let (a, b) = extent(&q, |p| p.x);
        let (c, e) = extent(&q, |p| p.y);
        x0 = x0.min(a);
        x1 = x1.max(b);
        y0 = y0.min(c);
        y1 = y1.max(e);
        if let Some((face, at)) = warp(&card, card_rect, rs, &full, size.0, size.1) {
            composite(&mut out, at, &face, 1.0, figcraft_color::BlendMode::Normal);
        }
    }
    // The dashed bounds, a padding out from the cards' union (the reference's selection box).
    dashed_rect(&mut out, Rect::new(x0, y0, x1, y1), 12.0, pw, ph);
    Some(out)
}

/// The generic card's outline as its own raster (`scale` px per card unit), transparent but for
/// the stroke, with the rect it covers in card units (the warp's source window).
fn card_texture(scale: f64) -> Option<(Pixmap, Rect)> {
    let (tw, th) = ((CARD_W * scale).round() as u32, (CARD_H * scale).round() as u32);
    let mut pix = Pixmap::new(tw, th)?;
    // The outline's centre line, inset half the stroke so it fits the card's bounds; the
    // rounded corners as cubic arcs (the circle's 0.5523 kappa).
    let s = CARD_STROKE / 2.0;
    let (x0, y0, x1, y1) = (s, s, CARD_W - s, CARD_H - s);
    let r = CARD_RADIUS.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    let k = 0.5523 * r;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to((x0 + r) as f32, y0 as f32);
    pb.line_to((x1 - r) as f32, y0 as f32);
    pb.cubic_to((x1 - r + k) as f32, y0 as f32, x1 as f32, (y0 + r - k) as f32, x1 as f32, (y0 + r) as f32);
    pb.line_to(x1 as f32, (y1 - r) as f32);
    pb.cubic_to(x1 as f32, (y1 - r + k) as f32, (x1 - r + k) as f32, y1 as f32, (x1 - r) as f32, y1 as f32);
    pb.line_to((x0 + r) as f32, y1 as f32);
    pb.cubic_to((x0 + r - k) as f32, y1 as f32, x0 as f32, (y1 - r + k) as f32, x0 as f32, (y1 - r) as f32);
    pb.line_to(x0 as f32, (y0 + r) as f32);
    pb.cubic_to(x0 as f32, (y0 + r - k) as f32, (x0 + r - k) as f32, y0 as f32, (x0 + r) as f32, y0 as f32);
    pb.close();
    let path = pb.finish()?;
    let (cr, cg, cb) = CARD_INK;
    let paint = tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(css(cr, cg, cb)), anti_alias: true, ..Default::default() };
    let stroke = tiny_skia::Stroke { width: CARD_STROKE as f32, ..Default::default() };
    pix.stroke_path(&path, &paint, &stroke, Transform::from_scale(scale as f32, scale as f32), None);
    Some((pix, Rect::new(0.0, 0.0, CARD_W, CARD_H)))
}

/// Dashed bars along the four sides of `quad` (a hand-laid dash: the raster draws fills, not
/// strokes), padded out by `pad` px and kept inside the pixmap.
fn dashed_rect(pix: &mut Pixmap, quad: Rect, pad: f64, pw: f64, ph: f64) {
    let x0 = (quad.x0 - pad).max(1.0).round();
    let y0 = (quad.y0 - pad).max(1.0).round();
    let x1 = (quad.x1 + pad).min(pw - 1.0).round();
    let y1 = (quad.y1 + pad).min(ph - 1.0).round();
    if x1 - x0 < 8.0 || y1 - y0 < 8.0 {
        return;
    }
    let (dash, gap) = (6.0f64, 5.0f64);
    let mut pb = tiny_skia::PathBuilder::new();
    // Pairs of bars on the two sides of one axis: `a`/`b` are the fixed coordinates, `t` walks
    // the other axis in dash-gap steps.
    let run = |pb: &mut tiny_skia::PathBuilder, mut t: f64, end: f64, a: f64, b: f64, vertical: bool| {
        while t < end {
            let e = (t + dash).min(end);
            if e - t >= 1.0 {
                for fixed in [a, b] {
                    let r = if vertical {
                        tiny_skia::Rect::from_xywh(fixed as f32, t as f32, 1.0, (e - t) as f32)
                    } else {
                        tiny_skia::Rect::from_xywh(t as f32, fixed as f32, (e - t) as f32, 1.0)
                    };
                    if let Some(r) = r {
                        pb.push_rect(r);
                    }
                }
            }
            t = e + gap;
        }
    };
    run(&mut pb, x0, x1, y0, y1 - 1.0, false);
    run(&mut pb, y0, y1, x0, x1 - 1.0, true);
    if let Some(path) = pb.finish() {
        let (r, g, b) = DASH_INK;
        let paint = tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(css(r, g, b)), anti_alias: false, ..Default::default() };
        pix.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
}

/// The fitted placement: the turned plane's local coordinates (the pivot kept in place by the
/// turn) onto the panel, with the pivot at its centre.
fn place_homography(pw: f64, ph: f64, fit: f64, pivot: (f64, f64)) -> Homography {
    Homography::from_affine(
        Affine::translate((pw / 2.0, ph / 2.0)) * Affine::scale(fit) * Affine::translate((-pivot.0, -pivot.1)),
    )
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
    let scale = |v: u8| (f64::from(v) * shade).round() as u8;
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

#[cfg(test)]
mod tests {
    use super::slab_thickness;
    use figcraft_doc::Transform3d;

    #[test]
    fn the_pivot_sits_in_the_middle_and_its_dot_is_reported_there() {
        use figcraft_doc::NodeKind;
        for (x, y, w, h) in [(0.0, 0.0, 100.0, 100.0), (300.0, 200.0, 400.0, 260.0)] {
            let mut d = figcraft_doc::Document::new("turn");
            let p = d.first_page().unwrap();
            let r = d.create(p, NodeKind::Rectangle, "R", x, y, w, h).unwrap();
            d.get_mut(r).unwrap().fills = vec![figcraft_color::Paint::solid(figcraft_color::Color::from_hex("FF0000").unwrap())];
            d.get_mut(r).unwrap().effects = vec![figcraft_doc::Effect {
                kind: figcraft_doc::EffectKind::Transform3d(Transform3d { rotate_x: 20.0, rotate_y: 30.0, ..Default::default() }),
                visible: true,
            }];
            let (pix, dot) = super::cube(&d, r, (460, 360)).unwrap();
            let (nx, ny) = dot.unwrap_or_else(|| panic!("{w}x{h} at {x},{y}: the dot is drawn"));
            assert!((nx - 0.5).abs() < 0.02 && (ny - 0.5).abs() < 0.02, "{w}x{h} at {x},{y}: dot at {nx},{ny}");
            // The slab covers the middle (not a corner sliver): the pivot stays at the centre
            // (the dot itself sits right there, so sample beside it).
            let mid = pix.pixel(200, 150).unwrap();
            assert!(mid.red() > 120 && mid.green() < 100, "{w}x{h} at {x},{y}: slab middle {mid:?}");
        }
    }

    #[test]
    fn the_preview_is_a_flat_plane_until_it_is_turned_or_given_depth() {
        let flat = Transform3d::default();
        assert_eq!(slab_thickness(&flat, 100.0, 80.0), 0.0, "the preview starts as the layer's own plane");
        let turned = Transform3d { rotate_x: 20.0, rotate_y: 30.0, ..Default::default() };
        assert!(slab_thickness(&turned, 100.0, 80.0) > 0.0, "turning it brings the 3D body");
        let deep = Transform3d { depth: 30.0, ..Default::default() };
        assert!(slab_thickness(&deep, 100.0, 80.0).abs() >= 30.0, "a depth extrudes the body");
        let z_only = Transform3d { rotate_z: 45.0, ..Default::default() };
        assert_eq!(slab_thickness(&z_only, 100.0, 80.0), 0.0, "a Z turn stays in the plane");
    }

    /// The set pixels in one horizontal third of `pix` (a column band over the row-major buffer).
    fn ink(pix: &tiny_skia::Pixmap, third: usize) -> u64 {
        let w = pix.width() as usize;
        let (from, to) = (w * third / 3, w * (third + 1) / 3);
        pix.pixels()
            .iter()
            .enumerate()
            .filter(|(i, c)| c.alpha() > 0 && *i % w >= from && *i % w < to)
            .count() as u64
    }

    #[test]
    fn the_cards_preview_shows_three_cards_with_the_middle_the_largest() {
        let pix = super::cards((460, 360), 800.0).unwrap();
        let (left, mid, right) = (ink(&pix, 0), ink(&pix, 1), ink(&pix, 2));
        assert!(left > 50 && mid > 50 && right > 50, "a card in every third: {left} {mid} {right}");
        assert!(mid > left && mid > right, "the middle card is the largest: {left} {mid} {right}");
        assert!(left > right, "the nearer of the two far cards is the larger: {left} {mid} {right}");
    }

    #[test]
    fn a_nearer_camera_spreads_the_cards_more() {
        // The far card's share of the middle card's ink: it falls as the camera closes in.
        let ratio = |distance: f64| {
            let pix = super::cards((460, 360), distance).unwrap();
            ink(&pix, 2) as f64 / ink(&pix, 1) as f64
        };
        let (near, mid, far) = (ratio(300.0), ratio(900.0), ratio(5000.0));
        assert!(near < mid && mid < far, "the far card shrinks as the camera closes: {near} {mid} {far}");
    }

    #[test]
    fn the_dashed_bounds_are_drawn_round_the_cards() {
        let pix = super::cards((460, 360), 800.0).unwrap();
        let (dr, dg, db) = super::DASH_INK;
        let dashes = pix.pixels().iter().filter(|c| c.alpha() == 255 && c.red() == dr && c.green() == dg && c.blue() == db).count();
        assert!(dashes > 40, "the dashed bounds' bars are drawn: {dashes}");
    }

    #[test]
    fn the_card_outlines_are_drawn() {
        let pix = super::cards((460, 360), 800.0).unwrap();
        let (cr, cg, cb) = super::CARD_INK;
        let ink = pix.pixels().iter().filter(|c| c.alpha() > 200 && c.red() == cr && c.green() == cg && c.blue() == cb).count();
        assert!(ink > 200, "the three cards' strokes are drawn: {ink}");
    }

    #[test]
    fn an_unsafe_distance_is_sanitized() {
        for distance in [0.0, -50.0, f64::NAN, f64::INFINITY, 1e12] {
            assert!(super::cards((460, 360), distance).is_some(), "{distance} still renders");
        }
    }
}