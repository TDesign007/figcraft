use super::menu::{add_menu_popup, context_popup, menu_check, menu_item, menu_item_enabled, menu_popup, row, row_context_menu, row_rect, row_target};
use super::{
    Blur, BlurType, Document, Effect, EffectKind, FieldEdit, FigApp, Glass, GridAlign, GridKind, Icon, Id, LayoutGrid, Lead, ListDrag,
    MAX_BLUR_RADIUS, NodeId, Noise, NoiseMode, Paint, Perspective, Rect, Sense, Shadow, StyleSlot, Texture, Transform3d, Ui, VariableType,
    common, icons, json, move_flags, moved_index, pos2, remove_flags, removed_index, style_picker, tr, trf, trn, vec2, widgets,
};
use crate::property_rows::PropertyKind;

/// Draw the effect list with add/style/type/toggle/remove/reorder, open settings, and keep an
/// open picker and settings flags aligned when rows move or are removed.
pub(super) fn effects(app: &mut FigApp, ui: &mut Ui, doc: &Document, sel: &[NodeId]) {
    let ids = sel.to_vec();
    let list = common(doc, sel, |n| n.effects.clone());
    let empty = list.as_ref().is_none_or(|l| l.is_empty());
    let (clicked, header, buttons) =
        widgets::section_header_buttons(ui, tr("Effects"), &[(Icon::Styles, tr("Styles")), (Icon::Plus, tr("Add effect"))], empty);
    if clicked == Some(0) {
        style_picker::open(app, ui, &ids, StyleSlot::Effect);
    }
    // The + lists the effect types (the target app's add menu). Perspective is a frame's camera:
    // the entry only lights up for a frame-like selection, dimmed and inert for any other. A
    // picked 3D transform (or a lit Perspective) opens its popover at once, anchored at the
    // button that added it.
    let frames = crate::selection_cache::every(doc, &ids, |n| n.kind.is_frame_like());
    if let Some(add) = buttons.get(1) {
        // An empty section's header is a + too: its click (the button's response does not see it)
        // toggles the same menu.
        let toggle = add.clicked() || (empty && clicked == Some(1));
        let at = add.rect;
        add_menu_popup(add, toggle).show(|ui| {
            let mut picked = None;
            for (label, ty) in Effect::TYPES {
                if menu_item_enabled(ui, tr(label), "", ty != "perspective" || frames) {
                    picked = Some(ty);
                }
            }
            if let Some(ty) = picked {
                let first = ids.first().copied();
                let before = first.and_then(|id| app.session.doc().ok().and_then(|d| d.node(id).map(|n| n.effects.len())));
                app.run("effect.add", json!({"ids": ids, "type": ty}));
                if let (Some(id), Some(index)) = (first, before) {
                    // The 3D transform and Perspective open their popovers in the same click
                    // (Figma's add menu), anchored at the button that added them.
                    if ty == "transform3d" {
                        app.ui.t3_popover = Some((id, index, at));
                        app.ui.t3_settings_open = false;
                        app.ui.t3_origin_open = false;
                    }
                    if ty == "perspective" {
                        app.ui.persp_popover = Some((id, index, at));
                    }
                }
                ui.close();
            }
        });
    }
    context_popup(&header).show(|ui| {
        if app.session.clipboard.effect.is_some() && menu_item(ui, tr("Paste effect"), "") {
            app.run("effect.paste", json!({"ids": ids}));
            ui.close();
        }
    });
    style_picker::chip(app, ui, doc, &ids, StyleSlot::Effect);
    let Some(list) = list else { return };
    let mut drag = ListDrag::new(ui, "effects");
    let mut removed = None;
    let selected_row = crate::theme::Tokens::get(ui.ctx()).selected_row;
    for (i, e) in list.iter().enumerate().rev() {
        let mut open = effect_open(ui.ctx(), i);
        // A click on the row's background highlights it for Cmd/Ctrl+C (`property_rows`).
        let highlight = crate::property_rows::fill(app, PropertyKind::Effect, i, selected_row);
        let header = row_target(ui, 32.0, highlight, |ui| {
            if icons::button(ui, Icon::Effects, 24.0, open, tr("Effect settings")).clicked() {
                open = !open;
                ui.data_mut(|d| d.insert_temp(Id::unique((FX_OPEN, i)), open));
            }
            let w = ui.available_width() - 60.0;
            let resp = widgets::dropdown(ui, tr(e.label()), w, None);
            if matches!(e.kind, EffectKind::Transform3d(_) | EffectKind::Perspective(_)) {
                // Figma's 3D transform and Perspective: the dropdown opens the effect's popover
                // (the preview and the fields), not the type list.
                if resp.clicked()
                    && let Some(first) = ids.first()
                {
                    if matches!(e.kind, EffectKind::Perspective(_)) {
                        app.ui.persp_popover = match app.ui.persp_popover {
                            Some((open_id, open_i, _)) if open_id == *first && open_i == i => None,
                            _ => Some((*first, i, resp.rect)),
                        };
                    } else {
                        app.ui.t3_popover = match app.ui.t3_popover {
                            Some((open_id, open_i, _)) if open_id == *first && open_i == i => None,
                            _ => Some((*first, i, resp.rect)),
                        };
                    }
                }
            } else {
                // The type menu: Perspective joins it dimmed for a non-frame selection, like the
                // + menu (the camera is meaningless on other layers).
                menu_popup(&resp).show(|ui| {
                    for (l, ty) in Effect::TYPES {
                        if menu_item_enabled(ui, tr(l), "", ty != "perspective" || frames)
                            && let Some(ne) = Effect::of_type(ty)
                        {
                            app.run("effect.set", json!({"ids": ids, "index": i, "effect": ne}));
                            ui.close();
                        }
                    }
                });
            }
            let (toggle, remove) = widgets::tight_end(ui, |ui| {
                (
                    icons::button(ui, if e.visible { Icon::Eye } else { Icon::EyeOff }, 24.0, false, tr("Toggle")).clicked(),
                    icons::button(ui, Icon::Minus, 24.0, false, tr("Remove")).clicked(),
                )
            });
            if toggle {
                let mut e2 = e.clone();
                e2.visible = !e2.visible;
                app.run("effect.set", json!({"ids": ids, "index": i, "effect": e2}));
            }
            if remove && app.run("effect.remove", json!({"ids": ids, "index": i})).is_some() {
                removed = Some(i);
            }
        });
        crate::property_rows::row(app, &header, PropertyKind::Effect, i, &ids);
        let header = header.rect;
        let can_paste = app.session.clipboard.effect.is_some();
        row_context_menu(ui, header, Id::unique(("effect_menu", i))).show(|ui| {
            if menu_item(ui, tr("Copy effect"), "") {
                crate::property_rows::copy(app, PropertyKind::Effect, i, &ids);
                ui.close();
            }
            if can_paste && menu_item(ui, tr("Paste effect"), "") {
                app.run("effect.paste", json!({"ids": ids, "index": i}));
                ui.close();
            }
        });
        if open {
            effect_settings(app, ui, &ids, i, e);
        }
        if drag.row_clicked(ui, i, header, header.with_max_y(ui.cursor().min.y.max(header.max.y)), || tr(e.label()).to_string()) {
            crate::property_rows::click(app, PropertyKind::Effect, i, &ids, header);
        }
    }
    if let Some((from, to)) = drag.finish(ui) {
        app.run("effect.move", json!({"ids": ids, "from": from, "to": to}));
        crate::property_rows::follow(app, PropertyKind::Effect, from, Some(to));
        move_flags(ui, FX_OPEN, list.len(), from, to);
        if let Some(crate::colorpicker::PickerTarget::Effect { index, .. }) = &mut app.ui.picker {
            *index = moved_index(*index, from, to);
        }
    }
    // Open settings and an open colour picker stay with their effects, or go with theirs.
    if let Some(i) = removed {
        crate::property_rows::follow(app, PropertyKind::Effect, i, None);
        remove_flags(ui, FX_OPEN, list.len(), i);
        if let Some(crate::colorpicker::PickerTarget::Effect { index, .. }) = &mut app.ui.picker {
            match removed_index(*index, i) {
                Some(k) => *index = k,
                None => app.ui.picker = None,
            }
        }
    }
}

