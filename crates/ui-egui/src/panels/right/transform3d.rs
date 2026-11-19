//! The 3D transform's popover (Figma's): the header with the layer's name, the preview — the
//! layer as an extruded slab turned by the effect's own values, dragged to turn it in real time,
//! with a reset — and the three turns (X, Y, Z) and the depth as fields. Everything applies
//! through `effect.set` like the panel's own fields, so the gestures are undoable steps.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, NodeId, Transform3d};
use serde_json::json;

use super::menu::row;
use super::{FigApp, Tokens};
use crate::popover::Popover;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::regular;
use crate::widgets::FieldEdit;

/// The popover's width (points), as measured on the target app's.
const WIDTH: f32 = 254.0;
/// The preview panel's height.
const PREVIEW_H: f32 = 145.0;
/// Drag-to-turn: degrees of turn per point of drag.
const TURN_PER_PT: f64 = 0.55;

/// The popover for the 3D transform of row `i` on layer `id`, opened at `at` (the row's screen
/// position). Clears the state itself when what it edits is gone (the layer, the effect).
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: egui::Rect, t: &Tokens) {
    let Some(name) = app.session.doc().ok().and_then(|doc| doc.node(id).map(|n| n.name.clone())) else {
        app.ui.t3_popover = None;
        return;
    };
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i).cloned()) else {
        app.ui.t3_popover = None;
        return;
    };
    let e = if effect.visible && matches!(effect.kind, EffectKind::Transform3d(_)) {
        effect.clone()
    } else {
        app.ui.t3_popover = None;
        return;
    };
    let t3 = match &e.kind {
        EffectKind::Transform3d(t) => t.clone(),
        _ => return,
    };
    let ids = [id];
    // The apply helper: one `effect.set`, or a gesture's batched frames (a drag, a scrub).
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction("3D transform");
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    fn with(e: &Effect, f: impl FnOnce(&mut Transform3d)) -> Effect {
        let mut e2 = e.clone();
        if let EffectKind::Transform3d(t2) = &mut e2.kind {
            f(t2);
        }
        e2
    }
    // The popover sits on the canvas side of the row: to its left, its top at the row's own,
    // never over the row itself (Figma's placement) — lifted to stay inside the window when the
    // row is near its bottom (the popover is about 380 points tall). The row's press is held out
    // of the outside-close.
    let screen_h = ctx.content_rect().height();
    let at = Pos2::new(
        (row_rect.left() - WIDTH - 8.0).max(8.0),
        row_rect.top().max(8.0).min((screen_h - 380.0).max(8.0)),
    );
    let held = row_rect;
    let mut pop = app.ui.t3_popover_pop.take().unwrap_or_else(|| Popover::with_id(Id::unique("t3_popover"), tr("3D transform")).at(at).pad(10.0));
    pop = pop.close_on_outside(true);
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: std::slice::from_ref(&held), escape: false }, |ui| {
                    ui.set_width(WIDTH - 16.0);
                    // The header: the effect's mark, the layer's name, the pivot picker (⊕), the
                    // close (the Popover's own header carries the title and the X).
                    row(ui, 30.0, |ui| {
                        cube_icon(ui, t);
                        ui.add_space(6.0);
                        name_pill(ui, t, &name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::button(ui, Icon::Close, 24.0, false, tr("Close")).clicked() {
                                app.ui.t3_popover = None;
                            }
                            // The pivot picker: nine preset points, the fields for a precise one.
                            if icons::button(ui, Icon::Effects, 24.0, app.ui.t3_origin_open, tr("Rotation point")).clicked() {
                                app.ui.t3_origin_open = !app.ui.t3_origin_open;
                            }
                        });
                    });
                    // The pivot picker's panel: the nine points on the layer, the precise X/Y.
                    if app.ui.t3_origin_open {
                        origin_panel(app, ui, &ids, i, t, &t3);
                        ui.add_space(4.0);
                    }
                    ui.add_space(2.0);
                    // The preview: the turned slab, dragged to turn it in real time, with a reset.
                    let (_, prect) = ui.allocate_space(vec2(WIDTH - 20.0, PREVIEW_H));
                    let resp = ui.interact(prect, Id::unique("t3_preview"), Sense::click_and_drag());
                    ui.painter().rect_filled(prect, CornerRadius::same(8), t.panel);
                    let painter = ui.painter().with_clip_rect(prect.intersect(ui.clip_rect()));
                    preview(app, &painter, prect, id, pw(), ph());
                    let reset_rect = Rect::from_min_size(prect.right_bottom() - vec2(30.0, 30.0), vec2(24.0, 24.0));
                    // The reset: an icon button drawn at the preview's bottom-right corner.
                    let rresp = ui.interact(reset_rect, Id::unique("t3_reset"), Sense::click());
                    let bg = if rresp.is_pointer_button_down_on() {
                        t.pressed
                    } else if rresp.hovered() {
                        t.hover
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(reset_rect, CornerRadius::same(5), bg);
                    icons::paint(ui.painter(), Rect::from_center_size(reset_rect.center(), Vec2::splat(16.0)), Icon::Reset, t.icon);
                    if rresp.clicked() {
                        set(app, with(&e, |t2: &mut Transform3d| *t2 = Transform3d { backface: t2.backface, ..Transform3d::default() }), None);
                    }
                    if resp.drag_started() {
                        app.session.begin_interaction("3D transform").ok();
                    }
                    if resp.dragged() {
                        let d = resp.drag_delta();
                        let turned = |t2: &mut Transform3d| {
                            t2.rotate_y = (t2.rotate_y + f64::from(d.x) * TURN_PER_PT).clamp(-360.0, 360.0);
                            t2.rotate_x = (t2.rotate_x + f64::from(d.y) * TURN_PER_PT).clamp(-360.0, 360.0);
                        };
                        set(app, with(&e, turned), Some(false));
                    }
                    if resp.drag_stopped() {
                        app.session.commit_interaction().ok();
                    }
                    ui.add_space(6.0);
                    // The turns: one label for the three rows, the fields in their column.
                    let fields_x = 88.0;
                    let field_w = WIDTH - 26.0 - fields_x;
                    let turns = [("X", t3.rotate_x, "rotateX"), ("Y", t3.rotate_y, "rotateY"), ("Z", t3.rotate_z, "rotateZ")];
                    for (lead, value, prop) in turns {
                        row(ui, 30.0, |ui| {
                            label_at(ui, &t, lead, 10.0);
                            ui.add_space(fields_x);
                            let ed = t3_field(app, ui, &ids, i, prop, crate::widgets::Lead::Text(lead), Some(value), "°", field_w);
                            if let Some(ed) = ed {
                                match ed {
                                    FieldEdit::Set(v) => set(app, with(&e, |t2: &mut Transform3d| set_turn(t2, prop, v)), None),
                                    FieldEdit::Scrub(v, done) => set(app, with(&e, |t2: &mut Transform3d| set_turn(t2, prop, v)), Some(done)),
                                    }
                            }
                        });
                    }
                    row(ui, 30.0, |ui| {
                        label_at(ui, &t, &tr("Depth"), 10.0);
                        ui.add_space(fields_x);
                        let ed = t3_field(app, ui, &ids, i, "depth", crate::widgets::Lead::Icon(Icon::DistributeV, ""), Some(t3.depth), "", field_w);
                        if let Some(ed) = ed {
                            match ed {
                                FieldEdit::Set(v) => set(app, with(&e, |t2: &mut Transform3d| t2.depth = v.clamp(-10_000.0, 10_000.0)), None),
                                FieldEdit::Scrub(v, done) => set(app, with(&e, |t2: &mut Transform3d| t2.depth = v.clamp(-10_000.0, 10_000.0)), Some(done)),
                            }
                        }
                    });
                    ui.add_space(8.0);
                });
    app.ui.t3_popover_pop = Some(pop);
    if closed {
        app.ui.t3_popover = None;
    }
}

