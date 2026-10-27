use eframe::egui;

use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub(super) fn draw_patch_tab(&mut self, ui: &mut egui::Ui, tab_index: usize) {
        let Some(definition) = self.patch_host.tab_definition(tab_index).cloned() else {
            ui.heading("补丁 Tab 不存在");
            return;
        };
        let snapshot = self
            .patch_host
            .tab_snapshot(tab_index)
            .cloned()
            .unwrap_or_default();

        ui.heading(&definition.title);
        ui.label(format!(
            "补丁: {} 版本化接口: {} / {}",
            definition.patch_name, definition.patch_id, definition.tab_id
        ));

        if self.patch_host.tab_loading(tab_index) {
            ui.label("正在刷新...");
        }
        if let Some(error) = self.patch_host.tab_error(tab_index) {
            ui.colored_label(egui::Color32::RED, error);
        }

        if !snapshot.summary.is_empty() {
            ui.separator();
            ui.label(&snapshot.summary);
        }

        if !snapshot.missing_lines.is_empty() {
            ui.separator();
            ui.label("clipboard_output.txt 中未找到对应 PNG 的行:");
            egui::ScrollArea::vertical()
                .id_salt(format!("patch_missing_{}", tab_index))
                .max_height(160.0)
                .show(ui, |ui| {
                    for line in &snapshot.missing_lines {
                        ui.label(line);
                    }
                });
        }

        ui.separator();
        ui.label("clipboard_output.txt");
        let mut content = snapshot.content;
        egui::ScrollArea::vertical()
            .id_salt(format!("patch_content_{}", tab_index))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut content)
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .interactive(false),
                );
            });
    }
}
