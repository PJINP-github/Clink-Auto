// html_table.rs - 扫描 <exe_dir>/temp/*.html，把每个 <table> 写进同名 .xlsx 的独立 sheet。
// 状态保存在 <exe_dir>/temp/.html_to_xlsx_state.json（记录每个 html 上次处理时的 mtime）。
use rust_xlsxwriter::{Format, Workbook};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Default, Serialize, Deserialize)]
struct HtmlState {
    #[serde(default)]
    html_mtimes: HashMap<String, u64>,
}

pub fn process_temp_html() -> Result<String, String> {
    let temp_dir = temp_dir().ok_or_else(|| "无法定位可执行文件所在目录".to_string())?;
    fs::create_dir_all(&temp_dir).map_err(|e| format!("创建 temp 目录失败: {}", e))?;
    let state_path = temp_dir.join(".html_to_xlsx_state.json");
    let mut state = load_state(&state_path)?;

    let mut entries: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = fs::read_dir(&temp_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if matches!(ext.to_ascii_lowercase().as_str(), "html" | "htm") {
                entries.push(path);
            }
        }
    }
    entries.sort();

    let mut reports: Vec<String> = Vec::new();
    let mut changed = false;
    for html_path in entries {
        let Some(file_name) = html_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
        else {
            continue;
        };
        let Some(mtime) = file_mtime_secs(&html_path) else {
            continue;
        };
        let needs = match state.html_mtimes.get(&file_name).copied() {
            Some(prev) => mtime > prev,
            None => true,
        };
        if !needs {
            continue;
        }
        let xlsx_path = html_path.with_extension("xlsx");
        match html_to_xlsx(&html_path, &xlsx_path) {
            Ok(sheet_count) => {
                let xlsx_name = xlsx_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                reports.push(format!(
                    "[OK] {} -> {} ({} sheet)",
                    file_name, xlsx_name, sheet_count
                ));
                state.html_mtimes.insert(file_name, mtime);
                changed = true;
            }
            Err(err) => {
                reports.push(format!("[FAIL] {}: {}", file_name, err));
            }
        }
    }

    if changed {
        save_state(&state_path, &state)?;
    }
    if reports.is_empty() {
        reports.push("temp/ 中没有需要转换的 html 文件".to_string());
    }
    Ok(reports.join("\n"))
}

