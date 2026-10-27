#![cfg(windows)]
#![allow(unused_must_use)]

use image::{DynamicImage, RgbaImage};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub fn capture_full_screen() -> Result<DynamicImage, String> {
    let (x, y, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    capture_region(x, y, width.max(1) as u32, height.max(1) as u32)
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage, String> {
    if width == 0 || height == 0 {
        return Err("截图区域为空".to_string());
    }

    unsafe {
        let screen_dc = windows::Win32::Graphics::Gdi::GetDC(None);
        if screen_dc.0.is_null() {
            return Err("获取屏幕设备上下文失败".to_string());
        }
        let memory_dc = CreateCompatibleDC(screen_dc);
        if memory_dc.0.is_null() {
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            return Err("创建截图设备上下文失败".to_string());
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
        if bitmap.0.is_null() {
            DeleteDC(memory_dc);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            return Err("创建截图位图失败".to_string());
        }
        let old_object = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
        if old_object.0.is_null() {
            DeleteObject(HGDIOBJ(bitmap.0));
            DeleteDC(memory_dc);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            return Err("选择截图位图失败".to_string());
        }
        let copied = BitBlt(
            memory_dc,
            0,
            0,
            width as i32,
            height as i32,
            screen_dc,
            x,
            y,
            SRCCOPY,
        );
        if copied.is_err() {
            SelectObject(memory_dc, old_object);
            DeleteObject(HGDIOBJ(bitmap.0));
            DeleteDC(memory_dc);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            return Err("复制屏幕像素失败".to_string());
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let lines = GetDIBits(
            memory_dc,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );
        SelectObject(memory_dc, old_object);
        DeleteObject(HGDIOBJ(bitmap.0));
        DeleteDC(memory_dc);
        windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);

        if lines == 0 {
            return Err("读取截图像素失败".to_string());
        }
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        RgbaImage::from_raw(width, height, pixels)
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(|| "创建截图图像失败".to_string())
    }
}
