//! The Glass effect popover: a light preview with a draggable direction handle, then the glass
//! controls in the reference tool's order. Every edit goes through `effect.set`, so scrubs stay
//! live and commit as one undo step, and every value keeps its variable binding.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, Glass, NodeId};
use serde_json::json;

use super::transform3d::label_at;
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::{BRAND, regular};
use crate::widgets::{FieldEdit, Lead};

const WIDTH: f32 = 230.0;
const LABEL_W: f32 = 72.0;
const VALUE_W: f32 = 64.0;
const PREVIEW: f32 = 58.0;
const ROW_H: f32 = 28.0;
const FIGMA_BLUE: Color32 = Color32::from_rgb(0x0A, 0x84, 0xFF);
const LIGHT_ORBIT: f32 = 17.0;

/// The Glass popover for effect `i` on layer `id`.
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: Rect, t: &Tokens) {
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i)).cloned() else {
        close(app);
        return;
    };
    if !effect.visible || !matches!(effect.kind, EffectKind::Glass(_)) {
        close(app);
        return;
    }
    let glass = match &effect.kind {
        EffectKind::Glass(g) => g.sanitized(),
        _ => return,
    };
    let ids = [id];
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction("Glass");
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    fn with(e: &Effect, f: impl FnOnce(&mut Glass)) -> Effect {
        let mut e2 = e.clone();
        if let EffectKind::Glass(g) = &mut e2.kind {
            f(g);
        }
        e2
    }
    let edit = |app: &mut FigApp, ed: Option<FieldEdit>, f: &dyn Fn(f64) -> Effect| match ed {
        Some(FieldEdit::Set(v)) => set(app, f(v), None),
        Some(FieldEdit::Scrub(v, done)) => set(app, f(v), Some(done)),
        None => {}
    };

    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - 370.0).max(8.0)));
    let mut pop = app
        .ui
        .glass_popover_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("glass_popover"), tr("Glass")).width(WIDTH).pad(10.0).no_header());
    app.ui.glass_preview_rect = None;
    app.ui.glass_slider_rects.clear();
    pop = pop.at(at).close_on_outside(app.ui.glass_popover_shown);
    let hold: Vec<Rect> = vec![row_rect];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH - 20.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut trigger_end = 0.0;
        flush_row(ui, 30.0, |ui| {
            let (icon_r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            draw_glass_mark(ui.painter(), icon_r, t.icon_secondary, t.panel);
            ui.add_space(6.0);
            let title = ui.painter().layout_no_wrap(tr("Glass").to_owned(), regular(11.0), t.text);
            let (title_r, _) = ui.allocate_exact_size(title.size(), Sense::hover());
            ui.painter().galley(title_r.min, title, t.text);
            ui.add_space(4.0);
            let (chevron_r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui.painter(), chevron_r, Icon::Chevron, t.icon_secondary);
            trigger_end = chevron_r.max.x;
        });
        let hr = ui.min_rect();
        let close_rect = Rect::from_min_size(pos2(hr.max.x - 34.0, hr.top() + 4.0), vec2(26.0, 26.0));
        let close = ui.interact(close_rect, Id::unique("glass_close"), Sense::click());
        crate::a11y::describe(&close, tr("Close"));
        if close.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if close.clicked() {
            app.ui.glass_popover = None;
        }
        crate::panels::right::effects::header_switch(app, ui, t, &ids, i, &effect, hr, trigger_end, row_rect);
        ui.add_space(3.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(7.0);

        flush_block(ui, 76.0, 9.0, |ui| {
            label_at(ui, t, tr("Light|glass"), 0.0);
            ui.add_space(LABEL_W);
            let (preview_rect, _) = ui.allocate_exact_size(Vec2::splat(PREVIEW), Sense::hover());
            app.ui.glass_preview_rect = Some(preview_rect);
            draw_preview(ui, preview_rect, glass.light_angle, t);
            let handle = flashlight_rect(preview_rect, glass.light_angle);
            let handle_resp = ui.interact(handle, Id::unique(("glass", "light_handle", i)), Sense::click_and_drag());
            crate::a11y::describe(&handle_resp, tr("Light|glass"));
            let angle = glass.light_angle;
            let moved = if handle_resp.drag_started() || handle_resp.dragged() {
                handle_resp.interact_pointer_pos().map(|p| light_angle_at(preview_rect.center(), p))
            } else {
                None
            };
            if let Some(v) = moved {
                set(app, with(&effect, |g| g.light_angle = v), Some(false));
            }
            if moved.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if handle_resp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
            if handle_resp.drag_stopped() {
                set(app, with(&effect, |g| g.light_angle = moved.unwrap_or(angle)), Some(true));
            }
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let w = ui.available_width().min(70.0);
                let ed = crate::variable_ui::number_field(
                    app,
                    ui,
                    &ids,
                    &format!("effects/{i}/lightAngle"),
                    Id::unique(("glass", "lightAngle", i)),
                    Lead::Named(tr("Light|glass")),
                    Some(glass.light_angle.round()),
                    "°",
                    w,
                );
                edit(app, ed, &|v| with(&effect, |g| g.light_angle = v.clamp(-180.0, 180.0)));
                let ed = crate::variable_ui::number_field(
                    app,
                    ui,
                    &ids,
                    &format!("effects/{i}/lightIntensity"),
                    Id::unique(("glass", "lightIntensity", i)),
                    Lead::Named(tr("Intensity")),
                    Some((glass.light_intensity * 100.0).round()),
                    "%",
                    w,
                );
                edit(app, ed, &|v| with(&effect, |g| g.light_intensity = (v / 100.0).clamp(0.0, 1.0)));
            });
        });
        ui.add_space(7.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(7.0);

        glass_slider(app, ui, t, &ids, i, "refraction", tr("Refraction"), glass.refraction * 100.0, 0.0, 100.0, 80.0, &effect, &set, |g, v| {
            g.refraction = (v / 100.0).clamp(0.0, 1.0)
        });
        glass_slider(app, ui, t, &ids, i, "depth", tr("Depth"), glass.depth, 1.0, 100.0, 20.0, &effect, &set, |g, v| g.depth = v.clamp(1.0, 1000.0));
        glass_slider(app, ui, t, &ids, i, "dispersion", tr("Dispersion"), glass.dispersion * 100.0, 0.0, 100.0, 50.0, &effect, &set, |g, v| {
            g.dispersion = (v / 100.0).clamp(0.0, 1.0)
        });
        glass_slider(app, ui, t, &ids, i, "radius", tr("Frost"), glass.frost, 0.0, Glass::MAX_FROST, 4.0, &effect, &set, |g, v| {
            g.frost = v.clamp(0.0, Glass::MAX_FROST)
        });
        glass_slider(app, ui, t, &ids, i, "splay", tr("Splay"), glass.splay * 100.0, 0.0, 100.0, 0.0, &effect, &set, |g, v| {
            g.splay = (v / 100.0).clamp(0.0, 1.0)
        });
        ui.add_space(6.0);
    });
    app.ui.glass_popover_pop = Some(pop);
    app.ui.glass_popover_shown = true;
    if closed {
        close(app);
    }
}

