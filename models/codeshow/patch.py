#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""patch_optimizations.py - 优化：上下按钮排序、标签页配置控制、清理无用警告"""
from __future__ import annotations

import argparse
import shutil
import sys
from datetime import datetime
from pathlib import Path


def find_root(start: Path) -> Path:
    current = start.resolve()
    if current.is_file():
        current = current.parent
    while True:
        if (current / "Cargo.toml").is_file() and (current / "src").is_dir():
            return current
        if current.parent == current:
            raise RuntimeError(f"未找到项目根: {start}")
        current = current.parent


def read_text(path: Path) -> str:
    raw = path.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        return raw.decode("utf-8-sig")
    return raw.decode("utf-8")


def write_text(path: Path, text: str, use_bom: bool) -> None:
    data = text.encode("utf-8")
    if use_bom:
        data = b"\xef\xbb\xbf" + data
    path.write_bytes(data)


def has_bom(path: Path) -> bool:
    try:
        return path.read_bytes().startswith(b"\xef\xbb\xbf")
    except OSError:
        return False


def backup(root: Path, path: Path, backup_root: Path) -> Path:
    target = backup_root / path.relative_to(root)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
    return target


def apply_patches(path: Path, patches, dry_run: bool) -> None:
    if not path.is_file():
        raise FileNotFoundError(f"目标文件不存在: {path}")
    raw = read_text(path)
    use_bom = has_bom(path)
    normalized = raw.replace("\r\n", "\n").replace("\r", "\n")
    changed = False
    new_text = normalized
    
    for label, old, new in patches:
        old_n = old.replace("\r\n", "\n").replace("\r", "\n")
        new_n = new.replace("\r\n", "\n").replace("\r", "\n")
        if new_n in new_text:
            print(f"[SKIP] {label}（已存在）")
            continue
        count = new_text.count(old_n)
        if count != 1:
            raise RuntimeError(f"{label}: old 匹配 {count} 次，应为 1 次")
        new_text = new_text.replace(old_n, new_n, 1)
        print(f"[OK] {label}")
        changed = True
        
    if not changed:
        return
    if dry_run:
        print(f"[DRY-RUN] 将修改 {path}")
        return
    if "\r\n" in raw:
        new_text = new_text.replace("\n", "\r\n")
    write_text(path, new_text, use_bom)
    print(f"[WRITE] {path}")


# ---------------------------------------------------------------------------
# 补丁定义
# ---------------------------------------------------------------------------

