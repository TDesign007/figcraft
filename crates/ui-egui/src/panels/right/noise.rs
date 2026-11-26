//! The Noise effect popover: grain size across and down, density, the mode's colours and their
//! opacity. It follows the reference's floating panel rather than expanding beneath the Effects
//! row, and every edit goes through `effect.set` so scrubs stay one undo step.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_color::Color;
use figcraft_doc::{Effect, EffectKind, NodeId, Noise, NoiseMode};
use serde_json::json;

use super::menu::{blend_menu, menu_popup};
use super::transform3d::label_at;
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::{regular, semibold};
use crate::widgets::{self, FieldEdit, Lead};

const WIDTH: f32 = 235.0;
const LABEL_W: f32 = 72.0;
const ROW_H: f32 = 24.0;
const SWATCH_W: f32 = 19.0;
const HEX_W: f32 = 61.0;
const PERCENT_W: f32 = 52.0;

/// The Noise popover for effect `i` on layer `id`.
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, id: NodeId, i: usize, row_rect: Rect, t: &Tokens) {
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i)).cloned() else {
        close(app);
        return;
    };
    if !effect.visible || !matches!(effect.kind, EffectKind::Noise(_)) {
        close(app);
        return;
    }
    let noise = match &effect.kind {
        EffectKind::Noise(n) => n.clone(),
        _ => return,
    };
    let (size_x, size_y) = noise.grain();
    let ids = [id];
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction("Noise");
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    let with = |f: &dyn Fn(&mut Noise)| {
        let mut e2 = effect.clone();
        if let EffectKind::Noise(n) = &mut e2.kind {
            f(n);
        }
        e2
    };
    let edit = |app: &mut FigApp, ed: Option<FieldEdit>, f: &dyn Fn(&mut Noise, f64)| {
        let Some(ed) = ed else { return };
        let (v, scrub) = match ed {
            FieldEdit::Set(v) => (v, None),
            FieldEdit::Scrub(v, done) => (v, Some(done)),
        };
        set(app, with(&|n| f(n, v)), scrub);
    };
    let edit_color = |app: &mut FigApp, secondary: bool, c: Color, scrub: Option<bool>| {
        set(
            app,
            with(&|n| {
                if secondary {
                    n.secondary = c;
                } else {
                    n.color = c;
                }
            }),
            scrub,
        );
    };

    let screen_h = ctx.content_rect().height();
    let expected_h = if noise.mode == NoiseMode::Duo { 262.0 } else { 232.0 };
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - expected_h).max(8.0)));
    let mut pop = app
        .ui
        .noise_popover_pop
        .take()
        .unwrap_or_else(|| Popover::with_id(Id::unique("noise_popover"), tr("Noise")).width(WIDTH).pad(0.0).no_header());
    pop = pop.at(at).close_on_outside(app.ui.noise_popover_shown);
    let hold: Vec<Rect> = vec![row_rect];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 0.0;

        let mut trigger_end = 0.0;
        flush_row(ui, 33.0, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            draw_noise_mark(ui.painter(), icon_rect.shrink(1.0), t.icon_secondary);
            ui.add_space(1.0);
            let title = ui.painter().layout_no_wrap(tr("Noise").to_owned(), semibold(11.0), t.text);
            let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
            ui.painter().galley(title_rect.min, title, t.text);
            ui.add_space(5.0);
            let (chevron_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui.painter(), chevron_rect, Icon::Chevron, t.icon_secondary);
            trigger_end = chevron_rect.max.x;
        });
        let header = ui.min_rect();
        let close_rect = Rect::from_min_size(pos2(header.max.x - 32.0, header.top() + 3.0), vec2(26.0, 26.0));
        let drop_rect = Rect::from_min_size(pos2(close_rect.left() - 30.0, header.top() + 3.0), vec2(26.0, 26.0));
        let droplet = draw_droplet_button(ui, drop_rect, t);
        menu_popup(&droplet).show(|ui| {
            if let Some(blend) = blend_menu(ui, false, |_| {}) {
                set(app, with(&|n| n.blend = blend), None);
                ui.close();
            }
        });
        let close = ui.interact(close_rect, Id::unique("noise_close"), Sense::click());
        crate::a11y::describe(&close, tr("Close"));
        if close.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if close.clicked() {
            app.ui.noise_popover = None;
        }
        crate::panels::right::effects::header_switch(app, ui, t, &ids, i, &effect, header, trigger_end, row_rect);
        ui.add_space(4.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(13.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            segmented(app, ui, t, id, noise.mode, i, &effect);
        });
        ui.add_space(12.0);

        flush_block(ui, 56.0, 0.0, |ui| {
            label_at(ui, t, tr("Noise size"), 2.0);
            ui.add_space(LABEL_W);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                let field_w = ui.available_width();
                let x = widgets::number_field_sized(ui, Id::unique(("noise", "sizeX", i)), Lead::Text("X"), Some(size_x), "", field_w, 0.1, ROW_H);
                if let Some(ed) = x {
                    let (value, scrub) = match ed {
                        FieldEdit::Set(value) => (value, None),
                        FieldEdit::Scrub(value, done) => (value, Some(done)),
                    };
                    set(
                        app,
                        with(&|n| {
                            if n.size_y.is_none() {
                                n.size_y = Some(size_y);
                            }
                            n.size = value.clamp(0.01, 1000.0);
                        }),
                        scrub,
                    );
                }
                let y = widgets::number_field_sized(ui, Id::unique(("noise", "sizeY", i)), Lead::Text("Y"), Some(size_y), "", field_w, 0.1, ROW_H);
                edit(app, y, &|n, value| n.size_y = Some(value.clamp(0.01, 1000.0)));
            });
        });
        ui.add_space(8.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            label_at(ui, t, tr("Density"), 2.0);
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let field = widgets::number_field_sized(
                ui,
                Id::unique(("noise", "density", i)),
                Lead::Icon(Icon::DotsGrid, tr("Density")),
                Some((noise.density * 100.0).round()),
                "%",
                w,
                1.0,
                ROW_H,
            );
            edit(app, field, &|n, value| n.density = (value / 100.0).clamp(0.0, 1.0));
        });
        ui.add_space(8.0);

        match noise.mode {
            NoiseMode::Mono => flush_block(ui, ROW_H, 0.0, |ui| {
                label_at(ui, t, tr("Color"), 2.0);
                ui.add_space(LABEL_W);
                color_row(app, ui, t, &ids, i, false, true, noise.color, &edit_color);
            }),
            NoiseMode::Duo => {
                flush_block(ui, ROW_H, 0.0, |ui| {
                    label_at(ui, t, tr("Color"), 2.0);
                    ui.add_space(LABEL_W);
                    color_row(app, ui, t, &ids, i, false, true, noise.color, &edit_color);
                });
                ui.add_space(8.0);
                flush_block(ui, ROW_H, 0.0, |ui| {
                    ui.add_space(LABEL_W);
                    color_row(app, ui, t, &ids, i, true, false, noise.secondary, &edit_color);
                });
            }
            NoiseMode::Multi => flush_block(ui, ROW_H, 0.0, |ui| {
                label_at(ui, t, tr("Opacity"), 2.0);
                ui.add_space(LABEL_W);
                let w = ui.available_width();
                let field = widgets::number_field_sized(
                    ui,
                    Id::unique(("noise", "opacity", i)),
                    Lead::Icon(Icon::OpacityGrid, tr("Opacity")),
                    Some((f64::from(noise.opacity) * 100.0).round()),
                    "%",
                    w,
                    1.0,
                    ROW_H,
                );
                edit(app, field, &|n, value| n.opacity = (value / 100.0).clamp(0.0, 1.0) as f32);
            }),
        }
        ui.add_space(16.0);
    });
    app.ui.noise_popover_pop = Some(pop);
    app.ui.noise_popover_shown = true;
    if closed {
        close(app);
    }
}

