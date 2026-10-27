use image::RgbImage;
use rapidocr_core::{
    config::PipelineConfig,
    model::{ModelCache, ModelDownloadMode, PPOCRV5_CH_MOBILE},
    RapidOcr,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const EMBEDDED_MODELS: &[EmbeddedModel] = &[
    EmbeddedModel {
        name: "ch_PP-OCRv5_rec_mobile.onnx",
        bytes: include_bytes!("../models/rapidocr/ch_PP-OCRv5_rec_mobile.onnx"),
    },
    EmbeddedModel {
        name: "ppocrv5_dict.txt",
        bytes: include_bytes!("../models/rapidocr/ppocrv5_dict.txt"),
    },
    EmbeddedModel {
        name: "ch_PP-OCRv5_det_mobile.onnx",
        bytes: include_bytes!("../models/rapidocr/ch_PP-OCRv5_det_mobile.onnx"),
    },
];

struct EmbeddedModel {
    name: &'static str,
    bytes: &'static [u8],
}

static OCR_ENGINE: OnceLock<Result<Mutex<RapidOcr>, String>> = OnceLock::new();
static OCR_ENGINE_FULL: OnceLock<Result<Mutex<RapidOcr>, String>> = OnceLock::new();

#[allow(dead_code)]
pub fn ocr_file(image_path: impl AsRef<Path>) -> Result<String, String> {
    let image_path = image_path.as_ref();
    if !image_path.exists() {
        return Err(format!(
            "image file does not exist: {}",
            image_path.display()
        ));
    }

    let image = image::open(image_path).map_err(|e| e.to_string())?;
    ocr_image(&image.to_rgb8())
}

#[allow(dead_code)]
pub fn ocr_image(image: &RgbImage) -> Result<String, String> {
    Ok(ocr_image_timed(image)?.0)
}

pub fn ocr_image_timed(image: &RgbImage) -> Result<(String, f64), String> {
    let mut engine = engine()?.lock().map_err(|_| "OCR engine lock poisoned")?;
    let timed = engine.run_image_timed(image).map_err(|e| e.to_string())?;
    let text = timed
        .output
        .lines
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    Ok((text, timed.timings.total_ms))
}

pub fn ocr_image_timed_full(image: &RgbImage) -> Result<(String, f64), String> {
    let mut engine = engine_full()?.lock().map_err(|_| "OCR engine lock poisoned")?;
    let timed = engine.run_image_timed(image).map_err(|e| e.to_string())?;
    let text = timed
        .output
        .lines
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    Ok((text, timed.timings.total_ms))
}

pub fn warm_up() {
    let Ok(engine) = engine() else {
        return;
    };
    if let Ok(mut engine) = engine.lock() {
        let image = RgbImage::from_pixel(64, 64, image::Rgb([255, 255, 255]));
        let _ = engine.run_image_timed(&image);
    }
}

fn engine() -> Result<&'static Mutex<RapidOcr>, String> {
    let entry = OCR_ENGINE.get_or_init(init_engine);
    match entry {
        Ok(engine) => Ok(engine),
        Err(err) => Err(err.clone()),
    }
}

fn engine_full() -> Result<&'static Mutex<RapidOcr>, String> {
    let entry = OCR_ENGINE_FULL.get_or_init(init_engine_full);
    match entry {
        Ok(engine) => Ok(engine),
        Err(err) => Err(err.clone()),
    }
}

fn init_engine() -> Result<Mutex<RapidOcr>, String> {
    let model_dir = std::env::var_os("RAPIDOCR_MODEL_DIR")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(resolve_default_model_dir)?;
    let model_diagnostics = model_dir_diagnostics(&model_dir);

    // The default OCR region is one horizontal Chinese line. Recognition-only
    // mode avoids detector over-segmentation on small UI labels and is faster.
    let pipeline = PipelineConfig::recognition_only();
    let cache = ModelCache::new(model_dir);
    cache
        .ensure_model_set_for_pipeline(&PPOCRV5_CH_MOBILE, pipeline, ModelDownloadMode::Never)
        .map_err(|e| format!("OCR model preparation failed: {e:#}\n{model_diagnostics}"))?;

    let mut cfg = cache.config_for(&PPOCRV5_CH_MOBILE).with_pipeline(pipeline);
    cfg.text_score = 0.5;
    let available_threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    cfg.inference.intra_threads = available_threads.min(4).max(1);
    cfg.inference.inter_threads = 1;
    cfg.inference.enable_cpu_mem_arena = true;

    RapidOcr::from_config(cfg)
        .map(Mutex::new)
        .map_err(|e| format!("OCR engine initialization failed: {e:#}\n{model_diagnostics}"))
}

