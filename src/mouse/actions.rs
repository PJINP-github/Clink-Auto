#[cfg(not(windows))]
use enigo::{Button, Coordinate, Mouse};
use enigo::{Direction, Enigo, Keyboard, Settings};
use std::thread;
use std::time::{Duration, Instant};

pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::platform::click_at(x, y);
        thread::sleep(Duration::from_millis(50));
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        if crate::wayland_portal::is_wayland_session() {
            crate::wayland_portal::click_at(x, y)?;
            thread::sleep(Duration::from_millis(50));
            return Ok(());
        }

        let mut enigo = create_enigo()?;
        enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| format!("移动鼠标失败: {}", e))?;
        thread::sleep(Duration::from_millis(30));
        enigo
            .button(Button::Left, Direction::Click)
            .map_err(|e| format!("点击鼠标失败: {}", e))?;
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }
}

pub fn right_click_at(x: i32, y: i32) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::platform::right_click_at(x, y);
        thread::sleep(Duration::from_millis(50));
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let mut enigo = create_enigo()?;
        enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| format!("移动鼠标失败: {}", e))?;
        thread::sleep(Duration::from_millis(30));
        enigo
            .button(Button::Right, Direction::Click)
            .map_err(|e| format!("右键点击鼠标失败: {}", e))?;
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }
}

pub fn saturation_click(
    rows: u32,
    clicks_per_row: u32,
    interval_ms: u64,
    region: (i32, i32, i32, i32),
) -> Result<(), String> {
    let (x1, y1, x2, y2) = region;
    if x1 == 0 && y1 == 0 && x2 == 0 && y2 == 0 {
        return Ok(());
    }

    let left = x1.min(x2);
    let top = y1.min(y2);
    let right = x1.max(x2);
    let bottom = y1.max(y2);
    let w = (right - left).max(1);
    let h = (bottom - top).max(1);
    let rows = rows.clamp(1, 3);
    let per_row = clicks_per_row.max(1);

    eprintln!(
        "[Saturation] Region {}x{} | {} rows x {} clicks/row | Interval {}ms",
        w, h, rows, per_row, interval_ms
    );
    #[cfg(not(windows))]
    let mut enigo = if crate::wayland_portal::is_wayland_session() {
        None
    } else {
        Some(create_enigo()?)
    };

    let start = Instant::now();
    let mut clicked = 0u32;
    for row in 0..rows {
        let y = top + ((row + 1) as i32 * h) / (rows as i32 + 1);
        for j in (0..per_row).rev() {
            let x = left + ((j + 1) as i32 * w) / (per_row as i32 + 1);
            #[cfg(windows)]
            crate::platform::click_at(x, y);
            #[cfg(not(windows))]
            {
                if crate::wayland_portal::is_wayland_session() {
                    crate::wayland_portal::click_at(x, y)?;
                } else if let Some(enigo) = enigo.as_mut() {
                    enigo
                        .move_mouse(x, y, Coordinate::Abs)
                        .map_err(|e| format!("移动鼠标失败: {}", e))?;
                    thread::sleep(Duration::from_millis(5));
                    enigo
                        .button(Button::Left, Direction::Click)
                        .map_err(|e| format!("点击鼠标失败: {}", e))?;
                }
            }
            clicked += 1;
            if interval_ms > 0 {
                thread::sleep(Duration::from_millis(interval_ms));
            }
        }
    }
    eprintln!(
        "[Saturation] Completed {} clicks in {:.1}ms",
        clicked,
        start.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}

pub fn paste() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::wayland_portal::is_wayland_session() {
        return crate::wayland_portal::paste();
    }

    let mut enigo = create_enigo()?;
    enigo
        .key(enigo::Key::Control, Direction::Press)
        .map_err(|e| format!("按下 Ctrl 失败: {}", e))?;
    enigo
        .key(enigo::Key::Unicode('v'), Direction::Press)
        .map_err(|e| format!("按下 V 失败: {}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo
        .key(enigo::Key::Unicode('v'), Direction::Release)
        .map_err(|e| format!("释放 V 失败: {}", e))?;
    enigo
        .key(enigo::Key::Control, Direction::Release)
        .map_err(|e| format!("释放 Ctrl 失败: {}", e))?;
    thread::sleep(Duration::from_millis(50));
    Ok(())
}

pub fn send_key_sequence(sequence: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::wayland_portal::is_wayland_session() {
        return crate::wayland_portal::send_key_sequence(sequence);
    }

    let mut enigo = create_enigo()?;
    for raw_part in sequence.split(';') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }
        if let Ok(delay_ms) = part.parse::<u64>() {
            thread::sleep(Duration::from_millis(delay_ms));
            continue;
        }

        send_key_sequence_part(&mut enigo, part)?;
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn send_key_sequence_part(enigo: &mut Enigo, part: &str) -> Result<(), String> {
    let mut modifiers = Vec::new();
    let mut key = None;

    for raw_token in part.split('+') {
        let token = raw_token.trim().to_lowercase();
        if token.is_empty() {
            continue;
        }
        match token.as_str() {
            "ctrl" | "control" => modifiers.push(enigo::Key::Control),
            "alt" => modifiers.push(enigo::Key::Alt),
            "shift" => modifiers.push(enigo::Key::Shift),
            "meta" | "super" | "win" => modifiers.push(enigo::Key::Meta),
            other if key.is_none() => key = Some(parse_key(other)?),
            other => return Err(format!("不支持的组合键片段: {}", other)),
        }
    }

    let Some(key) = key else {
        return Err(format!("按键序列片段缺少主键: {}", part));
    };

    for modifier in &modifiers {
        enigo
            .key(*modifier, Direction::Press)
            .map_err(|e| format!("按下修饰键失败: {}", e))?;
    }
    enigo
        .key(key, Direction::Click)
        .map_err(|e| format!("发送按键失败: {}", e))?;
    for modifier in modifiers.iter().rev() {
        enigo
            .key(*modifier, Direction::Release)
            .map_err(|e| format!("释放修饰键失败: {}", e))?;
    }
    Ok(())
}

fn create_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| {
        format!(
            "创建输入器失败: {}。Linux 完整自动化需要 X11/XWayland 会话和可用的 DISPLAY",
            e
        )
    })
}