fn close(app: &mut FigApp) {
    app.ui.noise_popover = None;
    app.ui.noise_popover_shown = false;
}

type ColorEdit<'a> = dyn Fn(&mut FigApp, bool, Color, Option<bool>) + 'a;

/// The reference's colour row: a split swatch, the hex and the colour's opacity.
#[allow(clippy::too_many_arguments)]
fn color_row(
    app: &mut FigApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    ids: &[NodeId],
    i: usize,
    secondary: bool,
    split: bool,
    c: Color,
    edit: &ColorEdit<'_>,
) {
    let (swatch_rect, swatch) = ui.allocate_exact_size(vec2(SWATCH_W, ROW_H), Sense::click());
    if split {
        draw_split_swatch(ui.painter(), swatch_rect, swatch_colour(c), t);
    } else {
        draw_solid_swatch(ui.painter(), swatch_rect, swatch_colour(c), t);
    }
    crate::a11y::describe(&swatch, tr("Color"));
    app.picker_control(crate::colorpicker::PickerTarget::Effect { ids: ids.to_vec(), index: i, secondary }, &swatch);
    ui.add_space(2.0);
    let hex_id = Id::unique(("noise", "hex", i, secondary));
    let hex_edit = widgets::text_field(ui, hex_id, &c.to_hex(), HEX_W, "");
    if let Some(hex) = hex_edit
        && let Some(next) = Color::from_hex_or_name(&hex)
    {
        edit(app, secondary, next.with_alpha(c.a), None);
    }
    ui.add_space(2.0);
    let alpha = |v: f64| c.with_alpha((v / 100.0).clamp(0.0, 1.0) as f32);
    match widgets::number_field_sized(
        ui,
        Id::unique(("noise", "alpha", i, secondary)),
        Lead::None,
        Some((f64::from(c.a) * 100.0).round()),
        "%",
        PERCENT_W,
        1.0,
        ROW_H,
    ) {
        Some(FieldEdit::Set(v)) => edit(app, secondary, alpha(v), None),
        Some(FieldEdit::Scrub(v, done)) => edit(app, secondary, alpha(v), Some(done)),
        None => {}
    }
}

