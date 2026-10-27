use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::capture_tools::{save_full_screen_capture, save_region_capture};
use crate::clipboard_tools::{append_system_clipboard_to_file, read_system_clipboard_text};
use crate::config::{Action, ActionStep, Configuration};
use crate::html_table;
use crate::mouse::{self, saturation_click};
use crate::ocr_runner::{recognize_region_text_timed, recognize_region_text_timed_full};
use crate::patches::PatchRegistry;
use crate::points_reader::{copy_next_point_to_clipboard, PointsReadOutcome};

#[allow(dead_code)]
pub fn execute_action(
    action: &Action,
    config: &Configuration,
    paused: &AtomicBool,
    add_log: &mut impl FnMut(String),
) -> Option<usize> {
    execute_action_with_patches(action, config, &PatchRegistry::empty(), paused, add_log)
}

pub fn execute_action_with_patches(
    action: &Action,
    config: &Configuration,
    patches: &PatchRegistry,
    paused: &AtomicBool,
    add_log: &mut impl FnMut(String),
) -> Option<usize> {
    add_log(format!(
        "ACTION_START name=[{}] steps={}",
        action.name,
        action.steps.len()
    ));
    for step in &action.steps {
        if paused.load(Ordering::SeqCst) {
            add_log("操作已暂停".to_string());
            break;
        }
        if let Some(next_action) = execute_step(step, config, patches, add_log) {
            return Some(next_action);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    add_log(format!("ACTION_END name=[{}]", action.name));
    None
}

fn execute_step(
    step: &ActionStep,
    config: &Configuration,
    patches: &PatchRegistry,
    add_log: &mut impl FnMut(String),
) -> Option<usize> {
    match step {
        ActionStep::Click(x, y) => match mouse::click_at(*x, *y) {
            Ok(()) => add_log(format!("点击 ({}, {})", x, y)),
            Err(err) => add_log(format!("点击失败: {}", err)),
        },
        ActionStep::RightClick(x, y) => match mouse::right_click_at(*x, *y) {
            Ok(()) => add_log(format!("右键点击 ({}, {})", x, y)),
            Err(err) => add_log(format!("右键点击失败: {}", err)),
        },
        ActionStep::SaturationClick {
            rows,
            clicks_per_row,
            interval_ms,
            region,
        } => match saturation_click(*rows, *clicks_per_row, *interval_ms, *region) {
            Ok(()) => add_log(format!(
                "饱和点击: {}行 x {}次, 区域 {:?}, 间隔 {}ms",
                rows, clicks_per_row, region, interval_ms
            )),
            Err(err) => add_log(format!("饱和点击失败: {}", err)),
        },
        ActionStep::Delay(ms) => {
            std::thread::sleep(Duration::from_millis(*ms));
            add_log(format!("延迟 {}ms", ms));
        }
        ActionStep::CopyClipboard => match append_system_clipboard_to_file() {
            Ok(path) => add_log(format!("剪贴板追加保存到 {}", path.display())),
            Err(err) => add_log(err),
        },
        ActionStep::ClickCategory(name) => execute_category_click(name, config, add_log),
        ActionStep::ScreenCapture => execute_screen_capture(add_log),
        ActionStep::ScreenCaptureRegion { region } => {
            execute_region_screen_capture(*region, add_log)
        }
        ActionStep::Paste => match mouse::paste() {
            Ok(()) => add_log("粘贴 (Ctrl+V)".to_string()),
            Err(err) => add_log(format!("粘贴失败: {}", err)),
        },
        ActionStep::KeyboardSequence(sequence) => match mouse::send_key_sequence(sequence) {
            Ok(()) => add_log(format!("发送按键序列: {}", sequence)),
            Err(err) => add_log(format!("发送按键序列失败: {}", err)),
        },
        ActionStep::OcrCheck {
            expected_text,
            region,
            action_on_match,
            action_on_mismatch,
        } => {
            add_log(format!(
                "OCR_CHECK start region=({},{})->({},{}) expected=[{}]",
                region.0, region.1, region.2, region.3, expected_text
            ));
            let (recognized, elapsed_ms) = match recognize_region_text_timed(*region) {
                Ok(result) => result,
                Err(error) => {
                    add_log(format!("OCR_CHECK error={}", error));
                    add_log(format!(
                        "OCR_CHECK branch=mismatch next_action={:?}",
                        action_on_mismatch
                    ));
                    return *action_on_mismatch;
                }
            };
            let matched = ocr_contains(expected_text, &recognized);
            add_log(format!(
                "OCR 结果: {} | 期望: {} | 匹配: {}",
                recognized, expected_text, matched
            ));
            add_log(format!(
                "OCR_CHECK result recognized=[{}] expected=[{}] matched={} elapsed_ms={:.1}",
                recognized, expected_text, matched, elapsed_ms
            ));
            add_log(format!(
                "OCR_CHECK branch={} next_action={:?}",
                if matched { "match" } else { "mismatch" },
                if matched {
                    action_on_match
                } else {
                    action_on_mismatch
                }
            ));
            return if matched {
                *action_on_match
            } else {
                *action_on_mismatch
            };
        }
        ActionStep::PointsTxtNextLine { action_on_empty } => {
            return execute_points_txt_next_line(*action_on_empty, add_log);
        }
        ActionStep::Patch {
            patch_id,
            action_id,
        } => match patches.execute_action(patch_id, action_id, add_log) {
            Ok(()) => add_log(format!("补丁动作执行完成: {} / {}", patch_id, action_id)),
            Err(error) => add_log(error),
        },
        ActionStep::HtmlToXlsx => match html_table::process_temp_html() {
            Ok(report) => {
                for line in report.lines() {
                    add_log(line.to_string());
                }
            }
            Err(err) => add_log(format!("HTML 转 xlsx 失败: {}", err)),
        },
        ActionStep::OcrSaveMultiLine {
            region,
            filename_prefix,
            use_full_pipeline,
        } => {
            let result = if *use_full_pipeline {
                recognize_region_text_timed_full(*region)
            } else {
                recognize_region_text_timed(*region)
            };
            match result {
                Ok((text, elapsed_ms)) => {
                    add_log(format!(
                        "OCR_MULTI 识别完成 elapsed_ms={:.1} region=({},{})->({},{})",
                        elapsed_ms, region.0, region.1, region.2, region.3
                    ));
                    match save_ocr_multi_line(filename_prefix, &text) {
                        Ok(path) => add_log(format!("OCR 多行结果已保存至 {}", path.display())),
                        Err(err) => add_log(format!("保存 OCR 结果失败: {}", err)),
                    }
                }
                Err(err) => add_log(format!("OCR_MULTI 识别失败: {}", err)),
            }
        }
    }
    None
}

pub(crate) fn ocr_contains(expected: &str, recognized: &str) -> bool {
    let recognized = normalize_ocr_text(recognized);
    parse_ocr_match_rules(expected)
        .iter()
        .any(|rule| rule.matches(&recognized))
}

fn normalize_ocr_text(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OcrMatchKind {
    Contains,
    Exact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OcrMatchRule {
    kind: OcrMatchKind,
    text: String,
}

impl OcrMatchRule {
    fn matches(&self, recognized: &str) -> bool {
        if self.text.is_empty() {
            return false;
        }
        match self.kind {
            OcrMatchKind::Contains => recognized.contains(&self.text),
            OcrMatchKind::Exact => recognized == self.text,
        }
    }
}

fn parse_ocr_match_rules(expected: &str) -> Vec<OcrMatchRule> {
    if !expected.contains("s:") && !expected.contains("d:") {
        return vec![OcrMatchRule {
            kind: OcrMatchKind::Contains,
            text: normalize_ocr_text(expected),
        }];
    }

    expected
        .split(';')
        .flat_map(parse_ocr_match_segment)
        .collect()
}

fn parse_ocr_match_segment(segment: &str) -> Vec<OcrMatchRule> {
    let mut rules = Vec::new();
    let mut current_kind = None;

    for part in segment.split(':') {
        match part {
            "s" => current_kind = Some(OcrMatchKind::Contains),
            "d" => current_kind = Some(OcrMatchKind::Exact),
            text => {
                let text = normalize_ocr_text(text);
                if !text.is_empty() {
                    if let Some(kind) = current_kind {
                        rules.push(OcrMatchRule { kind, text });
                    }
                }
            }
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    use super::ocr_contains;

    #[test]
    fn ocr_check_uses_substring_matching_without_whitespace() {
        assert!(ocr_contains("未发现异", "分析结果：未发现异常"));
        assert!(ocr_contains("52.0", "分析结果：52.0"));
        assert!(ocr_contains(
            "s:为空;s:失败:d:分析结果：",
            "分析结果：分析失败"
        ));
        assert!(!ocr_contains("算法调用失败", "分析结果：算法调用超时"));
    }
}

#[cfg(test)]
mod ocr_match_rule_tests {
    use super::ocr_contains;

    #[test]
    fn d_rule_requires_exact_whole_ocr_result() {
        assert!(ocr_contains("d:分析结果：;", "分析结果："));
        assert!(!ocr_contains("d:分析结果：;", "分析结果：005"));
        assert!(!ocr_contains(
            "s:为空;s:失败:d:分析结果：;",
            "分析结果：005"
        ));
    }

    #[test]
    fn s_rule_keeps_substring_matching() {
        assert!(ocr_contains("s:失败;", "分析结果：分析失败"));
        assert!(ocr_contains(
            "s:为空;s:失败:d:分析结果：;",
            "分析结果：分析失败"
        ));
    }
}

fn execute_points_txt_next_line(
    action_on_empty: Option<usize>,
    add_log: &mut impl FnMut(String),
) -> Option<usize> {
    match copy_next_point_to_clipboard() {
        Ok(PointsReadOutcome::Copied { value, line_number }) => {
            add_log(format!(
                "points.txt 第{}行已复制到剪贴板: {}",
                line_number, value
            ));
            None
        }
        Ok(PointsReadOutcome::EmptyLine { line_number }) => {
            add_log(format!("points.txt 第{}行为空，执行后续动作", line_number));
            action_on_empty
        }
        Ok(PointsReadOutcome::EndOfFile) => {
            add_log("points.txt 已读到文件末尾，执行后续动作".to_string());
            action_on_empty
        }
        Err(err) => {
            add_log(err);
            action_on_empty
        }
    }
}

fn execute_category_click(name: &str, config: &Configuration, add_log: &mut impl FnMut(String)) {
    let coord_opt = config
        .coordinate_categories
        .iter()
        .find(|c| c.name == name)
        .and_then(|cat| cat.coordinates.first().map(|c| (c.x, c.y)));
    if let Some((cx, cy)) = coord_opt {
        match mouse::click_at(cx, cy) {
            Ok(()) => add_log(format!("点击分类 {} at ({}, {})", name, cx, cy)),
            Err(err) => add_log(format!("点击分类 {} 失败: {}", name, err)),
        }
    }
}

fn execute_screen_capture(add_log: &mut impl FnMut(String)) {
    let base_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    match save_full_screen_capture(base_dir) {
        Ok(path) => add_log(format!("全屏截图已保存至 {}", path.display())),
        Err(err) => add_log(err),
    }
}

fn execute_region_screen_capture(region: (i32, i32, i32, i32), add_log: &mut impl FnMut(String)) {
    let base_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let filename_hint = read_system_clipboard_text().ok();
    match save_region_capture(base_dir, region, filename_hint.as_deref()) {
        Ok(path) => add_log(format!("区域截图已保存至 {}", path.display())),
        Err(err) => add_log(err),
    }
}

fn save_ocr_multi_line(prefix: &str, text: &str) -> Result<PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let temp_dir = exe_dir.join("temp");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建 temp 目录失败: {}", e))?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let safe_prefix: String = prefix
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control()
                || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let filename = if safe_prefix.is_empty() {
        format!("ocr_{}.txt", timestamp)
    } else {
        format!("ocr_{}_{}.txt", safe_prefix, timestamp)
    };
    let path = temp_dir.join(filename);
    std::fs::write(&path, text).map_err(|e| format!("写入 OCR txt 失败: {}", e))?;
    Ok(path)
}
