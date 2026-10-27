mod actions_step_fields;
mod actions_step_fields_basic;
mod actions_step_fields_ocr;
mod actions_step_kind;
mod actions_step_list;
mod actions_tab;
mod automation;
mod capture;
mod capture_region;
mod coordinates_tab;
mod init;
mod ocr_tab;
mod patches_tab;
mod run_hotkeys;
mod run_tab;
mod settings_tab;
mod update;

use chrono::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::action_executor;
use crate::config::{Action, ActionStep, Configuration};
use crate::mouse::MouseTracker;

#[derive(Clone, Copy)]
pub(super) enum RegionCaptureTarget {
    Step(usize, usize),
    OcrTest,
}

pub struct ClickAutomationApp {
    config: Configuration,
    patch_host: crate::patches::PatchHost,
    logs: Vec<String>,
    active_tab: usize,
    mouse_tracker: MouseTracker,
    mouse_x: i32,
    mouse_y: i32,
    capture_coordinate: bool,
    capture_target: Option<(usize, usize)>,
    capture_step_target: Option<(usize, usize)>,
    capture_key_listening: bool,
    capture_key_target: Option<usize>,
    capture_key_target_override: Option<usize>,
    captured_key_name: Option<String>,
    key_sequence_capture_target: Option<(usize, usize)>,
    key_sequence_capture_rx: Option<Receiver<String>>,
    key_sequence_capture_started: Option<Instant>,
    capture_region: bool,
    capture_region_target: Option<RegionCaptureTarget>,
    region_start: Option<(i32, i32)>,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    action_busy: Arc<AtomicBool>,
    action_log_tx: Sender<String>,
    action_log_rx: Receiver<String>,
    last_cooldown_start: Instant,
    cooldown_elapsed_ms: u128,
    last_space_toggle: Instant,
    execute_action_index: Option<usize>,
    log_filter: String,
    ocr_test_expected: String,
    ocr_test_region: (i32, i32, i32, i32),
    ocr_test_result: String,
    is_chinese: bool,
    toggle_lang: bool,
}

impl ClickAutomationApp {
    fn add_log(&mut self, message: &str) {
        self.logs.push(format!(
            "{} - {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            message
        ));
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.add_log(&format!("保存配置失败: {}", e));
        }
    }

    #[allow(dead_code)]
    fn execute_action(&mut self, action: &Action) {
        self.execute_action_chain(action, 0);
    }

    #[allow(dead_code)]
    fn execute_action_chain(&mut self, action: &Action, depth: usize) {
        const MAX_FOLLOW_UP_ACTIONS: usize = 32;
        if depth >= MAX_FOLLOW_UP_ACTIONS {
            self.add_log("动作链超过32次，已停止以避免循环退出");
            return;
        }

        let mut pending_logs = Vec::new();
        let patches = self.patch_host.registry();
        let next_action = action_executor::execute_action_with_patches(
            action,
            &self.config,
            &patches,
            &self.paused,
            &mut |msg| pending_logs.push(msg),
        );
        for log in pending_logs {
            self.add_log(&log);
        }
        if let Some(action_idx) = next_action {
            if let Some(next) = self.config.actions.get(action_idx).cloned() {
                self.execute_action_chain(&next, depth + 1);
            } else {
                self.add_log(&format!("后续动作索引不存在: {}", action_idx));
            }
        }
    }
}

impl ClickAutomationApp {
    fn start_action_async(&mut self, action: Action) -> bool {
        if self
            .action_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            self.add_log("动作仍在执行，跳过重复触发");
            return false;
        }

        let config = self.config.clone();
        let paused = Arc::clone(&self.paused);
        let busy = Arc::clone(&self.action_busy);
        let log_tx = self.action_log_tx.clone();
        let patches = self.patch_host.registry();
        std::thread::spawn(move || {
            const MAX_FOLLOW_UP_ACTIONS: usize = 32;
            let mut current = Some(action);

            for _ in 0..MAX_FOLLOW_UP_ACTIONS {
                let Some(action) = current.take() else {
                    break;
                };
                let mut send_log = |message| {
                    let _ = log_tx.send(message);
                };
                let next_index = action_executor::execute_action_with_patches(
                    &action,
                    &config,
                    &patches,
                    &paused,
                    &mut send_log,
                );
                current = next_index.and_then(|index| config.actions.get(index).cloned());
            }

            if current.is_some() {
                let _ = log_tx.send("动作链超过最大深度，已停止".to_string());
            }
            busy.store(false, Ordering::SeqCst);
        });
        true
    }

    fn drain_action_logs(&mut self) {
        while let Ok(message) = self.action_log_rx.try_recv() {
            self.add_log(&message);
        }
    }

    fn start_key_sequence_capture(&mut self, action_idx: usize, step_idx: usize) {
        if self.key_sequence_capture_rx.is_some() {
            self.add_log("按键序列监听仍在进行中");
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.key_sequence_capture_target = Some((action_idx, step_idx));
        self.key_sequence_capture_rx = Some(rx);
        self.key_sequence_capture_started = Some(Instant::now());
        std::thread::spawn(move || {
            let sequence = crate::mouse::listen_for_key_sequence(Duration::from_secs(
                crate::mouse::KEY_SEQUENCE_CAPTURE_SECONDS,
            ));
            let _ = tx.send(sequence);
        });
        self.add_log(&format!(
            "开始监听按键序列，{}秒内输入需要复现的按键",
            crate::mouse::KEY_SEQUENCE_CAPTURE_SECONDS
        ));
    }

    fn poll_key_sequence_capture(&mut self) {
        let captured = self
            .key_sequence_capture_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        let Some(sequence) = captured else {
            return;
        };

        if let Some((action_idx, step_idx)) = self.key_sequence_capture_target {
            let sequence_for_log = sequence.clone();
            if let Some(ActionStep::KeyboardSequence(target)) = self
                .config
                .actions
                .get_mut(action_idx)
                .and_then(|action| action.steps.get_mut(step_idx))
            {
                *target = sequence;
                if sequence_for_log.is_empty() {
                    self.add_log("按键序列监听结束：未捕获到可用按键");
                } else {
                    self.add_log(&format!("已捕获按键序列: {}", sequence_for_log));
                }
            } else {
                self.add_log("按键序列监听结束：目标步骤不存在");
            }
        }

        self.key_sequence_capture_target = None;
        self.key_sequence_capture_rx = None;
        self.key_sequence_capture_started = None;
    }
}
