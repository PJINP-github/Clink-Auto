# Click Automation

基于 Rust + egui 的桌面点击自动化工具，支持 Windows 与 Linux（X11 / XWayland，纯 Wayland 走 portal 授权）。通过可视化编排动作步骤、坐标捕获、OCR 判定与全局热键，实现可配置的循环自动化流程。

- 版本：`0.1.0`（Rust 2021，eframe/egui 0.29）
- 产物：单文件可执行程序，OCR 模型与中文字体已内嵌，运行时不依赖外部资源

## 功能特性

| 模块 | 说明 |
| --- | --- |
| 坐标分类 | 维护命名的坐标集合，供「点击分类」步骤引用 |
| 动作编排 | 每个动作由有序步骤组成，支持插入、上移、下移、删除、单步执行 |
| 坐标/区域捕获 | 在桌面右键点击两次即可捕获坐标点或矩形区域 |
| 自动运行 | 冷却周期触发默认动作，或由全局热键触发指定动作 |
| OCR | RapidOCR（PP-OCRv5）识别屏幕区域文字，支持匹配/不匹配分支跳转 |
| 截图 | 全屏或按区域截图，保存至 `screenshots/`，文件名可取自剪贴板 |
| 补丁包 | `patches/` 下的独立脚本包可扩展动作步骤与自定义 Tab，无需重新编译 |
| 密码门禁 | `config.yaml` 中 `passwd` 开启后，启动需输入动态密码 |

## 目录结构

```
auto_click/
├─ src/
│  ├─ main.rs              # 入口：字体加载、临时目录、启动 eframe
│  ├─ launcher.rs          # 登录页 → 主界面
│  ├─ config.rs            # 配置结构与 ActionStep 枚举
│  ├─ config_path.rs       # config.yaml 定位规则
│  ├─ action_executor.rs   # 动作/步骤执行与 OCR 匹配规则
│  ├─ ocr.rs / ocr_runner.rs   # OCR 引擎初始化与区域识别
│  ├─ capture_tools.rs     # 截图落盘与文件名策略
│  ├─ clipboard_tools.rs   # 剪贴板读写
│  ├─ html_table.rs        # temp/*.html → xlsx
│  ├─ points_reader.rs     # points.txt 逐行读取
│  ├─ patches.rs           # 补丁发现、校验与进程调用
│  ├─ password.rs          # 动态密码校验
│  ├─ linux_capture.rs     # X11 XGetImage 截图
│  ├─ wayland_portal.rs    # Screenshot / RemoteDesktop / GlobalShortcuts portal
│  ├─ windows_capture.rs   # GDI BitBlt 截图
│  ├─ app/                 # GUI 各 Tab 与事件处理
│  └─ mouse/               # 输入注入、热键监听、按键序列录制
├─ models/rapidocr/        # OCR 模型（也内嵌进可执行文件）
├─ patches/                # 补丁包目录（随发布包分发）
├─ scripts/                # 构建与辅助脚本
├─ development/            # 构建、补丁协议等开发者文档
├─ config.yaml             # 运行配置（与可执行文件同级）
└─ points.txt              # 「读取 points.txt」步骤的数据源
```

## 快速开始

### Windows

```powershell
.\scripts\setup-windows.ps1     # 首次：准备工具链
.\scripts\build-windows.ps1     # 构建
```

运行同目录下的 `click_automation.exe`。程序会自动在与可执行文件相同的目录创建 `config.yaml`、`temp/`、`screenshots/`。

### Linux

在 Windows PowerShell 中交叉构建（推荐，产物 glibc 要求更低）：

```powershell
.\scripts\build-linux.ps1 -InstallMissing
```

脚本会检查 Rust、`x86_64-unknown-linux-gnu` target、Zig 与 `cargo-zigbuild`，默认实际构建目标为 `x86_64-unknown-linux-gnu.2.31`。输出目录：`dist/linux-x86_64/`（含可执行文件、`config.yaml`、`models`、`patches`、字体、`run.sh`、`libonnxruntime*.so*`）。

目标机运行：

```bash
cd dist/linux-x86_64
chmod +x click_automation run.sh
./run.sh
```

`run.sh` 会把发布目录加入 `LD_LIBRARY_PATH`，并将 `RAPIDOCR_MODEL_DIR` 默认指向 `models/rapidocr`。

