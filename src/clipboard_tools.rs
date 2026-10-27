use arboard::Clipboard;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn read_system_clipboard_text() -> Result<String, String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("剪贴板初始化失败: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("读取剪贴板失败: {}", e))
}

pub fn append_system_clipboard_to_file() -> Result<PathBuf, String> {
    let text = read_system_clipboard_text()?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let output_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("clipboard_output.txt");

    let exists = output_path.exists();
    let file_not_empty = exists
        && fs::metadata(&output_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_path)
        .map_err(|e| format!("打开文件失败: {}", e))?;

    if file_not_empty {
        let _ = writeln!(file);
        let _ = writeln!(file);
    }

    write!(file, "{}", normalized).map_err(|e| format!("写入失败: {}", e))?;
    Ok(output_path)
}
