use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::mouse::KeyDetector;

use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub(super) fn toggle_auto_run(&mut self) {
        let running = self.running.load(Ordering::SeqCst);
        if running {
            self.running.store(false, Ordering::SeqCst);
            self.paused.store(false, Ordering::SeqCst);
            self.add_log("自动运行已停止");
        } else {
            self.running.store(true, Ordering::SeqCst);
            self.paused.store(false, Ordering::SeqCst);
            self.last_cooldown_start = Instant::now();
            self.cooldown_elapsed_ms = 0;
            self.add_log(&format!(
                "自动运行已启动，冷却周期: {}ms",
                self.config.run_config.cooldown_ms
            ));
        }
    }

    pub(super) fn toggle_pause(&mut self) {
        let paused = self.paused.load(Ordering::SeqCst);
        self.paused.store(!paused, Ordering::SeqCst);
        self.add_log(if paused {
            "自动运行已继续"
        } else {
            "自动运行已暂停"
        });
    }

    pub(super) fn handle_start_pause_keys(&mut self, ms: &crate::mouse::MouseState) {
        if ms.ctrl_alt_z_pressed {
            self.start_or_resume_with_delay("Ctrl+Alt+Z", 1700);
        }
        if ms.ctrl_m_pressed {
            self.start_or_resume_with_delay("Ctrl+M", 1700);
        }
        if ms.ctrl_alt_space {
            self.start_or_resume_with_delay("Ctrl+Alt+Space", 1700);
        } else if ms.space_pressed {
            self.pause_with_delay(1700);
        }
    }

    fn start_or_resume_with_delay(&mut self, label: &str, delay_ms: u128) {
        if self.last_space_toggle.elapsed().as_millis() < delay_ms {
            return;
        }
        let running = self.running.load(Ordering::SeqCst);
        let paused = self.paused.load(Ordering::SeqCst);
        if !running {
            self.toggle_auto_run();
            self.add_log(&format!("{}: 已启动", label));
        } else if paused {
            self.paused.store(false, Ordering::SeqCst);
            self.add_log(&format!("{}: 已继续", label));
        }
        self.last_space_toggle = Instant::now();
    }

    fn pause_with_delay(&mut self, delay_ms: u128) {
        if self.last_space_toggle.elapsed().as_millis() < delay_ms {
            return;
        }
        let running = self.running.load(Ordering::SeqCst);
        let paused = self.paused.load(Ordering::SeqCst);
        if running && !paused {
            self.paused.store(true, Ordering::SeqCst);
            self.add_log("空格: 已暂停");
            self.last_space_toggle = Instant::now();
        }
    }

    pub(super) fn poll_auto_run(&mut self) {
        if !self.running.load(Ordering::SeqCst) || self.paused.load(Ordering::SeqCst) {
            return;
        }
        self.cooldown_elapsed_ms = self.last_cooldown_start.elapsed().as_millis();
        if let Some(ai) = self.detect_triggered_action() {
            self.run_action_by_index(ai, "快捷键触发动作");
            self.reset_cooldown();
        } else if self.cooldown_elapsed_ms >= self.config.run_config.cooldown_ms as u128 {
            self.run_action_by_index(self.config.run_config.default_action, "执行默认动作");
            self.reset_cooldown();
        }
        std::thread::sleep(Duration::from_millis(0));
    }

    fn detect_triggered_action(&self) -> Option<usize> {
        let detector = KeyDetector::new()?;
        for ov in &self.config.run_config.hotkey_overrides {
            if let Some(kc) = detector.key_name_to_keycode(&ov.hotkey) {
                if detector.is_key_pressed(kc) && ov.action_index < self.config.actions.len() {
                    return Some(ov.action_index);
                }
            }
        }
        self.config
            .actions
            .iter()
            .enumerate()
            .find_map(|(ai, action)| {
                let kc = detector.key_name_to_keycode(&action.hotkey)?;
                (detector.is_key_pressed(kc)).then_some(ai)
            })
    }

    fn run_action_by_index(&mut self, index: usize, prefix: &str) {
        if let Some(action) = self.config.actions.get(index).cloned() {
            self.add_log(&format!("{}: {}", prefix, action.name));
            self.start_action_async(action);
        }
    }

    fn reset_cooldown(&mut self) {
        self.last_cooldown_start = Instant::now();
        self.cooldown_elapsed_ms = 0;
    }
}