fn close(app: &mut FigApp) {
    app.ui.glass_popover = None;
    app.ui.glass_popover_shown = false;
}

/// A row across the popover's full inner width, without the Design panel's extra left inset.
fn flush_row(ui: &mut egui::Ui, h: f32, add: impl FnOnce(&mut egui::Ui)) {
    flush_block(ui, h, (h - 24.0) / 2.0, add);
}

fn flush_block(ui: &mut egui::Ui, h: f32, inset_y: f32, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let inner = Rect::from_min_max(pos2(row.left(), row.top() + inset_y), pos2(row.right(), row.bottom() - inset_y));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = vec2(8.0, 0.0);
    add(&mut child);
}

fn draw_preview(ui: &mut egui::Ui, rect: Rect, angle: f64, t: &Tokens) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(8), t.field);
    let glass = Rect::from_center_size(rect.center(), Vec2::splat(28.0));
    painter.rect_filled(glass, CornerRadius::same(5), Color32::from_rgba_unmultiplied(180, 180, 180, 28));
    let mut mesh = egui::Mesh::default();
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(glass.left_top(), Color32::from_white_alpha(32));
    mesh.colored_vertex(glass.right_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(glass.right_bottom(), Color32::from_black_alpha(20));
    mesh.colored_vertex(glass.left_bottom(), Color32::from_white_alpha(8));
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    painter.with_clip_rect(glass.shrink(1.0)).add(egui::Shape::mesh(mesh));
    // The object never moves: only the flashlight turns around its centre.
    draw_light_beam(painter, rect, angle);
    let centre = Rect::from_center_size(rect.center(), Vec2::splat(6.0));
    painter.rect_stroke(centre, CornerRadius::same(1), Stroke::new(1.0, Color32::from_white_alpha(150)), egui::StrokeKind::Inside);
    draw_flashlight(painter, flashlight_rect(rect, angle).center(), rect.center(), t);
}

fn draw_glass_mark(painter: &egui::Painter, rect: Rect, ink: Color32, bg: Color32) {
    let c = rect.center();
    painter.circle_stroke(c, 6.8, Stroke::new(1.25, ink));
    // The reference mark leaves the lower-left rim open, like a tilted pane.
    let cut = vec![c, c + vec2(-6.5, 2.4), c + vec2(-4.6, 5.4), c + vec2(0.5, 6.8)];
    painter.add(egui::Shape::convex_polygon(cut, bg, Stroke::NONE));
}

pub(crate) fn light_angle_at(centre: Pos2, pointer: Pos2) -> f64 {
    let d = pointer - centre;
    (d.x.atan2(-d.y).to_degrees() as f64).round()
}

pub(crate) fn flashlight_rect(preview: Rect, angle: f64) -> Rect {
    let a = angle.to_radians() as f32;
    let centre = preview.center() + vec2(a.sin(), -a.cos()) * LIGHT_ORBIT;
    Rect::from_center_size(centre, Vec2::splat(20.0))
}

