//! The Layer blur and Background blur popovers: uniform radius, or a progressive ramp between a
//! start and an end radius. They float beside the Effects row like Figma's, and every edit goes
//! through `effect.set` so scrubs stay one undo step.

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};
use figcraft_doc::{Blur, BlurType, Effect, EffectKind, MAX_BLUR_RADIUS, NodeId};
use serde_json::json;

use super::transform3d::label_at;
use super::{FigApp, Tokens};
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::popover::Popover;
use crate::theme::{regular, semibold};
use crate::widgets::{FieldEdit, Lead};

const WIDTH: f32 = 238.0;
const LABEL_W: f32 = 71.0;
const ROW_H: f32 = 24.0;

/// Which blur effect a popover edits — they share one panel, apart from the mark and the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Layer,
    Background,
}

impl Kind {
    /// The effect a row of `kind` holds, if it is one of the two blurs.
    pub(crate) fn of(kind: &EffectKind) -> Option<Self> {
        match kind {
            EffectKind::LayerBlur(_) => Some(Self::Layer),
            EffectKind::BackgroundBlur(_) => Some(Self::Background),
            _ => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Layer => "Layer blur",
            Self::Background => "Background blur",
        }
    }

    /// The prefix of the panel's widget IDs and of its remembered `Popover`.
    fn salt(self) -> &'static str {
        match self {
            Self::Layer => "layer_blur",
            Self::Background => "bg_blur",
        }
    }

    /// The ID of the panel's egui area.
    fn panel_id(self) -> &'static str {
        match self {
            Self::Layer => "layer_blur_popover",
            Self::Background => "bg_blur_popover",
        }
    }

    fn blur<'a>(self, kind: &'a EffectKind) -> Option<&'a Blur> {
        match (self, kind) {
            (Self::Layer, EffectKind::LayerBlur(b)) | (Self::Background, EffectKind::BackgroundBlur(b)) => Some(b),
            _ => None,
        }
    }

    fn blur_mut<'a>(self, kind: &'a mut EffectKind) -> Option<&'a mut Blur> {
        match (self, kind) {
            (Self::Layer, EffectKind::LayerBlur(b)) | (Self::Background, EffectKind::BackgroundBlur(b)) => Some(b),
            _ => None,
        }
    }
}

/// The popover showing for `kind`: the layer it edits, its effect's index and the row it opened at.
pub(crate) fn state(app: &FigApp, kind: Kind) -> Option<(NodeId, usize, Rect)> {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover,
        Kind::Background => app.ui.bg_blur_popover,
    }
}

fn set_state(app: &mut FigApp, kind: Kind, at: Option<(NodeId, usize, Rect)>) {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover = at,
        Kind::Background => app.ui.bg_blur_popover = at,
    }
}

fn shown(app: &FigApp, kind: Kind) -> bool {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover_shown,
        Kind::Background => app.ui.bg_blur_popover_shown,
    }
}

fn set_shown(app: &mut FigApp, kind: Kind, shown: bool) {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover_shown = shown,
        Kind::Background => app.ui.bg_blur_popover_shown = shown,
    }
}

fn take_pop(app: &mut FigApp, kind: Kind) -> Option<Popover> {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover_pop.take(),
        Kind::Background => app.ui.bg_blur_popover_pop.take(),
    }
}

