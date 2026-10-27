#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use image::{DynamicImage, GenericImageView};
use percent_encoding::percent_decode_str;
use serde::de::Error as DeError;
use serde::Deserialize;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{
    DeserializeDict, ObjectPath, OwnedObjectPath, OwnedValue, Structure, Type, Value,
};

const PORTAL_DESTINATION: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const KEY_RELEASED: u32 = 0;
const KEY_PRESSED: u32 = 1;
const DEVICE_KEYBOARD_POINTER: u32 = 0b0011;
const SOURCE_MONITOR: u32 = 0b0001;
const CURSOR_HIDDEN: u32 = 0b0001;
const BTN_LEFT: i32 = 0x110;

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);
static REMOTE_DESKTOP: OnceLock<Mutex<Option<RemoteDesktopSession>>> = OnceLock::new();

pub fn is_wayland_session() -> bool {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();

    session_type.eq_ignore_ascii_case("wayland")
        || wayland_display.to_ascii_lowercase().contains("wayland")
}

pub fn capture_full_screen() -> Result<DynamicImage, String> {
    portal_screenshot()
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage, String> {
    if width == 0 || height == 0 {
        return Err("截图区域为空".to_string());
    }

    let screenshot = portal_screenshot()?;
    let (image_width, image_height) = screenshot.dimensions();
    let region = clamp_region(x, y, width, height, image_width, image_height)?;
    Ok(screenshot.crop_imm(region.x, region.y, region.width, region.height))
}

pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    with_remote_desktop(|session| {
        session.move_pointer_absolute(x, y)?;
        thread::sleep(Duration::from_millis(30));
        session.pointer_button(BTN_LEFT, KEY_PRESSED)?;
        thread::sleep(Duration::from_millis(20));
        session.pointer_button(BTN_LEFT, KEY_RELEASED)
    })
}

pub fn paste() -> Result<(), String> {
    send_key_sequence("CTRL+v")
}

pub fn send_key_sequence(sequence: &str) -> Result<(), String> {
    with_remote_desktop(|session| {
        for raw_part in sequence.split(';') {
            let part = raw_part.trim();
            if part.is_empty() {
                continue;
            }
            if let Ok(delay_ms) = part.parse::<u64>() {
                thread::sleep(Duration::from_millis(delay_ms));
                continue;
            }

            send_key_sequence_part(session, part)?;
            thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    })
}

fn send_key_sequence_part(session: &RemoteDesktopSession, part: &str) -> Result<(), String> {
    let mut modifiers = Vec::new();
    let mut key = None;

    for raw_token in part.split('+') {
        let token = raw_token.trim();
        if token.is_empty() {
            continue;
        }

        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.push(keysym_for_token("Control_L")?),
            "alt" => modifiers.push(keysym_for_token("Alt_L")?),
            "shift" => modifiers.push(keysym_for_token("Shift_L")?),
            "meta" | "super" | "win" => modifiers.push(keysym_for_token("Super_L")?),
            _ if key.is_none() => key = Some(keysym_for_token(token)?),
            _ => return Err(format!("不支持的组合键片段: {token}")),
        }
    }

    let Some(key) = key else {
        return Err(format!("按键序列片段缺少主键: {part}"));
    };

    for modifier in &modifiers {
        session.keyboard_keysym(*modifier, KEY_PRESSED)?;
    }
    session.keyboard_keysym(key, KEY_PRESSED)?;
    session.keyboard_keysym(key, KEY_RELEASED)?;
    for modifier in modifiers.iter().rev() {
        session.keyboard_keysym(*modifier, KEY_RELEASED)?;
    }

    Ok(())
}

fn portal_screenshot() -> Result<DynamicImage, String> {
    let conn = Connection::session().map_err(portal_error)?;
    let proxy = desktop_proxy(&conn, "org.freedesktop.portal.Screenshot")?;
    let handle_token = next_token("screenshot");
    let request = portal_request(&conn, &handle_token)?;
    let mut signals = request.receive_signal("Response").map_err(portal_error)?;

    let mut options = HashMap::<&str, Value<'_>>::new();
    options.insert("handle_token", Value::from(&handle_token));
    options.insert("modal", Value::from(true));
    options.insert("interactive", Value::from(false));

    proxy
        .call_method("Screenshot", &("", options))
        .map_err(portal_error)?;

    let response: ScreenshotResponse = wait_response(&mut signals)?;
    let path = file_uri_to_path(&response.uri)?;
    let image = image::open(&path).map_err(|e| format!("读取 Wayland portal 截图失败: {e}"))?;
    let _ = std::fs::remove_file(path);
    Ok(image)
}