pub(crate) fn light_beam_polygon(preview: Rect, angle: f64) -> [Pos2; 4] {
    let centre = preview.center();
    let handle = flashlight_rect(preview, angle).center();
    let beam = (centre - handle).normalized();
    let side = vec2(-beam.y, beam.x);
    [handle + beam * 6.2 + side * 1.5, handle + beam * 6.2 - side * 1.5, centre - side * 5.5, centre + side * 5.5]
}

fn draw_light_beam(painter: &egui::Painter, preview: Rect, angle: f64) {
    painter.with_clip_rect(preview.shrink(2.0)).add(egui::Shape::convex_polygon(
        light_beam_polygon(preview, angle).to_vec(),
        Color32::from_rgba_unmultiplied(10, 132, 255, 36),
        Stroke::NONE,
    ));
}

fn draw_flashlight(painter: &egui::Painter, at: Pos2, object_centre: Pos2, t: &Tokens) {
    let beam = (object_centre - at).normalized();
    let side = vec2(-beam.y, beam.x);
    let tail = at - beam * 5.8;
    let neck = at + beam * 1.2;
    let lens = at + beam * 6.2;
    let ink = Stroke::new(1.35, FIGMA_BLUE);
    let outline = |points: &[Pos2]| {
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], ink);
        }
        painter.line_segment([points[points.len() - 1], points[0]], ink);
    };
    outline(&[tail + side * 2.2, neck + side * 2.2, neck - side * 2.2, tail - side * 2.2]);
    outline(&[neck + side * 2.2, lens + side * 4.2, lens - side * 4.2, neck - side * 2.2]);
    painter.circle_filled(at, 1.0, t.field);
}

#[allow(clippy::too_many_arguments)]
fn glass_slider(
    app: &mut FigApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    ids: &[NodeId],
    i: usize,
    field: &str,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    default: f64,
    e: &Effect,
    set: &dyn Fn(&mut FigApp, Effect, Option<bool>),
    apply: impl Fn(&mut Glass, f64),
) {
    let with = |v: f64| {
        let mut e2 = e.clone();
        if let EffectKind::Glass(g) = &mut e2.kind {
            apply(g, v);
        }
        e2
    };
    flush_row(ui, ROW_H, |ui| {
        label_at(ui, t, label, 0.0);
        ui.add_space(LABEL_W);
        let total = ui.available_width();
        let slider_w = (total - VALUE_W - 2.0).max(36.0);
        let (well, resp) = ui.allocate_exact_size(vec2(slider_w, 24.0), Sense::click_and_drag());
        crate::a11y::describe(&resp, label);
        app.ui.glass_slider_rects.push(well);
        ui.painter().rect_filled(well, CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, t.field);
        let track = Rect::from_min_max(pos2(well.left() + 10.0, well.center().y - 2.0), pos2(well.right() - 10.0, well.center().y + 2.0));
        let m = ((value - min) / (max - min)).clamp(0.0, 1.0) as f32;
        let kx = track.left() + m * track.width();
        ui.painter().rect_filled(track, CornerRadius::same(2), t.border);
        ui.painter().rect_filled(Rect::from_min_max(track.min, pos2(kx, track.max.y)), CornerRadius::same(2), BRAND);
        let knob = Rect::from_center_size(pos2(kx, track.center().y), Vec2::splat(12.0));
        ui.painter().circle_filled(knob.center(), 6.0, Color32::WHITE);
        ui.painter().rect_stroke(knob, CornerRadius::same(3), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
        let value_at = |p: Pos2| min + (((p.x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0) as f64) * (max - min);
        let mut edit = None;
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            edit = Some((value_at(p).round(), None));
        }
        if resp.drag_started()
            && let Some(p) = resp.interact_pointer_pos()
        {
            edit = Some((value_at(p).round(), Some(false)));
        }
        if resp.dragged()
            && let Some(p) = resp.interact_pointer_pos()
        {
            edit = Some((value_at(p).round(), Some(false)));
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if resp.drag_stopped()
            && let Some(p) = resp.interact_pointer_pos()
        {
            edit = Some((value_at(p).round(), Some(true)));
        }
        if resp.double_clicked() {
            edit = Some((default, None));
        }
        ui.add_space(2.0);
        let field_width = ui.available_width().min(VALUE_W);
        let ed = crate::variable_ui::number_field(
            app,
            ui,
            ids,
            &format!("effects/{i}/{field}"),
            Id::unique(("glass", field, i)),
            Lead::None,
            Some(value.round()),
            "",
            field_width,
        );
        if let Some((v, done)) = edit {
            set(app, with(v), done);
        } else if let Some(ed) = ed {
            match ed {
                FieldEdit::Set(v) => set(app, with(v), None),
                FieldEdit::Scrub(v, done) => set(app, with(v), Some(done)),
            }
        }
        if resp.hovered() && edit.is_none() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    });
}