fn html_to_xlsx(html_path: &Path, xlsx_path: &Path) -> Result<usize, String> {
    let html_text = fs::read_to_string(html_path)
        .map_err(|e| format!("读取 {} 失败: {}", html_path.display(), e))?;
    let document = Html::parse_document(&html_text);
    
    let table_selector =
        Selector::parse("table").map_err(|e| format!("table selector 失败: {:?}", e))?;
    let caption_selector =
        Selector::parse("caption").map_err(|e| format!("caption selector 失败: {:?}", e))?;
    let tr_selector = Selector::parse("tr").map_err(|e| format!("tr selector 失败: {:?}", e))?;
    let th_selector = Selector::parse("th").map_err(|e| format!("th selector 失败: {:?}", e))?;
    let td_selector = Selector::parse("td").map_err(|e| format!("td selector 失败: {:?}", e))?;

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();
    let mut sheet_count = 0usize;
    let mut used_names: HashSet<String> = HashSet::new();

    for table in document.select(&table_selector) {
        // 忽略嵌套在其他 table 内部的 table，防止把布局表格当成数据表
        let is_nested_table = table.ancestors().any(|node| {
            scraper::ElementRef::wrap(node)
                .map(|el| el.value().name() == "table")
                .unwrap_or(false)
        });
        if is_nested_table {
            continue;
        }

        let caption = table
            .select(&caption_selector)
            .next()
            .map(|c| c.text().collect::<String>().trim().to_string());
        let mut sheet_name = match caption {
            Some(name) if !name.is_empty() => sanitize_sheet_name(&name),
            _ => format!("表{}", sheet_count + 1),
        };
        if !used_names.insert(sheet_name.clone()) {
            let base = sheet_name.clone();
            let mut i = 2usize;
            loop {
                let candidate = format!("{}_{}", base, i);
                if used_names.insert(candidate.clone()) {
                    sheet_name = candidate;
                    break;
                }
                i += 1;
                if i > 10000 {
                    break;
                }
            }
        }

        let sheet = workbook.add_worksheet();
        sheet
            .set_name(&sheet_name)
            .map_err(|e| format!("设置 sheet 名失败: {}", e))?;

        let mut row = 0u32;
        for tr in table.select(&tr_selector) {
            // 忽略嵌套在其他 tr 内部的 tr
            let is_nested_tr = tr.ancestors().any(|node| {
                scraper::ElementRef::wrap(node)
                    .map(|el| el.value().name() == "tr")
                    .unwrap_or(false)
            });
            if is_nested_tr {
                continue;
            }

            // 忽略显式隐藏的行
            let is_hidden = tr.value().attr("style")
                .map(|s| {
                    let s = s.replace(" ", "").to_lowercase();
                    s.contains("display:none") || s.contains("visibility:hidden")
                })
                .unwrap_or(false);
            if is_hidden {
                continue;
            }

            let mut col = 0u16;
            for cell in tr.select(&th_selector).chain(tr.select(&td_selector)) {
                // 清理文本：去除多余的缩进，保留段落内的换行
                let raw_text = cell.text().collect::<String>();
                let text = raw_text
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                
                let is_header = cell.value().name() == "th";
                
                // 解析 colspan（跨列），如果存在则写入相应的列数
                let colspan = cell
                    .value()
                    .attr("colspan")
                    .and_then(|v| v.parse::<u16>().ok())
                    .unwrap_or(1)
                    .max(1);

                let mut write_to_cell = |c: u16| -> Result<(), String> {
                    if is_header {
                        sheet
                            .write_string_with_format(row, c, &text, &header_format)
                            .map(|_| ())
                            .map_err(|e| format!("写入表头失败: {}", e))
                    } else if let Ok(n) = text.parse::<f64>() {
                        sheet
                            .write_number(row, c, n)
                            .map(|_| ())
                            .map_err(|e| format!("写入数字失败: {}", e))
                    } else {
                        sheet
                            .write_string(row, c, &text)
                            .map(|_| ())
                            .map_err(|e| format!("写入文本失败: {}", e))
                    }
                };

                // 写入第一格
                write_to_cell(col)?;
                // 如果有跨列，把后续格子也填上相同内容（或者你可以选择留空）
                for _ in 1..colspan {
                    col += 1;
                    write_to_cell(col)?;
                }
                
                col += 1;
            }
            row += 1;
        }
        sheet_count += 1;
    }

    if sheet_count == 0 {
        return Err("html 中没有找到 <table>".to_string());
    }
    workbook
        .save(xlsx_path)
        .map_err(|e| format!("保存 xlsx 失败: {}", e))?;
    Ok(sheet_count)
}

fn sanitize_sheet_name(name: &str) -> String {
    let mut s = name.to_string();
    for ch in &['\\', '/', '?', '*', '[', ']', ':'] {
        s = s.replace(*ch, "_");
    }
    if s.chars().count() > 31 {
        s = s.chars().take(31).collect();
    }
    if s.is_empty() {
        s.push_str("Sheet");
    }
    s
}

fn file_mtime_secs(path: &Path) -> Option<u64> {
    let mtime = fs::metadata(path).ok()?.modified().ok()?;
    mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn temp_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("temp")))
        .or_else(|| std::env::current_dir().ok().map(|p| p.join("temp")))
}

fn load_state(path: &Path) -> Result<HtmlState, String> {
    if !path.exists() {
        return Ok(HtmlState::default());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("读取状态文件失败: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("解析状态文件失败: {}", e))
}

fn save_state(path: &Path, state: &HtmlState) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(state).map_err(|e| format!("序列化状态失败: {}", e))?;
    fs::write(path, text).map_err(|e| format!("写入状态文件失败: {}", e))
}