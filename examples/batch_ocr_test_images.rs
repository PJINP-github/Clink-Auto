#[path = "../src/ocr.rs"]
mod ocr;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() -> Result<(), String> {
    let image_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("development/identifying_02_ocr/test_image"));
    let output_path = image_dir.join("结果.txt");
    let mut image_paths = fs::read_dir(&image_dir)
        .map_err(|e| format!("读取图片目录失败 {}: {}", image_dir.display(), e))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && is_supported_image(path))
        .collect::<Vec<_>>();
    image_paths.sort();

    let mut report = String::new();
    report.push_str("OCR 批量推理结果\n");
    report.push_str(&format!("图片目录: {}\n", image_dir.display()));
    report.push_str(&format!("图片数量: {}\n\n", image_paths.len()));

    for path in &image_paths {
        let start = Instant::now();
        let result = ocr::ocr_file(path);
        let elapsed = start.elapsed();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        report.push_str(&format!("文件: {file_name}\n"));
        match result {
            Ok(text) => {
                report.push_str("状态: 成功\n");
                report.push_str("识别结果:\n");
                if text.is_empty() {
                    report.push_str("<空>\n");
                } else {
                    report.push_str(&text);
                    report.push('\n');
                }
            }
            Err(err) => {
                report.push_str("状态: 失败\n");
                report.push_str(&format!("错误: {err}\n"));
            }
        }
        report.push_str(&format!("推理耗时: {:.3}s\n\n", elapsed.as_secs_f64()));
    }

    fs::write(&output_path, &report)
        .map_err(|e| format!("写入测试结果失败 {}: {}", output_path.display(), e))?;
    print!("{report}");
    Ok(())
}

fn is_supported_image(path: &Path) -> bool {
    image::ImageFormat::from_path(path).is_ok()
}
