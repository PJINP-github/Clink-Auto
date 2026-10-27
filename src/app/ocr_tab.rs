use eframe::egui;

use crate::action_executor::ocr_contains;
use crate::ocr_runner::recognize_region_text;

use super::{ClickAutomationApp, RegionCaptureTarget};

impl ClickAutomationApp {
    pub(super) fn draw_ocr_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("OCR验证");
        ui.horizontal(|ui| {
            ui.label("期望文本:");
            ui.text_edit_singleline(&mut self.ocr_test_expected);
        });
        ui.horizontal(|ui| {
            ui.label(format!(
                "区域: ({}, {}) - ({}, {})",
                self.ocr_test_region.0,
                self.ocr_test_region.1,
                self.ocr_test_region.2,
                self.ocr_test_region.3
            ));
            if ui.button("设置OCR区域").clicked() {
                self.start_region_capture(RegionCaptureTarget::OcrTest);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("开始识别").clicked() {
                self.run_ocr_test();
            }
            if ui.button("清空结果").clicked() {
                self.ocr_test_result.clear();
            }
        });
        self.draw_ocr_capture_status(ui);
        ui.separator();
        ui.label("识别结果:");
        ui.add(
            egui::TextEdit::multiline(&mut self.ocr_test_result)
                .desired_rows(8)
                .desired_width(f32::INFINITY),
        );
    }

    fn run_ocr_test(&mut self) {
        if !valid_region(self.ocr_test_region) {
            self.ocr_test_result = "OCR区域无效，请先设置有效区域".to_string();
            self.add_log("OCR测试失败: 区域无效");
            return;
        }

        match recognize_region_text(self.ocr_test_region) {
            Ok(text) => {
                let matched = self.ocr_test_expected.is_empty()
                    || ocr_contains(&self.ocr_test_expected, &text);
                self.ocr_test_result = format!("{}\n\n匹配期望: {}", text, matched);
                self.add_log(&format!("OCR测试完成，匹配: {}", matched));
            }
            Err(err) => {
                self.ocr_test_result = err.clone();
                self.add_log(&format!("OCR测试失败: {}", err));
            }
        }
    }

    fn draw_ocr_capture_status(&mut self, ui: &mut egui::Ui) {
        if !self.capture_region {
            return;
        }

        ui.separator();
        if self.region_start.is_some() {
            ui.label("区域捕获中...请在桌面右键点击终点");
        } else {
            ui.label("区域捕获中...请在桌面右键点击起点");
        }
        ui.label(format!(
            "当前鼠标位置: ({}, {})",
            self.mouse_x, self.mouse_y
        ));
        if ui.button("取消区域捕获").clicked() {
            self.capture_region = false;
            self.capture_region_target = None;
            self.region_start = None;
        }
    }
}

fn valid_region(region: (i32, i32, i32, i32)) -> bool {
    region.2 > region.0 && region.3 > region.1
}
