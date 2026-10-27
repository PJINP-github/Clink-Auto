use eframe::egui;

use crate::app::ClickAutomationApp;
use crate::config::Configuration;
use crate::password;

struct LoginApp {
    password_input: String,
    error: String,
}

impl LoginApp {
    fn new() -> Self {
        Self {
            password_input: String::new(),
            error: String::new(),
        }
    }

    fn check(&self) -> bool {
        password::verify_password(&self.password_input)
    }
}

enum AppState {
    Login(LoginApp),
    Main(ClickAutomationApp),
}

pub struct Launcher {
    state: AppState,
}

impl Launcher {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Configuration::load().unwrap_or_default();
        if config.passwd {
            Self {
                state: AppState::Main(ClickAutomationApp::new()),
            }
        } else {
            Self {
                state: AppState::Login(LoginApp::new()),
            }
        }
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut should_switch_to_main = false;

        match &mut self.state {
            AppState::Login(login) => draw_login(ctx, login, &mut should_switch_to_main),
            AppState::Main(main_app) => main_app.update(ctx, frame),
        }

        if should_switch_to_main {
            self.state = AppState::Main(ClickAutomationApp::new());
        }
    }
}

fn draw_login(ctx: &egui::Context, login: &mut LoginApp, should_switch_to_main: &mut bool) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("请输入动态密码");
        ui.add(egui::TextEdit::singleline(&mut login.password_input).password(true));
        if ui.button("进入").clicked() {
            if login.check() {
                *should_switch_to_main = true;
            } else {
                login.error = "密码错误".to_string();
            }
        }
        if !login.error.is_empty() {
            ui.colored_label(egui::Color32::RED, &login.error);
        }
    });
}
