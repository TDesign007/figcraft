//! The 3D transform's render: the layer drawn flat on its plane (its content, inner shadows,
//! layer effects), then projected through its homography — the turn about the layer's centre
//! through the placement, with the nearest frame's camera distance. Drop shadows turn with the
//! plane: cast in plane space, warped under the layer. Children flatten onto the plane (Figma's
//! default "flat plane"; "keep nested layers 3D" is separate work).
//!
//! Limitations against the GPU original: a backdrop effect (background blur, glass) on the
//! turned layer itself is skipped — its backdrop read assumes an unwarped screen alignment — and
//! a turned layer culls against a bound around its box (a close perspective reaching far
//! outside it clips at that bound).

use figcraft_doc::{EffectKind, Node, Transform3d};
use figcraft_geom::{Affine, Homography, Rect};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::draw::{Ctx, OverFills};
use crate::paint::composite;
use crate::{layer_budget, shadow};

/// The layer's first visible 3D transform, sanitised (`None`: none).
pub(crate) fn visible(n: &Node) -> Option<Transform3d> {
    n.effects
        .iter()
        .filter(|e| e.visible)
        .find_map(|e| match &e.kind {
            EffectKind::Transform3d(t) => Some(t.sanitized()),
            _ => None,
        })
}

/// Whether `n`'s 3D transform moves what it paints past its box (a culling margin's worth).
pub(crate) fn margin(n: &Node) -> f64 {
    match visible(n) {
        Some(t) if !t.is_identity() => (n.width.hypot(n.height)) / 2.0,
        _ => 0.0,
    }
}

