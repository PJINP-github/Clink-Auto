# Linux 交叉构建与运行说明

本项目在 Linux 上的完整自动化能力面向 X11 会话，以及启用了 XWayland 的 Wayland 桌面。纯 Wayland 不允许普通应用无授权地做全局按键监听、输入注入和任意屏幕截图，因此如果不接入桌面环境专属门户或虚拟输入授权，无法保证全部功能可用。

## Windows 到 Linux 发布构建

默认 Rust target：`x86_64-unknown-linux-gnu`。

在 Windows PowerShell 中执行：

```powershell
.\scripts\build-linux.ps1 -InstallMissing
```

脚本会检查 Rust、Linux Rust target、Zig 和 `cargo-zigbuild`。默认实际构建目标会加上 Zig glibc 后缀 `x86_64-unknown-linux-gnu.2.31`，以降低运行时 glibc 要求，适配 Debian 12/13、Ubuntu、HikvisionOS/openEuler 22.03 这类较新的 x86_64 Linux。

输出目录：`dist/linux-x86_64/`。

发布目录会收集 Linux 可执行文件、`config.yaml`、`models`、`patches`、`notosansscregular.otf`、`run.sh` 启动脚本，以及构建目录中发现的 `libonnxruntime*.so*`。同时 OCR 模型和 CJK 字体已经嵌入可执行文件，外部模型缺失时程序会自动释放内置模型到缓存目录再加载。

## Linux 运行前提

需要在图形会话内运行。X11/XWayland 需要 `DISPLAY`；纯 Wayland 需要 `WAYLAND_DISPLAY`、`xdg-desktop-portal` 以及桌面对应的 portal 后端。

Debian/Ubuntu 常见运行库：

```bash
sudo apt-get update
sudo apt-get install -y libx11-6 libxcb1 libxcb-render0 libxcb-shape0 libxcb-xfixes0 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libegl1 libgl1 libfontconfig1
```

如果在 Linux 上原生构建，还需要开发包：

```bash
sudo apt-get install -y build-essential pkg-config libx11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev libfontconfig1-dev
```

在 Linux 机器上运行发布包：

```bash
cd dist/linux-x86_64
chmod +x click_automation run.sh
./run.sh
```

`run.sh` 会把发布目录加入 `LD_LIBRARY_PATH`，并默认将 `RAPIDOCR_MODEL_DIR` 指向 `models/rapidocr`。如需使用其他模型目录，可手动设置 `RAPIDOCR_MODEL_DIR`；如需控制内置 OCR 模型释放位置，可设置 `CLICK_AUTOMATION_CACHE_DIR`。

## 平台边界

- 截图和 OCR 截图使用原生 X11 `XGetImage`，不再依赖 ImageMagick `import`。
- Linux 鼠标位置、全局热键、坐标捕获、区域捕获和按键序列录制使用 X11 指针/键盘状态。
- 鼠标键盘注入仍通过 `enigo`，失败时写入应用日志，不再直接退出进程。
- 纯 Wayland 可以启动 GUI；截图、输入注入和固定全局热键会尝试使用 XDG desktop portal，经用户或桌面环境授权后可用。
## Wayland portal 支持

纯 Wayland 下不做无授权绕过。本项目在 Linux 上新增了授权式 Wayland 后端：

- 截图/OCR 截图：在 `XDG_SESSION_TYPE=wayland` 或存在 `WAYLAND_DISPLAY` 时，优先调用 `org.freedesktop.portal.Screenshot`，用户或桌面环境授权后获取截图，再按配置区域裁剪。
- 输入注入：Wayland 下点击、粘贴和按键序列会创建并缓存 `org.freedesktop.portal.RemoteDesktop` 会话，同时请求 `ScreenCast` monitor stream，用 portal 的 `NotifyPointerMotionAbsolute`、`NotifyPointerButton`、`NotifyKeyboardKeysym` 发送事件。
- 全局热键：纯 Wayland 下不监听所有按键，改为注册 `org.freedesktop.portal.GlobalShortcuts` 快捷键，接收 `Activated` 事件来触发开始/暂停类全局热键。
- 如果桌面环境没有实现对应 portal、用户拒绝授权，或系统策略禁用了远程桌面/截图权限，相关功能会返回明确错误；这仍然是 Wayland 的安全模型要求。

目标系统需要可用的 `xdg-desktop-portal` 以及桌面对应后端，例如 GNOME/KDE/wlroots 的 portal backend。不同桌面对 GlobalShortcuts 和 RemoteDesktop 的实现程度不同，仍建议在目标发行版的真实 Wayland 会话中做截图、点击、键盘序列和热键冒烟测试。
