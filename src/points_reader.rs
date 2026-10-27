use arboard::Clipboard;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static POINTS_CURSOR: OnceLock<Mutex<PointsCursor>> = OnceLock::new();

#[derive(Default)]
struct PointsCursor {
    path: Option<PathBuf>,
    lines: Vec<String>,
    next_index: usize,
}

pub enum PointsReadOutcome {
    Copied { value: String, line_number: usize },
    EmptyLine { line_number: usize },
    EndOfFile,
}

pub fn copy_next_point_to_clipboard() -> Result<PointsReadOutcome, String> {
    let path = points_path();
    let cursor = POINTS_CURSOR.get_or_init(|| Mutex::new(PointsCursor::default()));
    let mut cursor = cursor
        .lock()
        .map_err(|_| "points.txt 读取状态锁已被污染".to_string())?;
    cursor.ensure_loaded(path)?;

    if cursor.next_index >= cursor.lines.len() {
        return Ok(PointsReadOutcome::EndOfFile);
    }

    let line_number = cursor.next_index + 1;
    let value = cursor.lines[cursor.next_index].clone();
    cursor.next_index += 1;

    if value.trim().is_empty() {
        return Ok(PointsReadOutcome::EmptyLine { line_number });
    }

    Clipboard::new()
        .map_err(|e| format!("剪贴板初始化失败: {}", e))?
        .set_text(value.clone())
        .map_err(|e| format!("写入剪贴板失败: {}", e))?;

    Ok(PointsReadOutcome::Copied { value, line_number })
}

fn points_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("points.txt")))
        .unwrap_or_else(|| PathBuf::from("points.txt"))
}

impl PointsCursor {
    fn ensure_loaded(&mut self, path: PathBuf) -> Result<(), String> {
        if self.path.as_ref() == Some(&path) && !self.lines.is_empty() {
            return Ok(());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取 points.txt 失败 ({}): {}", path.display(), e))?;
        self.lines = content.lines().map(str::to_string).collect();
        self.next_index = 0;
        self.path = Some(path);
        Ok(())
    }
}