fn init_engine_full() -> Result<Mutex<RapidOcr>, String> {
    let model_dir = std::env::var_os("RAPIDOCR_MODEL_DIR")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(resolve_default_model_dir)?;
    let model_diagnostics = model_dir_diagnostics(&model_dir);
    // 全流程：det + rec，用于多行文本识别
    // 若 rapidocr-core 的 PipelineConfig 没有实现 Default，
    // 请替换为 det+rec 全流程构造函数（如 PipelineConfig::det_and_rec()）。
    let pipeline = PipelineConfig::default();
    let cache = ModelCache::new(model_dir);
    cache
        .ensure_model_set_for_pipeline(&PPOCRV5_CH_MOBILE, pipeline, ModelDownloadMode::Never)
        .map_err(|e| format!("OCR model preparation failed: {e:#}\n{model_diagnostics}"))?;
    let mut cfg = cache
        .config_for(&PPOCRV5_CH_MOBILE)
        .with_pipeline(pipeline);
    cfg.text_score = 0.5;
    let available_threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    cfg.inference.intra_threads = available_threads.min(4).max(1);
    cfg.inference.inter_threads = 1;
    cfg.inference.enable_cpu_mem_arena = true;
    RapidOcr::from_config(cfg)
        .map(Mutex::new)
        .map_err(|e| format!("OCR engine (full) initialization failed: {e:#}\n{model_diagnostics}"))
}

fn model_dir_diagnostics(model_dir: &Path) -> String {
    let file_status = EMBEDDED_MODELS
        .iter()
        .map(|model| {
            let path = model_dir.join(model.name);
            let status = if path.is_file() { "present" } else { "missing" };
            format!("  - {}: {status}", model.name)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "OCR model directory: {}\nRAPIDOCR_MODEL_DIR: {}\nModel files:\n{}",
        model_dir.display(),
        std::env::var_os("RAPIDOCR_MODEL_DIR")
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "not set".to_string()),
        file_status
    )
}

fn resolve_default_model_dir() -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models/rapidocr"),
        PathBuf::from("models/rapidocr"),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("models/rapidocr")))
            .unwrap_or_else(|| PathBuf::from("models/rapidocr")),
    ];

    for candidate in candidates {
        if has_required_models(&candidate) {
            return Ok(candidate);
        }
    }

    materialize_embedded_models()
}

fn has_required_models(model_dir: &Path) -> bool {
    EMBEDDED_MODELS
        .iter()
        .all(|model| model_dir.join(model.name).is_file())
}

fn materialize_embedded_models() -> Result<PathBuf, String> {
    let model_dir = embedded_model_cache_dir();
    fs::create_dir_all(&model_dir).map_err(|e| {
        format!(
            "创建内置 OCR 模型缓存目录失败 {}: {}",
            model_dir.display(),
            e
        )
    })?;

    for model in EMBEDDED_MODELS {
        let path = model_dir.join(model.name);
        let needs_write = fs::metadata(&path)
            .map(|metadata| metadata.len() != model.bytes.len() as u64)
            .unwrap_or(true);
        if needs_write {
            fs::write(&path, model.bytes)
                .map_err(|e| format!("释放内置 OCR 模型失败 {}: {}", path.display(), e))?;
        }
    }

    Ok(model_dir)
}

fn embedded_model_cache_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("CLICK_AUTOMATION_CACHE_DIR") {
        return PathBuf::from(path).join("rapidocr");
    }

    platform_cache_root()
        .unwrap_or_else(std::env::temp_dir)
        .join("click_automation")
        .join("rapidocr")
}

fn platform_cache_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
            return Some(PathBuf::from(path));
        }
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".cache"))
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}
