#[cfg(windows)]
use super::rdev_backend::RdevBackend;
#[cfg(target_os = "linux")]
use super::wayland_shortcuts::WaylandShortcutBackend;
#[cfg(target_os = "linux")]
use super::x11_backend::X11Backend;

#[derive(Clone, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub right_pressed: bool,
    #[allow(dead_code)]
    pub left_just_pressed: bool,
    #[allow(dead_code)]
    pub left_just_released: bool,
    pub space_pressed: bool,
    pub ctrl_alt_space: bool,
    pub ctrl_alt_z_pressed: bool,
    pub ctrl_m_pressed: bool,
}

enum Backend {
    #[cfg(target_os = "linux")]
    X11(X11Backend),
    #[cfg(target_os = "linux")]
    WaylandShortcuts(WaylandShortcutBackend),
    #[cfg(target_os = "linux")]
    Unavailable,
    #[cfg(windows)]
    Rdev(RdevBackend),
}

pub struct MouseTracker {
    backend: Backend,
}

impl MouseTracker {
    pub fn start() -> Self {
        #[cfg(target_os = "linux")]
        if let Some(backend) = X11Backend::open() {
            return Self {
                backend: Backend::X11(backend),
            };
        }

        #[cfg(target_os = "linux")]
        if crate::wayland_portal::is_wayland_session() {
            if let Some(backend) = WaylandShortcutBackend::start() {
                return Self {
                    backend: Backend::WaylandShortcuts(backend),
                };
            }
        }

        #[cfg(target_os = "linux")]
        {
            eprintln!("X11 unavailable; mouse tracking and global hotkeys are disabled until DISPLAY is available");
            return Self {
                backend: Backend::Unavailable,
            };
        }

        #[cfg(windows)]
        Self {
            backend: Backend::Rdev(RdevBackend::start()),
        }
    }

    pub fn read_and_clear(&mut self) -> MouseState {
        match &mut self.backend {
            #[cfg(target_os = "linux")]
            Backend::X11(backend) => backend.read(),
            #[cfg(target_os = "linux")]
            Backend::WaylandShortcuts(backend) => backend.read(),
            #[cfg(target_os = "linux")]
            Backend::Unavailable => MouseState::default(),
            #[cfg(windows)]
            Backend::Rdev(backend) => backend.read(),
        }
    }
}