fn with_remote_desktop<T>(
    f: impl FnOnce(&RemoteDesktopSession) -> Result<T, String>,
) -> Result<T, String> {
    let lock = REMOTE_DESKTOP.get_or_init(|| Mutex::new(None));
    let mut guard = lock
        .lock()
        .map_err(|_| "Wayland RemoteDesktop 会话锁已损坏".to_string())?;

    if guard.is_none() {
        *guard = Some(RemoteDesktopSession::start()?);
    }

    let result = f(guard.as_ref().expect("session is initialized"));
    if result.is_err() {
        *guard = None;
    }
    result
}

struct RemoteDesktopSession {
    conn: Connection,
    session_handle: OwnedObjectPath,
    stream_id: u32,
    stream_origin: (i32, i32),
}

impl RemoteDesktopSession {
    fn start() -> Result<Self, String> {
        let conn = Connection::session().map_err(portal_error)?;
        let (session_handle, response) = {
            let remote = desktop_proxy(&conn, "org.freedesktop.portal.RemoteDesktop")?;
            let screencast = desktop_proxy(&conn, "org.freedesktop.portal.ScreenCast")?;
            let session_handle = create_session(&conn, &remote, "rd")?;

            select_remote_devices(&conn, &remote, &session_handle)?;
            select_screencast_source(&conn, &screencast, &session_handle)?;
            let response = start_remote_desktop(&conn, &remote, &session_handle)?;
            (session_handle, response)
        };
        let stream = response.streams.first().ok_or_else(|| {
            "Wayland RemoteDesktop 已授权输入，但未返回 ScreenCast stream，无法执行绝对坐标点击"
                .to_string()
        })?;

        Ok(Self {
            conn,
            session_handle,
            stream_id: stream.0,
            stream_origin: stream.1.position.unwrap_or((0, 0)),
        })
    }

    fn move_pointer_absolute(&self, x: i32, y: i32) -> Result<(), String> {
        let proxy = desktop_proxy(&self.conn, "org.freedesktop.portal.RemoteDesktop")?;
        let local_x = x - self.stream_origin.0;
        let local_y = y - self.stream_origin.1;
        proxy
            .call_method(
                "NotifyPointerMotionAbsolute",
                &(
                    self.session_handle.as_ref(),
                    empty_options(),
                    self.stream_id,
                    f64::from(local_x),
                    f64::from(local_y),
                ),
            )
            .map_err(portal_error)?;
        Ok(())
    }

    fn pointer_button(&self, button: i32, state: u32) -> Result<(), String> {
        let proxy = desktop_proxy(&self.conn, "org.freedesktop.portal.RemoteDesktop")?;
        proxy
            .call_method(
                "NotifyPointerButton",
                &(self.session_handle.as_ref(), empty_options(), button, state),
            )
            .map_err(portal_error)?;
        Ok(())
    }

    fn keyboard_keysym(&self, keysym: i32, state: u32) -> Result<(), String> {
        let proxy = desktop_proxy(&self.conn, "org.freedesktop.portal.RemoteDesktop")?;
        proxy
            .call_method(
                "NotifyKeyboardKeysym",
                &(self.session_handle.as_ref(), empty_options(), keysym, state),
            )
            .map_err(portal_error)?;
        Ok(())
    }
}

fn create_session(
    conn: &Connection,
    proxy: &Proxy<'_>,
    prefix: &str,
) -> Result<OwnedObjectPath, String> {
    let handle_token = next_token(&format!("{prefix}_create"));
    let session_handle_token = next_token(&format!("{prefix}_session"));
    let request = portal_request(conn, &handle_token)?;
    let mut signals = request.receive_signal("Response").map_err(portal_error)?;

    let mut options = HashMap::<&str, Value<'_>>::new();
    options.insert("handle_token", Value::from(&handle_token));
    options.insert("session_handle_token", Value::from(&session_handle_token));

    proxy
        .call_method("CreateSession", &options)
        .map_err(portal_error)?;

    let response: CreateSessionResponse = wait_response(&mut signals)?;
    Ok(response.session_handle)
}

