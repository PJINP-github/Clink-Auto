use crate::config::ActionStep;

use super::{ClickAutomationApp, RegionCaptureTarget};

pub(super) fn apply_captured_region(app: &mut ClickAutomationApp, region: (i32, i32, i32, i32)) {
    match app.capture_region_target {
        Some(RegionCaptureTarget::Step(action_idx, step_idx)) => {
            apply_step_region(app, action_idx, step_idx, region);
        }
        Some(RegionCaptureTarget::OcrTest) => {
            app.ocr_test_region = region;
            log_region(app, "OCR测试区域已设置", region);
        }
        None => {}
    }
}

fn apply_step_region(
    app: &mut ClickAutomationApp,
    action_idx: usize,
    step_idx: usize,
    region: (i32, i32, i32, i32),
) {
    if let Some(step) = app
        .config
        .actions
        .get_mut(action_idx)
        .and_then(|action| action.steps.get_mut(step_idx))
    {
        match step {
            ActionStep::SaturationClick { region: target, .. }
            | ActionStep::OcrCheck { region: target, .. }
            | ActionStep::OcrSaveMultiLine { region: target, .. }
            | ActionStep::ScreenCaptureRegion { region: target } => {
                *target = region;
                log_region(app, "区域已设置", region);
            }
            _ => {}
        }
    }
}

fn log_region(app: &mut ClickAutomationApp, label: &str, region: (i32, i32, i32, i32)) {
    app.add_log(&format!(
        "{}: ({}, {}) - ({}, {})",
        label, region.0, region.1, region.2, region.3
    ));
}
