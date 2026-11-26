//! The Drop shadow and Inner shadow popovers: position, blur, spread and colour, floating beside
//! the Effects row like Figma's. They share one panel, apart from the mark and the words; every
//! edit goes through `effect.set` so scrubs stay one undo step.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_color::{Color, Paint};
use figcraft_doc::{Effect, EffectKind, NodeId, Shadow, VariableType};
use serde_json::json;

use super::menu::{blend_menu, menu_popup};
use super::transform3d::label_at;
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::semibold;
use crate::widgets::{self, FieldEdit, Lead};

const WIDTH: f32 = 238.0;
const LABEL_W: f32 = 71.0;
const ROW_H: f32 = 24.0;
const ALPHA_W: f32 = 44.0;

/// Which shadow a popover edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Drop,
    Inner,
}

impl Kind {
    /// The shadow effect a row of `kind` holds.
    pub(crate) fn of(kind: &EffectKind) -> Option<Self> {
        match kind {
            EffectKind::DropShadow(_) => Some(Self::Drop),
            EffectKind::InnerShadow(_) => Some(Self::Inner),
            _ => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Drop => "Drop shadow",
            Self::Inner => "Inner shadow",
        }
    }

    /// The prefix of the panel's widget IDs and of its remembered `Popover`.
    fn salt(self) -> &'static str {
        match self {
            Self::Drop => "shadow",
            Self::Inner => "inner_shadow",
        }
    }

    /// The ID of the panel's egui area.
    fn panel_id(self) -> &'static str {
        match self {
            Self::Drop => "shadow_popover",
            Self::Inner => "inner_shadow_popover",
        }
    }

    fn shadow<'a>(self, kind: &'a EffectKind) -> Option<&'a Shadow> {
        match (self, kind) {
            (Self::Drop, EffectKind::DropShadow(s)) | (Self::Inner, EffectKind::InnerShadow(s)) => Some(s),
            _ => None,
        }
    }

    fn shadow_mut<'a>(self, kind: &'a mut EffectKind) -> Option<&'a mut Shadow> {
        match (self, kind) {
            (Self::Drop, EffectKind::DropShadow(s)) | (Self::Inner, EffectKind::InnerShadow(s)) => Some(s),
            _ => None,
        }
    }
}

/// The popover showing for `kind`: the layer it edits, its effect's index and the row it opened at.
pub(crate) fn state(app: &FigApp, kind: Kind) -> Option<(NodeId, usize, Rect)> {
    match kind {
        Kind::Drop => app.ui.shadow_popover,
        Kind::Inner => app.ui.inner_shadow_popover,
    }
}

fn set_state(app: &mut FigApp, kind: Kind, at: Option<(NodeId, usize, Rect)>) {
    match kind {
        Kind::Drop => app.ui.shadow_popover = at,
        Kind::Inner => app.ui.inner_shadow_popover = at,
    }
}

fn shown(app: &FigApp, kind: Kind) -> bool {
    match kind {
        Kind::Drop => app.ui.shadow_popover_shown,
        Kind::Inner => app.ui.inner_shadow_popover_shown,
    }
}

fn set_shown(app: &mut FigApp, kind: Kind, shown: bool) {
    match kind {
        Kind::Drop => app.ui.shadow_popover_shown = shown,
        Kind::Inner => app.ui.inner_shadow_popover_shown = shown,
    }
}

fn take_pop(app: &mut FigApp, kind: Kind) -> Option<Popover> {
    match kind {
        Kind::Drop => app.ui.shadow_popover_pop.take(),
        Kind::Inner => app.ui.inner_shadow_popover_pop.take(),
    }
}

fn put_pop(app: &mut FigApp, kind: Kind, pop: Popover) {
    match kind {
        Kind::Drop => app.ui.shadow_popover_pop = Some(pop),
        Kind::Inner => app.ui.inner_shadow_popover_pop = Some(pop),
    }
}

