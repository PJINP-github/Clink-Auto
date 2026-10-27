use eframe::egui;

use crate::config::ActionStep;
use crate::patches::PatchActionDefinition;

pub(super) fn draw_step_kind(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    step: &mut ActionStep,
    patch_actions: &[PatchActionDefinition],
) {
    egui::ComboBox::from_id_salt(format!("step_{}_{}", action_idx, step_idx))
        .selected_text(step_label(step))
        .show_ui(ui, |ui| {
            ui.selectable_value(step, ActionStep::Click(0, 0), "点击坐标");
            ui.selectable_value(step, ActionStep::RightClick(0, 0), "右键点击坐标");
            ui.selectable_value(step, saturation_step(), "饱和点击");
            ui.selectable_value(step, ActionStep::Delay(500), "延迟");
            ui.selectable_value(step, ActionStep::CopyClipboard, "复制剪贴板");
            ui.selectable_value(step, ActionStep::ClickCategory(String::new()), "点击分类");
            ui.selectable_value(step, ActionStep::ScreenCapture, "全屏截图");
            ui.selectable_value(
                step,
                ActionStep::ScreenCaptureRegion {
                    region: (0, 0, 0, 0),
                },
                "按区域截图",
            );
            ui.selectable_value(step, ActionStep::Paste, "粘贴");
            ui.selectable_value(
                step,
                ActionStep::KeyboardSequence(String::new()),
                "按键序列",
            );
            ui.selectable_value(step, ocr_check_step(), "OCR 检查");
            ui.selectable_value(
                step,
                ActionStep::PointsTxtNextLine {
                    action_on_empty: None,
                },
                "读取 points.txt",
            );
            ui.selectable_value(step, ActionStep::HtmlToXlsx, "HTML 转 xlsx");
            ui.selectable_value(step, ocr_save_multi_line_step(), "OCR 多行保存");
            for patch in patch_actions {
                ui.selectable_value(
                    step,
                    ActionStep::Patch {
                        patch_id: patch.patch_id.clone(),
                        action_id: patch.action_id.clone(),
                    },
                    patch.display_name(),
                );
            }
        });
}

pub(super) fn step_label(step: &ActionStep) -> &'static str {
    match step {
        ActionStep::Click(_, _) => "点击坐标",
        ActionStep::RightClick(_, _) => "右键点击坐标",
        ActionStep::SaturationClick { .. } => "饱和点击",
        ActionStep::Delay(_) => "延迟",
        ActionStep::CopyClipboard => "复制剪贴板",
        ActionStep::ClickCategory(_) => "点击分类",
        ActionStep::ScreenCapture => "全屏截图",
        ActionStep::ScreenCaptureRegion { .. } => "按区域截图",
        ActionStep::Paste => "粘贴",
        ActionStep::KeyboardSequence(_) => "按键序列",
        ActionStep::OcrCheck { .. } => "OCR 检查",
        ActionStep::PointsTxtNextLine { .. } => "读取 points.txt",
        ActionStep::HtmlToXlsx => "HTML 转 xlsx",
        ActionStep::OcrSaveMultiLine { .. } => "OCR 多行保存",
        ActionStep::Patch { .. } => "补丁动作",
    }
}

fn ocr_save_multi_line_step() -> ActionStep {
    ActionStep::OcrSaveMultiLine {
        region: (0, 0, 0, 0),
        filename_prefix: String::new(),
        use_full_pipeline: true,
    }
}

fn saturation_step() -> ActionStep {
    ActionStep::SaturationClick {
        rows: 1,
        clicks_per_row: 12,
        interval_ms: 1,
        region: (0, 0, 0, 0),
    }
}

fn ocr_check_step() -> ActionStep {
    ActionStep::OcrCheck {
        expected_text: String::new(),
        region: (0, 0, 0, 0),
        action_on_match: None,
        action_on_mismatch: None,
    }
}
