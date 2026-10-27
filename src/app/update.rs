use eframe::egui;

use crate::mouse::listen_for_next_key;

use super::ClickAutomationApp;

impl eframe::App for ClickAutomationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        self.drain_action_logs();
        self.patch_host.poll();
        self.handle_language_toggle();
        self.handle_key_capture();
        self.poll_key_sequence_capture();

        let mouse_state = self.mouse_tracker.read_and_clear();
        self.mouse_x = mouse_state.x;
        self.mouse_y = mouse_state.y;

        self.handle_start_pause_keys(&mouse_state);
        self.poll_auto_run();
        self.handle_mouse_capture(&mouse_state);

        // 防止 active_tab 被隐藏导致白屏
        let mut allowed_tabs = Vec::new();
        if self.is_tab_visible("coordinates") { allowed_tabs.push(0); }
        if self.is_tab_visible("actions") { allowed_tabs.push(1); }
        if self.is_tab_visible("run") { allowed_tabs.push(2); }
        if self.is_tab_visible("logs") { allowed_tabs.push(3); }
        if self.is_tab_visible("ocr") { allowed_tabs.push(4); }
        if self.is_tab_visible("settings") { allowed_tabs.push(5); }
        if self.is_tab_visible("patches") {
            for i in 0..self.patch_host.tab_count() {
                allowed_tabs.push(6 + i);
            }
        }
        if !allowed_tabs.contains(&self.active_tab) {
            self.active_tab = *allowed_tabs.first().unwrap_or(&1);
        }

        self.draw_ui(ctx);
        self.execute_pending_action();
        self.save_config();
    }
}

impl ClickAutomationApp {
    fn is_tab_visible(&self, tab_name: &str) -> bool {
        if self.config.visible_tabs.is_empty() {
            return matches!(tab_name, "actions" | "run");
        }
        self.config.visible_tabs.iter().any(|t| t == tab_name)
    }

    fn handle_language_toggle(&mut self) {
        if self.toggle_lang {
            self.is_chinese = !self.is_chinese;
            self.toggle_lang = false;
        }
    }

    fn handle_key_capture(&mut self) {
        if self.capture_key_listening && self.captured_key_name.is_none() {
            self.captured_key_name = listen_for_next_key();
        }
        if let Some(name) = self.captured_key_name.take() {
            if let Some(ai) = self.capture_key_target {
                if let Some(action) = self.config.actions.get_mut(ai) {
                    action.hotkey = name.clone();
                }
            } else if let Some(oi) = self.capture_key_target_override {
                if let Some(override_config) = self.config.run_config.hotkey_overrides.get_mut(oi) {
                    override_config.hotkey = name.clone();
                }
            }
            self.add_log(&format!("已捕获按键: {}", name));
            self.capture_key_listening = false;
            self.capture_key_target = None;
            self.capture_key_target_override = None;
        }
    }

    fn draw_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if self.is_tab_visible("coordinates") {
                    ui.selectable_value(&mut self.active_tab, 0, "坐标");
                }
                if self.is_tab_visible("actions") {
                    ui.selectable_value(&mut self.active_tab, 1, "动作");
                }
                if self.is_tab_visible("run") {
                    ui.selectable_value(&mut self.active_tab, 2, "运行");
                }
                if self.is_tab_visible("logs") {
                    ui.selectable_value(&mut self.active_tab, 3, "日志");
                }
                if self.is_tab_visible("ocr") {
                    ui.selectable_value(&mut self.active_tab, 4, "OCR验证");
                }
                if self.is_tab_visible("settings") {
                    ui.selectable_value(&mut self.active_tab, 5, "设置");
                }
                if self.is_tab_visible("patches") {
                    for patch_index in 0..self.patch_host.tab_count() {
                        if let Some(tab) = self.patch_host.tab_definition(patch_index) {
                            ui.selectable_value(&mut self.active_tab, 6 + patch_index, &tab.title);
                        }
                    }
                }
                ui.separator();
                if ui
                    .button(if self.is_chinese { "English" } else { "中文" })
                    .clicked()
                {
                    self.toggle_lang = true;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            0 => self.draw_coordinates_tab(ui),
            1 => self.draw_actions_tab(ui),
            2 => self.draw_run_tab(ui),
            3 => self.draw_logs_tab(ui),
            4 => self.draw_ocr_tab(ui),
            5 => self.draw_settings_tab(ui),
            index if index >= 6 => self.draw_patch_tab(ui, index - 6),
            _ => {}
        });
    }

    fn execute_pending_action(&mut self) {
        if let Some(action_idx) = self.execute_action_index.take() {
            if let Some(action) = self.config.actions.get(action_idx).cloned() {
                self.start_action_async(action);
            }
        }
    }
}