/// Salt of the flags that keep effect rows' settings open.
const FX_OPEN: &str = "fx_open";

/// Whether effect `i`'s settings are open in the Design panel.
pub(crate) fn effect_open(ctx: &egui::Context, i: usize) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::unique((FX_OPEN, i)))).unwrap_or(false)
}

/// A number field of effect `i` that can be bound to a variable (`effects/<i>/<prop>`).
fn fx_field(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], i: usize, prop: &str, lead: Lead, value: f64, width: f32) -> Option<FieldEdit> {
    fx_field_in(app, ui, ids, i, prop, lead, Some(value), "", width)
}

/// [`fx_field`] showing `value` as given (a glass percentage) with `suffix` after it.
pub(super) fn fx_field_in(
    app: &mut FigApp,
    ui: &mut Ui,
    ids: &[NodeId],
    i: usize,
    prop: &str,
    lead: Lead,
    value: Option<f64>,
    suffix: &str,
    width: f32,
) -> Option<FieldEdit> {
    crate::variable_ui::number_field(app, ui, ids, &format!("effects/{i}/{prop}"), Id::unique(("fx", prop, i)), lead, value, suffix, width)
}

/// Draw type-specific effect fields for shadows, blurs, noise, texture and glass, using live
/// scrub edits that stay one undo step and bindable colour fields.
fn effect_settings(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], i: usize, e: &Effect) {
    let half = widgets::col_width(ui.available_width());
    let set = |app: &mut FigApp, e2: Effect, scrub: Option<bool>| {
        let p = json!({"ids": ids, "index": i, "effect": e2});
        match scrub {
            None => {
                app.run("effect.set", p);
            }
            Some(done) => {
                let _ = app.session.begin_interaction("Effect");
                let _ = app.session.execute("effect.set", &p);
                if done {
                    let _ = app.session.commit_interaction();
                }
            }
        }
    };
    let edit = |app: &mut FigApp, ed: Option<FieldEdit>, f: &dyn Fn(f64) -> Effect| match ed {
        Some(FieldEdit::Set(v)) => set(app, f(v), None),
        Some(FieldEdit::Scrub(v, d)) => set(app, f(v), Some(d)),
        None => {}
    };
    // A new colour for the effect (`secondary`: a duo noise's second colour).
    let edit_color = |app: &mut FigApp, secondary: bool, c: figcraft_color::Color, scrub: Option<bool>| {
        let mut e2 = e.clone();
        if let Some(slot) = e2.color_mut(secondary) {
            *slot = c;
        }
        set(app, e2, scrub);
    };
    match &e.kind {
        EffectKind::DropShadow(s) | EffectKind::InnerShadow(s) => {
            let inner = matches!(e.kind, EffectKind::InnerShadow(_));
            let mk = |s2: Shadow| Effect { kind: if inner { EffectKind::InnerShadow(s2) } else { EffectKind::DropShadow(s2) }, visible: e.visible };
            row(ui, 32.0, |ui| {
                let ed = fx_field(app, ui, ids, i, "offsetX", Lead::Text("X"), s.offset.0, half);
                edit(app, ed, &|v| mk(Shadow { offset: (v, s.offset.1), ..s.clone() }));
                let ed = fx_field(app, ui, ids, i, "offsetY", Lead::Text("Y"), s.offset.1, half);
                edit(app, ed, &|v| mk(Shadow { offset: (s.offset.0, v), ..s.clone() }));
            });
            row(ui, 32.0, |ui| {
                let ed = fx_field(app, ui, ids, i, "radius", Lead::Text(tr("Blur")), s.radius, half);
                edit(app, ed, &|v| mk(Shadow { radius: v.max(0.0), ..s.clone() }));
                let ed = fx_field(app, ui, ids, i, "spread", Lead::Text(tr("Spread")), s.spread, half);
                edit(app, ed, &|v| mk(Shadow { spread: v, ..s.clone() }));
            });
            // The colour: a bound variable's chip, else swatch, hex and alpha with a variable button.
            let color_field = format!("effects/{i}/color");
            row(ui, 32.0, |ui| {
                crate::variable_ui::bindable(app, ui, ids, &color_field, VariableType::Color, ui.available_width(), |app, ui| {
                    fx_color(app, ui, ids, i, e, false, &edit_color)
                });
            });
            if !inner {
                row(ui, 32.0, |ui| {
                    if widgets::checkbox(ui, s.show_behind, tr("Show behind transparent areas")) {
                        set(app, mk(Shadow { show_behind: !s.show_behind, ..s.clone() }), None);
                    }
                });
            }
        }
        EffectKind::LayerBlur(b) | EffectKind::BackgroundBlur(b) => {
            let with = |f: &dyn Fn(&mut Blur)| {
                let mut e2 = e.clone();
                if let Some(b2) = e2.blur_mut() {
                    f(b2);
                }
                e2
            };
            row(ui, 32.0, |ui| {
                let resp = widgets::dropdown(ui, tr(b.blur_type.label()), ui.available_width(), None);
                menu_popup(&resp).show(|ui| {
                    for t in BlurType::ALL {
                        if menu_item(ui, tr(t.label()), "") {
                            set(app, with(&|b2| b2.blur_type = t), None);
                            ui.close();
                        }
                    }
                });
            });
            row(ui, 32.0, |ui| {
                if b.is_progressive() {
                    // The ramp's start and end points are dragged on the canvas.
                    let ed = fx_field(app, ui, ids, i, "startRadius", Lead::Text(tr("Start")), b.start_radius, half);
                    edit(app, ed, &|v| with(&|b2| b2.start_radius = v.clamp(0.0, MAX_BLUR_RADIUS)));
                    let ed = fx_field(app, ui, ids, i, "radius", Lead::Text(tr("End")), b.radius, half);
                    edit(app, ed, &|v| with(&|b2| b2.radius = v.clamp(0.0, MAX_BLUR_RADIUS)));
                } else {
                    let ed = fx_field(app, ui, ids, i, "radius", Lead::Text(tr("Blur")), b.radius, ui.available_width());
                    edit(app, ed, &|v| with(&|b2| b2.radius = v.clamp(0.0, MAX_BLUR_RADIUS)));
                }
            });
        }
        EffectKind::Noise(nz) => {
            let with = |f: &dyn Fn(&mut Noise)| {
                tweaked(e, |k| {
                    if let EffectKind::Noise(n) = k {
                        f(n);
                    }
                })
            };
            row(ui, 32.0, |ui| {
                let resp = widgets::dropdown(ui, tr(nz.mode.label()), ui.available_width(), None);
                menu_popup(&resp).show(|ui| {
                    for m in NoiseMode::ALL {
                        if menu_item(ui, tr(m.label()), "") {
                            set(app, with(&|n| n.mode = m), None);
                            ui.close();
                        }
                    }
                });
            });
            row(ui, 32.0, |ui| {
                let ed = widgets::number_field(ui, Id::unique(("fx_ns", i)), Lead::Text(tr("Size")), Some(nz.size), "", half, 0.1);
                edit(app, ed, &|v| {
                    with(&|n| {
                        n.size = v.clamp(0.01, 1000.0);
                        n.size_y = None;
                    })
                });
                let ed = widgets::number_field(
                    ui,
                    Id::unique(("fx_nd", i)),
                    Lead::Text(tr("Density")),
                    Some((nz.density * 100.0).round()),
                    "%",
                    half,
                    1.0,
                );
                edit(app, ed, &|v| with(&|n| n.density = (v / 100.0).clamp(0.0, 1.0)));
            });
            match nz.mode {
                NoiseMode::Mono => row(ui, 32.0, |ui| fx_color(app, ui, ids, i, e, false, &edit_color)),
                NoiseMode::Duo => {
                    row(ui, 32.0, |ui| fx_color(app, ui, ids, i, e, false, &edit_color));
                    row(ui, 32.0, |ui| fx_color(app, ui, ids, i, e, true, &edit_color));
                }
                NoiseMode::Multi => row(ui, 32.0, |ui| {
                    let op = Some((f64::from(nz.opacity) * 100.0).round());
                    let ed = widgets::number_field(ui, Id::unique(("fx_no", i)), Lead::Text(tr("Opacity")), op, "%", half, 1.0);
                    edit(app, ed, &|v| with(&|n| n.opacity = (v / 100.0).clamp(0.0, 1.0) as f32));
                }),
            }
        }
        EffectKind::Texture(tx) => {
            let with = |f: &dyn Fn(&mut Texture)| {
                tweaked(e, |k| {
                    if let EffectKind::Texture(t) = k {
                        f(t);
                    }
                })
            };
            row(ui, 32.0, |ui| {
                let ed = widgets::number_field(ui, Id::unique(("fx_ts", i)), Lead::Text(tr("Size")), Some(tx.size), "", half, 0.1);
                edit(app, ed, &|v| {
                    with(&|t| {
                        t.size = v.clamp(0.01, 1000.0);
                        t.size_y = None;
                    })
                });
                let ed = widgets::number_field(ui, Id::unique(("fx_tr", i)), Lead::Text(tr("Radius")), Some(tx.radius), "", half, 1.0);
                edit(app, ed, &|v| with(&|t| t.radius = v.max(0.0)));
            });
            row(ui, 32.0, |ui| {
                if widgets::checkbox(ui, tx.clip_to_shape, tr("Clip to shape")) {
                    set(app, with(&|t| t.clip_to_shape = !tx.clip_to_shape), None);
                }
            });
        }
        EffectKind::Glass(g) => {
            let with = |f: &dyn Fn(&mut Glass)| {
                tweaked(e, |k| {
                    if let EffectKind::Glass(gl) = k {
                        f(gl);
                    }
                })
            };
            // Each field takes a variable (`effects/<i>/<field>`), read as the field shows it:
            // percentages for the 0–1 values, and the frost as `radius` (see `Glass::set_bound`).
            let pct = |v: f64| Some((v * 100.0).round());
            let unit = |v: f64| (v / 100.0).clamp(0.0, 1.0);
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "lightAngle", Lead::Text(tr("Light|glass")), Some(g.light_angle.round()), "°", half);
                edit(app, ed, &|v| with(&|gl| gl.light_angle = v.clamp(-180.0, 180.0)));
                let ed = fx_field_in(app, ui, ids, i, "lightIntensity", Lead::Text(tr("Intensity")), pct(g.light_intensity), "%", half);
                edit(app, ed, &|v| with(&|gl| gl.light_intensity = unit(v)));
            });
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "refraction", Lead::Text(tr("Refraction")), pct(g.refraction), "%", half);
                edit(app, ed, &|v| with(&|gl| gl.refraction = unit(v)));
                let ed = fx_field_in(app, ui, ids, i, "depth", Lead::Text(tr("Depth")), Some(g.depth), "", half);
                edit(app, ed, &|v| with(&|gl| gl.depth = v.clamp(1.0, 1000.0)));
            });
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "dispersion", Lead::Text(tr("Dispersion")), pct(g.dispersion), "%", half);
                edit(app, ed, &|v| with(&|gl| gl.dispersion = unit(v)));
                let ed = fx_field_in(app, ui, ids, i, "radius", Lead::Text(tr("Frost")), Some(g.sanitized().frost), "", half);
                edit(app, ed, &|v| with(&|gl| gl.frost = v.clamp(0.0, Glass::MAX_FROST)));
            });
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "splay", Lead::Text(tr("Splay")), pct(g.splay), "%", half);
                edit(app, ed, &|v| with(&|gl| gl.splay = unit(v)));
            });
        }
        EffectKind::Transform3d(t) => {
            let with = |f: &dyn Fn(&mut Transform3d)| {
                tweaked(e, |k| {
                    if let EffectKind::Transform3d(t2) = k {
                        f(t2);
                    }
                })
            };
            // Figma's 3D transform settings: the two turns in degrees and the depth along the
            // turned plane's normal.
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "rotateX", Lead::Text("X"), Some(t.rotate_x), "°", half);
                edit(app, ed, &|v| with(&|t2: &mut Transform3d| t2.rotate_x = v.clamp(-360.0, 360.0)));
                let ed = fx_field_in(app, ui, ids, i, "rotateY", Lead::Text("Y"), Some(t.rotate_y), "°", half);
                edit(app, ed, &|v| with(&|t2: &mut Transform3d| t2.rotate_y = v.clamp(-360.0, 360.0)));
            });
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "depth", Lead::Text(tr("Depth")), Some(t.depth), "", half);
                edit(app, ed, &|v| with(&|t2: &mut Transform3d| t2.depth = v.clamp(-10_000.0, 10_000.0)));
            });
            row(ui, 32.0, |ui| {
                // Figma's backface setting: when the plane faces away, the mirrored front shows
                // only with this on.
                if widgets::checkbox(ui, t.backface, tr("Backface")) {
                    set(app, with(&|t2: &mut Transform3d| t2.backface = !t2.backface), None);
                }
            });
        }
        EffectKind::Perspective(p) => {
            let with = |f: &dyn Fn(&mut Perspective)| {
                tweaked(e, |k| {
                    if let EffectKind::Perspective(p2) = k {
                        f(p2);
                    }
                })
            };
            row(ui, 32.0, |ui| {
                let ed = fx_field_in(app, ui, ids, i, "distance", Lead::Text(tr("Distance")), Some(p.distance), "", half);
                edit(app, ed, &|v| with(&|p2: &mut Perspective| p2.distance = v.clamp(1.0, 100_000.0)));
            });
        }
    }
}

