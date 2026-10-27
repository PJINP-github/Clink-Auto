use eframe::egui;

use crate::config::CoordinateCategory;

use super::actions_tab::ActionTabEvents;

pub(super) fn draw_click(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    x: &mut i32,
    y: &mut i32,
    events: &mut ActionTabEvents,
) {
    if ui.button("捕获").clicked() {
        events.capture_step = Some((action_idx, step_idx));
    }
    ui.label(format!("X: {}", x));
    ui.label(format!("Y: {}", y));
}

pub(super) fn draw_saturation(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    rows: &mut u32,
    clicks_per_row: &mut u32,
    interval_ms: &mut u64,
    region: &mut (i32, i32, i32, i32),
    events: &mut ActionTabEvents,
) {
    ui.label("行数:");
    ui.add(egui::DragValue::new(rows).range(1..=100));
    ui.label("每行点击:");
    ui.add(egui::DragValue::new(clicks_per_row).range(1..=1000));
    ui.label("间隔ms:");
    ui.add(egui::DragValue::new(interval_ms).range(0..=10000));
    draw_region_button(ui, action_idx, step_idx, region, "设置区域", events);
}
pub(super) fn draw_screen_capture_region(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    region: &(i32, i32, i32, i32),
    events: &mut ActionTabEvents,
) {
    draw_region_button(ui, action_idx, step_idx, region, "截图区域", events);
}

pub(super) fn draw_category(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    name: &mut String,
    categories: &[CoordinateCategory],
) {
    egui::ComboBox::from_id_salt(format!("cat_{}_{}", action_idx, step_idx))
        .selected_text(name.as_str())
        .show_ui(ui, |ui| {
            for cat in categories {
                ui.selectable_value(name, cat.name.clone(), &cat.name);
            }
        });
}

pub(super) fn draw_keyboard_sequence(ui: &mut egui::Ui, sequence: &mut String) {
    ui.label("序列格式: ctrl+c;500;esc");
    ui.text_edit_singleline(sequence);
}

fn draw_region_button(
    ui: &mut egui::Ui,
    action_idx: usize,
    step_idx: usize,
    region: &(i32, i32, i32, i32),
    button_label: &str,
    events: &mut ActionTabEvents,
) {
    if ui.button(button_label).clicked() {
        events.capture_region_for = Some((action_idx, step_idx));
    }
    ui.label(format!(
        "R:({},{})-({},{})",
        region.0, region.1, region.2, region.3
    ));
}