Linux 运行库（Debian/Ubuntu）：

```bash
sudo apt-get install -y libx11-6 libxcb1 libxcb-render0 libxcb-shape0 libxcb-xfixes0 \
  libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libegl1 libgl1 libfontconfig1
```

原生构建还需开发包（`build-essential pkg-config libx11-dev libxcb1-dev …`），详见 `development/linux-build.md`。

## 配置说明

配置文件查找顺序：`$APPIMAGE` 同级目录 → 可执行文件同级目录 → 当前工作目录。GUI 中修改后会自动保存（也可在「设置」页手动保存/加载）。

```yaml
coordinate_categories:
  - name: 分类 1
    coordinates:
      - { x: 100, y: 200 }

actions:
  - name: 动作 1
    hotkey: ''                 # 触发键，如 Control_L / F8 / a
    steps:
      - !Click [1693, 142]
      - !Delay 500
      - !OcrCheck
        expected_text: "s:成功"
        region: [0, 0, 800, 200]
        action_on_match: null
        action_on_mismatch: null

run_config:
  cooldown_ms: 1000            # 自动运行冷却周期
  default_action: 0            # 冷却到期执行的默认动作索引
  hotkey_overrides:            # 全局快捷键覆盖：按键 → 动作
    - hotkey: F8
      action_index: 0

passwd: false                  # true 时启动需输入动态密码
visible_tabs: [actions, run]   # 可见 Tab；为空时等价于 [actions, run]
```

`visible_tabs` 可选值：`coordinates`、`actions`、`run`、`logs`、`ocr`、`settings`、`patches`。存在补丁 Tab 时，`patches` 会展开为若干页签。

### 动作步骤类型

| 步骤 | 字段 / 格式 | 说明 |
| --- | --- | --- |
| `Click(x, y)` | 坐标 | 左键单击 |
| `RightClick(x, y)` | 坐标 | 右键单击 |
| `SaturationClick` | `rows, clicks_per_row, interval_ms, region` | 区域内网格化点击；运行时行数上限 3 |
| `Delay(ms)` | 毫秒 | 延时，上限 60000 |
| `CopyClipboard` | — | 将剪贴板文本追加写入工作目录 `clipboard_output.txt` |
| `ClickCategory(name)` | 分类名 | 点击该分类的第一个坐标 |
| `ScreenCapture` | — | 全屏截图到 `screenshots/` |
| `ScreenCaptureRegion` | `region` | 区域截图；文件名优先取剪贴板中的合法文本 |
| `Paste` | — | 发送 Ctrl+V |
| `KeyboardSequence` | `"ctrl+c;500;esc"` | 分号分隔，纯数字段为延时毫秒；可用「监听」按钮录制 5 秒 |
| `OcrCheck` | `expected_text, region, action_on_match, action_on_mismatch` | 识别后按匹配结果跳转后续动作 |
| `PointsTxtNextLine` | `action_on_empty` | 读取 `points.txt` 下一行并写入剪贴板；空行/EOF 时跳转 |
| `HtmlToXlsx` | — | 扫描 `temp/*.html`，每个 `<table>` 写入同名 `.xlsx` 的一个 sheet |
| `OcrSaveMultiLine` | `region, filename_prefix, use_full_pipeline` | 多行 OCR 结果保存到 `temp/ocr_<前缀>_<时间戳>.txt` |
| `Patch` | `patch_id, action_id` | 调用补丁包中的动作 |

### OCR 匹配规则

默认对期望文本做子串匹配，比较前会去除所有空白字符。需要多规则时使用 `s:`（包含）/ `d:`（全等）前缀，以 `;` 分隔：

```
s:为空;s:失败;d:分析结果：
```

上例表示：识别结果包含「为空」或「失败」，或整体等于「分析结果：」即视为命中。

## 自动运行与热键

启动自动运行后按 `cooldown_ms` 周期执行默认动作；若命中热键覆盖或动作自身热键，则改为执行对应动作。同一时刻只允许一个动作在跑，重复触发会被跳过并记录日志。

内置控制键（与 GUI 按钮等效）：

| 按键 | 行为 |
| --- | --- |
| `Ctrl+Alt+Z` / `Ctrl+M` / `Ctrl+Alt+Space` | 启动或继续 |
| `Space` | 暂停 |