/// Show `kind`'s popover for effect `i` on layer `id`, opened at the row `at`. The other effects'
/// popovers close: only one panel is up at a time.
pub(crate) fn open(app: &mut FigApp, kind: Kind, id: NodeId, i: usize, at: Rect) {
    crate::panels::right::effects::close_effect_popovers(app);
    set_state(app, kind, Some((id, i, at)));
}

/// `open`, unless it already shows that effect: then it closes.
pub(crate) fn toggle(app: &mut FigApp, kind: Kind, id: NodeId, i: usize, at: Rect) {
    if state(app, kind).is_some_and(|(open_id, open_i, _)| open_id == id && open_i == i) {
        close(app, kind);
    } else {
        open(app, kind, id, i, at);
    }
}

/// Hide `kind`'s popover (it is closed, not just put out of the way).
pub(crate) fn close(app: &mut FigApp, kind: Kind) {
    set_state(app, kind, None);
    set_shown(app, kind, false);
}

/// `kind`'s popover, for effect `i` on layer `id`.
pub(crate) fn popover(app: &mut FigApp, ctx: &egui::Context, kind: Kind, id: NodeId, i: usize, row_rect: Rect, t: &Tokens) {
    let Some(effect) = app.session.doc().ok().and_then(|doc| doc.node(id)).and_then(|n| n.effects.get(i)).cloned() else {
        close(app, kind);
        return;
    };
    let Some(shadow) = kind.shadow(&effect.kind).cloned().filter(|_| effect.visible) else {
        close(app, kind);
        return;
    };
    let ids = [id];
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction(tr(kind.title()));
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    let with = |f: &dyn Fn(&mut Shadow)| {
        let mut e2 = effect.clone();
        if let Some(s) = kind.shadow_mut(&mut e2.kind) {
            f(s);
        }
        e2
    };
    let edit = |app: &mut FigApp, ed: Option<FieldEdit>, f: &dyn Fn(&mut Shadow, f64)| {
        let Some(ed) = ed else { return };
        let (v, scrub) = match ed {
            FieldEdit::Set(v) => (v, None),
            FieldEdit::Scrub(v, done) => (v, Some(done)),
        };
        set(app, with(&|s| f(s, v)), scrub);
    };
    let edit_color = |app: &mut FigApp, c: Color, scrub: Option<bool>| set(app, with(&|s| s.color = c), scrub);

    let salt = kind.salt();
    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - 218.0).max(8.0)));
    let mut pop = take_pop(app, kind).unwrap_or_else(|| Popover::with_id(Id::unique(kind.panel_id()), tr(kind.title())).width(WIDTH).pad(0.0).no_header());
    pop = pop.at(at).close_on_outside(shown(app, kind));
    let hold: Vec<Rect> = vec![row_rect];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 0.0;

        let mut trigger_end = 0.0;
        flush_header(ui, 34.0, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            draw_shadow_mark(ui.painter(), icon_rect, kind, t.icon_secondary, fill_of(t));
            ui.add_space(1.0);
            let title = ui.painter().layout_no_wrap(tr(kind.title()).to_owned(), semibold(11.0), t.text);
            let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
            ui.painter().galley(title_rect.min, title, t.text);
            ui.add_space(5.0);
            let (chevron_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui.painter(), chevron_rect, Icon::Chevron, t.icon_secondary);
            trigger_end = chevron_rect.max.x;
        });
        let header = ui.min_rect();
        let close_rect = Rect::from_min_size(pos2(header.max.x - 32.0, header.top() + 4.0), vec2(26.0, 26.0));
        let drop_rect = Rect::from_min_size(pos2(close_rect.left() - 30.0, header.top() + 4.0), vec2(26.0, 26.0));
        let droplet = draw_droplet_button(ui, drop_rect, t);
        menu_popup(&droplet).show(|ui| {
            if let Some(blend) = blend_menu(ui, false, |_| {}) {
                set(app, with(&|s| s.blend = blend), None);
                ui.close();
            }
        });
        let close_button = ui.interact(close_rect, Id::unique((salt, "close")), Sense::click());
        crate::a11y::describe(&close_button, tr("Close"));
        if close_button.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(5), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(close_rect.center(), Vec2::splat(16.0)), Icon::Close, t.icon);
        if close_button.clicked() {
            set_state(app, kind, None);
        }
        crate::panels::right::effects::header_switch(app, ui, t, &ids, i, &effect, header, trigger_end, row_rect);
        ui.add_space(4.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, t.border));
        ui.add_space(13.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            label_at(ui, t, tr("Position"), 0.0);
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let ed = crate::variable_ui::number_field(
                app,
                ui,
                &ids,
                &format!("effects/{i}/offsetX"),
                Id::unique((salt, "x", i)),
                Lead::Text("X"),
                Some(shadow.offset.0),
                "",
                w,
            );
            edit(app, ed, &|s, v| s.offset.0 = v);
        });
        ui.add_space(8.0);
        flush_block(ui, ROW_H, 0.0, |ui| {
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let ed = crate::variable_ui::number_field(
                app,
                ui,
                &ids,
                &format!("effects/{i}/offsetY"),
                Id::unique((salt, "y", i)),
                Lead::Text("Y"),
                Some(shadow.offset.1),
                "",
                w,
            );
            edit(app, ed, &|s, v| s.offset.1 = v);
        });
        ui.add_space(8.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            label_at(ui, t, tr("Blur"), 0.0);
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let ed = crate::variable_ui::number_field(
                app,
                ui,
                &ids,
                &format!("effects/{i}/radius"),
                Id::unique((salt, "radius", i)),
                Lead::Icon(Icon::DotsGrid, tr("Blur")),
                Some(shadow.radius),
                "",
                w,
            );
            edit(app, ed, &|s, v| s.radius = v.max(0.0));
        });
        ui.add_space(8.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            label_at(ui, t, tr("Spread"), 0.0);
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let ed = crate::variable_ui::number_field(
                app,
                ui,
                &ids,
                &format!("effects/{i}/spread"),
                Id::unique((salt, "spread", i)),
                Lead::Icon(Icon::Spread, tr("Spread")),
                Some(shadow.spread),
                "",
                w,
            );
            edit(app, ed, &|s, v| s.spread = v);
        });
        ui.add_space(8.0);

        flush_block(ui, ROW_H, 0.0, |ui| {
            label_at(ui, t, tr("Color"), 0.0);
            ui.add_space(LABEL_W);
            let w = ui.available_width();
            let field = format!("effects/{i}/color");
            crate::variable_ui::bindable(app, ui, &ids, &field, VariableType::Color, w, |app, ui| {
                color_row(app, ui, t, &ids, i, kind.salt(), shadow.color, &edit_color);
            });
        });
        ui.add_space(15.0);
    });
    put_pop(app, kind, pop);
    set_shown(app, kind, true);
    if closed {
        close(app, kind);
    }
}

