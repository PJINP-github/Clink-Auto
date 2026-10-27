use eframe::egui;

use crate::config::Configuration;
use crate::patches::PatchHost;

use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub(super) fn draw_logs_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("日志");
        ui.horizontal(|ui| {
            ui.label("过滤:");
            ui.text_edit_singleline(&mut self.log_filter);
            if ui.button("清空").clicked() {
                self.logs.clear();
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            for log in self.logs.iter().rev() {
                if self.log_filter.is_empty() || log.contains(&self.log_filter) {
                    ui.label(log);
                }
            }
        });
    }

    pub(super) fn draw_settings_tab(&mut self, ui: &mut egui::Ui) {
        if ui.button("保存配置").clicked() {
            self.save_config();
            self.add_log("配置已保存");
        }
        if ui.button("加载配置").clicked() {
            match Configuration::load() {
                Ok(config) => {
                    self.config = config;
                    self.add_log("配置已加载");
                }
                Err(e) => self.add_log(&format!("加载配置失败: {}", e)),
            }
        }
        if ui.button("重新加载补丁").clicked() {
            let (patch_host, warnings) = PatchHost::discover();
            self.patch_host = patch_host;
            self.add_log(&format!(
                "已重新加载补丁: {} 个动作, {} 个 Tab",
                self.patch_host.action_definitions().len(),
                self.patch_host.tab_count()
            ));
            for warning in warnings {
                self.add_log(&format!("补丁加载警告: {}", warning));
            }
            if self.active_tab >= 6 + self.patch_host.tab_count() {
                self.active_tab = 2;
            }
        }
    }
}
