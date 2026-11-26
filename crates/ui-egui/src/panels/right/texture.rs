//! The Texture effect popover: size across and down, the scatter radius, and clipping to the
//! layer's shape. It follows the reference's floating panel rather than expanding beneath the
//! Effects row.

use egui::{CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, NodeId, Texture};
use serde_json::json;

use super::transform3d::label_at;
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::{regular, semibold};
use crate::widgets::{self, FieldEdit, Lead};

const WIDTH: f32 = 230.0;
const LABEL_W: f32 = 72.0;
const ROW_H: f32 = 32.0;

/// The Texture popover for effect `i` on layer `id`.
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: Rect, t: &Tokens) {
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i)).cloned() else {
        close(app);
        return;
    };
    if !effect.visible || !matches!(effect.kind, EffectKind::Texture(_)) {
        close(app);
        return;
    }
    let texture = match &effect.kind {
        EffectKind::Texture(texture) => texture,
        _ => return,
    };
    let (size_x, size_y) = texture.grain();
    let ids = [id];
    let set = |app: &mut FigApp, texture: Texture| {
        let effect = with_texture(&effect, |current| *current = texture.clone());
        app.run("effect.set", json!({"ids": ids, "index": i, "effect": effect}));
    };

    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - 235.0).max(8.0)));
    let mut pop = app
        .ui
        .texture_popover_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("texture_popover"), tr("Texture")).width(WIDTH).pad(0.0).no_header());
    pop = pop.at(at).close_on_outside(app.ui.texture_popover_shown);
    let hold: Vec<Rect> = vec![row_rect];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut trigger_end = 0.0;
        flush_row(ui, 30.0, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            draw_texture_mark(ui.painter(), icon_rect.translate(vec2(-4.0, -1.5)), t.icon_secondary);
            ui.add_space(1.0);
            let title = ui.painter().layout_no_wrap(tr("Texture").to_owned(), regular(11.0), t.text);
            let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
            ui.painter().galley(title_rect.min, title, t.text);
            ui.add_space(4.0);
            let (chevron_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui.painter(), chevron_rect, Icon::Chevron, t.icon_secondary);
            trigger_end = chevron_rect.max.x;
        });
        let header = ui.min_rect();
        let close_rect = Rect::from_min_size(pos2(header.max.x - 28.0, header.top() + 4.0), vec2(26.0, 26.0));
        let close = ui.interact(close_rect, Id::unique("texture_close"), Sense::click());
        crate::a11y::describe(&close, tr("Close"));
        if close.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if close.clicked() {
            app.ui.texture_popover = None;
        }
        crate::panels::right::effects::header_switch(app, ui, t, &ids, i, &effect, header, trigger_end, row_rect);
        ui.add_space(6.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(3.0);

        flush_block(ui, 72.0, 4.5, |ui| {
            label_at(ui, t, tr("Size"), 2.0);
            ui.add_space(LABEL_W);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                let field_w = ui.available_width();
                let x = widgets::number_field_sized(ui, Id::unique(("texture", "sizeX", i)), Lead::Text("X"), Some(size_x), "", field_w, 0.1, 30.0);
                if let Some(edit) = x {
                    let mut next = texture.clone();
                    if next.size_y.is_none() {
                        next.size_y = Some(size_y);
                    }
                    match edit {
                        FieldEdit::Set(value) | FieldEdit::Scrub(value, _) => next.size = value.clamp(0.01, 1000.0),
                    }
                    set(app, next);
                }
                let y = widgets::number_field_sized(ui, Id::unique(("texture", "sizeY", i)), Lead::Text("Y"), Some(size_y), "", field_w, 0.1, 30.0);
                if let Some(edit) = y {
                    let mut next = texture.clone();
                    match edit {
                        FieldEdit::Set(value) | FieldEdit::Scrub(value, _) => next.size_y = Some(value.clamp(0.01, 1000.0)),
                    }
                    set(app, next);
                }
            });
        });
        ui.add_space(0.0);
        flush_row(ui, ROW_H, |ui| {
            label_at(ui, t, tr("Corner|texture"), 2.0);
            ui.add_space(LABEL_W);
            let field_w = ui.available_width();
            let radius_id = Id::unique(("texture", "radius", i));
            let field = widgets::number_field_sized(
                ui,
                radius_id,
                Lead::Icon(Icon::CornerRadius, tr("Corner|texture")),
                Some(texture.radius),
                "",
                field_w,
                1.0,
                30.0,
            );
            if let Some(field) = ui.ctx().read_response(radius_id.with("field")) {
                let mark = Rect::from_center_size(pos2(field.rect.left() - 12.0, field.rect.center().y), Vec2::splat(18.0));
                ui.painter().circle_filled(mark.center(), 9.0, t.field);
                draw_radius_mark(ui.painter(), mark, t.icon_secondary);
            }
            if let Some(edit) = field {
                let mut next = texture.clone();
                match edit {
                    FieldEdit::Set(value) | FieldEdit::Scrub(value, _) => next.radius = value.max(0.0),
                }
                set(app, next);
            }
        });
        ui.add_space(13.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(16.0);
        flush_block(ui, 22.0, 0.0, |ui| {
            if texture_checkbox(ui, texture.clip_to_shape, tr("Clip to shape")) {
                let mut next = texture.clone();
                next.clip_to_shape = !next.clip_to_shape;
                set(app, next);
            }
        });
        ui.add_space(8.0);
    });
    app.ui.texture_popover_pop = Some(pop);
    app.ui.texture_popover_shown = true;
    if closed {
        close(app);
    }
}

