//! Tüm metin giriş kutularında sağ tık menüsü: Kes, Kopyala, Yapıştır, Sil, Tümünü seç.
use crate::app::*;
use crate::i18n_ui::ti;
use eframe::egui::{self, Event, Id, Key, Modifiers, Pos2};

#[derive(Clone, Copy, PartialEq)]
pub enum CtxAct {
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
}

#[derive(Default)]
pub struct CtxMenu {
    pub open: Option<(Id, Pos2)>,
    pub pending: Option<(Id, CtxAct)>,
}

impl App {
    /// Her karede çağrılır: sağ tıkı yakalar, menüyü çizer.
    pub fn text_context_menu(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            let mut found = None;
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if let Some(layer) = ctx.layer_id_at(pos) {
                    found = ctx.viewport(|v| {
                        v.this_pass
                            .widgets
                            .get_layer(layer)
                            .filter(|w| w.rect.contains(pos) && egui::TextEdit::load_state(ctx, w.id).is_some())
                            .min_by(|a, b| (a.rect.width() * a.rect.height()).total_cmp(&(b.rect.width() * b.rect.height())))
                            .map(|w| w.id)
                    });
                    if let Some(id) = found {
                        ctx.memory_mut(|m| m.request_focus(id));
                        self.ctx_menu.open = Some((id, pos));
                    }
                }
            }
            if found.is_none() {
                self.ctx_menu.open = None;
            }
        }
        let Some((id, pos)) = self.ctx_menu.open else { return };
        let has_sel = egui::TextEdit::load_state(ctx, id).and_then(|s| s.cursor.char_range()).map(|r| r.primary.index != r.secondary.index).unwrap_or(false);
        let mut act: Option<CtxAct> = None;
        let area = egui::Area::new(Id::new("text_ctx_menu")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(150.0);
                ui.style_mut().spacing.button_padding = egui::vec2(10.0, 4.0);
                for (a, label, enabled) in [
                    (CtxAct::Cut, ti("Kes"), has_sel),
                    (CtxAct::Copy, ti("Kopyala"), has_sel),
                    (CtxAct::Paste, ti("Yapıştır"), true),
                    (CtxAct::Delete, ti("Sil"), has_sel),
                    (CtxAct::SelectAll, ti("Tümünü seç"), true),
                ] {
                    if ui.add_enabled(enabled, egui::Button::new(label).frame(false).min_size(egui::vec2(140.0, 22.0))).clicked() {
                        act = Some(a);
                    }
                    if a == CtxAct::Paste || a == CtxAct::Delete {
                        ui.separator();
                    }
                }
            });
        });
        let outside = ctx.input(|i| i.pointer.any_pressed()) && !area.response.contains_pointer();
        if act.is_some() {
            self.ctx_menu.pending = act.map(|a| (id, a));
            self.ctx_menu.open = None;
            ctx.request_repaint();
        } else if outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
            // sağ tık başka bir yerdeyse yukarıda yeniden belirlenir; burada yalnızca kapat
            if !ctx.input(|i| i.pointer.secondary_pressed()) {
                self.ctx_menu.open = None;
            }
        }
    }

    /// `eframe::App::raw_input_hook` içinden: seçilen eylemi odaklı kutuya olay olarak iletir.
    pub fn apply_text_action(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let Some((id, act)) = self.ctx_menu.pending.take() else { return };
        ctx.memory_mut(|m| m.request_focus(id));
        let key = |k: Key, mods: Modifiers| {
            [true, false].map(|pressed| Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: mods })
        };
        match act {
            CtxAct::Cut => raw.events.push(Event::Cut),
            CtxAct::Copy => raw.events.push(Event::Copy),
            CtxAct::Paste => {
                if let Some(t) = arboard::Clipboard::new().and_then(|mut c| c.get_text()).ok().filter(|t| !t.is_empty()) {
                    raw.events.push(Event::Paste(t));
                }
            }
            CtxAct::Delete => raw.events.extend(key(Key::Delete, Modifiers::NONE)),
            CtxAct::SelectAll => raw.events.extend(key(Key::A, Modifiers::COMMAND)),
        }
    }
}
