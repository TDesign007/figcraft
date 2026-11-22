//! The Perspective popover (Figma's): the header with the frame's name, the preview — the frame's
//! content drawn three times receding in depth under the effect's own camera — and the distance,
//! scrubbed on a slider whose range adapts to the frame's size (Figma's: from a safe nearest
//! camera to one where the perspective barely reads), with the vanishing-point toggle at its end.
//! Everything applies through `effect.set` like the panel's own fields, so the gestures are
//! undoable steps.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, NodeId, Perspective};
use serde_json::json;

use super::menu::row;
use super::transform3d::{label_at, name_pill};
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::regular;
use crate::widgets::{FieldEdit, Lead};

/// The popover's width (points), the 3D transform's.
const WIDTH: f32 = 250.0;
/// The preview panel's height.
const PREVIEW_H: f32 = 180.0;
/// The label column's width: "Perspective" clears it, the slider and the field fill the rest.
const LABEL_W: f32 = 78.0;
/// The distance the controls fall back to (the doc's default camera).
const DEFAULT_DISTANCE: f64 = 800.0;

/// The popover for the Perspective effect of row `i` on frame `id`, opened at `at` (the row's
/// screen position). Clears the state itself when what it edits is gone.
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: egui::Rect, t: &Tokens) {
    // The popover closes with what it edits: the layer, the effect row, its visibility.
    let Some(name) = app.session.doc().ok().and_then(|doc| doc.node(id).map(|n| n.name.clone())) else {
        app.ui.persp_popover = None;
        app.ui.persp_popover_shown = false;
        return;
    };
    let Some((node_w, node_h)) = app.session.doc().ok().and_then(|doc| doc.node(id)).map(|n| (n.width, n.height)) else {
        app.ui.persp_popover = None;
        app.ui.persp_popover_shown = false;
        return;
    };
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i).cloned()) else {
        app.ui.persp_popover = None;
        app.ui.persp_popover_shown = false;
        return;
    };
    let e = if effect.visible && matches!(effect.kind, EffectKind::Perspective(_)) {
        effect.clone()
    } else {
        app.ui.persp_popover = None;
        app.ui.persp_popover_shown = false;
        return;
    };
    let distance = match &e.kind {
        EffectKind::Perspective(p) => p.sanitized().distance,
        _ => return,
    };
    let ids = [id];
    // The apply helper: one `effect.set`, or a gesture's batched frames (a scrub).
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction("Perspective");
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    fn with(e: &Effect, f: impl FnOnce(&mut Perspective)) -> Effect {
        let mut e2 = e.clone();
        if let EffectKind::Perspective(p) = &mut e2.kind {
            f(p);
        }
        e2
    }
    // The popover sits on the canvas side of the row: to its left, its top at the row's own,
    // never over the row itself (Figma's placement) — lifted to stay inside the window when the
    // row is near its bottom. The row's press is held out of the outside-close.
    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - 380.0).max(8.0)));
    let mut pop = app
        .ui
        .persp_popover_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("persp_popover"), tr("Perspective")).width(WIDTH).pad(10.0).no_header());
    // The anchor follows the row and the panel every frame (the chrome persists between frames).
    // Its opening frame spares the press that opened it (a click that adds the effect from the
    // add menu lands outside the panel and would otherwise close it on that same frame).
    pop = pop.at(at).close_on_outside(app.ui.persp_popover_shown);
    let hold: Vec<egui::Rect> = vec![row_rect];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH - 20.0);
        // One header row, like the 3D transform's: the effect's mark and title, the
        // frame's name as a badge, then the close.
        row(ui, 34.0, |ui| {
            let (icon_r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
            icons::paint(ui.painter(), icon_r, Icon::Perspective, t.icon_secondary);
            ui.add_space(6.0);
            // The same text style as the effect names in the list (the dropdown's
            // regular 11), like the 3D transform's title.
            let title = ui.painter().layout_no_wrap(tr("Perspective").to_string(), regular(11.0), t.text);
            let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
            ui.painter().galley(title_rect.min, title, t.text);
            ui.add_space(6.0);
            // The badge only where it clears the pinned close (the right ~40 points);
            // it elides to what is left.
            let room = (ui.max_rect().max.x - 48.0 - ui.cursor().min.x).max(0.0);
            if room >= 40.0 {
                name_pill(ui, t, &name, room.min(84.0));
            }
        });
        let hr = ui.min_rect();
        let close_rect = Rect::from_min_size(pos2(hr.max.x - 8.0 - 26.0, hr.center().y - 13.0), vec2(26.0, 26.0));
        let close = ui.interact(close_rect, Id::unique("persp_close"), Sense::click());
        crate::a11y::describe(&close, tr("Close"));
        if close.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if close.clicked() {
            app.ui.persp_popover = None;
        }
        ui.add_space(4.0);
        // The preview: the frame's content as three cards receding in depth under
        // this camera, rebuilt whenever the document (the distance included) moves.
        let (_, prect) = ui.allocate_space(vec2(WIDTH - 20.0, PREVIEW_H));
        app.ui.persp_preview_rect = Some(prect);
        ui.painter().rect_filled(prect, CornerRadius::same(10), t.panel);
        let painter = ui.painter().with_clip_rect(prect.intersect(ui.clip_rect()));
        preview(app, &painter, prect, id, distance, pw(), ph());
        ui.add_space(8.0);
        // The distance: the label, then the slider well — the track with its
        // frustum fan, and the vanishing-point toggle at the end.
        row(ui, 32.0, |ui| {
            label_at(ui, t, tr("Perspective"), 0.0);
            ui.add_space(LABEL_W);
            let (v, scrub) = distance_slider(app, ui, t, distance, range(node_w, node_h), &e, &set);
            if let Some(v) = v {
                set(app, with(&e, |p: &mut Perspective| p.distance = v.clamp(1.0, 100_000.0)), scrub);
            }
        });
        ui.add_space(4.0);
        // The precise distance, beside the effect's own mark (Figma's value row).
        row(ui, 32.0, |ui| {
            ui.add_space(LABEL_W);
            let field_w = ui.available_width();
            let ed = crate::variable_ui::number_field(
                app,
                ui,
                &ids,
                &format!("persp/{i}/distance"),
                Id::unique(("perspfx", "distance", i)),
                Lead::Icon(Icon::Perspective, ""),
                Some(distance),
                "",
                field_w,
            );
            if let Some(ed) = ed {
                match ed {
                    FieldEdit::Set(v) => set(app, with(&e, |p: &mut Perspective| p.distance = v.clamp(1.0, 100_000.0)), None),
                    FieldEdit::Scrub(v, done) => set(app, with(&e, |p: &mut Perspective| p.distance = v.clamp(1.0, 100_000.0)), Some(done)),
                }
            }
        });
        ui.add_space(8.0);
    });
    app.ui.persp_popover_pop = Some(pop);
    app.ui.persp_popover_shown = true;
    if closed {
        app.ui.persp_popover = None;
        app.ui.persp_popover_shown = false;
    }
}