/// Applies a new colour to an effect: (app, secondary colour, colour, scrubbing: `Some(done)`).
type ColorEdit<'a> = dyn Fn(&mut FigApp, bool, figcraft_color::Color, Option<bool>) + 'a;

/// `e` with `f` applied to its kind.
fn tweaked(e: &Effect, f: impl FnOnce(&mut EffectKind)) -> Effect {
    let mut e2 = e.clone();
    f(&mut e2.kind);
    e2
}

/// The swatch (opens the colour picker), hex and opacity of effect `i`'s colour (`secondary`: a
/// duo noise's second colour), in the current row; `edit` applies a new one.
fn fx_color(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], i: usize, e: &Effect, secondary: bool, edit: &ColorEdit<'_>) {
    let Some(c) = e.color(secondary) else { return };
    let sw = widgets::swatch(ui, &Paint::solid(c), 16.0);
    app.picker_control(crate::colorpicker::PickerTarget::Effect { ids: ids.to_vec(), index: i, secondary }, &sw);
    if let Some(h) = widgets::text_field(ui, Id::unique(("fx_c", i, secondary)), &c.to_hex(), 80.0, "")
        && let Some(nc) = figcraft_color::Color::from_hex_or_name(&h)
    {
        edit(app, secondary, nc.with_alpha(c.a), None);
    }
    let alpha = |v: f64| c.with_alpha((v / 100.0).clamp(0.0, 1.0) as f32);
    match widgets::number_field(ui, Id::unique(("fx_a", i, secondary)), Lead::None, Some((f64::from(c.a) * 100.0).round()), "%", 56.0, 1.0) {
        Some(FieldEdit::Set(v)) => edit(app, secondary, alpha(v), None),
        Some(FieldEdit::Scrub(v, done)) => edit(app, secondary, alpha(v), Some(done)),
        None => {}
    }
}