/// Draw `n` turned in 3D: flat on its plane, then projected onto `pix` through its homography.
pub(crate) fn draw_warped(ctx: &Ctx, pix: &mut Pixmap, n: &Node, xf: Affine, depth: usize, t3: &Transform3d) {
    // The plane raster covers the layer's local paint bounds plus what its layer effects reach,
    // at the surface's scale.
    let page = crate::draw::to_page(ctx, xf);
    let local = crate::draw::bounds_within(ctx.doc, n, page, 0, ctx.lod.then_some(ctx.scale), Some(ctx.bounds)).0;
    let rs = ctx.scale.max(1e-6);
    let pad = n.layer_effects_reach().max(0.0);
    let b = local.inflate(pad, pad);
    let lw = ((b.width() * rs).ceil().min(16_384.0) as u32).max(1);
    let lh = ((b.height() * rs).ceil().min(16_384.0) as u32).max(1);
    let Some(_held) = layer_budget::Held::take(lw, lh, 1) else { return };
    let Some(mut plane) = Pixmap::new(lw, lh) else { return };
    let lxf = Affine::scale(rs) * Affine::translate((-b.x0, -b.y0));
    // Inner shadows paint over the fills, under the children and strokes, in plane space.
    let shape = crate::draw::node_outline(ctx.doc, n);
    let has_inner = shape.is_some() && n.effects.iter().any(|e| e.visible && matches!(e.kind, EffectKind::InnerShadow(_)));
    let inner = |l: &mut Pixmap| {
        let fills = shadow::inner_from_pixels(ctx.doc, n).then(|| l.clone());
        for e in n.effects.iter().filter(|e| e.visible) {
            if let EffectKind::InnerShadow(s) = &e.kind
                && let Some(inner) = match &fills {
                    Some(f) => shadow::inner_shadow_of(f, ctx.scale, s),
                    None => shape.as_ref().and_then(|sh| shadow::inner_shadow(sh, lw, lh, lxf, ctx.scale, s)),
                }
            {
                l.draw_pixmap(0, 0, inner.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
        }
    };
    let over_fills: OverFills = has_inner.then_some(&inner as &dyn Fn(&mut Pixmap));
    // The node's own backdrop effects (background blur, glass) read the screen under it, which a
    // turned plane can't sample aligned: they are left out (see the module notes).
    crate::draw::draw_content(ctx, &mut plane, n, lxf, depth, over_fills);
    crate::draw::layer_effects(ctx, &mut plane, n, n.id, lxf);
    // The homography from the plane's own coordinates to this surface.
    let pivot = (n.width / 2.0, n.height / 2.0);
    let d = figcraft_doc::transform3d::perspective_of(ctx.doc, n.id);
    let h = Homography::from_affine(xf).mul(figcraft_doc::transform3d::plane_homography(t3, pivot, d));
    // A plane facing away draws only when the back shows (as the mirrored front: the inverse
    // mapping handles the mirror).
    let quad = h.quad(Rect::new(0.0, 0.0, n.width, n.height));
    if !t3.backface && signed_area(&quad) < 0.0 {
        return;
    }
    let op = n.opacity.clamp(0.0, 1.0) as f32;
    // Drop shadows turn with the plane: cast in plane space, warped under the layer.
    let caster = shadow::Caster::new(ctx.doc, n, &plane, lxf, ctx.scale);
    for e in n.effects.iter().filter(|e| e.visible) {
        if let EffectKind::DropShadow(s) = &e.kind
            && let Some(sh) = shadow::drop_shadow(&caster, s)
            && let Some((warped, at)) = warp(&sh, b, rs, &h, pix.width(), pix.height())
        {
            composite(pix, at, &warped, op, s.blend);
        }
    }
    if let Some((warped, at)) = warp(&plane, b, rs, &h, pix.width(), pix.height()) {
        composite(pix, at, &warped, op, n.blend);
    }
}

/// The plane `src` (covering local rect `r` at `rs` px per unit) projected through `h` (local →
/// surface), bilinear and perspective-correct per pixel, into a surface-aligned pixmap over the
/// projection's bounds. Returns the pixmap and where it sits on the surface. Each surface pixel
/// takes its plane pixel through the inverse, so a turn in either direction, a mirror (the
/// plane's back) and a perspective all come out right.
pub(crate) fn warp(src: &Pixmap, r: Rect, rs: f64, h: &Homography, dw: u32, dh: u32) -> Option<(Pixmap, (i32, i32))> {
    let inv = h.inverse()?;
    let q = h.quad(r);
    let (qx0, qx1) = quad_extent(&q, |p| p.x);
    let (qy0, qy1) = quad_extent(&q, |p| p.y);
    let x0 = qx0.floor().max(0.0);
    let y0 = qy0.floor().max(0.0);
    let x1 = qx1.ceil().min(f64::from(dw));
    let y1 = qy1.ceil().min(f64::from(dh));
    let w = (x1 - x0).ceil();
    let hh = (y1 - y0).ceil();
    if w < 1.0 || hh < 1.0 || !w.is_finite() || !hh.is_finite() {
        return None;
    }
    let lw = w.min(16_384.0) as u32;
    let lh = hh.min(16_384.0) as u32;
    let Some(mut out) = Pixmap::new(lw, lh) else { return None };
    let (sw, sh) = (i64::from(src.width()), i64::from(src.height()));
    let sp = src.pixels();
    let out_pixels = out.pixels_mut();
    for j in 0..lh as i64 {
        for i in 0..lw as i64 {
            // The destination pixel's centre, through the inverse, into the plane's coordinates.
            let (lx, ly) = inv.apply(x0 + i as f64 + 0.5, y0 + j as f64 + 0.5);
            if !lx.is_finite() || !ly.is_finite() {
                continue;
            }
            let u = (lx - r.x0) * rs;
            let v = (ly - r.y0) * rs;
            let c = bilinear(sp, sw, sh, u, v);
            if let Some(c) = c {
                out_pixels[(j * lw as i64 + i) as usize] = c;
            }
        }
    }
    Some((out, (x0 as i32, y0 as i32)))
}

/// A bilinear sample of premultiplied `pixels` at `(u, v)` px (outside: nothing draws).
fn bilinear(
    pixels: &[tiny_skia::PremultipliedColorU8],
    w: i64,
    h: i64,
    u: f64,
    v: f64,
) -> Option<tiny_skia::PremultipliedColorU8> {
    if u < -0.5 || v < -0.5 || u >= w as f64 + 0.5 || v >= h as f64 + 0.5 {
        return None;
    }
    let ux = u.floor();
    let vy = v.floor();
    let fx = (u - ux) as f32;
    let fy = (v - vy) as f32;
    let mut acc = [0.0f32; 4];
    let mut weight = 0.0f32;
    for (gx, gy, gw) in [(0.0, 0.0, (1.0 - fx) * (1.0 - fy)), (1.0, 0.0, fx * (1.0 - fy)), (0.0, 1.0, (1.0 - fx) * fy), (1.0, 1.0, fx * fy)] {
        let (sx, sy) = (ux + gx, vy + gy);
        if sx < 0.0 || sy < 0.0 || sx >= w as f64 || sy >= h as f64 {
            continue;
        }
        let c = pixels[(sy * w as f64 + sx) as usize];
        let rgba = [c.red(), c.green(), c.blue(), c.alpha()];
        for (a, value) in acc.iter_mut().zip(rgba) {
            *a += value as f32 * gw;
        }
        weight += gw;
    }
    if weight <= 0.0 {
        return None;
    }
    let round = |a: f32| a.round() as u8;
    tiny_skia::PremultipliedColorU8::from_rgba(round(acc[0]), round(acc[1]), round(acc[2]), round(acc[3]))
}

/// The smallest and largest coordinate along `at`.
fn quad_extent(q: &[figcraft_geom::Point; 4], at: impl Fn(&figcraft_geom::Point) -> f64) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in q {
        let v = at(p);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

/// The quad's signed area (positive: the plane faces this surface; negative: its back does).
fn signed_area(q: &[figcraft_geom::Point; 4]) -> f64 {
    let mut area = 0.0;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        area += a.x * b.y - b.x * a.y;
    }
    area / 2.0
}
