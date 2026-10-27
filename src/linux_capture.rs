#![cfg(target_os = "linux")]

use image::{DynamicImage, RgbaImage};
use x11_dl::xlib::{self, Display, Window};

pub fn capture_full_screen() -> Result<DynamicImage, String> {
    if crate::wayland_portal::is_wayland_session() {
        return crate::wayland_portal::capture_full_screen();
    }

    let connection = X11Connection::open()?;
    let (width, height) = connection.root_size()?;
    connection.capture_region(0, 0, width, height)
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage, String> {
    if crate::wayland_portal::is_wayland_session() {
        return crate::wayland_portal::capture_region(x, y, width, height);
    }

    if width == 0 || height == 0 {
        return Err("截图区域为空".to_string());
    }

    let connection = X11Connection::open()?;
    connection.capture_region(x, y, width, height)
}

pub fn cursor_position() -> Result<(i32, i32), String> {
    let connection = X11Connection::open()?;
    connection.cursor_position()
}

pub fn pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    let image = capture_region(x, y, 1, 1)?.to_rgba8();
    let pixel = image.get_pixel(0, 0).0;
    Ok((pixel[0], pixel[1], pixel[2]))
}

struct X11Connection {
    xlib: xlib::Xlib,
    display: *mut Display,
    root: Window,
}

impl X11Connection {
    fn open() -> Result<Self, String> {
        if std::env::var_os("DISPLAY").is_none() {
            return Err("Linux 截图需要 X11/XWayland 图形会话，请确认 DISPLAY 已设置".to_string());
        }

        let xlib = xlib::Xlib::open()
            .map_err(|e| format!("加载 libX11 失败: {e:?}，请安装系统运行库 libX11/libx11-6"))?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return Err(
                    "连接 X11 显示服务失败，请确认当前会话是 X11 或启用了 XWayland".to_string(),
                );
            }
            let root = (xlib.XDefaultRootWindow)(display);
            Ok(Self {
                xlib,
                display,
                root,
            })
        }
    }

    fn root_size(&self) -> Result<(u32, u32), String> {
        unsafe {
            let mut root_return: Window = 0;
            let mut x = 0;
            let mut y = 0;
            let mut width = 0;
            let mut height = 0;
            let mut border_width = 0;
            let mut depth = 0;
            let status = (self.xlib.XGetGeometry)(
                self.display,
                self.root,
                &mut root_return,
                &mut x,
                &mut y,
                &mut width,
                &mut height,
                &mut border_width,
                &mut depth,
            );
            if status != 0 && width > 0 && height > 0 {
                return Ok((width, height));
            }

            let screen = (self.xlib.XDefaultScreen)(self.display);
            let width = (self.xlib.XDisplayWidth)(self.display, screen);
            let height = (self.xlib.XDisplayHeight)(self.display, screen);
            if width > 0 && height > 0 {
                Ok((width as u32, height as u32))
            } else {
                Err("获取 X11 根窗口尺寸失败".to_string())
            }
        }
    }

    fn capture_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<DynamicImage, String> {
        let (screen_width, screen_height) = self.root_size()?;
        let region = clamp_region(x, y, width, height, screen_width, screen_height)?;

        let image = unsafe {
            (self.xlib.XGetImage)(
                self.display,
                self.root,
                region.x,
                region.y,
                region.width,
                region.height,
                (self.xlib.XAllPlanes)(),
                xlib::ZPixmap,
            )
        };
        if image.is_null() {
            return Err(
                "X11 截图失败：XGetImage 未返回图像，可能是会话权限或窗口系统限制".to_string(),
            );
        }

        let image = XImageHandle {
            xlib: &self.xlib,
            image,
        };
        ximage_to_dynamic(&image)
    }

    fn cursor_position(&self) -> Result<(i32, i32), String> {
        unsafe {
            let mut root_return: Window = 0;
            let mut child_return: Window = 0;
            let mut root_x = 0;
            let mut root_y = 0;
            let mut win_x = 0;
            let mut win_y = 0;
            let mut mask = 0;
            let status = (self.xlib.XQueryPointer)(
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
            if status == 0 {
                Err("读取 X11 鼠标位置失败".to_string())
            } else {
                Ok((root_x, root_y))
            }
        }
    }
}

