use eframe::egui;

use crate::config::Action;

use super::actions_tab::ActionTabEvents;

pub(super) fn draw_ocr(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    actions: &[Action],
    expected_text: &mut String,
    region: &mut (i32, i32, i32, i32),
    action_on_match: &mut Option<usize>,
    action_on_mismatch: &mut Option<usize>,
    events: &mut ActionTabEvents,
) {
    ui.label("期望文本:");
    ui.text_edit_singleline(expected_text);
    ui.label(format!(
        "区域: ({}, {}) - ({}, {})",
        region.0, region.1, region.2, region.3
    ));
    if ui.button("设置OCR区域").clicked() {
        events.capture_region_for = Some((action_idx, step_idx));
    }
    draw_follow_up_action(
        ui,
        action_idx,
        step_idx,
        "命中动作:",
        action_on_match,
        actions,
    );
    draw_follow_up_action(
        ui,
        action_idx,
        step_idx,
        "未命中动作:",
        action_on_mismatch,
        actions,
    );
}

pub(super) fn draw_points_txt_next_line(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    actions: &[Action],
    action_on_empty: &mut Option<usize>,
) {
    ui.label("读取可执行文件同目录 points.txt 的下一行，并复制到剪贴板");
    draw_follow_up_action(
        ui,
        action_idx,
        step_idx,
        "空行/结束动作:",
        action_on_empty,
        actions,
    );
}

fn draw_follow_up_action(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    label: &str,
    action_index: &mut Option<usize>,
    actions: &[Action],
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let selected_text = selected_action_text(*action_index, actions);
        egui::ComboBox::from_id_salt(format!(
            "follow_up_action_{}_{}_{}",
            action_idx, step_idx, label
        ))
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            ui.selectable_value(action_index, None, "不执行");
            for (idx, action) in actions.iter().enumerate() {
                ui.selectable_value(action_index, Some(idx), &action.name);
            }
        });
    });
}

fn selected_action_text(action_index: Option<usize>, actions: &[Action]) -> String {
    action_index
        .and_then(|idx| actions.get(idx).map(|action| action.name.clone()))
        .unwrap_or_else(|| "不执行".to_string())
}

pub(super) fn draw_ocr_save_multi_line(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    region: &mut (i32, i32, i32, i32),
    filename_prefix: &mut String,
    use_full_pipeline: &mut bool,
    events: &mut ActionTabEvents,
) {
    ui.label("OCR 多行识别，结果按时间戳保存到 <exe_dir>/temp/ocr_<前缀>_<ts>.txt");
    ui.label(format!(
        "区域: ({}, {}) - ({}, {})",
        region.0, region.1, region.2, region.3
    ));
    if ui.button("设置 OCR 区域").clicked() {
        events.capture_region_for = Some((action_idx, step_idx));
    }
    ui.horizontal(|ui| {
        ui.label("文件名前缀:");
        ui.text_edit_singleline(filename_prefix);
    });
    ui.checkbox(use_full_pipeline, "使用全流程 (det+rec)，支持多行；关闭则单行识别");
}