fn parse_key(token: &str) -> Result<enigo::Key, String> {
    let key = match token {
        "esc" | "escape" => enigo::Key::Escape,
        "enter" | "return" => enigo::Key::Return,
        "tab" => enigo::Key::Tab,
        "space" => enigo::Key::Space,
        "backspace" | "back_space" => enigo::Key::Backspace,
        "delete" | "del" => enigo::Key::Delete,
        "insert" | "ins" => enigo::Key::Insert,
        "up" => enigo::Key::UpArrow,
        "down" => enigo::Key::DownArrow,
        "left" => enigo::Key::LeftArrow,
        "right" => enigo::Key::RightArrow,
        "home" => enigo::Key::Home,
        "end" => enigo::Key::End,
        "pageup" | "page_up" | "prior" => enigo::Key::PageUp,
        "pagedown" | "page_down" | "next" => enigo::Key::PageDown,
        "f1" => enigo::Key::F1,
        "f2" => enigo::Key::F2,
        "f3" => enigo::Key::F3,
        "f4" => enigo::Key::F4,
        "f5" => enigo::Key::F5,
        "f6" => enigo::Key::F6,
        "f7" => enigo::Key::F7,
        "f8" => enigo::Key::F8,
        "f9" => enigo::Key::F9,
        "f10" => enigo::Key::F10,
        "f11" => enigo::Key::F11,
        "f12" => enigo::Key::F12,
        "printscreen" | "print" => enigo::Key::PrintScr,
        "pause" => enigo::Key::Pause,
        "minus" => enigo::Key::Unicode('-'),
        "equal" => enigo::Key::Unicode('='),
        "bracketleft" => enigo::Key::Unicode('['),
        "bracketright" => enigo::Key::Unicode(']'),
        "semicolon" => enigo::Key::Unicode(';'),
        "apostrophe" => enigo::Key::Unicode('\''),
        "comma" => enigo::Key::Unicode(','),
        "period" => enigo::Key::Unicode('.'),
        "slash" => enigo::Key::Unicode('/'),
        "backslash" => enigo::Key::Unicode('\\'),
        "grave" => enigo::Key::Unicode('`'),
        key if key.len() == 1 => {
            let ch = key.chars().next().unwrap();
            enigo::Key::Unicode(ch)
        }
        other => return Err(format!("不支持的按键: {}", other)),
    };
    Ok(key)
}