impl Drop for X11Connection {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

struct XImageHandle<'a> {
    xlib: &'a xlib::Xlib,
    image: *mut xlib::XImage,
}

impl Drop for XImageHandle<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XDestroyImage)(self.image);
        }
    }
}

#[derive(Clone, Copy)]
struct CaptureRegion {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn clamp_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Result<CaptureRegion, String> {
    let left = i64::from(x).max(0);
    let top = i64::from(y).max(0);
    let right = (i64::from(x) + i64::from(width)).min(i64::from(screen_width));
    let bottom = (i64::from(y) + i64::from(height)).min(i64::from(screen_height));

    if right <= left || bottom <= top {
        return Err(format!(
            "截图区域超出屏幕范围: ({x}, {y}) {width}x{height}，屏幕 {screen_width}x{screen_height}"
        ));
    }

    Ok(CaptureRegion {
        x: left as i32,
        y: top as i32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

fn ximage_to_dynamic(handle: &XImageHandle<'_>) -> Result<DynamicImage, String> {
    let image = unsafe { &*handle.image };
    if image.data.is_null() {
        return Err("X11 截图数据为空".to_string());
    }
    if image.red_mask == 0 || image.green_mask == 0 || image.blue_mask == 0 {
        return Err("当前 X11 视觉格式不包含 RGB 掩码，暂不支持调色板模式截图".to_string());
    }
    if image.width <= 0 || image.height <= 0 {
        return Err("X11 返回了无效截图尺寸".to_string());
    }

    let width = image.width as u32;
    let height = image.height as u32;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);

    for row in 0..height {
        for col in 0..width {
            let pixel = read_pixel_value(handle, image, col as i32, row as i32)?;
            pixels.push(component(pixel, image.red_mask as u64));
            pixels.push(component(pixel, image.green_mask as u64));
            pixels.push(component(pixel, image.blue_mask as u64));
            pixels.push(255);
        }
    }

    RgbaImage::from_raw(width, height, pixels)
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| "创建截图图像失败".to_string())
}

fn read_pixel_value(
    handle: &XImageHandle<'_>,
    image: &xlib::XImage,
    x: i32,
    y: i32,
) -> Result<u64, String> {
    let bytes_per_pixel = match image.bits_per_pixel {
        16 => 2,
        24 => 3,
        32 => 4,
        _ => {
            return Ok(unsafe { (handle.xlib.XGetPixel)(handle.image, x, y) as u64 });
        }
    };
    if image.bytes_per_line <= 0 {
        return Err("X11 截图行跨度无效".to_string());
    }

    let offset = y as usize * image.bytes_per_line as usize + x as usize * bytes_per_pixel;
    let bytes = unsafe {
        std::slice::from_raw_parts((image.data as *const u8).add(offset), bytes_per_pixel)
    };
    Ok(read_ordered_integer(bytes, image.byte_order))
}

fn read_ordered_integer(bytes: &[u8], byte_order: i32) -> u64 {
    if byte_order == xlib::MSBFirst {
        bytes
            .iter()
            .fold(0, |value, byte| (value << 8) | u64::from(*byte))
    } else {
        bytes.iter().enumerate().fold(0, |value, (index, byte)| {
            value | (u64::from(*byte) << (index * 8))
        })
    }
}

fn component(pixel: u64, mask: u64) -> u8 {
    if mask == 0 {
        return 0;
    }

    let shift = mask.trailing_zeros();
    let value = (pixel & mask) >> shift;
    let max = mask >> shift;
    if max == 0 {
        0
    } else {
        ((value * 255 + max / 2) / max) as u8
    }
}
