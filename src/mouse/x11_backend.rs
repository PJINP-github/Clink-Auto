use x11_dl::keysym::{XK_Alt_L, XK_Control_L, XK_m, XK_space, XK_z};
use x11_dl::xlib::{self, Display, Window};

use super::tracker::MouseState;
use super::x11_backend_input::keysym_to_keycode;

pub(super) struct X11Backend {
    pub(super) xlib: xlib::Xlib,
    pub(super) display: *mut Display,
    root: Window,
    pub(super) prev_left: bool,
    pub(super) prev_right: bool,
    pub(super) prev_space: bool,
    pub(super) prev_z: bool,
    pub(super) prev_m: bool,
    pub(super) kc_space: u32,
    pub(super) kc_ctrl_l: u32,
    pub(super) kc_alt_l: u32,
    pub(super) kc_z: u32,
    pub(super) kc_m: u32,
}

unsafe impl Send for X11Backend {}
unsafe impl Sync for X11Backend {}

impl X11Backend {
    pub(super) fn open() -> Option<Self> {
        let xlib = xlib::Xlib::open().ok()?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return None;
            }
            Some(Self::new(xlib, display))
        }
    }

    fn new(xlib: xlib::Xlib, display: *mut Display) -> Self {
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        Self {
            kc_space: keysym_to_keycode(&xlib, display, XK_space as u64),
            kc_ctrl_l: keysym_to_keycode(&xlib, display, XK_Control_L as u64),
            kc_alt_l: keysym_to_keycode(&xlib, display, XK_Alt_L as u64),
            kc_z: keysym_to_keycode(&xlib, display, XK_z as u64),
            kc_m: keysym_to_keycode(&xlib, display, XK_m as u64),
            xlib,
            display,
            root,
            prev_left: false,
            prev_right: false,
            prev_space: false,
            prev_z: false,
            prev_m: false,
        }
    }

    pub(super) fn read(&mut self) -> MouseState {
        let (x, y, mask) = self.query_pointer();
        let mouse = self.read_mouse_buttons(mask);
        let keys = self.read_keys();

        MouseState {
            x,
            y,
            right_pressed: mouse.0,
            left_just_pressed: mouse.1,
            left_just_released: mouse.2,
            space_pressed: keys.0,
            ctrl_alt_space: keys.1,
            ctrl_alt_z_pressed: keys.2,
            ctrl_m_pressed: keys.3,
        }
    }

    fn query_pointer(&self) -> (i32, i32, u32) {
        unsafe {
            let mut root_return: Window = 0;
            let mut child_return: Window = 0;
            let mut root_x: i32 = 0;
            let mut root_y: i32 = 0;
            let mut win_x: i32 = 0;
            let mut win_y: i32 = 0;
            let mut mask: u32 = 0;

            (self.xlib.XQueryPointer)(
                self.display,
                self.root,
                &mut root_return,
                &mut child_return,
                &mut root_x,
                &mut root_y,
                &mut win_x,
                &mut win_y,
                &mut mask,
            );
            (root_x, root_y, mask)
        }
    }
}