/// A number field of the popover's effect `i`, with ids of the popover's own (the panel's effect
/// fields live on another layer and must not share one).
#[allow(clippy::too_many_arguments)]
fn t3_field(
    app: &mut FigApp,
    ui: &mut egui::Ui,
    ids: &[NodeId],
    i: usize,
    prop: &str,
    lead: crate::widgets::Lead,
    value: Option<f64>,
    suffix: &str,
    width: f32,
) -> Option<crate::widgets::FieldEdit> {
    crate::variable_ui::number_field(app, ui, ids, &format!("t3/{i}/{prop}"), Id::unique(("t3fx", prop, i)), lead, value, suffix, width)
}

/// The turn one field edits, by its property name.
fn set_turn(t: &mut Transform3d, prop: &str, v: f64) {
    let v = v.clamp(-360.0, 360.0);
    match prop {
        "rotateX" => t.rotate_x = v,
        "rotateY" => t.rotate_y = v,
        _ => t.rotate_z = v,
    }
}

/// The preview raster's pixel size (twice the panel's points, for a crisp texture).
fn pw() -> u32 {
    ((WIDTH - 20.0) * 2.0).round() as u32
}

fn ph() -> u32 {
    (PREVIEW_H * 2.0).round() as u32
}

/// A label at the row's left inset.
fn label_at(ui: &mut egui::Ui, t: &Tokens, label: &str, inset: f32) {
    let galley = ui.painter().layout_no_wrap(label.to_string(), regular(12.0), t.menu_text);
    let cy = ui.cursor().center().y - galley.size().y / 2.0;
    ui.painter().galley(pos2(ui.cursor().min.x + inset, cy), galley, t.menu_text);
}

