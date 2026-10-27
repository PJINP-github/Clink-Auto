use eframe::egui;
use std::sync::atomic::Ordering;

use crate::config::HotkeyOverride;

use super::run_hotkeys::draw_override_row;
use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub(super) fn draw_run_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("自动运行控制");
        ui.horizontal(|ui| {
            ui.label("冷却周期 (ms):");
            ui.add(egui::DragValue::new(&mut self.config.run_config.cooldown_ms).range(0..=60000));
        });
        self.draw_default_action(ui);
        self.draw_hotkey_overrides(ui);
        self.draw_run_controls(ui);
    }

    fn draw_default_action(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("默认动作:");
        let default_text = self
            .config
            .actions
            .get(self.config.run_config.default_action)
            .map(|action| action.name.clone())
            .unwrap_or_else(|| "无".to_string());
        egui::ComboBox::from_id_salt("run_default_action")
            .selected_text(&default_text)
            .show_ui(ui, |ui| {
                for (ai, action) in self.config.actions.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.config.run_config.default_action,
                        ai,
                        &action.name,
                    );
                }
            });
    }

    fn draw_hotkey_overrides(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("全局快捷键覆盖:");
        let mut listen_override = None;
        if ui.button("添加快捷键覆盖").clicked() {
            self.config
                .run_config
                .hotkey_overrides
                .push(HotkeyOverride {
                    hotkey: String::new(),
                    action_index: 0,
                });
        }
        let mut remove = Vec::new();
        for (ov_idx, ov) in self
            .config
            .run_config
            .hotkey_overrides
            .iter_mut()
            .enumerate()
        {
            ui.horizontal(|ui| {
                draw_override_row(
                    ui,
                    ov_idx,
                    ov,
                    &self.config.actions,
                    &mut listen_override,
                    &mut remove,
                )
            });
        }
        for &idx in remove.iter().rev() {
            self.config.run_config.hotkey_overrides.remove(idx);
        }
        if let Some(oi) = listen_override {
            self.capture_key_listening = true;
            self.capture_key_target = None;
            self.capture_key_target_override = Some(oi);
            self.add_log("按下任意键捕获...");
        }
    }

    fn draw_run_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            let running = self.running.load(Ordering::SeqCst);
            let paused = self.paused.load(Ordering::SeqCst);
            if ui.button(if running { "停止" } else { "启动" }).clicked() {
                self.toggle_auto_run();
            }
            if running && ui.button(if paused { "继续" } else { "暂停" }).clicked() {
                self.toggle_pause();
            }
        });
        self.draw_run_status(ui);
    }

    fn draw_run_status(&self, ui: &mut egui::Ui) {
        let running = self.running.load(Ordering::SeqCst);
        let paused = self.paused.load(Ordering::SeqCst);
        ui.label(format!(
            "状态: {}",
            if running {
                if paused {
                    "已暂停"
                } else {
                    "运行中"
                }
            } else {
                "已停止"
            }
        ));
        if running && !paused {
            let cooldown = self.config.run_config.cooldown_ms as u128;
            let remaining = cooldown.saturating_sub(self.cooldown_elapsed_ms);
            ui.label(format!("下次执行剩余: {}ms", remaining));
        }
    }
}
