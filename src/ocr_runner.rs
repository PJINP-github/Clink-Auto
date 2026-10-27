use crate::ocr;

pub fn recognize_region_text(region: (i32, i32, i32, i32)) -> Result<String, String> {
    Ok(recognize_region_text_timed(region)?.0)
}

pub fn recognize_region_text_timed(region: (i32, i32, i32, i32)) -> Result<(String, f64), String> {
    let (x1, y1, x2, y2) = region;
    let width = (x2 - x1).max(1) as u32;
    let height = (y2 - y1).max(1) as u32;
    let screen = capture_region(x1, y1, width, height)?;
    ocr::ocr_image_timed(&screen.to_rgb8()).map_err(|e| format!("OCR recognition failed: {e}"))
}

pub fn recognize_region_text_timed_full(
    region: (i32, i32, i32, i32),
) -> Result<(String, f64), String> {
    let (x1, y1, x2, y2) = region;
    let width = (x2 - x1).max(1) as u32;
    let height = (y2 - y1).max(1) as u32;
    let screen = capture_region(x1, y1, width, height)?;
    ocr::ocr_image_timed_full(&screen.to_rgb8())
        .map_err(|e| format!("OCR (full) recognition failed: {e}"))
}

#[cfg(windows)]
fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<image::DynamicImage, String> {
    crate::windows_capture::capture_region(x, y, width, height)
}

#[cfg(target_os = "linux")]
fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<image::DynamicImage, String> {
    crate::linux_capture::capture_region(x, y, width, height)
}
