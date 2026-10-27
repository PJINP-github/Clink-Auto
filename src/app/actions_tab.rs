use eframe::egui;

use crate::config::Action;

use super::actions_step_list::draw_steps;
use super::{ClickAutomationApp, RegionCaptureTarget};

impl ClickAutomationApp {
    pub(super) fn draw_actions_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("动作");
        if ui.button("添加动作 +").clicked() {
            self.config.actions.push(Action {
                name: format!("动作 {}", self.config.actions.len() + 1),
                hotkey: String::new(),
                steps: vec![],
            });
        }
        let mut events = ActionTabEvents::default();
        let action_choices = self.config.actions.clone();
        let patch_actions = self.patch_host.action_definitions().to_vec();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (action_idx, action) in self.config.actions.iter_mut().enumerate() {
                draw_action(
                    ui,
                    action_idx,
                    &action_choices,
                    action,
                    &self.config.coordinate_categories,
                    &patch_actions,
                    &mut events,
                );
            }
        });
        self.apply_action_tab_events(events);
        self.draw_region_capture_status(ui);
    }

    fn apply_action_tab_events(&mut self, events: ActionTabEvents) {
        if let Some(step) = events.capture_step {
            self.start_coordinate_capture(None, Some(step));
        }
        if let Some(target) = events.capture_region_for {
            self.start_region_capture(RegionCaptureTarget::Step(target.0, target.1));
        }
        if let Some(ai) = events.listen_action_hotkey {
            self.capture_key_listening = true;
            self.capture_key_target = Some(ai);
            self.capture_key_target_override = None;
            self.add_log("按下任意键捕获...");
        }
        if let Some((action_idx, step_idx)) = events.listen_keyboard_sequence {
            self.start_key_sequence_capture(action_idx, step_idx);
        }
        if let Some(ai) = events.execute_action {
            self.execute_action_index = Some(ai);
        }
    }

    fn draw_region_capture_status(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        if self.capture_region {
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
        if let Some(started_at) = self.key_sequence_capture_started {
            let elapsed = started_at.elapsed().as_secs();
            let remaining = crate::mouse::KEY_SEQUENCE_CAPTURE_SECONDS.saturating_sub(elapsed);
            ui.label(format!("按键序列监听中...剩余 {} 秒", remaining));
        }
    }
}

#[derive(Default)]
pub(super) struct ActionTabEvents {
    pub(super) capture_step: Option<(usize, usize)>,
    pub(super) capture_region_for: Option<(usize, usize)>,
    pub(super) listen_action_hotkey: Option<usize>,
    pub(super) listen_keyboard_sequence: Option<(usize, usize)>,
    pub(super) execute_action: Option<usize>,
}

fn draw_action(
    ui: &mut egui::Ui,
    action_idx: usize,
    actions: &[Action],
    action: &mut Action,
    categories: &[crate::config::CoordinateCategory],
    patch_actions: &[crate::patches::PatchActionDefinition],
    events: &mut ActionTabEvents,
) {
    let action_name = action.name.clone();
    ui.collapsing(&action_name, |ui| {
        ui.label("动作名称:");
        ui.text_edit_singleline(&mut action.name);
        ui.label("触发键:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut action.hotkey);
            if ui.button("监听").clicked() {
                events.listen_action_hotkey = Some(action_idx);
            }
        });
        ui.separator();
        draw_steps(
            ui,
            action_idx,
            actions,
            action,
            categories,
            patch_actions,
            events,
        );
    });
}
