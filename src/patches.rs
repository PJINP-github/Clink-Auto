use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PatchActionDefinition {
    pub patch_id: String,
    pub action_id: String,
    pub patch_name: String,
    pub name: String,
}

impl PatchActionDefinition {
    pub fn display_name(&self) -> String {
        format!("{}: {}", self.patch_name, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct PatchTabDefinition {
    pub patch_id: String,
    pub tab_id: String,
    pub patch_name: String,
    pub title: String,
    pub refresh_seconds: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatchTabSnapshot {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub missing_lines: Vec<String>,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PatchManifest {
    id: String,
    version: String,
    name: String,
    #[serde(default)]
    actions: Vec<PatchActionManifest>,
    #[serde(default)]
    tabs: Vec<PatchTabManifest>,
}

#[derive(Debug, Clone, Deserialize)]
struct PatchActionManifest {
    id: String,
    name: String,
    command: CommandSpec,
}

#[derive(Debug, Clone, Deserialize)]
struct PatchTabManifest {
    id: String,
    title: String,
    command: CommandSpec,
    #[serde(default = "default_refresh_seconds")]
    refresh_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CommandSpec {
    Path(String),
    Detailed {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

impl CommandSpec {
    fn command_path(&self) -> &str {
        match self {
            Self::Path(path) => path,
            Self::Detailed { command, .. } => command,
        }
    }

    fn args(&self) -> &[String] {
        match self {
            Self::Path(_) => &[],
            Self::Detailed { args, .. } => args,
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedPatch {
    manifest: PatchManifest,
    directory: PathBuf,
}

#[derive(Debug)]
struct PatchRegistryData {
    executable_directory: PathBuf,
    patches: Vec<LoadedPatch>,
    actions: Vec<PatchActionDefinition>,
}

#[derive(Clone, Debug)]
pub struct PatchRegistry {
    inner: Arc<PatchRegistryData>,
}

impl PatchRegistry {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(PatchRegistryData {
                executable_directory: executable_directory(),
                patches: Vec::new(),
                actions: Vec::new(),
            }),
        }
    }

    pub fn action_definitions(&self) -> &[PatchActionDefinition] {
        &self.inner.actions
    }

    pub fn execute_action(
        &self,
        patch_id: &str,
        action_id: &str,
        add_log: &mut impl FnMut(String),
    ) -> Result<(), String> {
        let patch = self
            .inner
            .patches
            .iter()
            .find(|patch| patch.manifest.id == patch_id)
            .ok_or_else(|| format!("补丁不存在: {}", patch_id))?;
        let action = patch
            .manifest
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .ok_or_else(|| format!("补丁动作不存在: {} / {}", patch_id, action_id))?;

        let output = run_command(
            &self.inner.executable_directory,
            patch,
            &action.command,
            action_id,
        )?;
        log_process_output(add_log, "PATCH_ACTION", &output);
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "补丁动作执行失败: {} / {} (exit={})",
                patch_id,
                action_id,
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        }
    }

    fn execute_tab(&self, definition: &PatchTabDefinition) -> Result<PatchTabSnapshot, String> {
        let patch = self
            .inner
            .patches
            .iter()
            .find(|patch| patch.manifest.id == definition.patch_id)
            .ok_or_else(|| format!("补丁不存在: {}", definition.patch_id))?;
        let tab = patch
            .manifest
            .tabs
            .iter()
            .find(|tab| tab.id == definition.tab_id)
            .ok_or_else(|| {
                format!(
                    "补丁 Tab 不存在: {} / {}",
                    definition.patch_id, definition.tab_id
                )
            })?;

        let output = run_command(
            &self.inner.executable_directory,
            patch,
            &tab.command,
            &definition.tab_id,
        )?;
        if !output.status.success() {
            return Err(format!(
                "补丁 Tab 刷新失败: {} / {} (exit={}){}",
                definition.patch_id,
                definition.tab_id,
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                process_stderr(&output)
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(text.trim()).map_err(|error| {
            format!(
                "补丁 Tab 返回数据无效: {} / {}: {}",
                definition.patch_id, definition.tab_id, error
            )
        })
    }
}

pub struct PatchHost {
    registry: PatchRegistry,
    tabs: Vec<PatchTabRuntime>,
}

struct PatchTabRuntime {
    definition: PatchTabDefinition,
    snapshot: PatchTabSnapshot,
    error: Option<String>,
    receiver: Option<Receiver<Result<PatchTabSnapshot, String>>>,
    next_refresh: Instant,
}

impl PatchHost {
    pub fn discover() -> (Self, Vec<String>) {
        let executable_directory = executable_directory();
        let patches_directory = executable_directory.join("patches");
        let mut warnings = Vec::new();
        let mut loaded_patches = Vec::new();

        if patches_directory.exists() {
            match fs::read_dir(&patches_directory) {
                Ok(entries) => {
                    let mut directories = entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| path.is_dir())
                        .collect::<Vec<_>>();
                    directories.sort();

                    for directory in directories {
                        match load_patch(&directory) {
                            Ok(patch) => loaded_patches.push(patch),
                            Err(error) => warnings.push(error),
                        }
                    }
                }
                Err(error) => warnings.push(format!(
                    "读取补丁目录失败 {}: {}",
                    patches_directory.display(),
                    error
                )),
            }
        }

        let mut used_patch_ids = HashSet::new();
        let mut patches = Vec::new();
        for patch in loaded_patches {
            if !used_patch_ids.insert(patch.manifest.id.clone()) {
                warnings.push(format!("忽略重复补丁 ID: {}", patch.manifest.id));
                continue;
            }
            patches.push(patch);
        }

        let mut actions = Vec::new();
        let mut tabs = Vec::new();
        for patch in &patches {
            for action in &patch.manifest.actions {
                actions.push(PatchActionDefinition {
                    patch_id: patch.manifest.id.clone(),
                    action_id: action.id.clone(),
                    patch_name: patch.manifest.name.clone(),
                    name: action.name.clone(),
                });
            }
            for tab in &patch.manifest.tabs {
                tabs.push(PatchTabDefinition {
                    patch_id: patch.manifest.id.clone(),
                    tab_id: tab.id.clone(),
                    patch_name: patch.manifest.name.clone(),
                    title: tab.title.clone(),
                    refresh_seconds: tab.refresh_seconds.max(1),
                });
            }
        }

        let registry = PatchRegistry {
            inner: Arc::new(PatchRegistryData {
                executable_directory,
                patches,
                actions,
            }),
        };
        let now = Instant::now();
        let tabs = tabs
            .into_iter()
            .map(|definition| PatchTabRuntime {
                definition,
                snapshot: PatchTabSnapshot::default(),
                error: None,
                receiver: None,
                next_refresh: now,
            })
            .collect();

        (Self { registry, tabs }, warnings)
    }

    pub fn registry(&self) -> PatchRegistry {
        self.registry.clone()
    }

    pub fn action_definitions(&self) -> &[PatchActionDefinition] {
        self.registry.action_definitions()
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn tab_definition(&self, index: usize) -> Option<&PatchTabDefinition> {
        self.tabs.get(index).map(|tab| &tab.definition)
    }

    pub fn tab_snapshot(&self, index: usize) -> Option<&PatchTabSnapshot> {
        self.tabs.get(index).map(|tab| &tab.snapshot)
    }

    pub fn tab_error(&self, index: usize) -> Option<&str> {
        self.tabs.get(index).and_then(|tab| tab.error.as_deref())
    }

    pub fn tab_loading(&self, index: usize) -> bool {
        self.tabs
            .get(index)
            .and_then(|tab| tab.receiver.as_ref())
            .is_some()
    }

    pub fn poll(&mut self) {
        let now = Instant::now();
        for tab in &mut self.tabs {
            let mut finished = false;
            if let Some(receiver) = tab.receiver.as_ref() {
                match receiver.try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(snapshot) => {
                                tab.snapshot = snapshot;
                                tab.error = None;
                            }
                            Err(error) => tab.error = Some(error),
                        }
                        finished = true;
                    }
                    Err(TryRecvError::Disconnected) => {
                        tab.error = Some("补丁 Tab 刷新线程已断开".to_string());
                        finished = true;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
            if finished {
                tab.receiver = None;
            }

            if tab.receiver.is_none() && now >= tab.next_refresh {
                let registry = self.registry.clone();
                let definition = tab.definition.clone();
                let (sender, receiver) = mpsc::channel();
                tab.receiver = Some(receiver);
                tab.next_refresh = now + Duration::from_secs(definition.refresh_seconds.max(1));
                thread::spawn(move || {
                    let result = registry.execute_tab(&definition);
                    let _ = sender.send(result);
                });
            }
        }
    }
}

fn default_refresh_seconds() -> u64 {
    5
}

fn executable_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_patch(directory: &Path) -> Result<LoadedPatch, String> {
    let manifest_path = directory.join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("读取补丁清单失败 {}: {}", manifest_path.display(), error))?;
    let manifest: PatchManifest = serde_json::from_str(&content)
        .map_err(|error| format!("解析补丁清单失败 {}: {}", manifest_path.display(), error))?;

    if manifest.id.trim().is_empty() {
        return Err(format!("补丁 ID 为空: {}", manifest_path.display()));
    }
    if manifest.name.trim().is_empty() {
        return Err(format!("补丁名称为空: {}", manifest_path.display()));
    }
    if manifest.version.trim().is_empty() {
        return Err(format!("补丁版本为空: {}", manifest_path.display()));
    }

    let canonical_directory = fs::canonicalize(directory)
        .map_err(|error| format!("解析补丁目录失败 {}: {}", directory.display(), error))?;
    for action in &manifest.actions {
        validate_command(&canonical_directory, &action.command)
            .map_err(|error| format!("补丁动作无效 {} / {}: {}", manifest.id, action.id, error))?;
    }
    for tab in &manifest.tabs {
        validate_command(&canonical_directory, &tab.command)
            .map_err(|error| format!("补丁 Tab 无效 {} / {}: {}", manifest.id, tab.id, error))?;
    }

    Ok(LoadedPatch {
        manifest,
        directory: canonical_directory,
    })
}

fn validate_command(directory: &Path, spec: &CommandSpec) -> Result<(), String> {
    resolve_command_path(directory, spec.command_path()).map(|_| ())
}

fn resolve_command_path(directory: &Path, command_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(command_path);
    if relative.is_absolute() {
        return Err("命令路径必须是补丁目录内的相对路径".to_string());
    }

    let candidate = directory.join(relative);
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("命令文件不可用 {}: {}", candidate.display(), error))?;
    if !canonical.starts_with(directory) {
        return Err("命令路径不能跳出补丁目录".to_string());
    }
    if !canonical.is_file() {
        return Err(format!("命令不是文件: {}", canonical.display()));
    }
    Ok(canonical)
}

fn run_command(
    executable_directory: &Path,
    patch: &LoadedPatch,
    spec: &CommandSpec,
    operation_id: &str,
) -> Result<Output, String> {
    let command_path = resolve_command_path(&patch.directory, spec.command_path())?;
    let extension = command_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut command = if extension == "ps1" {
        #[cfg(windows)]
        {
            let mut command = Command::new("powershell.exe");
            command.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-File",
            ]);
            command.arg(&command_path);
            command
        }
        #[cfg(not(windows))]
        {
            let mut command = Command::new("pwsh");
            command.args(["-NoProfile", "-File"]);
            command.arg(&command_path);
            command
        }
    } else if extension == "sh" {
        let mut command = Command::new("sh");
        command.arg(&command_path);
        command
    } else if cfg!(windows) && matches!(extension.as_str(), "cmd" | "bat") {
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/C"]);
        command.arg(&command_path);
        command
    } else {
        Command::new(&command_path)
    };

    hide_patch_process_window(&mut command);

    command
        .args(spec.args())
        .current_dir(executable_directory)
        .env("CLICK_AUTOMATION_BASE_DIR", executable_directory)
        .env("CLICK_AUTOMATION_PATCH_DIR", &patch.directory)
        .env("CLICK_AUTOMATION_PATCH_ID", &patch.manifest.id)
        .env("CLICK_AUTOMATION_OPERATION_ID", operation_id)
        .output()
        .map_err(|error| format!("启动补丁命令失败 {}: {}", command_path.display(), error))
}

fn hide_patch_process_window(command: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

fn log_process_output(add_log: &mut impl FnMut(String), prefix: &str, output: &Output) {
    for line in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        add_log(format!("{} {}", prefix, line));
    }
    for line in String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        add_log(format!("{} stderr: {}", prefix, line));
    }
}

fn process_stderr(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {}", stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandSpec, PatchTabSnapshot};

    #[test]
    fn tab_snapshot_defaults_missing_fields() {
        let snapshot: PatchTabSnapshot = serde_json::from_str(r#"{"summary":"ok"}"#).unwrap();
        assert_eq!(snapshot.summary, "ok");
        assert!(snapshot.missing_lines.is_empty());
        assert!(snapshot.content.is_empty());
    }

    #[test]
    fn command_spec_supports_short_and_detailed_forms() {
        let short: CommandSpec = serde_json::from_str(r#""run.ps1""#).unwrap();
        assert_eq!(short.command_path(), "run.ps1");
        assert!(short.args().is_empty());

        let detailed: CommandSpec =
            serde_json::from_str(r#"{"command":"run.ps1","args":["--test"]}"#).unwrap();
        assert_eq!(detailed.command_path(), "run.ps1");
        assert_eq!(detailed.args(), ["--test"]);
    }
}