PATCHES = [
    # ---------------- 1. 上下移动按钮替换拖拽 ----------------
    (
        "actions_step_list.rs: 用上下按钮替换拖拽",
        "src/app/actions_step_list.rs",
        [
            (
                "actions_step_list.rs: 替换逻辑",
                """    let mut remove = Vec::new();
    let mut insert_idx = None;
    let drag_source_id = egui::Id::new("dnd_drag_source");

    for (step_idx, step) in action.steps.iter_mut().enumerate() {
        let drag_id = ui.make_persistent_id(("step_drag", action_idx, step_idx));

        let row_resp = ui
            .scope(|ui| {
                ui.horizontal_wrapped(|ui| {
                    // 拖拽手柄：使用 push_id 为其赋予唯一 ID
                    let drag_resp = ui
                        .push_id(drag_id, |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new("≡").size(16.0))
                                    .sense(egui::Sense::click_and_drag()),
                            )
                        })
                        .inner
                        .on_hover_cursor(egui::CursorIcon::Grab);

                    if drag_resp.drag_started() {
                        ui.data_mut(|d| d.insert_temp(drag_source_id, (action_idx, step_idx)));
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
            })
            .response;

        // 检测拖拽悬停并画插入线
        if let Some((src_a, _src_i)) =
            ui.data_mut(|d| d.get_temp::<(usize, usize)>(drag_source_id))
        {
            if src_a == action_idx {
                if let Some(pointer) = ui.ctx().pointer_hover_pos() {
                    if row_resp.rect.expand2(egui::vec2(0.0, 4.0)).contains(pointer) {
                        let is_top = pointer.y < row_resp.rect.center().y;
                        insert_idx = Some(if is_top { step_idx } else { step_idx + 1 });
                        let y = if is_top {
                            row_resp.rect.top()
                        } else {
                            row_resp.rect.bottom()
                        };
                        ui.painter().hline(
                            row_resp.rect.x_range(),
                            y,
                            egui::Stroke::new(3.0_f32, egui::Color32::LIGHT_BLUE),
                        );
                    }
                }
            }
        }
    }

    // 处理删除
    for &idx in remove.iter().rev() {
        action.steps.remove(idx);
    }

    // 处理拖拽释放
    if ui.ctx().input(|i| i.pointer.any_released()) {
        if let Some((src_a, src_i)) =
            ui.data_mut(|d| d.remove_temp::<(usize, usize)>(drag_source_id))
        {
            if src_a == action_idx {
                if let Some(mut tgt_i) = insert_idx {
                    if src_i < tgt_i {
                        tgt_i -= 1;
                    }
                    if src_i != tgt_i {
                        let step = action.steps.remove(src_i);
                        action.steps.insert(tgt_i, step);
                    }
                }
            }
        }
    }""",
                """    let mut remove = Vec::new();
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
    }"""
            )
        ]
    ),
    # ---------------- 2. 标签页配置 ----------------
    (
        "config.rs: 新增 visible_tabs",
        "src/config.rs",
        [
            (
                "config.rs: 在 Configuration 结构体加 visible_tabs",
                """    pub run_config: RunConfig,
    #[serde(default)]
    pub passwd: bool,
}""",
                """    pub run_config: RunConfig,
    #[serde(default)]
    pub passwd: bool,
    #[serde(default = "default_visible_tabs")]
    pub visible_tabs: Vec<String>,
}

fn default_visible_tabs() -> Vec<String> {
    vec!["actions".to_string(), "run".to_string()]
}"""
            ),
            (
                "config.rs: Default 实现加 visible_tabs",
                """            passwd: false,
        }
    }
}""",
                """            passwd: false,
            visible_tabs: default_visible_tabs(),
        }
    }
}"""
            )
        ]
    ),
    (
        "update.rs: 过滤标签",
        "src/app/update.rs",
        [
            (
                "update.rs: 添加 is_tab_visible 和修正 draw_ui",
                """impl ClickAutomationApp {
    fn handle_language_toggle(&mut self) {""",
                """impl ClickAutomationApp {
    fn is_tab_visible(&self, tab_name: &str) -> bool {
        if self.config.visible_tabs.is_empty() {
            return matches!(tab_name, "actions" | "run");
        }
        self.config.visible_tabs.iter().any(|t| t == tab_name)
    }

    fn handle_language_toggle(&mut self) {"""
            ),
            (
                "update.rs: draw_ui 中按条件渲染",
                """                ui.selectable_value(&mut self.active_tab, 0, "坐标");
                ui.selectable_value(&mut self.active_tab, 1, "动作");
                ui.selectable_value(&mut self.active_tab, 2, "运行");
                ui.selectable_value(&mut self.active_tab, 3, "日志");
                ui.selectable_value(&mut self.active_tab, 4, "OCR验证");
                ui.selectable_value(&mut self.active_tab, 5, "设置");
                for patch_index in 0..self.patch_host.tab_count() {
                    if let Some(tab) = self.patch_host.tab_definition(patch_index) {
                        ui.selectable_value(&mut self.active_tab, 6 + patch_index, &tab.title);
                    }
                }""",
                """                if self.is_tab_visible("coordinates") {
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
                }"""
            ),
            (
                "update.rs: update 防止停留在隐藏标签",
                """        self.handle_mouse_capture(&mouse_state);
        self.draw_ui(ctx);""",
                """        self.handle_mouse_capture(&mouse_state);

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

        self.draw_ui(ctx);"""
            )
        ]
    ),
    # ---------------- 3. 清理警告 ----------------
    (
        "action_executor.rs: allow dead_code",
        "src/action_executor.rs",
        [
            (
                "action_executor.rs: execute_action 前加 allow",
                "pub fn execute_action(",
                "#[allow(dead_code)]\npub fn execute_action("
            )
        ]
    ),
    (
        "app/mod.rs: allow dead_code",
        "src/app/mod.rs",
        [
            (
                "app/mod.rs: execute_action 加 allow",
                "    fn execute_action(&mut self, action: &Action) {",
                "    #[allow(dead_code)]\n    fn execute_action(&mut self, action: &Action) {"
            ),
            (
                "app/mod.rs: execute_action_chain 加 allow",
                "    fn execute_action_chain(&mut self, action: &Action, depth: usize) {",
                "    #[allow(dead_code)]\n    fn execute_action_chain(&mut self, action: &Action, depth: usize) {"
            )
        ]
    ),
    (
        "mouse/tracker.rs: allow dead_code",
        "src/mouse/tracker.rs",
        [
            (
                "mouse/tracker.rs: left_just_pressed 字段加 allow",
                "    pub left_just_pressed: bool,",
                "    #[allow(dead_code)]\n    pub left_just_pressed: bool,"
            ),
            (
                "mouse/tracker.rs: left_just_released 字段加 allow",
                "    pub left_just_released: bool,",
                "    #[allow(dead_code)]\n    pub left_just_released: bool,"
            )
        ]
    ),
    (
        "ocr.rs: allow dead_code",
        "src/ocr.rs",
        [
            (
                "ocr.rs: ocr_file 加 allow",
                "pub fn ocr_file(image_path: impl AsRef<Path>) -> Result<String, String> {",
                "#[allow(dead_code)]\npub fn ocr_file(image_path: impl AsRef<Path>) -> Result<String, String> {"
            ),
            (
                "ocr.rs: ocr_image 加 allow",
                "pub fn ocr_image(image: &RgbImage) -> Result<String, String> {",
                "#[allow(dead_code)]\npub fn ocr_image(image: &RgbImage) -> Result<String, String> {"
            )
        ]
    ),
    (
        "patches.rs: allow dead_code",
        "src/patches.rs",
        [
            (
                "patches.rs: empty 加 allow",
                "    pub fn empty() -> Self {",
                "    #[allow(dead_code)]\n    pub fn empty() -> Self {"
            )
        ]
    ),
    (
        "platform.rs: 全局 allow dead_code",
        "src/platform.rs",
        [
            (
                "platform.rs: 顶部加全局 allow",
                "// 跨平台支持模块",
                "#![allow(dead_code)]\n// 跨平台支持模块"
            )
        ]
    ),
    (
        "windows_capture.rs: 消除 unused_must_use",
        "src/windows_capture.rs",
        [
            (
                "windows_capture.rs: 加 allow",
                "#![cfg(windows)]",
                "#![cfg(windows)]\n#![allow(unused_must_use)]"
            )
        ]
    )
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    root = find_root(Path(__file__).resolve())
    print(f"[ROOT] {root}")

    backup_root = None
    if not args.dry_run:
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        backup_root = root / "tools" / "upload" / f".backup-optimize-{stamp}"
        backup_root.mkdir(parents=True, exist_ok=True)

    try:
        for label, filename, patches in PATCHES:
            path = root / filename
            if backup_root is not None:
                backup(root, path, backup_root)
                print(f"[BACKUP] {path.relative_to(root)}")
            apply_patches(path, patches, args.dry_run)
    except RuntimeError as exc:
        print(f"\n[ERROR] {exc}", file=sys.stderr)
        if backup_root is not None:
            print(f"       已备份到: {backup_root}", file=sys.stderr)
        return 1

    print("\n[DONE] 优化补丁完成。")
    print("请手动执行 `cargo build --release` 验证编译。")
    return 0


if __name__ == "__main__":
    sys.exit(main())