use x11_dl::xlib::{self, Display};

use super::x11_backend::X11Backend;

impl X11Backend {
    pub(super) fn read_mouse_buttons(&mut self, mask: u32) -> (bool, bool, bool) {
        let left_down = (mask & 256) != 0;
        let right_down = (mask & 1024) != 0;
        let right_pressed = right_down && !self.prev_right;
        let left_pressed = left_down && !self.prev_left;
        let left_released = !left_down && self.prev_left;
        self.prev_right = right_down;
        self.prev_left = left_down;
        (right_pressed, left_pressed, left_released)
    }

    pub(super) fn read_keys(&mut self) -> (bool, bool, bool, bool) {
        let mut keys: [u8; 32] = [0; 32];
        unsafe {
            (self.xlib.XQueryKeymap)(self.display, keys.as_mut_ptr() as *mut i8);
        }
        let space_down = key_is_down(&keys, self.kc_space);
        let ctrl_down = key_is_down(&keys, self.kc_ctrl_l);
        let alt_down = key_is_down(&keys, self.kc_alt_l);
        let z_down = key_is_down(&keys, self.kc_z);
        let m_down = key_is_down(&keys, self.kc_m);

        let space_pressed = space_down && !self.prev_space;
        let ctrl_alt_space = space_pressed && ctrl_down && alt_down;
        let ctrl_alt_z = z_down && ctrl_down && alt_down && !self.prev_z;
        let ctrl_m = m_down && ctrl_down && !self.prev_m;
        self.prev_space = space_down;
        self.prev_z = z_down;
        self.prev_m = m_down;
        (space_pressed, ctrl_alt_space, ctrl_alt_z, ctrl_m)
    }
}

impl Drop for X11Backend {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

pub(super) fn keysym_to_keycode(xlib: &xlib::Xlib, display: *mut Display, keysym: u64) -> u32 {
    let kc = unsafe { (xlib.XKeysymToKeycode)(display, keysym) };
    if kc == 0 {
        0
    } else {
        kc as u32
    }
}

pub(super) fn key_is_down(keys: &[u8; 32], keycode: u32) -> bool {
    let byte_idx = (keycode / 8) as usize;
    let bit_idx = (keycode % 8) as u8;
    byte_idx < 32 && (keys[byte_idx] & (1 << bit_idx)) != 0
}