fn select_remote_devices(
    conn: &Connection,
    proxy: &Proxy<'_>,
    session_handle: &OwnedObjectPath,
) -> Result<(), String> {
    let handle_token = next_token("rd_devices");
    let request = portal_request(conn, &handle_token)?;
    let mut signals = request.receive_signal("Response").map_err(portal_error)?;

    let mut options = HashMap::<&str, Value<'_>>::new();
    options.insert("handle_token", Value::from(&handle_token));
    options.insert("types", Value::from(DEVICE_KEYBOARD_POINTER));

    proxy
        .call_method("SelectDevices", &(session_handle.as_ref(), options))
        .map_err(portal_error)?;
    let _: EmptyResponse = wait_response(&mut signals)?;
    Ok(())
}

fn select_screencast_source(
    conn: &Connection,
    proxy: &Proxy<'_>,
    session_handle: &OwnedObjectPath,
) -> Result<(), String> {
    let handle_token = next_token("rd_source");
    let request = portal_request(conn, &handle_token)?;
    let mut signals = request.receive_signal("Response").map_err(portal_error)?;

    let mut options = HashMap::<&str, Value<'_>>::new();
    options.insert("handle_token", Value::from(&handle_token));
    options.insert("types", Value::from(SOURCE_MONITOR));
    options.insert("multiple", Value::from(false));
    options.insert("cursor_mode", Value::from(CURSOR_HIDDEN));

    proxy
        .call_method("SelectSources", &(session_handle.as_ref(), options))
        .map_err(portal_error)?;
    let _: EmptyResponse = wait_response(&mut signals)?;
    Ok(())
}

fn start_remote_desktop(
    conn: &Connection,
    proxy: &Proxy<'_>,
    session_handle: &OwnedObjectPath,
) -> Result<RemoteStartResponse, String> {
    let handle_token = next_token("rd_start");
    let request = portal_request(conn, &handle_token)?;
    let mut signals = request.receive_signal("Response").map_err(portal_error)?;

    let mut options = HashMap::<&str, Value<'_>>::new();
    options.insert("handle_token", Value::from(&handle_token));

    proxy
        .call_method("Start", &(session_handle.as_ref(), "", options))
        .map_err(portal_error)?;
    wait_response(&mut signals)
}

fn desktop_proxy<'a>(conn: &'a Connection, interface: &'a str) -> Result<Proxy<'a>, String> {
    Proxy::new(conn, PORTAL_DESTINATION, PORTAL_PATH, interface).map_err(portal_error)
}

fn portal_request<'a>(conn: &'a Connection, handle_token: &str) -> Result<Proxy<'a>, String> {
    let unique_name = conn
        .unique_name()
        .ok_or_else(|| "获取 D-Bus 唯一名称失败".to_string())?
        .trim_start_matches(':')
        .replace('.', "_");
    let path = format!("/org/freedesktop/portal/desktop/request/{unique_name}/{handle_token}");

    Proxy::new(
        conn,
        PORTAL_DESTINATION,
        path,
        "org.freedesktop.portal.Request",
    )
    .map_err(portal_error)
}