/// Draw the layout-grid list with add/style/type/toggle/remove/reorder, the settings popover
/// (the row's grid icon toggles it) closing on another selection, on a press outside it, or
/// with the grid it edits.
pub(super) fn grids(app: &mut FigApp, ui: &mut Ui, doc: &Document, sel: &[NodeId]) {
    let ids = sel.to_vec();
    // The popover edits the selection it opened on: another selection closes it (Figma's).
    if let Some((open_ids, _, _)) = &app.ui.grid_popover
        && open_ids != &ids
    {
        app.ui.grid_popover = None;
    }
    let list = common(doc, sel, |n| n.grids.clone());
    let empty = list.as_ref().is_none_or(|l| l.is_empty());
    if let Some(i) = widgets::section_header(ui, tr("Layout grid"), &[(Icon::Styles, tr("Styles")), (Icon::Plus, tr("Add layout grid"))], empty) {
        if i == 1 {
            app.run("grid.add", json!({"ids": ids}));
        } else {
            style_picker::open(app, ui, &ids, StyleSlot::Grid);
        }
    }
    style_picker::chip(app, ui, doc, &ids, StyleSlot::Grid);
    let Some(list) = list else { return };
    // The last grid first, like paints and effects; rows drag to reorder.
    let mut drag = ListDrag::new(ui, "grids");
    for (i, g) in list.iter().enumerate().rev() {
        // The grid icon is lit while this grid's settings popover is open.
        let open = app.ui.grid_popover.as_ref().is_some_and(|(_, j, _)| *j == i);
        let header = row_rect(ui, 32.0, |ui| {
            let grid_btn = icons::button(ui, Icon::Grid, 24.0, open, tr("Layout grid settings"));
            if grid_btn.clicked() {
                // The settings live in a popover beside the row (Figma's), not below it: the
                // icon toggles it, one grid's at a time.
                app.ui.grid_popover = if open { None } else { Some((ids.clone(), i, grid_btn.rect)) };
            }
            let w = ui.available_width() - 60.0;
            let resp = widgets::dropdown(ui, &grid_label(g), w, None);
            menu_popup(&resp).width(220.0).show(|ui| {
                for (l, kind) in [(tr("Grid"), GridKind::Grid), (tr("Columns"), GridKind::Columns), (tr("Rows"), GridKind::Rows)] {
                    if menu_item(ui, l, "") {
                        let fresh = match kind {
                            GridKind::Grid => LayoutGrid::default(),
                            GridKind::Columns => LayoutGrid::columns(),
                            GridKind::Rows => LayoutGrid::rows(),
                        };
                        app.set_props(&ids, json!({"grids": grids_with(&list, i, |x| *x = fresh)}));
                        ui.close();
                    }
                }
            });
            let (toggle, remove) = widgets::tight_end(ui, |ui| {
                (
                    icons::button(ui, if g.visible { Icon::Eye } else { Icon::EyeOff }, 24.0, false, tr("Toggle")).clicked(),
                    icons::button(ui, Icon::Minus, 24.0, false, tr("Remove")).clicked(),
                )
            });
            if toggle {
                app.set_props(&ids, json!({"grids": grids_with(&list, i, |x| x.visible = !x.visible)}));
            }
            if remove {
                app.run("grid.remove", json!({"ids": ids, "index": i}));
            }
        });
        drag.row(ui, i, header, header.with_max_y(ui.cursor().min.y.max(header.max.y)), || grid_label(g));
    }
    if let Some((from, to)) = drag.finish(ui) {
        app.run("grid.move", json!({"ids": ids, "from": from, "to": to}));
    }
}