fn with_texture(effect: &Effect, change: impl FnOnce(&mut Texture)) -> Effect {
    let mut edited = effect.clone();
    if let EffectKind::Texture(texture) = &mut edited.kind {
        change(texture);
    }
    edited
}

fn close(app: &mut FigApp) {
    app.ui.texture_popover = None;
    app.ui.texture_popover_shown = false;
}

fn draw_texture_mark(painter: &egui::Painter, rect: Rect, ink: egui::Color32) {
    let inset = 3.1;
    let step = (rect.width() - 2.0 * inset) / 3.0;
    for row in 0..4 {
        for col in 0..4 {
            painter.circle_filled(pos2(rect.left() + inset + col as f32 * step, rect.top() + inset + row as f32 * step), 0.7, ink);
        }
    }
}

fn draw_radius_mark(painter: &egui::Painter, rect: Rect, ink: egui::Color32) {
    let c = rect.center();
    painter.circle_stroke(c, 3.0, Stroke::new(1.0, ink));
    for point in 0..8 {
        let angle = point as f32 * std::f32::consts::TAU / 8.0;
        painter.circle_filled(c + vec2(angle.cos(), angle.sin()) * 6.5, 0.9, ink);
    }
}

fn texture_checkbox(ui: &mut egui::Ui, on: bool, label: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
    crate::a11y::checkbox_tri(&resp, label, Some(on));
    let box_rect = Rect::from_min_size(pos2(rect.left(), rect.center().y - 8.0), Vec2::splat(16.0));
    if on {
        ui.painter().rect_filled(box_rect, CornerRadius::same(3), t.accent);
        icons::paint(ui.painter(), box_rect.shrink(3.0), Icon::Check, t.accent_on);
    } else {
        ui.painter().rect_stroke(box_rect, CornerRadius::same(3), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
    }
    let text = ui.painter().layout_no_wrap(label.to_owned(), semibold(12.0), t.text);
    ui.painter().galley(pos2(box_rect.right() + 8.0, rect.center().y - text.size().y / 2.0), text, t.text);
    resp.clicked()
}

/// A row across the popover's full inner width, without the Design panel's extra left inset.
fn flush_row(ui: &mut egui::Ui, h: f32, add: impl FnOnce(&mut egui::Ui)) {
    flush_block(ui, h, (h - 24.0) / 2.0, add);
}

fn flush_block(ui: &mut egui::Ui, h: f32, inset_y: f32, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let inner = Rect::from_min_max(pos2(row.left() + 10.0, row.top() + inset_y), pos2(row.right() - 10.0, row.bottom() - inset_y));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = vec2(8.0, 0.0);
    add(&mut child);
}
