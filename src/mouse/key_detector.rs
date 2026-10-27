#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use x11_dl::xlib::{self, Display, KeySym};

#[cfg(windows)]
use super::key_names_windows::windows_key_name_to_keycode;
#[cfg(windows)]
use crate::platform::is_key_pressed as win_is_key_pressed;

pub struct KeyDetector {
    #[cfg(target_os = "linux")]
    xlib: xlib::Xlib,
    #[cfg(target_os = "linux")]
    display: *mut Display,
}

impl KeyDetector {
    pub fn new() -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            let xlib = xlib::Xlib::open().ok()?;
            unsafe {
                let display = (xlib.XOpenDisplay)(std::ptr::null());
                if display.is_null() {
                    return None;
                }
                Some(Self { xlib, display })
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Some(Self {})
        }
    }

    pub fn is_key_pressed(&self, keycode: u32) -> bool {
        #[cfg(target_os = "linux")]
        unsafe {
            let mut keys: [u8; 32] = [0; 32];
            (self.xlib.XQueryKeymap)(self.display, keys.as_mut_ptr() as *mut i8);
            key_is_down(&keys, keycode)
        }
        #[cfg(windows)]
        {
            win_is_key_pressed(keycode as i32)
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            false
        }
    }

    pub fn key_name_to_keycode(&self, name: &str) -> Option<u32> {
        #[cfg(target_os = "linux")]
        unsafe {
            let c_name = CString::new(name).ok()?;
            let keysym: KeySym = (self.xlib.XStringToKeysym)(c_name.as_ptr());
            if keysym == 0 {
                return None;
            }
            let keycode = (self.xlib.XKeysymToKeycode)(self.display, keysym);
            if keycode == 0 {
                None
            } else {
                Some(keycode as u32)
            }
        }
        #[cfg(windows)]
        {
            windows_key_name_to_keycode(name)
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            None
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for KeyDetector {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

#[cfg(target_os = "linux")]
fn key_is_down(keys: &[u8; 32], keycode: u32) -> bool {
    let byte_idx = (keycode / 8) as usize;
    let bit_idx = (keycode % 8) as u8;
    byte_idx < 32 && (keys[byte_idx] & (1 << bit_idx)) != 0
}