fn wait_response<T>(signals: &mut zbus::blocking::SignalIterator<'_>) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Type,
{
    let message = signals
        .next()
        .ok_or_else(|| "等待 Wayland portal 授权响应失败".to_string())?;
    let body = message.body();
    let (code, response): (u32, T) = body.deserialize().map_err(portal_error)?;

    match code {
        0 => Ok(response),
        1 => Err("Wayland portal 授权已被用户取消".to_string()),
        code => Err(format!("Wayland portal 返回失败状态: {code}")),
    }
}

fn empty_options() -> HashMap<&'static str, Value<'static>> {
    HashMap::new()
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf, String> {
    let Some(path) = uri.strip_prefix("file://") else {
        return Err(format!("Wayland portal 返回了非 file URI: {uri}"));
    };
    let decoded = percent_decode_str(path).decode_utf8_lossy().to_string();
    Ok(PathBuf::from(decoded))
}

fn next_token(prefix: &str) -> String {
    let id = TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("click_automation_{prefix}_{id}")
}

fn portal_error(error: impl std::fmt::Display) -> String {
    format!("Wayland portal 调用失败: {error}")
}

fn keysym_for_token(token: &str) -> Result<i32, String> {
    let lower = token.to_ascii_lowercase();
    let keysym = match lower.as_str() {
        "esc" | "escape" => 0xff1b,
        "enter" | "return" => 0xff0d,
        "tab" => 0xff09,
        "space" => 0x0020,
        "backspace" | "back_space" => 0xff08,
        "delete" | "del" => 0xffff,
        "insert" | "ins" => 0xff63,
        "up" => 0xff52,
        "down" => 0xff54,
        "left" => 0xff51,
        "right" => 0xff53,
        "home" => 0xff50,
        "end" => 0xff57,
        "pageup" | "page_up" | "prior" => 0xff55,
        "pagedown" | "page_down" | "next" => 0xff56,
        "f1" => 0xffbe,
        "f2" => 0xffbf,
        "f3" => 0xffc0,
        "f4" => 0xffc1,
        "f5" => 0xffc2,
        "f6" => 0xffc3,
        "f7" => 0xffc4,
        "f8" => 0xffc5,
        "f9" => 0xffc6,
        "f10" => 0xffc7,
        "f11" => 0xffc8,
        "f12" => 0xffc9,
        "printscreen" | "print" => 0xff61,
        "pause" => 0xff13,
        "control_l" => 0xffe3,
        "alt_l" => 0xffe9,
        "shift_l" => 0xffe1,
        "super_l" => 0xffeb,
        "minus" => '-' as i32,
        "equal" => '=' as i32,
        "bracketleft" => '[' as i32,
        "bracketright" => ']' as i32,
        "semicolon" => ';' as i32,
        "apostrophe" => '\'' as i32,
        "comma" => ',' as i32,
        "period" => '.' as i32,
        "slash" => '/' as i32,
        "backslash" => '\\' as i32,
        "grave" => '`' as i32,
        _ if lower.chars().count() == 1 => lower.chars().next().unwrap() as i32,
        _ => return Err(format!("Wayland RemoteDesktop 暂不支持按键: {token}")),
    };
    Ok(keysym)
}

#[derive(Clone, Copy)]
struct CaptureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn clamp_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Result<CaptureRegion, String> {
    let left = i64::from(x).max(0);
    let top = i64::from(y).max(0);
    let right = (i64::from(x) + i64::from(width)).min(i64::from(screen_width));
    let bottom = (i64::from(y) + i64::from(height)).min(i64::from(screen_height));

    if right <= left || bottom <= top {
        return Err(format!(
            "截图区域超出 Wayland portal 返回图像范围: ({x}, {y}) {width}x{height}, 图像 {screen_width}x{screen_height}"
        ));
    }

    Ok(CaptureRegion {
        x: left as u32,
        y: top as u32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct ScreenshotResponse {
    uri: String,
}

#[derive(Type, Debug)]
#[zvariant(signature = "dict")]
struct CreateSessionResponse {
    session_handle: OwnedObjectPath,
}

impl<'de> Deserialize<'de> for CreateSessionResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<String, OwnedValue>::deserialize(deserializer)?;
        let value = map.get("session_handle").ok_or_else(|| {
            D::Error::custom("missing session_handle in portal CreateSession response")
        })?;

        let session_handle = if let Ok(path) = <&ObjectPath<'_>>::try_from(value) {
            path.clone().into()
        } else if let Ok(path) =
            OwnedObjectPath::try_from(value.try_clone().map_err(D::Error::custom)?)
        {
            path
        } else if let Ok(path) = <&str>::try_from(value) {
            ObjectPath::try_from(path).map_err(D::Error::custom)?.into()
        } else {
            return Err(D::Error::custom("invalid session_handle type"));
        };

        Ok(Self { session_handle })
    }
}

#[derive(Deserialize, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct EmptyResponse {}

#[derive(Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct RemoteStartResponse {
    streams: Vec<PortalStream>,
}

impl<'de> Deserialize<'de> for RemoteStartResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut map = HashMap::<String, OwnedValue>::deserialize(deserializer)?;
        let streams = match map.remove("streams") {
            Some(value) => Vec::<PortalStream>::try_from(value).map_err(D::Error::custom)?,
            None => Vec::new(),
        };
        Ok(Self { streams })
    }
}

#[derive(Clone, Type, Debug)]
struct PortalStream(u32, PortalStreamProperties);

impl<'a> TryFrom<Value<'a>> for PortalStream {
    type Error = zbus::zvariant::Error;

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        let structure = Structure::try_from(value)?;
        let mut fields = structure.into_fields().into_iter();
        let stream_id = fields
            .next()
            .ok_or(zbus::zvariant::Error::IncorrectType)
            .and_then(u32::try_from)?;
        let properties = fields
            .next()
            .map(PortalStreamProperties::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(Self(stream_id, properties))
    }
}

