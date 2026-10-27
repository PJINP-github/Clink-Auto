mod actions;
mod key_capture;
mod key_detector;
#[cfg(windows)]
mod key_names;
#[cfg(windows)]
mod key_names_windows;
#[cfg(windows)]
mod rdev_backend;
mod tracker;
#[cfg(target_os = "linux")]
mod wayland_shortcuts {
    pub(super) use crate::wayland_portal::global_shortcuts::WaylandShortcutBackend;
}
#[cfg(target_os = "linux")]
mod x11_backend;
#[cfg(target_os = "linux")]
mod x11_backend_input;

pub use actions::{click_at, paste, right_click_at, saturation_click, send_key_sequence};
pub use key_capture::{listen_for_key_sequence, listen_for_next_key, KEY_SEQUENCE_CAPTURE_SECONDS};
pub use key_detector::KeyDetector;
pub use tracker::{MouseState, MouseTracker};