fn segmented(app: &mut FigApp, ui: &mut egui::Ui, t: &Tokens, id: NodeId, mode: NoiseMode, i: usize, effect: &Effect) {
    let width = ui.available_width();
    let (track, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    ui.painter().rect_filled(track, CornerRadius::same(6), t.panel);
    let item_w = width / 3.0;
    for (index, m) in NoiseMode::ALL.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(track.left() + item_w * index as f32, track.top()), vec2(item_w, ROW_H));
        let resp = ui.interact(rect, Id::unique(("noise", "tab", index, i)), Sense::click());
        let on = *m == mode;
        if on {
            ui.painter().rect_filled(rect, CornerRadius::same(6), t.field);
            ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(6), t.hover);
        }
        let ink = if on { t.text } else { t.text_secondary };
        let font = if on { semibold(12.0) } else { regular(12.0) };
        let galley = ui.painter().layout_no_wrap(tr(m.label()).to_owned(), font, ink);
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
        if resp.clicked() && !on {
            let mut e2 = effect.clone();
            if let EffectKind::Noise(n) = &mut e2.kind {
                n.mode = *m;
            }
            app.run("effect.set", json!({"ids": [id], "index": i, "effect": e2}));
        }
    }
}

fn draw_noise_mark(painter: &egui::Painter, rect: Rect, ink: Color32) {
    painter.rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.0, ink), egui::StrokeKind::Inside);
    for row in 0..3 {
        for col in 0..3 {
            let at = rect.min + vec2(4.0 + col as f32 * 3.2, 4.0 + row as f32 * 3.2);
            painter.circle_filled(at, 0.85, ink);
        }
    }
}

fn draw_droplet_button(ui: &mut egui::Ui, rect: Rect, t: &Tokens) -> egui::Response {
    let resp = ui.interact(rect, Id::unique("noise_blend"), Sense::click());
    crate::a11y::describe(&resp, tr("Blend mode"));
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(5), t.hover);
    }
    let c = rect.center();
    let points = vec![
        c + vec2(0.0, -6.0),
        c + vec2(2.0, -2.6),
        c + vec2(4.6, 1.0),
        c + vec2(5.0, 2.9),
        c + vec2(3.2, 5.2),
        c + vec2(0.0, 6.2),
        c + vec2(-3.2, 5.2),
        c + vec2(-5.0, 2.9),
        c + vec2(-4.6, 1.0),
        c + vec2(-2.0, -2.6),
    ];
    ui.painter().add(egui::Shape::closed_line(points, Stroke::new(1.15, t.icon_secondary)));
    resp
}

fn draw_split_swatch(painter: &egui::Painter, rect: Rect, colour: Color32, t: &Tokens) {
    painter.rect_filled(rect, CornerRadius::same(4), Color32::WHITE);
    let left = Rect::from_min_max(rect.min, pos2(rect.center().x, rect.max.y));
    painter.rect_filled(left, CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 }, colour);
    painter.vline(rect.center().x, rect.y_range(), Stroke::new(1.0, t.border_strong));
    painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
}

/// The swatch shows the colour itself: its opacity is the percentage field beside it.
fn swatch_colour(c: Color) -> Color32 {
    let [r, g, b, _] = c.to_rgba8();
    Color32::from_rgb(r, g, b)
}

fn draw_solid_swatch(painter: &egui::Painter, rect: Rect, colour: Color32, t: &Tokens) {
    painter.rect_filled(rect, CornerRadius::same(4), colour);
    painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
}

/// A row across the popover's full inner width, with the reference's 10 pt left and 15 pt right
/// inset.
fn flush_row(ui: &mut egui::Ui, h: f32, add: impl FnOnce(&mut egui::Ui)) {
    flush_block(ui, h, (h - 24.0) / 2.0, add);
}

fn flush_block(ui: &mut egui::Ui, h: f32, inset_y: f32, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let inner = Rect::from_min_max(pos2(row.left() + 12.0, row.top() + inset_y), pos2(row.right() - 15.0, row.bottom() - inset_y));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    add(&mut child);
}
