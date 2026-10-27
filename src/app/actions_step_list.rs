use eframe::egui;

use crate::config::{Action, ActionStep, CoordinateCategory};
use crate::patches::PatchActionDefinition;

use super::actions_step_fields::draw_step_fields;
use super::actions_step_kind::draw_step_kind;
use super::actions_tab::ActionTabEvents;

pub(super) fn draw_steps(
    ui: &mut egui::Ui,
    action_idx: usize,
    actions: &[Action],
    action: &mut Action,
    categories: &[CoordinateCategory],
    patch_actions: &[PatchActionDefinition],
    events: &mut ActionTabEvents,
) {
    ui.label("动作步骤:");
    if ui.button("添加步骤").clicked() {
        action.steps.push(ActionStep::Click(0, 0));
    }

    let mut remove = Vec::new();
    let mut move_up = None;
    let mut move_down = None;
    let total_steps = action.steps.len();

    for (step_idx, step) in action.steps.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            // 上移按钮
            if ui.add_enabled(step_idx > 0, egui::Button::new("⬆").small()).clicked() {
                move_up = Some(step_idx);
            }
            // 下移按钮
            if ui.add_enabled(step_idx + 1 < total_steps, egui::Button::new("⬇").small()).clicked() {
                move_down = Some(step_idx);
            }

            draw_step_kind(ui, action_idx, step_idx, step, patch_actions);
            draw_step_fields(ui, action_idx, step_idx, actions, step, categories, events);
            if matches!(step, ActionStep::KeyboardSequence(_)) && ui.button("监听").clicked()
            {
                events.listen_keyboard_sequence = Some((action_idx, step_idx));
            }
            if ui.button("删除").clicked() {
                remove.push(step_idx);
            }
        });
    }

    // 处理删除
    for &idx in remove.iter().rev() {
        action.steps.remove(idx);
    }
    // 处理上移
    if let Some(idx) = move_up {
        action.steps.swap(idx, idx - 1);
    }
    // 处理下移
    if let Some(idx) = move_down {
        action.steps.swap(idx, idx + 1);
    }

    if ui.button("执行").clicked() {
        events.execute_action = Some(action_idx);
    }
}