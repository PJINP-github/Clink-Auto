use eframe::egui;

use crate::config::{Action, HotkeyOverride};

pub(super) fn draw_override_row(
    ui: &mut egui::Ui,
    ov_idx: usize,
    ov: &mut HotkeyOverride,
    actions: &[Action],
    listen_override: &mut Option<usize>,
    remove: &mut Vec<usize>,
) {
    ui.label("按键:");
    ui.add(
        egui::TextEdit::singleline(&mut ov.hotkey)
            .desired_width(72.0)
            .char_limit(6),
    );
    if ui.button("监听").clicked() {
        *listen_override = Some(ov_idx);
    }
    ui.label("→ 动作:");
    draw_action_picker(ui, ov_idx, ov, actions);
    if ui.button("删除").clicked() {
        remove.push(ov_idx);
    }
}

fn draw_action_picker(
    ui: &mut egui::Ui,
    ov_idx: usize,
    ov: &mut HotkeyOverride,
    actions: &[Action],
) {
    let ov_text = actions
        .get(ov.action_index)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "无".to_string());
    egui::ComboBox::from_id_salt(format!("run_ov_{}", ov_idx))
        .selected_text(&ov_text)
        .show_ui(ui, |ui| {
            for (ai, action) in actions.iter().enumerate() {
                ui.selectable_value(&mut ov.action_index, ai, &action.name);
            }
        });
}
