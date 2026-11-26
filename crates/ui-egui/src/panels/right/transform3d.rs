//! The 3D transform's popover (Figma's): the header with the layer's name, the preview — the
//! layer as an extruded slab turned by the effect's own values, dragged to turn it in real time,
//! with a reset — and the three turns (X, Y, Z) and the depth as fields. Everything applies
//! through `effect.set` like the panel's own fields, so the gestures are undoable steps.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, NodeId, Transform3d};
use serde_json::json;

use super::menu::{row, row_rect as menu_row_rect};
use super::semibold;
use super::{FigApp, Tokens};
use crate::popover::Popover;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::regular;
use crate::widgets::FieldEdit;

/// The popover's width (points), as measured on the target app's.
const WIDTH: f32 = 250.0;
/// The preview panel's height.
const PREVIEW_H: f32 = 180.0;
/// Drag-to-turn: degrees of turn per point of drag.
const TURN_PER_PT: f64 = 0.55;

/// The popover for the 3D transform of row `i` on layer `id`, opened at `at` (the row's screen
/// position). Clears the state itself when what it edits is gone (the layer, the effect).
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: egui::Rect, t: &Tokens) {
    let Some(name) = app.session.doc().ok().and_then(|doc| doc.node(id).map(|n| n.name.clone())) else {
        app.ui.t3_popover = None;
        app.ui.t3_popover_shown = false;
        return;
    };
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i).cloned()) else {
        app.ui.t3_popover = None;
        app.ui.t3_popover_shown = false;
        return;
    };
    let e = if effect.visible && matches!(effect.kind, EffectKind::Transform3d(_)) {
        effect.clone()
    } else {
        app.ui.t3_popover = None;
        app.ui.t3_popover_shown = false;
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
    // Fully on the canvas, to the left of the right panel (never over the panel or the row);
    // lifted to stay inside the window when the row is near its bottom.
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new(
        (panel_left - WIDTH - 8.0).max(8.0),
        row_rect.top().max(8.0).min((screen_h - 430.0).max(8.0)),
    );
    let held = row_rect;
    let mut pop = app
        .ui
        .t3_popover_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("t3_popover"), tr("3D transform")).width(WIDTH).pad(10.0).no_header());
    // The anchor follows the row and the panel every frame (the chrome persists between frames).
    // Its opening frame spares the press that opened it (a click that adds the effect from the
    // add menu lands outside the panel and would otherwise close it on that same frame).
    pop = pop.at(at).close_on_outside(app.ui.t3_popover_shown);
    let mut hold: Vec<egui::Rect> = vec![held];
    hold.extend(app.ui.t3_settings_rect);
    hold.extend(app.ui.t3_pivot_rect);
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
                    ui.set_width(WIDTH - 20.0);
                    // One header row, as the target app's: the effect's mark and title, the
                    // layer's name as a badge, then the pivot picker, the settings and the close.
                    let mut trigger_end = 0.0;
                    row(ui, 34.0, |ui| {
                        cube_icon(ui, t);
                        ui.add_space(6.0);
                        // The title centres on the row like the icon and the header's buttons: its own height
// allocated through the row's centred layout (painted at the cursor's top it sat high).
                        // The same text style as the effect names in the list (the dropdown's
                        // regular 11): weight, size and family match them exactly.
                        let title = ui.painter().layout_no_wrap(tr("3D transform").to_string(), regular(11.0), t.text);
                        let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
                        ui.painter().galley(title_rect.min, title, t.text);
                        ui.add_space(4.0);
                        let (chevron_rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                        icons::paint(ui.painter(), chevron_rect, Icon::Chevron, t.icon_secondary);
                        trigger_end = chevron_rect.max.x;
                        ui.add_space(6.0);
                        // The badge only where it clears the pinned icons (the three take the
                        // right 90 points of the row); it elides to what is left.
                        let room = (ui.max_rect().max.x - 96.0 - ui.cursor().min.x).max(0.0);
                        if room >= 40.0 {
                            name_pill(ui, t, &name, room.min(84.0));
                        }
                    });
                    let hr = ui.min_rect();
                    crate::panels::right::effects::header_switch(app, ui, t, &ids, i, &effect, hr, trigger_end, row_rect);
                    // The three icons pinned to the header's right edge (painted and hit-tested
                    // at fixed places, so a long name can never cover them): the pivot picker,
                    // the settings and the close.
                    let icon_at = |n: usize| {
                        let right = hr.max.x - 8.0 - 26.0 * (n as f32 + 1.0) - 6.0 * n as f32;
                        Rect::from_min_size(pos2(right, hr.center().y - 13.0), vec2(26.0, 26.0))
                    };
                    let icon_btn = |ui: &mut egui::Ui, r: Rect, active: bool, id: Id, label: &str, paint: &dyn Fn(&egui::Painter)| {
                        let resp = ui.interact(r, id, Sense::click());
                        crate::a11y::describe(&resp, label);
                        let bg = if active {
                            t.selected_row
                        } else if resp.hovered() {
                            t.hover
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(r, CornerRadius::same(5), bg);
                        paint(ui.painter());
                        resp.clicked()
                    };
                    let close_rect = icon_at(0);
                    if icon_btn(ui, close_rect, false, Id::unique("t3_close"), tr("Close"), &|p| {
                        icons::paint(p, Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
                    }) {
                        app.ui.t3_popover = None;
                    }
                    let set_rect = icon_at(1);
                    app.ui.t3_settings_btn_rect = Some(set_rect);
                    if icon_btn(ui, set_rect, app.ui.t3_settings_open, Id::unique("t3_settings_btn"), tr("Settings"), &|p| {
                        icons::paint(p, Rect::from_center_size(set_rect.center(), Vec2::splat(16.0)), Icon::Adjust, t.icon);
                    }) {
                        app.ui.t3_settings_open = !app.ui.t3_settings_open;
                    }
                    let pivot_rect = icon_at(2);
                    app.ui.t3_pivot_btn_rect = Some(pivot_rect);
                    if icon_btn(ui, pivot_rect, app.ui.t3_origin_open, Id::unique("t3_pivot_btn"), tr("Rotation point"), &|p| {
                        crosshair(p, pivot_rect.center(), t.icon);
                    }) {
                        app.ui.t3_origin_open = !app.ui.t3_origin_open;
                    }
                    ui.add_space(4.0);
                    // The preview: the turned slab, dragged to turn it in real time, with a reset.
                    let (_, prect) = ui.allocate_space(vec2(WIDTH - 20.0, PREVIEW_H));
                    app.ui.t3_preview_rect = Some(prect);
                    // The drag covers the preview except the reset's corner: a click-only widget
                    // inside a drag widget's rect never sees the press.
                    let drag_rect = Rect::from_min_max(prect.min, pos2(prect.max.x - 30.0, prect.max.y - 30.0));
                    let resp = ui.interact(drag_rect, Id::unique("t3_preview"), Sense::click_and_drag());
                    ui.painter().rect_filled(prect, CornerRadius::same(10), t.panel);
                    let painter = ui.painter().with_clip_rect(prect.intersect(ui.clip_rect()));
                    preview(app, &painter, prect, id, pw(), ph());
                    let reset_rect = Rect::from_min_size(prect.right_bottom() - vec2(30.0, 30.0), vec2(24.0, 24.0));
                    app.ui.t3_reset_rect = Some(reset_rect);
                    // The preview is a handle: grab it to turn the plane into the 3D model, and its
                    // centre point — the pivot — is the grab point for the orientation, with a ring
                    // and a grab cursor (the target app's preview).
                    let hover = ui.ctx().input(|i| i.pointer.hover_pos());
                    let over_reset = hover.is_some_and(|p| reset_rect.contains(p));
                    let over = hover.is_some_and(|p| prect.contains(p)) && !over_reset;
                    // The dot's real place: the pivot's projection, not necessarily the panel's
                    // centre once the plane is turned in perspective.
                    let dot = app.ui.t3_dot_norm.map_or(prect.center(), |n| pos2(prect.min.x + n.x * prect.width(), prect.min.y + n.y * prect.height()));
                    if over {
                        ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                    }
                    if resp.dragged() || (over && hover.is_some_and(|p| p.distance(dot) <= 10.0)) {
                        ui.painter().circle_stroke(dot, 9.0, Stroke::new(1.0, Color32::from_rgb(0x0D, 0x99, 0xFF).gamma_multiply(0.55)));
                    }
                    // The reset: an icon button drawn at the preview's bottom-right corner.
                    let rresp = ui.interact(reset_rect, Id::unique("t3_reset"), Sense::click());
                    if rresp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
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
                    ui.add_space(8.0);
                    // The turns: one "Rotation" label for the group, the fields in one column.
                    let fields_x = 80.0;
                    let turns = [("X", t3.rotate_x, "rotateX"), ("Y", t3.rotate_y, "rotateY"), ("Z", t3.rotate_z, "rotateZ")];
                    for (n, (lead, value, prop)) in turns.into_iter().enumerate() {
                        row(ui, 34.0, |ui| {
                            if n == 0 {
                                label_at(ui, t, tr("Rotation"), 0.0);
                            }
                            ui.add_space(fields_x);
                            let field_w = ui.available_width();
                            let ed = t3_field(app, ui, &ids, i, prop, crate::widgets::Lead::Text(lead), Some(value), "°", field_w);
                            if let Some(ed) = ed {
                                match ed {
                                    FieldEdit::Set(v) => set(app, with(&e, |t2: &mut Transform3d| set_turn(t2, prop, v)), None),
                                    FieldEdit::Scrub(v, done) => set(app, with(&e, |t2: &mut Transform3d| set_turn(t2, prop, v)), Some(done)),
                                    }
                            }
                        });
                    }
                    ui.add_space(10.0);
                    row(ui, 34.0, |ui| {
                        label_at(ui, t, tr("Depth"), 0.0);
                        ui.add_space(fields_x);
                        let field_w = ui.available_width();
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
    app.ui.t3_popover_shown = true;
    if closed {
        app.ui.t3_popover = None;
        app.ui.t3_popover_shown = false;
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

/// A label at the row's left inset. It uses the panel's secondary ink (not `menu_text`, which
/// is white: invisible on the light panel); the text is the translated key.
pub(crate) fn label_at(ui: &mut egui::Ui, t: &Tokens, label: &str, inset: f32) {
    let galley = ui.painter().layout_no_wrap(label.to_string(), regular(12.0), t.text_secondary);
    let cy = ui.cursor().center().y - galley.size().y / 2.0;
    ui.painter().galley(pos2(ui.cursor().min.x + inset, cy), galley, t.text_secondary);
}

/// The pivot picker's panel: a 3 × 3 grid of points on the layer (the corners, the edge
/// midpoints, the centre), the centre picked by default, and the precise X/Y in the layer's
/// percent. A click on a point (or a field edit) moves the turn's pivot through `effect.set`.
fn origin_panel(app: &mut FigApp, ui: &mut egui::Ui, ids: &[NodeId], i: usize, t: &Tokens, t3: &Transform3d) {
    let panel_w = ui.available_width();
    let grid_h = 96.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(panel_w, grid_h), Sense::click());
    // The grid's well: the field fill reads on the light and the dark popover alike.
    ui.painter().rect_filled(rect, CornerRadius::same(8), t.field);
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
    let e_now = e_of_ids(app, ids, i);
    row(ui, 30.0, |ui| {
        let half = (ui.available_width() - 8.0) / 2.0;
        let ed = t3_field(app, ui, ids, i, "originX", crate::widgets::Lead::Text("X"), Some(origin.0), "%", half);
        if let Some(ed) = ed {
            match ed {
                crate::widgets::FieldEdit::Set(v) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.0 = v.clamp(0.0, 100.0)), None),
                crate::widgets::FieldEdit::Scrub(v, done) => t3_set(app, ids, i, tweaked_effect(e_now.clone(), |t2: &mut Transform3d| t2.origin.0 = v.clamp(0.0, 100.0)), Some(done)),
            }
        }
        ui.add_space(8.0);
        let half2 = ui.available_width();
        let ed = t3_field(app, ui, ids, i, "originY", crate::widgets::Lead::Text("Y"), Some(origin.1), "%", half2);
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
    app.session.doc().ok().and_then(|doc| doc.node(ids[0])).and_then(|n| n.effects.get(i).cloned()).unwrap_or_else(Effect::transform3d)
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

/// The settings popover, on the canvas side of the main one and top-aligned with it: the
/// backface toggle, Figma's two-segment switch — "—" hides the mirrored back, "✓" shows it.
pub(crate) fn settings_popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: egui::Rect, t: &Tokens) {
    // The effect must still be there (the popover closes with what it edits).
    let Some(e) = app
        .session
        .doc()
        .ok()
        .and_then(|doc| doc.node(id))
        .and_then(|n| n.effects.get(i).cloned())
        .filter(|e| e.visible && matches!(e.kind, EffectKind::Transform3d(_)))
    else {
        app.ui.t3_settings_open = false;
        return;
    };
    let backface = matches!(&e.kind, EffectKind::Transform3d(t) if t.backface);
    let settings_w = 250.0;
    // To the LEFT of the main popover, top-aligned with it. Prefer the main popover's live rect so
    // clamp/lift of the taller panel keeps both tops matching; fall back to the same placement
    // math the main popover uses while its first (invisible) frame still has no size.
    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let main_at = Pos2::new(
        (panel_left - WIDTH - 8.0).max(8.0),
        row_rect.top().max(8.0).min((screen_h - 430.0).max(8.0)),
    );
    let at = app
        .ui
        .t3_popover_pop
        .as_ref()
        .filter(|p| p.rect.is_positive())
        .map(|p| Pos2::new((p.rect.left() - settings_w - 8.0).max(8.0), p.rect.top()))
        .unwrap_or_else(|| Pos2::new((main_at.x - settings_w - 8.0).max(8.0), main_at.y.max(8.0)));
    let mut pop = app
        .ui
        .t3_settings_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("t3_settings"), tr("Settings")).width(settings_w).pad(10.0).no_header());
    // Its opening frame spares the press that opened it (a click that adds the effect from the
    // add menu lands outside the panel and would otherwise close it on that same frame).
    pop = pop.at(at).close_on_outside(app.ui.t3_popover_shown);
    let mut hold: Vec<Rect> = app.ui.t3_settings_btn_rect.into_iter().collect();
    hold.extend(app.ui.t3_popover_pop.as_ref().map(|p| p.rect));
    hold.extend(app.ui.t3_pivot_rect);
    let mut close_rect = Rect::NOTHING;
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(settings_w - 20.0);
        // The header: the title at the top left and the close at the top right (the reference's
        // settings window), then a hairline across the panel.
        let head = menu_row_rect(ui, 40.0, |ui| {
            let title = ui.painter().layout_no_wrap(tr("Settings").to_string(), semibold(12.0), t.text);
            let at = pos2(ui.cursor().min.x + 2.0, ui.cursor().center().y - title.size().y / 2.0);
            ui.painter().galley(at, title, t.text);
        });
        let x_rect = Rect::from_min_size(pos2(head.max.x - 8.0 - 24.0, head.center().y - 12.0), vec2(24.0, 24.0));
        close_rect = x_rect;
        let x_resp = ui.interact(x_rect, Id::unique("t3_settings_close"), Sense::click());
        crate::a11y::describe(&x_resp, tr("Close"));
        if x_resp.hovered() {
            ui.painter().rect_filled(x_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(x_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if x_resp.clicked() {
            app.ui.t3_settings_open = false;
        }
        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(line, CornerRadius::ZERO, t.border);
        ui.add_space(8.0);
        row(ui, 40.0, |ui| {
            // The label — secondary panel text so it reads on the light/dark panel fill.
            let label = ui.painter().layout_no_wrap(tr("Backface").to_string(), regular(12.0), t.text_secondary);
            let cy = ui.cursor().center().y - label.size().y / 2.0;
            ui.painter().galley(pos2(ui.cursor().min.x + 2.0, cy), label, t.text_secondary);
            ui.add_space(56.0);
            // Figma's two-segment switch: "—" (the back hidden) and "✓" (shown), the picked
            // segment filled and outlined.
            let seg_w = 66.0;
            let seg_h = 34.0;
            let (box_rect, _) = ui.allocate_exact_size(vec2(seg_w * 2.0, seg_h), Sense::click());
            // The switch's well, then the picked half as an outlined rounded rect (the reference).
            ui.painter().rect_filled(box_rect, CornerRadius::same(8), t.field);
            ui.painter().rect_stroke(box_rect, CornerRadius::same(8), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            let halves = [
                (Rect::from_min_size(box_rect.min, vec2(seg_w, seg_h)), false, false),
                (Rect::from_min_size(pos2(box_rect.min.x + seg_w, box_rect.min.y), vec2(seg_w, seg_h)), true, true),
            ];
            for (half, shows, _is_right) in halves {
                let active = backface == shows;
                let resp = ui.interact(half, Id::unique(("t3_backface", shows)), Sense::click());
                if active {
                    ui.painter().rect_filled(half.shrink(2.0), CornerRadius::same(6), t.panel);
                    ui.painter().rect_stroke(half.shrink(2.0), CornerRadius::same(6), Stroke::new(1.0, t.field_hover_border), egui::StrokeKind::Inside);
                } else if resp.hovered() {
                    ui.painter().rect_filled(half.shrink(2.0), CornerRadius::same(6), t.hover);
                }
                let col = if active { t.text } else { t.icon_secondary };
                if shows {
                    icons::paint(ui.painter(), Rect::from_center_size(half.center(), Vec2::splat(16.0)), Icon::Check, col);
                } else {
                    let c = half.center();
                    ui.painter().line_segment([pos2(c.x - 7.0, c.y), pos2(c.x + 7.0, c.y)], Stroke::new(1.5, col));
                }
                if resp.clicked() && !active {
                    let mut e2 = e.clone();
                    if let EffectKind::Transform3d(t2) = &mut e2.kind {
                        t2.backface = shows;
                    }
                    t3_set(app, &[id], i, e2, None);
                }
            }
        });
        ui.add_space(4.0);
        app.ui.t3_settings_rect = Some(ui.min_rect().expand(12.0));
    });
    pop.close = close_rect;
    app.ui.t3_settings_pop = Some(pop);
    if closed {
        app.ui.t3_settings_open = false;
    }
}

/// The pivot picker's own popover: a small floating panel **above** the main one, centred on it —
/// the nine preset points on the layer and the precise X/Y in the layer's percent. It never
/// expands the main popover; when there is no room above, it opens under the main one instead.
pub(crate) fn pivot_popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: egui::Rect, t: &Tokens) {
    // The effect must still be there (the picker closes with what it edits).
    let Some(e) = app
        .session
        .doc()
        .ok()
        .and_then(|doc| doc.node(id))
        .and_then(|n| n.effects.get(i).cloned())
        .filter(|e| e.visible && matches!(e.kind, EffectKind::Transform3d(_)))
    else {
        app.ui.t3_origin_open = false;
        return;
    };
    let t3 = match &e.kind {
        EffectKind::Transform3d(t) => t.clone(),
        _ => return,
    };
    let pivot_w = 230.0;
    // Above the main popover, its centre on the main one's: the picker floats over the canvas
    // there, clear of the effect rows and the Settings panel (which sits to the main one's left).
    let screen = ctx.content_rect();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let main_at = Pos2::new(
        (panel_left - WIDTH - 8.0).max(8.0),
        row_rect.top().max(8.0).min((screen.height() - 430.0).max(8.0)),
    );
    let main_live = app.ui.t3_popover_pop.as_ref().filter(|p| p.rect.is_positive()).map(|p| p.rect);
    let (main_left, main_top, main_bottom) =
        main_live.map(|r| (r.left(), r.top(), r.bottom())).unwrap_or((main_at.x, main_at.y, main_at.y + 430.0));
    // The picker's height as laid out last frame (the first frame paints invisibly before it
    // knows its size; the grid and the two fields settle at about 150).
    let pivot_h = app
        .ui
        .t3_pivot_pop
        .as_ref()
        .filter(|p| p.rect.is_positive())
        .map(|p| p.rect.height())
        .unwrap_or(150.0);
    let x = main_left + (WIDTH - pivot_w) / 2.0;
    let above = main_top - pivot_h - 8.0;
    let at = if above >= screen.min.y + 8.0 {
        Pos2::new(x, above)
    } else {
        // No room above (the main popover sits near the window's top): under it instead.
        Pos2::new(x, main_bottom + 8.0)
    };
    let mut pop = app
        .ui
        .t3_pivot_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("t3_pivot"), tr("Rotation point")).width(pivot_w).pad(10.0).no_header());
    // Its opening frame spares the press that opened it (a click that adds the effect from the
    // add menu lands outside the panel and would otherwise close it on that same frame).
    pop = pop.at(at).close_on_outside(app.ui.t3_popover_shown);
    let mut hold: Vec<Rect> = app.ui.t3_pivot_btn_rect.into_iter().collect();
    hold.extend(app.ui.t3_popover_pop.as_ref().map(|p| p.rect));
    hold.extend(app.ui.t3_settings_pop.as_ref().map(|p| p.rect));
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(pivot_w - 20.0);
        origin_panel(app, ui, &[id], i, t, &t3);
        ui.add_space(4.0);
        app.ui.t3_pivot_rect = Some(ui.min_rect().expand(12.0));
    });
    app.ui.t3_pivot_pop = Some(pop);
    if closed {
        app.ui.t3_origin_open = false;
    }
}

