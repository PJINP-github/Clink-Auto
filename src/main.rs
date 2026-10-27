#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;

mod action_executor;
mod app;
mod capture_tools;
mod clipboard_tools;
mod config;
mod config_path;
mod html_table;
mod launcher;
#[cfg(target_os = "linux")]
mod linux_capture;
mod mouse;
mod ocr;
mod ocr_runner;
mod password;
mod patches;
mod platform;
mod points_reader;
#[cfg(target_os = "linux")]
mod wayland_portal;
#[cfg(windows)]
mod windows_capture;

#[cfg(any(windows, target_os = "linux"))]
fn load_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    const EMBEDDED_CJK_FONT: &[u8] = include_bytes!("../notosansscregular.otf");
    fonts.font_data.insert(
        "embedded_cjk".to_string(),
        egui::FontData::from_static(EMBEDDED_CJK_FONT),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .unwrap()
            .insert(0, "embedded_cjk".to_string());
    }
    ctx.set_fonts(fonts);
}

#[cfg(not(any(windows, target_os = "linux")))]
fn load_custom_fonts(_ctx: &egui::Context) {}

fn main() {
    install_debug_panic_hook();
    ensure_temp_dir();

    if let Err(error) = run() {
        report_startup_error(&error);
        wait_for_debug_exit();
        std::process::exit(1);
    }
}

fn ensure_temp_dir() {
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
    {
        let _ = std::fs::create_dir_all(exe_dir.join("temp"));
    }
}

fn run() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Click Automation",
        native_options,
        Box::new(|cc| {
            load_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(launcher::Launcher::new(cc)))
        }),
    )
}

#[cfg(debug_assertions)]
fn install_debug_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("程序启动失败：{panic_info}");
        if let Some(location) = panic_info.location() {
            eprintln!(
                "发生位置：{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        eprintln!("Backtrace:\n{}", std::backtrace::Backtrace::force_capture());
        wait_for_debug_exit();
    }));
}

#[cfg(not(debug_assertions))]
fn install_debug_panic_hook() {}

#[cfg(debug_assertions)]
fn report_startup_error(error: &eframe::Error) {
    eprintln!("程序启动失败：{error:?}");
    eprintln!(
        "可执行文件：{}",
        std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|error| format!("<获取失败：{error}>"))
    );
    eprintln!(
        "当前目录：{}",
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|error| format!("<获取失败：{error}>"))
    );
}

#[cfg(not(debug_assertions))]
fn report_startup_error(_error: &eframe::Error) {}

#[cfg(debug_assertions)]
fn wait_for_debug_exit() {
    eprintln!("按 Enter 退出...");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}

#[cfg(not(debug_assertions))]
fn wait_for_debug_exit() {}