fn put_pop(app: &mut FigApp, kind: Kind, pop: Popover) {
    match kind {
        Kind::Layer => app.ui.layer_blur_popover_pop = Some(pop),
        Kind::Background => app.ui.bg_blur_popover_pop = Some(pop),
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
    let Some(blur) = kind.blur(&effect.kind).map(Blur::sanitized).filter(|_| effect.visible) else {
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
    let with = |f: &dyn Fn(&mut Blur)| {
        let mut e2 = effect.clone();
        if let Some(b) = kind.blur_mut(&mut e2.kind) {
            f(b);
        }
        e2
    };
    let edit = |app: &mut FigApp, ed: Option<FieldEdit>, f: &dyn Fn(&mut Blur, f64)| {
        let Some(ed) = ed else { return };
        let (v, scrub) = match ed {
            FieldEdit::Set(v) => (v, None),
            FieldEdit::Scrub(v, done) => (v, Some(done)),
        };
        set(app, with(&|b| f(b, v)), scrub);
    };

    let salt = kind.salt();
    let screen_h = ctx.content_rect().height();
    let expected_h = if blur.is_progressive() { 158.0 } else { 126.0 };
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(row_rect.left());
    let at = Pos2::new((panel_left - WIDTH - 8.0).max(8.0), row_rect.top().max(8.0).min((screen_h - expected_h).max(8.0)));
    let mut pop = take_pop(app, kind).unwrap_or_else(|| Popover::with_id(Id::unique(kind.panel_id()), tr(kind.title())).width(WIDTH).pad(0.0).no_header());
    pop = pop.at(at).close_on_outside(shown(app, kind));
    // The ramp's two knobs on the canvas are the popover's too: pressing one drags it.
    let mut hold: Vec<Rect> = vec![row_rect];
    hold.extend(crate::handles::blur_knob_rects(app, id, i));
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 0.0;

        let mut trigger_end = 0.0;
        flush_header(ui, 34.0, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            match kind {
                Kind::Layer => draw_layer_blur_mark(ui.painter(), icon_rect, t.icon_secondary),
                Kind::Background => draw_backdrop_blur_mark(ui.painter(), icon_rect.shrink(1.0), t.icon_secondary),
            }
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
            segmented(app, ui, t, kind, id, blur.blur_type, i, &effect);
        });
        ui.add_space(8.0);

        if blur.is_progressive() {
            flush_block(ui, ROW_H, 0.0, |ui| {
                label_at(ui, t, tr("Start|blur"), 0.0);
                ui.add_space(LABEL_W);
                let w = ui.available_width();
                let ed = crate::variable_ui::number_field(
                    app,
                    ui,
                    &ids,
                    &format!("effects/{i}/startRadius"),
                    Id::unique((salt, "start", i)),
                    Lead::Icon(Icon::DotsGrid, tr("Start|blur")),
                    Some(blur.start_radius),
                    "",
                    w,
                );
                edit(app, ed, &|b, v| b.start_radius = v.clamp(0.0, MAX_BLUR_RADIUS));
            });
            ui.add_space(8.0);
            flush_block(ui, ROW_H, 0.0, |ui| {
                label_at(ui, t, tr("End|blur"), 0.0);
                ui.add_space(LABEL_W);
                let w = ui.available_width();
                let ed = crate::variable_ui::number_field(
                    app,
                    ui,
                    &ids,
                    &format!("effects/{i}/radius"),
                    Id::unique((salt, "end", i)),
                    Lead::Icon(Icon::DotsGrid, tr("End|blur")),
                    Some(blur.radius),
                    "",
                    w,
                );
                edit(app, ed, &|b, v| b.radius = v.clamp(0.0, MAX_BLUR_RADIUS));
            });
        } else {
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
                    Some(blur.radius),
                    "",
                    w,
                );
                edit(app, ed, &|b, v| b.radius = v.clamp(0.0, MAX_BLUR_RADIUS));
            });
        }
        ui.add_space(15.0);
    });
    put_pop(app, kind, pop);
    set_shown(app, kind, true);
    if closed {
        close(app, kind);
    }
}

fn segmented(app: &mut FigApp, ui: &mut egui::Ui, t: &Tokens, kind: Kind, id: NodeId, mode: BlurType, i: usize, effect: &Effect) {
    let width = ui.available_width();
    let (track, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    ui.painter().rect_filled(track, CornerRadius::same(6), t.field);
    let item_w = width / 2.0;
    for (index, m) in BlurType::ALL.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(track.left() + item_w * index as f32, track.top()), vec2(item_w, ROW_H));
        let resp = ui.interact(rect, Id::unique((kind.salt(), "tab", index, i)), Sense::click());
        let on = *m == mode;
        if on {
            ui.painter().rect_filled(rect, CornerRadius::same(6), t.pressed);
            ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(6), t.hover);
        }
        let ink = if on { t.text } else { t.text_secondary };
        let font = if on { semibold(12.0) } else { regular(12.0) };
        let label = match m {
            BlurType::Uniform => tr("Uniform"),
            BlurType::Progressive => tr("Progressive|blur"),
        };
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, ink);
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
        if resp.clicked() && !on {
            let mut e2 = effect.clone();
            if let Some(b) = kind.blur_mut(&mut e2.kind) {
                b.blur_type = *m;
            }
            app.run("effect.set", json!({"ids": [id], "index": i, "effect": e2}));
        }
    }
}

/// The background blur's mark: a rounded frame around a 3×3 dot grid.
fn draw_backdrop_blur_mark(painter: &egui::Painter, rect: Rect, ink: Color32) {
    painter.rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.0, ink), egui::StrokeKind::Inside);
    for row in 0..3 {
        for col in 0..3 {
            let at = rect.min + vec2(4.0 + col as f32 * 3.2, 4.0 + row as f32 * 3.2);
            painter.circle_filled(at, 0.85, ink);
        }
    }
}

/// The layer blur's mark: a 5×5 dot grid, densest and brightest at its middle (no frame).
fn draw_layer_blur_mark(painter: &egui::Painter, rect: Rect, ink: Color32) {
    let step = rect.width() / 5.0;
    for row in 0..5 {
        for col in 0..5 {
            let at = rect.min + vec2(step * (col as f32 + 0.5), step * (row as f32 + 0.5));
            let far = ((col as f32 - 2.0).abs()).max((row as f32 - 2.0).abs());
            let (alpha, r) = match far as u8 {
                0 => (255, step * 0.30),
                1 => (235, step * 0.28),
                _ => (150, step * 0.24),
            };
            painter.circle_filled(at, r, Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), alpha));
        }
    }
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