type ColorEdit<'a> = dyn Fn(&mut FigApp, Color, Option<bool>) + 'a;

/// The reference's colour field: one well holding the swatch, the hex and the opacity (with the
/// divider between them).
fn color_row(app: &mut FigApp, ui: &mut egui::Ui, t: &Tokens, ids: &[NodeId], i: usize, salt: &str, c: Color, edit: &ColorEdit<'_>) {
    let w = ui.available_width();
    let at = ui.cursor().min;
    ui.painter().rect_filled(Rect::from_min_size(at, vec2(w, widgets::FIELD_H)), CornerRadius::same(5), t.field);
    ui.add_space(6.0);
    let (sw, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    widgets::paint_swatch(ui.painter(), sw, &Paint::solid(c));
    let swatch = ui.interact(sw, Id::unique((salt, "swatch", i)), Sense::click());
    crate::a11y::describe(&swatch, tr("Color"));
    app.picker_control(crate::colorpicker::PickerTarget::Effect { ids: ids.to_vec(), index: i, secondary: false }, &swatch);
    ui.add_space(6.0);
    let hex_w = w - 28.0 - 8.0 - ALPHA_W;
    if let Some(h) = widgets::text_field(ui, Id::unique((salt, "hex", i)), &c.to_hex(), hex_w, "")
        && let Some(next) = Color::from_hex_or_name(&h)
    {
        edit(app, next.with_alpha(c.a), None);
    }
    let div_x = ui.cursor().min.x + 3.5;
    ui.painter().rect_filled(Rect::from_min_max(pos2(div_x, at.y + 4.0), pos2(div_x + 1.0, at.y + widgets::FIELD_H - 4.0)), CornerRadius::ZERO, t.border);
    let alpha = |v: f64| c.with_alpha((v / 100.0).clamp(0.0, 1.0) as f32);
    match widgets::number_field(ui, Id::unique((salt, "alpha", i)), Lead::None, Some((f64::from(c.a) * 100.0).round()), "%", ALPHA_W, 1.0) {
        Some(FieldEdit::Set(v)) => edit(app, alpha(v), None),
        Some(FieldEdit::Scrub(v, done)) => edit(app, alpha(v), Some(done)),
        None => {}
    }
}

/// The droplet button: the effect's blend mode menu, like Figma's.
fn draw_droplet_button(ui: &mut egui::Ui, rect: Rect, t: &Tokens) -> egui::Response {
    let resp = ui.interact(rect, Id::unique("shadow_blend"), Sense::click());
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

/// The panel's fill (the popover's: the menu tone in the dark theme, the panel tone in the light).
fn fill_of(t: &Tokens) -> Color32 {
    if t.dark { t.menu } else { t.panel }
}

/// The shadow's mark: a rounded square whose shadow falls outside (a drop shadow) or lies along
/// the inside of its top edge (an inner shadow).
fn draw_shadow_mark(painter: &egui::Painter, rect: Rect, kind: Kind, ink: Color32, fill: Color32) {
    let body = Rect::from_min_size(rect.min + vec2(1.5, 1.0), vec2(11.0, 11.0));
    let shade = if fill.r() > 128 { Color32::from_black_alpha(60) } else { Color32::from_white_alpha(60) };
    match kind {
        Kind::Drop => {
            painter.rect_filled(body.translate(vec2(0.0, 2.5)), CornerRadius::same(3), shade);
            painter.rect_filled(body, CornerRadius::same(3), fill);
        }
        Kind::Inner => {
            painter.rect_filled(body, CornerRadius::same(3), fill);
            let band = Rect::from_min_max(body.min, pos2(body.max.x, body.top() + 3.5));
            painter.rect_filled(band, CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, shade);
        }
    }
    painter.rect_stroke(body, CornerRadius::same(3), Stroke::new(1.2, ink), egui::StrokeKind::Inside);
}

/// The header's row, inset like the reference (12 pt left).
fn flush_header(ui: &mut egui::Ui, h: f32, add: impl FnOnce(&mut egui::Ui)) {
    flush_at(ui, h, (h - 24.0) / 2.0, 12.0, 8.0, add);
}

/// A body row across the popover, inset 15 pt left and 16 pt right.
fn flush_block(ui: &mut egui::Ui, h: f32, inset_y: f32, add: impl FnOnce(&mut egui::Ui)) {
    flush_at(ui, h, inset_y, 15.0, 16.0, add);
}

fn flush_at(ui: &mut egui::Ui, h: f32, inset_y: f32, left: f32, right: f32, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let inner = Rect::from_min_max(pos2(row.left() + left, row.top() + inset_y), pos2(row.right() - right, row.bottom() - inset_y));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    add(&mut child);
}
