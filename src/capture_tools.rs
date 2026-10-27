use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub fn save_full_screen_capture(base_dir: impl AsRef<Path>) -> Result<PathBuf, String> {
    let filepath = screenshot_path(base_dir, "screenshot")?;

    #[cfg(windows)]
    {
        let image = crate::windows_capture::capture_full_screen()?;
        image
            .save(&filepath)
            .map_err(|e| format!("保存截图失败: {}", e))?;
        return Ok(filepath);
    }

    #[cfg(target_os = "linux")]
    {
        let image = crate::linux_capture::capture_full_screen()?;
        image
            .save(&filepath)
            .map_err(|e| format!("保存截图失败: {}", e))?;
        return Ok(filepath);
    }
}

pub fn save_region_capture(
    base_dir: impl AsRef<Path>,
    region: (i32, i32, i32, i32),
    filename_hint: Option<&str>,
) -> Result<PathBuf, String> {
    let (x, y, width, height) = region_dimensions(region)?;
    let filepath = region_screenshot_path(base_dir, filename_hint)?;

    #[cfg(windows)]
    {
        let image = crate::windows_capture::capture_region(x, y, width, height)?;
        image
            .save(&filepath)
            .map_err(|e| format!("保存区域截图失败: {}", e))?;
        return Ok(filepath);
    }

    #[cfg(target_os = "linux")]
    {
        let image = crate::linux_capture::capture_region(x, y, width, height)?;
        image
            .save(&filepath)
            .map_err(|e| format!("保存区域截图失败: {}", e))?;
        return Ok(filepath);
    }
}

fn region_screenshot_path(
    base_dir: impl AsRef<Path>,
    filename_hint: Option<&str>,
) -> Result<PathBuf, String> {
    let screenshots_dir = screenshots_dir(base_dir)?;
    if let Some(filename) = filename_hint.and_then(clipboard_region_filename) {
        return Ok(unique_path(screenshots_dir.join(filename)));
    }

    default_screenshot_path_in(screenshots_dir, "screenshot_region")
}

fn screenshot_path(base_dir: impl AsRef<Path>, prefix: &str) -> Result<PathBuf, String> {
    default_screenshot_path_in(screenshots_dir(base_dir)?, prefix)
}

fn screenshots_dir(base_dir: impl AsRef<Path>) -> Result<PathBuf, String> {
    let screenshots_dir = base_dir.as_ref().join("screenshots");
    fs::create_dir_all(&screenshots_dir)
        .map_err(|e| format!("创建 screenshots 目录失败: {}", e))?;
    Ok(screenshots_dir)
}

fn default_screenshot_path_in(screenshots_dir: PathBuf, prefix: &str) -> Result<PathBuf, String> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    Ok(screenshots_dir.join(format!("{}_{}.png", prefix, timestamp)))
}

fn clipboard_region_filename(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.contains('\n')
        || trimmed.contains('\r')
        || trimmed.ends_with('.')
        || trimmed.ends_with(' ')
        || trimmed.encode_utf16().count() > 240
        || trimmed.chars().any(is_invalid_filename_char)
    {
        return None;
    }

    let path = Path::new(trimmed);
    if path.file_name().and_then(|name| name.to_str()) != Some(trimmed) {
        return None;
    }

    let stem = path.file_stem()?.to_str()?.trim();
    if stem.is_empty() || is_windows_reserved_filename(stem) {
        return None;
    }

    if trimmed.to_ascii_lowercase().ends_with(".png") {
        Some(trimmed.to_string())
    } else {
        Some(format!("{}.png", trimmed))
    }
}

fn is_invalid_filename_char(ch: char) -> bool {
    ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
}

fn is_windows_reserved_filename(stem: &str) -> bool {
    matches!(
        stem.trim_end_matches('.').to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let Some(parent) = path.parent() else {
        return path;
    };
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return path;
    };
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");

    for index in 1..=999 {
        let candidate = parent.join(format!("{}_{}.{}", stem, index, extension));
        if !candidate.exists() {
            return candidate;
        }
    }

    path
}

fn region_dimensions(region: (i32, i32, i32, i32)) -> Result<(i32, i32, u32, u32), String> {
    let (x1, y1, x2, y2) = region;
    let width = x2
        .checked_sub(x1)
        .filter(|width| *width > 0)
        .ok_or_else(|| "截图区域无效，请先设置有效区域".to_string())?;
    let height = y2
        .checked_sub(y1)
        .filter(|height| *height > 0)
        .ok_or_else(|| "截图区域无效，请先设置有效区域".to_string())?;
    Ok((x1, y1, width as u32, height as u32))
}

#[cfg(test)]
mod tests {
    use super::{clipboard_region_filename, region_dimensions};

    #[test]
    fn region_dimensions_normalize_to_capture_size() {
        assert_eq!(region_dimensions((10, 20, 110, 70)), Ok((10, 20, 100, 50)));
    }

    #[test]
    fn region_dimensions_reject_empty_regions() {
        assert!(region_dimensions((10, 20, 10, 70)).is_err());
        assert!(region_dimensions((10, 20, 110, 20)).is_err());
    }

    #[test]
    fn clipboard_region_filename_uses_plain_text_as_png_name() {
        assert_eq!(
            clipboard_region_filename("35kV庙金线313避雷器A相表计外观"),
            Some("35kV庙金线313避雷器A相表计外观.png".to_string())
        );
        assert_eq!(
            clipboard_region_filename("report.PNG"),
            Some("report.PNG".to_string())
        );
        assert_eq!(
            clipboard_region_filename("report.v1"),
            Some("report.v1.png".to_string())
        );
    }

    #[test]
    fn clipboard_region_filename_rejects_unsafe_values() {
        assert_eq!(clipboard_region_filename(""), None);
        assert_eq!(clipboard_region_filename("line1\nline2"), None);
        assert_eq!(clipboard_region_filename("dir/name"), None);
        assert_eq!(clipboard_region_filename("bad:name"), None);
        assert_eq!(clipboard_region_filename("CON"), None);
        assert_eq!(clipboard_region_filename(&"a".repeat(241)), None);
    }
}
