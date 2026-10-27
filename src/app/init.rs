use chrono::Local;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use crate::config::Configuration;
use crate::mouse::MouseTracker;
use crate::patches::PatchHost;

use super::ClickAutomationApp;

impl ClickAutomationApp {
    pub fn new() -> Self {
        let config = Configuration::load().unwrap_or_default();
        let running = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let action_busy = Arc::new(AtomicBool::new(false));
        let (action_log_tx, action_log_rx) = mpsc::channel();
        let (patch_host, patch_warnings) = PatchHost::discover();
        std::thread::spawn(crate::ocr::warm_up);

        let mut logs = vec![format!(
            "{} - 应用已启动",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        )];
        logs.extend(
            patch_warnings
                .into_iter()
                .map(|warning| format!("补丁加载警告: {}", warning)),
        );

        Self {
            config,
            patch_host,
            logs,
            active_tab: 2,
            mouse_tracker: MouseTracker::start(),
            mouse_x: 0,
            mouse_y: 0,
            capture_coordinate: false,
            capture_target: None,
            capture_step_target: None,
            capture_key_listening: false,
            capture_key_target: None,
            capture_key_target_override: None,
            captured_key_name: None,
            key_sequence_capture_target: None,
            key_sequence_capture_rx: None,
            key_sequence_capture_started: None,
            capture_region: false,
            capture_region_target: None,
            region_start: None,
            running,
            paused,
            action_busy,
            action_log_tx,
            action_log_rx,
            last_cooldown_start: Instant::now(),
            cooldown_elapsed_ms: 0,
            last_space_toggle: Instant::now(),
            execute_action_index: None,
            log_filter: String::new(),
            ocr_test_expected: String::new(),
            ocr_test_region: (0, 0, 0, 0),
            ocr_test_result: String::new(),
            is_chinese: cfg!(target_os = "linux") || cfg!(windows),
            toggle_lang: false,
        }
    }
}
