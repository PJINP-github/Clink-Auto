use eframe::egui;

use crate::config::{Coordinate, CoordinateCategory};

use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub(super) fn draw_coordinates_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("坐标分类");
        if ui.button("添加分类 +").clicked() {
            self.config.coordinate_categories.push(CoordinateCategory {
                name: format!("分类 {}", self.config.coordinate_categories.len() + 1),
                coordinates: vec![],
            });
        }
        let mut capture_target = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut remove_indices = Vec::new();
            for (cat_idx, cat) in self.config.coordinate_categories.iter_mut().enumerate() {
                if draw_category(ui, cat_idx, cat, &mut capture_target) {
                    remove_indices.push(cat_idx);
                }
            }
            for &idx in remove_indices.iter().rev() {
                self.config.coordinate_categories.remove(idx);
            }
        });
        if capture_target.is_some() {
            self.start_coordinate_capture(capture_target, None);
        }
        self.draw_coordinate_capture_status(ui);
        ui.label(format!("鼠标: ({}, {})", self.mouse_x, self.mouse_y));
    }

    fn draw_coordinate_capture_status(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        if self.capture_coordinate {
            ui.label("正在捕获...在桌面右键点击捕获坐标");
            ui.label(format!(
                "当前鼠标位置: ({}, {})",
                self.mouse_x, self.mouse_y
            ));
            if ui.button("取消捕获").clicked() {
                self.cancel_coordinate_capture();
            }
        }
    }
}

fn draw_category(
    ui: &mut egui::Ui,
    cat_idx: usize,
    cat: &mut CoordinateCategory,
    capture_target: &mut Option<(usize, usize)>,
) -> bool {
    let cat_name = cat.name.clone();
    let mut should_remove = false;
    ui.collapsing(&cat_name, |ui| {
        ui.label("分类名称:");
        ui.text_edit_singleline(&mut cat.name);
        if ui.button("添加坐标").clicked() {
            cat.coordinates.push(Coordinate { x: 0, y: 0 });
        }
        draw_coordinates(ui, cat_idx, cat, capture_target);
        if ui.button("删除分类").clicked() {
            should_remove = true;
        }
    });
    should_remove
}

fn draw_coordinates(
    ui: &mut egui::Ui,
    cat_idx: usize,
    cat: &mut CoordinateCategory,
    capture_target: &mut Option<(usize, usize)>,
) {
    let mut remove_indices = Vec::new();
    for (coord_idx, coord) in cat.coordinates.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("坐标 {}:", coord_idx + 1));
            if ui.button("捕获").clicked() {
                *capture_target = Some((cat_idx, coord_idx));
            }
            ui.label(format!("X: {}", coord.x));
            ui.label(format!("Y: {}", coord.y));
            if ui.button("删除").clicked() {
                remove_indices.push(coord_idx);
            }
        });
    }
    for &idx in remove_indices.iter().rev() {
        cat.coordinates.remove(idx);
    }
}
