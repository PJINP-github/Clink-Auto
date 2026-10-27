use super::capture_region::apply_captured_region;
use super::{ClickAutomationApp, RegionCaptureTarget};
use crate::config::ActionStep;
use crate::mouse::MouseState;

impl ClickAutomationApp {
    pub(super) fn cancel_coordinate_capture(&mut self) {
        self.capture_coordinate = false;
        self.capture_target = None;
        self.capture_step_target = None;
    }

    pub(super) fn start_coordinate_capture(
        &mut self,
        target: Option<(usize, usize)>,
        step: Option<(usize, usize)>,
    ) {
        self.capture_region = false;
        self.capture_region_target = None;
        self.region_start = None;
        self.capture_coordinate = true;
        self.capture_target = target;
        self.capture_step_target = step;
        self.add_log("在桌面右键点击捕获坐标");
    }

    pub(super) fn start_region_capture(&mut self, target: RegionCaptureTarget) {
        self.capture_coordinate = false;
        self.capture_target = None;
        self.capture_step_target = None;
        self.capture_region = true;
        self.capture_region_target = Some(target);
        self.region_start = None;
        self.add_log("请在桌面右键点击两次: 第一次为起点, 第二次为终点");
    }

    pub(super) fn handle_mouse_capture(&mut self, ms: &MouseState) {
        if self.capture_coordinate && ms.right_pressed {
            self.finish_coordinate_capture();
        }
        if self.capture_region && ms.right_pressed {
            self.capture_region_click();
        }
    }

    fn finish_coordinate_capture(&mut self) {
        if let Some((cat_idx, coord_idx)) = self.capture_target {
            if let Some(coord) = self
                .config
                .coordinate_categories
                .get_mut(cat_idx)
                .and_then(|c| c.coordinates.get_mut(coord_idx))
            {
                coord.x = self.mouse_x;
                coord.y = self.mouse_y;
                self.add_log(&format!("坐标已捕获: ({}, {})", self.mouse_x, self.mouse_y));
            }
        } else if let Some((ai, si)) = self.capture_step_target {
            if let Some(step) = self
                .config
                .actions
                .get_mut(ai)
                .and_then(|a| a.steps.get_mut(si))
            {
                match step {
                    ActionStep::Click(x, y) | ActionStep::RightClick(x, y) => {
                        *x = self.mouse_x;
                        *y = self.mouse_y;
                        self.add_log(&format!(
                            "步骤坐标已捕获: ({}, {})",
                            self.mouse_x, self.mouse_y
                        ));
                    }
                    _ => {}
                }
            }
        }
        self.cancel_coordinate_capture();
    }

    fn capture_region_click(&mut self) {
        if self.region_start.is_none() {
            self.region_start = Some((self.mouse_x, self.mouse_y));
            self.add_log(&format!(
                "区域起点: ({}, {}), 请右键点击终点",
                self.mouse_x, self.mouse_y
            ));
            return;
        }
        let (x1, y1) = self.region_start.unwrap();
        let (x2, y2) = (self.mouse_x, self.mouse_y);
        let region = (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
        apply_captured_region(self, region);
        self.capture_region = false;
        self.capture_region_target = None;
        self.region_start = None;
    }
}