/// The slider's adaptive range, the reference's own ends: the nearest camera is half the frame's
/// diagonal (under it the perspective drowns the frame itself), the farthest a hundred diagonals
/// (where the perspective barely reads) — a 278-diagonal selection reads 139..27800, as the
/// reference's track ends do.
pub(crate) fn range(w: f64, h: f64) -> (f64, f64) {
    let diag = w.hypot(h).max(1.0);
    let min = (diag / 2.0).max(1.0);
    let max = (diag * 100.0).min(100_000.0);
    (min, max)
}

/// The distance slider: a field well holding the track (filled to the knob), the camera's
/// frustum fan drawn from the knob to the track's end (longer and wider over a nearer camera),
/// and the vanishing-point toggle at the end. The track maps the distance logarithmically over
/// `range`; a double-click falls back to the default. Returns `(value, scrub)` for the gesture's
/// frame (`None`: nothing moved — the scrub brackets the batched undo step, `None`: a plain one).
fn distance_slider(
    app: &mut FigApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    distance: f64,
    (min, max): (f64, f64),
    e: &Effect,
    set: &dyn Fn(&mut FigApp, Effect, Option<bool>),
) -> (Option<f64>, Option<bool>) {
    let (well, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    app.ui.persp_slider_rect = Some(well);
    ui.painter().rect_filled(well, CornerRadius::same(6), t.field);
    let cy = well.center().y;
    // The toggle's place at the well's right end, then the track between the insets.
    let toggle_rect = Rect::from_center_size(pos2(well.max.x - 16.0, cy), Vec2::splat(16.0));
    let track = Rect::from_min_max(pos2(well.min.x + 10.0, cy - 1.5), pos2(toggle_rect.left() - 8.0, cy + 1.5));
    let painter = ui.painter().with_clip_rect(well);
    // The knob's place: the distance mapped logarithmically, the reference's way round — the
    // track's left end is the far camera (the perspective barely reads, the layer looks normal),
    // the right end the near one (the strongest), so a drag rightward pushes the camera into
    // the scene.
    let m = |d: f64| ((max.ln() - d.max(1.0).ln()) / (max.ln() - min.ln()).max(1e-9)).clamp(0.0, 1.0) as f32;
    let at = |m: f32| track.min.x + m * (track.max.x - track.min.x);
    // The frustum fan: the camera's cone, apex on the knob, opening toward the scene at the
    // track's right end — vanishing at the far end (nothing dramatic to see) and filling the
    // track at the near one, where the perspective reads strongest.
    let kx = at(m(distance));
    let fan_h = (track.max.x - kx) * m(distance);
    painter.add(egui::Shape::convex_polygon(
        vec![pos2(kx, cy), pos2(track.max.x, cy - fan_h), pos2(track.max.x, cy + fan_h)],
        t.icon_secondary.gamma_multiply(0.28),
        Stroke::NONE,
    ));
    // The track: the plain rail, then the filled part up to the knob.
    painter.rect_filled(track, CornerRadius::same(2), t.border);
    painter.rect_filled(Rect::from_min_max(track.min, pos2(kx, track.max.y)), CornerRadius::same(2), t.accent);
    // The vanishing-point toggle: an outlined square, its centre dotted when the marker shows.
    let toggle = ui.interact(toggle_rect, Id::unique("persp_vp"), Sense::click());
    crate::a11y::describe(&toggle, tr("Vanishing point"));
    let ink = if app.ui.persp_vp { t.accent } else { t.icon_secondary };
    ui.painter().rect_stroke(toggle_rect.shrink(2.0), CornerRadius::same(4), Stroke::new(1.0, ink), egui::StrokeKind::Inside);
    if app.ui.persp_vp {
        ui.painter().circle_filled(toggle_rect.center(), 2.5, t.accent);
    }
    if toggle.clicked() {
        app.ui.persp_vp = !app.ui.persp_vp;
    }
    if toggle.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    // The knob: a white rounded square with the accent ring, over the fan.
    let knob = Rect::from_center_size(pos2(kx, cy), Vec2::splat(10.0));
    painter.rect_filled(knob, CornerRadius::same(3), Color32::WHITE);
    painter.rect_stroke(knob, CornerRadius::same(3), Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    // The gesture: a press jumps to it and drags scrub (one batched undo step, the value read
    // logarithmically off the track), the release commits, a double-click falls back to the
    // default camera.
    let resp = ui.interact(track.expand(6.0), Id::unique("persp_track"), Sense::click_and_drag());
    let value = |px: f32| {
        let m = ((px - track.min.x) / (track.max.x - track.min.x).max(1.0)).clamp(0.0, 1.0) as f64;
        (max.ln() - m * (max.ln() - min.ln())).exp()
    };
    let mut moved = None;
    let mut scrub = None;
    if resp.drag_started() {
        app.session.begin_interaction("Perspective").ok();
        if let Some(pt) = resp.interact_pointer_pos() {
            moved = Some(value(pt.x));
        }
    }
    if resp.dragged()
        && let Some(pt) = resp.interact_pointer_pos()
    {
        moved = Some(value(pt.x));
        scrub = Some(false);
    }
    if resp.drag_stopped() {
        app.session.commit_interaction().ok();
        if let Some(pt) = resp.interact_pointer_pos() {
            moved = Some(value(pt.x));
        }
        scrub = Some(true);
    }
    if resp.double_clicked() {
        set(app, tweaked(e, |p: &mut Perspective| p.distance = DEFAULT_DISTANCE), None);
        moved = None;
        scrub = None;
    }
    if moved.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    (moved, scrub)
}

/// `e` with `f` applied to its Perspective kind.
fn tweaked(e: &Effect, f: impl FnOnce(&mut Perspective)) -> Effect {
    let mut e2 = e.clone();
    if let EffectKind::Perspective(p) = &mut e2.kind {
        f(p);
    }
    e2
}

/// The preview raster's pixel size (twice the panel's points, for a crisp texture).
fn pw() -> u32 {
    ((WIDTH - 20.0) * 2.0).round() as u32
}

fn ph() -> u32 {
    (PREVIEW_H * 2.0).round() as u32
}

/// The depth cards into `prect`, through the render crate's composer, uploaded as a texture
/// (rebuilt when the values move or the frame's render does).
fn preview(app: &mut FigApp, painter: &egui::Painter, prect: Rect, id: NodeId, distance: f64, pw: u32, ph: u32) {
    let revision = app.session.ds().map(|d| d.revision).unwrap_or(0);
    let key = (revision, pw, ph);
    let rebuild = match &app.ui.persp_preview {
        Some((k, _)) => *k != key,
        None => true,
    };
    if rebuild {
        let doc = app.session.doc().ok().cloned();
        if let Some(doc) = doc
            && let Some(pix) = figcraft_render::preview3d::cards(&doc, id, (pw, ph), distance)
        {
            let image = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
            let handle = match &app.ui.persp_preview {
                Some((_, handle)) if handle.size() == image.size => {
                    let mut cloned = handle.clone();
                    cloned.set(image, egui::TextureOptions::LINEAR);
                    cloned
                }
                _ => painter.ctx().load_texture("persp-preview", image, egui::TextureOptions::LINEAR),
            };
            app.ui.persp_preview = Some((key, handle));
        }
    }
    if let Some((_, handle)) = &app.ui.persp_preview {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(handle.id(), prect, uv, Color32::WHITE);
    }
}
