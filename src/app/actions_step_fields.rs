use eframe::egui;

use crate::config::{ActionStep, CoordinateCategory};

use super::actions_step_fields_basic::{
    draw_category, draw_click, draw_keyboard_sequence, draw_saturation, draw_screen_capture_region,
};
use super::actions_step_fields_ocr::{draw_ocr, draw_ocr_save_multi_line, draw_points_txt_next_line};
use super::actions_tab::ActionTabEvents;

pub(super) fn draw_step_fields(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    actions: &[crate::config::Action],
    step: &mut ActionStep,
    categories: &[CoordinateCategory],
    events: &mut ActionTabEvents,
) {
    match step {
        ActionStep::Click(x, y) => draw_click(ui, action_idx, step_idx, x, y, events),
        ActionStep::RightClick(x, y) => draw_click(ui, action_idx, step_idx, x, y, events),
        ActionStep::SaturationClick {
            rows,
            clicks_per_row,
            interval_ms,
            region,
        } => draw_saturation(
            ui,
            action_idx,
            step_idx,
            rows,
            clicks_per_row,
            interval_ms,
            region,
            events,
        ),
        ActionStep::Delay(ms) => {
            ui.add(egui::DragValue::new(ms).range(0..=60000));
            ui.label("ms");
        }
        ActionStep::ClickCategory(name) => {
            draw_category(ui, action_idx, step_idx, name, categories)
        }
        ActionStep::ScreenCapture => {
            ui.label("截取全屏并保存到 images/");
        }
        ActionStep::ScreenCaptureRegion { region } => {
            draw_screen_capture_region(ui, action_idx, step_idx, region, events);
        }
        ActionStep::Paste => {
            ui.label("粘贴剪贴板内容 (Ctrl+V)");
        }
        ActionStep::KeyboardSequence(sequence) => draw_keyboard_sequence(ui, sequence),
        ActionStep::OcrCheck {
            expected_text,
            region,
            action_on_match,
            action_on_mismatch,
        } => draw_ocr(
            ui,
            action_idx,
            step_idx,
            actions,
            expected_text,
            region,
            action_on_match,
            action_on_mismatch,
            events,
        ),
        ActionStep::PointsTxtNextLine { action_on_empty } => {
            draw_points_txt_next_line(ui, action_idx, step_idx, actions, action_on_empty)
        }
        ActionStep::Patch {
            patch_id,
            action_id,
        } => {
            ui.label(format!("补丁: {} / {}", patch_id, action_id));
        }
        ActionStep::HtmlToXlsx => {
            ui.label("扫描 <exe_dir>/temp/*.html，每个 <table> 转为同目录同名 .xlsx 的一个 sheet");
        }
        ActionStep::OcrSaveMultiLine {
            region,
            filename_prefix,
            use_full_pipeline,
        } => draw_ocr_save_multi_line(
            ui,
            action_idx,
            step_idx,
            region,
            filename_prefix,
            use_full_pipeline,
            events,
        ),
        ActionStep::CopyClipboard => {}
    }
}