/// The pivot picker's panel: a 3 × 3 grid of points on the layer (the corners, the edge
/// midpoints, the centre), the centre picked by default, and the precise X/Y in the layer's
/// percent. A click on a point (or a field edit) moves the turn's pivot through `effect.set`.
fn origin_panel(app: &mut FigApp, ui: &mut egui::Ui, ids: &[NodeId], i: usize, t: &Tokens, t3: &Transform3d) {
    let panel_w = WIDTH - 20.0;
    let grid_h = 96.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(panel_w, grid_h), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(8), t.panel);
    // The nine points, the picked one a blue rounded square (the centre by default).
    let origin = t3.sanitized().origin;
    let picked = |col: usize, row: usize| ((col * 50) as f64 - origin.0).abs() < 1.0 && ((row * 50) as f64 - origin.1).abs() < 1.0;
    let mut picked_cell: Option<(usize, usize)> = None;
    for row in 0..3 {
        for col in 0..3 {
            let cx = rect.min.x + 24.0 + (rect.width() - 48.0) * (col as f32 / 2.0);
            let cy = rect.min.y + 18.0 + (rect.height() - 36.0) * (row as f32 / 2.0);
            if picked(col, row) {
                picked_cell = Some((col, row));
                let sq = Rect::from_center_size(pos2(cx, cy), vec2(14.0, 14.0));
                ui.painter().rect_filled(sq, CornerRadius::same(4), Color32::from_rgb(0x0D, 0x99, 0xFF));
            } else {
                ui.painter().circle_filled(pos2(cx, cy), 2.5, t.icon_secondary);
            }
        }
    }
    // A click picks the cell under the pointer.
    if resp.clicked()
        && let Some(at) = resp.hover_pos()
    {
        let col = (((at.x - rect.min.x - 24.0) / (rect.width() - 48.0).max(1.0)) * 2.0).round().clamp(0.0, 2.0) as usize;
        let row_i = (((at.y - rect.min.y - 18.0) / (rect.height() - 36.0).max(1.0)) * 2.0).round().clamp(0.0, 2.0) as usize;
        if Some((col, row_i)) != picked_cell {
            let e2 = tweaked_effect(e_of_ids(app, ids, i), |t2: &mut Transform3d| t2.origin = ((col * 50) as f64, (row_i * 50) as f64));
            t3_set(app, ids, i, e2, None);
        }
    }
    // The precise origin, in the layer's percent.
    let half = (panel_w - 12.0) / 2.0;
    let e_now = e_of_ids(app, ids, i);
    row(ui, 30.0, |ui| {
        let ed = t3_field(app, ui, ids, i, "originX", crate::widgets::Lead::Text("X"), Some(origin.0), "%", half);
        if let Some(ed) = ed {
            match ed {
                crate::widgets::FieldEdit::Set(v) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.0 = v.clamp(0.0, 100.0)), None),
                crate::widgets::FieldEdit::Scrub(v, done) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.0 = v.clamp(0.0, 100.0)), Some(done)),
            }
        }
        let ed = t3_field(app, ui, ids, i, "originY", crate::widgets::Lead::Text("Y"), Some(origin.1), "%", half);
        if let Some(ed) = ed {
            match ed {
                crate::widgets::FieldEdit::Set(v) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.1 = v.clamp(0.0, 100.0)), None),
                crate::widgets::FieldEdit::Scrub(v, done) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.1 = v.clamp(0.0, 100.0)), Some(done)),
            }
        }
    });
}

