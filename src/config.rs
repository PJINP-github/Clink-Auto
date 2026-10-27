use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::config_path::config_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateCategory {
    pub name: String,
    pub coordinates: Vec<Coordinate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionStep {
    /// Click at specific desktop coordinates
    Click(i32, i32),
    /// Right-click at specific desktop coordinates
    RightClick(i32, i32),
    /// Saturation click: rows, clicks_per_row, interval_ms, region (x1,y1,x2,y2)
    SaturationClick {
        rows: u32,
        clicks_per_row: u32,
        interval_ms: u64,
        region: (i32, i32, i32, i32),
    },
    /// Delay in milliseconds
    Delay(u64),
    /// Copy clipboard content to clipboard_output.txt
    CopyClipboard,
    /// Click first coordinate of a category
    ClickCategory(String),
    /// Take a full screen screenshot and save to images/
    ScreenCapture,
    /// Take a screenshot of a configured rectangular region and save to screenshots/
    ScreenCaptureRegion { region: (i32, i32, i32, i32) },
    /// Paste clipboard content (Ctrl+V) at current focus
    Paste,
    /// Simulate keyboard sequence, e.g. "ctrl+c;500;esc"
    KeyboardSequence(String),
    /// OCR recognition in a rectangular region with optional follow-up actions
    OcrCheck {
        expected_text: String,
        region: (i32, i32, i32, i32),
        action_on_match: Option<usize>,
        action_on_mismatch: Option<usize>,
    },
    /// Read the next non-empty line from points.txt beside the executable into clipboard.
    PointsTxtNextLine { action_on_empty: Option<usize> },
    /// Execute an action supplied by an independently deployed patch package.
    Patch { patch_id: String, action_id: String },
    /// Scan `temp/` for HTML files and convert any `<table>` to xlsx.
    HtmlToXlsx,
    /// OCR a rectangular region (multi-line) and save the result to `<exe_dir>/temp/ocr_<prefix>_<ts>.txt`.
    OcrSaveMultiLine {
        region: (i32, i32, i32, i32),
        filename_prefix: String,
        use_full_pipeline: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub hotkey: String,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub cooldown_ms: u64,
    pub default_action: usize,
    pub hotkey_overrides: Vec<HotkeyOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyOverride {
    pub hotkey: String,
    pub action_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub coordinate_categories: Vec<CoordinateCategory>,
    pub actions: Vec<Action>,
    pub run_config: RunConfig,
    #[serde(default)]
    pub passwd: bool,
    #[serde(default = "default_visible_tabs")]
    pub visible_tabs: Vec<String>,
}

fn default_visible_tabs() -> Vec<String> {
    vec!["actions".to_string(), "run".to_string()]
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            coordinate_categories: vec![],
            actions: vec![],
            run_config: RunConfig {
                cooldown_ms: 1000,
                default_action: 0,
                hotkey_overrides: vec![],
            },
            passwd: false,
            visible_tabs: default_visible_tabs(),
        }
    }
}

impl Configuration {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let content = serde_yaml::to_string(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        config_path()
    }
}