热键名使用 X11 keysym 风格（`Control_L`、`F8`、`space`、`a` 等），Windows 与 Linux 均接受同一套名称。

## 补丁包

补丁包放在可执行文件旁的 `patches/<包目录>/`，必须包含 `manifest.json`：

```json
{
  "id": "example",
  "version": "1.0.0",
  "name": "Example patch",
  "actions": [{ "id": "example-action", "name": "Example action", "command": "run.ps1" }],
  "tabs": [{ "id": "example-tab", "title": "Example tab", "command": "tab.ps1", "refresh_seconds": 5 }]
}
```

- `command` 支持相对路径、`.ps1`、`.sh`，或 `{ "command": "...", "args": [...] }`；拒绝绝对路径与跳出包目录的路径。
- 子进程环境变量：`CLICK_AUTOMATION_BASE_DIR`、`CLICK_AUTOMATION_PATCH_DIR`、`CLICK_AUTOMATION_PATCH_ID`、`CLICK_AUTOMATION_OPERATION_ID`。
- Tab 命令须输出一个 JSON 对象，字段：`summary`、`missing_lines`、`content`。完整协议见 `development/patches.md`。

## 平台支持

| 能力 | Windows | Linux X11 / XWayland | Linux 纯 Wayland |
| --- | --- | --- | --- |
| 截图 | GDI | XGetImage | `org.freedesktop.portal.Screenshot`（需授权） |
| 点击 / 粘贴 / 按键序列 | Win32 或 enigo | enigo | `RemoteDesktop` portal（需授权） |
| 全局热键 | rdev | XQueryKeymap 轮询 | `GlobalShortcuts` portal（需授权） |

纯 Wayland 不做无授权绕过。若桌面未实现相应 portal 或用户拒绝授权，相关功能会返回明确错误。建议在目标发行版的真实会话中做截图、点击、按键序列与热键的冒烟测试。

## 日志与排查

「日志」页保留最近 500 条，支持关键字过滤；关键流程有固定标签便于检索：

- `ACTION_START name=[...] steps=N` / `ACTION_END name=[...]`
- `OCR_CHECK start … result recognized=[…] expected=[…] matched=… elapsed_ms=…`
- `OCR_CHECK branch=match|mismatch next_action=...`
- `PATCH_ACTION ...`

常见问题：

- **`GLIBCXX_3.4.29` / `GLIBC_2.3x` not found**：可执行文件链接了构建机较新的运行库。请使用 `cargo-zigbuild` 交叉构建（`build-linux.ps1` 默认 `…-gnu.2.31`），或在目标机安装匹配的 `libstdc++`。排查记录见 `development/development.md`。
- **`X11 截图失败` / `连接 X11 显示服务失败`**：确认 `DISPLAY` 已设置且会话为 X11 或启用 XWayland。
- **`OCR model preparation failed`**：检查 `RAPIDOCR_MODEL_DIR`，或删除缓存目录让其重新释放内置模型（`CLICK_AUTOMATION_CACHE_DIR` 可自定义位置）。

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `RAPIDOCR_MODEL_DIR` | 指定 OCR 模型目录，覆盖默认查找 |
| `CLICK_AUTOMATION_CACHE_DIR` | 指定内置 OCR 模型释放的缓存根目录 |
| `CLICK_AUTOMATION_BASE_DIR` / `_PATCH_DIR` / `_PATCH_ID` / `_OPERATION_ID` | 补丁子进程专用，由宿主注入 |

## 开发

```bash
cargo build            # 调试构建（含 panic 回溯与启动错误提示）
cargo build --release  # 发布构建（strip + LTO + panic=abort）
cargo test             # 运行单元测试
```

相关文档：

- `development/linux-build.md` —— Linux 交叉构建、运行库与 portal 边界
- `development/patches.md` —— 补丁包协议
- `development/Guidance.md` —— AI 助手读取上下文的约定
- `AGENTS.md` —— 项目级编码准则

## 注意事项

- 密码门禁：`passwd: true` 时使用 HMAC 派生的 15 分钟轮换动态密码；代码中同时存在固定备用密码，正式部署前请替换或移除。
- 配置文件、`points.txt`、`temp/`、`screenshots/`、`clipboard_output.txt` 均以可执行文件目录为基准（工作目录可能不同）。
- 首次 OCR 会在线程中预热，冷启动首帧识别耗时偏高属正常现象。