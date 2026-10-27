#![allow(dead_code)]
// 跨平台支持模块
// 根据不同操作系统提供键盘、鼠标等底层操作接口

#[cfg(target_os = "linux")]
pub mod linux_impl {
    pub fn is_key_pressed(_vk: i32) -> bool {
        false
    }

    pub fn is_ctrl_alt_key(_key_vk: i32) -> bool {
        false
    }

    pub fn get_cursor_pos() -> Option<(i32, i32)> {
        crate::linux_capture::cursor_position().ok()
    }

    pub fn get_pixel_color(x: i32, y: i32) -> Option<(u8, u8, u8)> {
        crate::linux_capture::pixel_color(x, y).ok()
    }

    #[allow(dead_code)]
    pub fn click_at(_x: i32, _y: i32) {}
}

#[cfg(windows)]
pub mod windows_impl {
    pub fn is_key_pressed(vk: i32) -> bool {
        unsafe {
            windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) as u32 & 0x8000 != 0
        }
    }

    pub fn is_ctrl_alt_key(key_vk: i32) -> bool {
        is_key_pressed(windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32)
            && is_key_pressed(windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU.0 as i32)
            && is_key_pressed(key_vk)
    }

    pub fn get_cursor_pos() -> Option<(i32, i32)> {
        unsafe {
            let mut point = windows::Win32::Foundation::POINT::default();
            if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).is_ok() {
                Some((point.x, point.y))
            } else {
                None
            }
        }
    }

    pub fn get_pixel_color(x: i32, y: i32) -> Option<(u8, u8, u8)> {
        unsafe {
            let hdc = windows::Win32::Graphics::Gdi::GetDC(None);
            if hdc.0 != std::ptr::null_mut() {
                let color = windows::Win32::Graphics::Gdi::GetPixel(hdc, x, y);
                windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc);
                if color.0 != 0xFFFFFFFF {
                    // CLR_INVALID
                    let r = (color.0 & 0xff) as u8;
                    let g = ((color.0 >> 8) & 0xff) as u8;
                    let b = ((color.0 >> 16) & 0xff) as u8;
                    return Some((r, g, b));
                }
            }
            None
        }
    }

    #[allow(dead_code)]
    pub fn click_at(x: i32, y: i32) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(x, y);
            windows::Win32::UI::Input::KeyboardAndMouse::mouse_event(
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0002u32), // MOUSEEVENTF_LEFTDOWN
                0,
                0,
                0,
                0,
            );
            windows::Win32::UI::Input::KeyboardAndMouse::mouse_event(
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0004u32), // MOUSEEVENTF_LEFTUP
                0,
                0,
                0,
                0,
            );
        }
    }

    #[allow(dead_code)]
    pub fn right_click_at(x: i32, y: i32) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(x, y);
            windows::Win32::UI::Input::KeyboardAndMouse::mouse_event(
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0008u32), // MOUSEEVENTF_RIGHTDOWN
                0,
                0,
                0,
                0,
            );
            windows::Win32::UI::Input::KeyboardAndMouse::mouse_event(
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0010u32), // MOUSEEVENTF_RIGHTUP
                0,
                0,
                0,
                0,
            );
        }
    }
}

// 公共接口 - 选择对应平台的实现
#[cfg(target_os = "linux")]
pub use linux_impl::*;

#[cfg(windows)]
pub use windows_impl::*;

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn init_platform() {
    // Linux平台初始化
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn init_platform() {
    // Windows平台初始化
}