#[derive(Clone, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct PortalStreamProperties {
    position: Option<(i32, i32)>,
    #[allow(dead_code)]
    size: Option<(i32, i32)>,
}

impl<'a> TryFrom<Value<'a>> for PortalStreamProperties {
    type Error = zbus::zvariant::Error;

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        let map = HashMap::<String, OwnedValue>::try_from(value)?;
        let position = optional_tuple_i32(map.get("position"));
        let size = optional_tuple_i32(map.get("size"));
        Ok(Self { position, size })
    }
}

fn optional_tuple_i32(value: Option<&OwnedValue>) -> Option<(i32, i32)> {
    let value = value?.try_clone().ok()?;
    <(i32, i32)>::try_from(value).ok()
}

pub mod global_shortcuts {
    use super::*;
    use std::sync::mpsc::{Receiver, Sender};

    pub struct WaylandShortcutBackend {
        rx: Receiver<String>,
    }

    impl WaylandShortcutBackend {
        pub fn start() -> Option<Self> {
            let (tx, rx) = std::sync::mpsc::channel();
            thread::spawn(move || {
                if let Err(err) = run_shortcuts(tx) {
                    eprintln!("{err}");
                }
            });
            Some(Self { rx })
        }

        pub fn read(&mut self) -> crate::mouse::MouseState {
            let mut state = crate::mouse::MouseState::default();
            while let Ok(id) = self.rx.try_recv() {
                match id.as_str() {
                    "pause_space" => state.space_pressed = true,
                    "start_ctrl_alt_space" => state.ctrl_alt_space = true,
                    "start_ctrl_alt_z" => state.ctrl_alt_z_pressed = true,
                    "start_ctrl_m" => state.ctrl_m_pressed = true,
                    _ => {}
                }
            }
            state
        }
    }

    fn run_shortcuts(tx: Sender<String>) -> Result<(), String> {
        let conn = Connection::session().map_err(portal_error)?;
        let proxy = desktop_proxy(&conn, "org.freedesktop.portal.GlobalShortcuts")?;
        let session_handle = create_session(&conn, &proxy, "gs")?;
        bind_shortcuts(&conn, &proxy, &session_handle)?;

        let mut signals = proxy.receive_signal("Activated").map_err(portal_error)?;
        for message in &mut signals {
            let body = message.body();
            let (_session, shortcut_id, _timestamp, _options): (
                OwnedObjectPath,
                String,
                u64,
                HashMap<String, OwnedValue>,
            ) = body.deserialize().map_err(portal_error)?;
            if tx.send(shortcut_id).is_err() {
                break;
            }
        }
        Ok(())
    }

    fn bind_shortcuts(
        conn: &Connection,
        proxy: &Proxy<'_>,
        session_handle: &OwnedObjectPath,
    ) -> Result<(), String> {
        let handle_token = next_token("gs_bind");
        let request = portal_request(conn, &handle_token)?;
        let mut signals = request.receive_signal("Response").map_err(portal_error)?;

        let shortcuts = vec![
            new_shortcut("pause_space", "暂停自动运行", "space"),
            new_shortcut(
                "start_ctrl_alt_space",
                "启动或继续自动运行",
                "CTRL+ALT+space",
            ),
            new_shortcut("start_ctrl_alt_z", "启动或继续自动运行", "CTRL+ALT+z"),
            new_shortcut("start_ctrl_m", "启动或继续自动运行", "CTRL+m"),
        ];

        let mut options = HashMap::<&str, Value<'_>>::new();
        options.insert("handle_token", Value::from(&handle_token));

        proxy
            .call_method(
                "BindShortcuts",
                &(session_handle.as_ref(), shortcuts, "", options),
            )
            .map_err(portal_error)?;

        let _: EmptyResponse = wait_response(&mut signals)?;
        Ok(())
    }

    fn new_shortcut<'a>(
        id: &'a str,
        description: &'a str,
        preferred_trigger: &'a str,
    ) -> (&'a str, HashMap<&'a str, Value<'a>>) {
        let mut info = HashMap::new();
        info.insert("description", Value::from(description));
        info.insert("preferred_trigger", Value::from(preferred_trigger));
        (id, info)
    }
}