/// A layout grid's name in its row ("Grid (10)", "5 columns").
fn grid_label(g: &LayoutGrid) -> String {
    match g.kind {
        GridKind::Grid => trf("Grid ({size})", &[("size", &g.size)]),
        GridKind::Columns if auto_count(g) => tr("Auto columns").to_string(),
        GridKind::Rows if auto_count(g) => tr("Auto rows").to_string(),
        GridKind::Columns => trn("{n} columns", g.count.max(1) as usize, &[]),
        GridKind::Rows => trn("{n} rows", g.count.max(1) as usize, &[]),
    }
}

/// Fixed-size columns or rows counted Auto: as many as fit.
fn auto_count(g: &LayoutGrid) -> bool {
    g.count == 0 && g.align != GridAlign::Stretch
}

/// `list` with grid `i` changed by `f`.
fn grids_with(list: &[LayoutGrid], i: usize, f: impl FnOnce(&mut LayoutGrid)) -> Vec<LayoutGrid> {
    let mut gs = list.to_vec();
    if let Some(g) = gs.get_mut(i) {
        f(g);
    }
    gs
}

/// Layout grid `i`'s settings: size (grids); count, type, width or height, margin or offset and
/// gutter (columns and rows); colour. The numbers can be bound to number variables
/// (`layoutGrids/<i>/<size|count|gutter|offset>`).
fn grid_settings(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], list: &[LayoutGrid], i: usize, g: &LayoutGrid) {
    let col = widgets::col_width(ui.available_width());
    let set = |f: &dyn Fn(&mut LayoutGrid)| json!({"ids": ids, "props": {"grids": grids_with(list, i, f)}});
    // One bindable number field, applied with `apply` (a scrub is one undo step).
    let field = |app: &mut FigApp, ui: &mut Ui, name: &str, value: f64, apply: &dyn Fn(&mut LayoutGrid, f64)| {
        let key = format!("layoutGrids/{i}/{name}");
        let e = crate::variable_ui::number_field(app, ui, ids, &key, Id::unique(("grid_field", name, i)), Lead::None, Some(value), "", col);
        app.field_run(e, "node.set", |v| set(&|x| apply(x, v)));
    };
    let size = |x: &mut LayoutGrid, v: f64| x.size = v.max(1.0);
    let offset = |x: &mut LayoutGrid, v: f64| x.offset = v.max(0.0);
    let gutter = |x: &mut LayoutGrid, v: f64| x.gutter = v.max(0.0);
    if g.kind == GridKind::Grid {
        widgets::sublabels(ui, tr("Size"), None);
        row(ui, 32.0, |ui| field(app, ui, "size", g.size, &size));
    } else {
        let cols = g.kind == GridKind::Columns;
        widgets::sublabels(ui, tr("Count"), Some(tr("Type")));
        row(ui, 32.0, |ui| {
            // 0 is Auto for fixed-size columns (rows); a stretched grid has at least one.
            let count = (!auto_count(g)).then(|| f64::from(g.count.max(1)));
            let key = format!("layoutGrids/{i}/count");
            let id = Id::unique(("grid_field", "count", i));
            // A stretched grid has at least one band, so 0 (Auto) isn't a count it can take.
            let e = if g.align == GridAlign::Stretch {
                crate::variable_ui::number_field_or(app, ui, ids, &key, id, count, col, "", Lead::None, "")
            } else {
                crate::variable_ui::number_field_auto(app, ui, ids, &key, id, count, col, Lead::None, "")
            };
            app.field_run(e, "node.set", |v| {
                set(&|x| x.count = if x.align == GridAlign::Stretch { v.round().clamp(1.0, 1000.0) } else { v.round().clamp(0.0, 1000.0) } as u32)
            });
            let names =
                if cols { [tr("Stretch"), tr("Left"), tr("Center"), tr("Right")] } else { [tr("Stretch"), tr("Top"), tr("Center"), tr("Bottom")] };
            let aligns = [GridAlign::Stretch, GridAlign::Min, GridAlign::Center, GridAlign::Max];
            let cur = aligns.iter().position(|a| *a == g.align).unwrap_or(0);
            let resp = widgets::dropdown(ui, names.get(cur).copied().unwrap_or_default(), col, None);
            menu_popup(&resp).show(|ui| {
                for (k, (name, a)) in names.iter().zip(aligns).enumerate() {
                    if menu_check(ui, k == cur, name, "") {
                        // Fixed-size columns need a size of their own.
                        app.run(
                            "node.set",
                            set(&|x| {
                                x.align = a;
                                if a != GridAlign::Stretch && !(x.size.is_finite() && x.size > 0.0) {
                                    x.size = 60.0;
                                }
                                if a == GridAlign::Stretch {
                                    x.count = x.count.max(1);
                                }
                            }),
                        );
                        ui.close();
                    }
                }
            });
        });
        let span = if cols { tr("Width") } else { tr("Height") };
        match g.align {
            GridAlign::Stretch => {
                widgets::sublabels(ui, span, Some(tr("Margin")));
                row(ui, 32.0, |ui| {
                    // The frame, divided by the count: shown as Auto and not editable.
                    ui.add_enabled_ui(false, |ui| {
                        let auto = widgets::Empty::Text(tr("Auto"));
                        widgets::number_field_bindable(ui, Id::unique(("grid_field", "size", i)), Lead::Named(span), None, "", col, 1.0, None, auto)
                    });
                    field(app, ui, "offset", g.offset, &offset);
                });
                widgets::sublabels(ui, tr("Gutter"), None);
                row(ui, 32.0, |ui| field(app, ui, "gutter", g.gutter, &gutter));
            }
            GridAlign::Center => {
                widgets::sublabels(ui, span, Some(tr("Gutter")));
                row(ui, 32.0, |ui| {
                    field(app, ui, "size", g.size, &size);
                    field(app, ui, "gutter", g.gutter, &gutter);
                });
            }
            GridAlign::Min | GridAlign::Max => {
                widgets::sublabels(ui, span, Some(tr("Offset")));
                row(ui, 32.0, |ui| {
                    field(app, ui, "size", g.size, &size);
                    field(app, ui, "offset", g.offset, &offset);
                });
                widgets::sublabels(ui, tr("Gutter"), None);
                row(ui, 32.0, |ui| field(app, ui, "gutter", g.gutter, &gutter));
            }
        }
    }
    // The colour: swatch, hex and alpha.
    widgets::sublabels(ui, tr("Color"), None);
    row(ui, 32.0, |ui| {
        let (sw, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
        widgets::paint_swatch(ui.painter(), sw, &Paint::solid(g.color));
        if let Some(h) = widgets::text_field(ui, Id::unique(("grid_hex", i)), &g.color.to_hex(), 80.0, "")
            && let Some(c) = figcraft_color::Color::from_hex_or_name(&h)
        {
            app.run("node.set", set(&|x| x.color = c.with_alpha(x.color.a)));
        }
        let e = widgets::number_field(ui, Id::unique(("grid_alpha", i)), Lead::None, Some((f64::from(g.color.a) * 100.0).round()), "%", 56.0, 1.0);
        app.field_run(e, "node.set", |v| set(&|x| x.color = x.color.with_alpha((v / 100.0).clamp(0.0, 1.0) as f32)));
    });
}

/// The settings popover's width (the Perspective popover's).
const POPOVER_W: f32 = 250.0;

/// The layout grid's settings popover (Figma's): the row's grid icon opens it beside the row, on
/// the canvas side, and it edits the grid like the panel's own fields do. Its title is the grid's
/// name, which moves with it when the type changes. Clears the state itself when what it edits is
/// gone.
pub(crate) fn grid_popover(app: &mut FigApp, ctx: &egui::Context, ids: &[NodeId], i: usize, anchor: Rect) {
    // It closes with what it edits: the selection, the grid row.
    let Some(list) = app.session.doc().ok().and_then(|doc| common(doc, ids, |n| n.grids.clone())) else {
        app.ui.grid_popover = None;
        app.ui.grid_popover_shown = false;
        return;
    };
    let Some(g) = list.get(i).cloned() else {
        app.ui.grid_popover = None;
        app.ui.grid_popover_shown = false;
        return;
    };
    // Beside the row, on the canvas side (the Perspective popover's placement) — lifted to stay
    // inside the window. The row's press is held out of the outside-close, so the icon's own
    // click can toggle it shut.
    let screen_h = ctx.content_rect().height();
    let panel_left = app.ui.right_panel_rect.map(|r| r.left()).unwrap_or(anchor.left());
    let at = pos2((panel_left - POPOVER_W - 8.0).max(8.0), anchor.top().max(8.0).min((screen_h - 340.0).max(8.0)));
    let mut pop = app
        .ui
        .grid_popover_pop
        .take()
        .unwrap_or_else(|| crate::popover::Popover::with_id(Id::unique("grid_popover"), grid_label(&g)).width(POPOVER_W));
    // The title follows the grid's name (the type dropdown renames it).
    pop.title = grid_label(&g);
    pop = pop.at(at).close_on_outside(app.ui.grid_popover_shown);
    let hold = vec![anchor];
    let closed = pop.show(ctx, crate::popover::Open::At(at), crate::popover::Hold { rects: &hold, escape: false }, |ui| {
        grid_settings(app, ui, ids, &list, i, &g);
    });
    app.ui.grid_popover_pop = Some(pop);
    app.ui.grid_popover_shown = true;
    if closed {
        app.ui.grid_popover = None;
        app.ui.grid_popover_shown = false;
    }
}