/// A small isometric-ish cube, the header's mark.
fn cube_icon(ui: &mut egui::Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let c = r.center();
    let s = 5.0;
    let top = [pos2(c.x, c.y - s), pos2(c.x + s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x - s, c.y - s / 2.0)];
    let left = [pos2(c.x - s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x, c.y + s), pos2(c.x - s, c.y + s / 2.0)];
    let right = [pos2(c.x + s, c.y - s / 2.0), pos2(c.x, c.y), pos2(c.x, c.y + s), pos2(c.x + s, c.y + s / 2.0)];
    // The mark shades from the theme's secondary ink, so its faces read on the light and the
    // dark panel alike (menu_text is white: invisible on the light one).
    let shade = |k: f32| {
        let v = (f32::from(t.icon_secondary.r()) * k).round().clamp(70.0, 235.0) as u8;
        Color32::from_gray(v)
    };
    for (face, k) in [(top, 1.0), (left, 0.6), (right, 0.8)] {
        ui.painter().add(egui::Shape::convex_polygon(face.to_vec(), shade(k), Stroke::NONE));
    }
}

/// The layer's name in a rounded pill, elided to `max_w`.
pub(crate) fn name_pill(ui: &mut egui::Ui, t: &Tokens, name: &str, max_w: f32) {
    let inner = (max_w - 12.0).max(0.0);
    let text = crate::widgets::elide_font(ui, name, inner, regular(11.0));
    let galley = ui.painter().layout_no_wrap(text, regular(11.0), t.text_secondary);
    let w = galley.size().x.min(inner);
    let (r, _) = ui.allocate_exact_size(vec2(w + 12.0, 20.0), Sense::hover());
    ui.painter().rect_stroke(r, CornerRadius::same(5), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    ui.painter().galley(pos2(r.min.x + 6.0, r.center().y - galley.size().y / 2.0), galley, t.text_secondary);
}

/// The rotation-point mark: a crosshair circle with four ticks (the reference's ⊕).
fn crosshair(painter: &egui::Painter, c: Pos2, col: Color32) {
    let r = 6.0;
    painter.circle_stroke(c, r, Stroke::new(1.2, col));
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        painter.line_segment([pos2(c.x + dx * r, c.y + dy * r), pos2(c.x + dx * (r + 3.0), c.y + dy * (r + 3.0))], Stroke::new(1.2, col));
    }
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
            && let Some((pix, dot)) = figcraft_render::preview3d::cube(&doc, id, (pw, ph))
        {
            app.ui.t3_dot_norm = Some(dot.map_or(vec2(0.5, 0.5), |d| vec2(d.0, d.1)));
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