/// The effect of row `i` on layer `id` (the popover's own edits read it fresh each frame).
fn e_of_ids(app: &FigApp, ids: &[NodeId], i: usize) -> Effect {
    app.session.doc().ok().and_then(|doc| doc.node(ids[0])).and_then(|n| n.effects.get(i).cloned()).unwrap_or_else(|| Effect::transform3d())
}

/// `effect.set` for the popover's effect (one command, or a gesture's batched frames).
fn t3_set(app: &mut FigApp, ids: &[NodeId], i: usize, e2: Effect, scrub: Option<bool>) {
    let p = json!({"ids": ids, "index": i, "effect": e2});
    match scrub {
        None => {
            app.run("effect.set", p);
        }
        Some(done) => {
            let _ = app.session.begin_interaction("3D transform");
            let _ = app.session.execute("effect.set", &p);
            if done {
                let _ = app.session.commit_interaction();
            }
        }
    }
}

/// `e` with `f` applied to its kind.
fn tweaked_effect(e: Effect, f: impl FnOnce(&mut Transform3d)) -> Effect {
    let mut e2 = e;
    if let EffectKind::Transform3d(t2) = &mut e2.kind {
        f(t2);
    }
    e2
}

/// A small isometric-ish cube, the header's mark.
fn cube_icon(ui: &mut egui::Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let c = r.center();
    let s = 5.0;
    let top = [pos2(c.x, c.y - s), pos2(c.x + s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x - s, c.y - s / 2.0)];
    let left = [pos2(c.x - s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x, c.y + s), pos2(c.x - s, c.y + s / 2.0)];
    let right = [pos2(c.x + s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x, c.y + s), pos2(c.x + s, c.y + s / 2.0)];
    let shade = |k: f32| {
        let v = (f32::from(t.menu_text.r()) * k).round().clamp(70.0, 235.0) as u8;
        Color32::from_gray(v)
    };
    for (face, k) in [(top, 1.0), (left, 0.6), (right, 0.8)] {
        ui.painter().add(egui::Shape::convex_polygon(face.to_vec(), shade(k), Stroke::NONE));
    }
}

/// The layer's name in a rounded pill (truncated to what fits).
fn name_pill(ui: &mut egui::Ui, t: &Tokens, name: &str) {
    let galley = ui.painter().layout_no_wrap(name.to_string(), regular(11.0), t.menu_text);
    let w = galley.size().x.min(72.0);
    let (r, _) = ui.allocate_exact_size(vec2(w + 12.0, 20.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(6), t.panel);
    ui.painter().galley(pos2(r.min.x + 6.0, r.center().y - galley.size().y / 2.0), galley, t.menu_text);
}

/// The turned slab into `prect`, through the render crate's preview composer, uploaded as a
/// texture (rebuilt when the values move or the layer's render does).
fn preview(app: &mut FigApp, painter: &egui::Painter, prect: Rect, id: NodeId, pw: u32, ph: u32) {
    let revision = app.session.ds().map(|d| d.revision).unwrap_or(0);
    let key = (revision, pw, ph);
    let rebuild = match &app.ui.t3_preview {
        Some((k, _)) => *k != key,
        None => true,
    };
    if rebuild {
        let doc = app.session.doc().ok().cloned();
        if let Some(doc) = doc
            && let Some(pix) = figcraft_render::preview3d::cube(&doc, id, (pw, ph))
        {
            let image = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
            let handle = match &app.ui.t3_preview {
                Some((_, handle)) if handle.size() == image.size => {
                    let mut cloned = handle.clone();
                    cloned.set(image, egui::TextureOptions::LINEAR);
                    cloned
                }
                _ => painter.ctx().load_texture("t3-preview", image, egui::TextureOptions::LINEAR),
            };
            app.ui.t3_preview = Some((key, handle));
        }
    }
    if let Some((_, handle)) = &app.ui.t3_preview {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(handle.id(), prect, uv, Color32::WHITE);
    }
}
