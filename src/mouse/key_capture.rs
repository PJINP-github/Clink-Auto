#[cfg(windows)]
use rdev::EventType;
#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(windows)]
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
#[cfg(target_os = "linux")]
use x11_dl::xlib::{self, Display};

#[cfg(windows)]
use super::key_names::rdev_key_to_name;
#[cfg(target_os = "linux")]
use super::x11_backend_input::{key_is_down, keysym_to_keycode};

pub const KEY_SEQUENCE_CAPTURE_SECONDS: u64 = 5;

pub fn listen_for_next_key() -> Option<String> {
    #[cfg(windows)]
    {
        return listen_for_next_key_rdev();
    }

    #[cfg(target_os = "linux")]
    {
        return listen_for_next_key_x11();
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

#[cfg(windows)]
fn listen_for_next_key_rdev() -> Option<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let (etx, erx) = mpsc::channel::<rdev::Event>();
        thread::spawn(move || {
            if let Err(e) = rdev::listen(move |event| {
                let _ = etx.send(event);
            }) {
                eprintln!("rdev key listen error: {:?}", e);
            }
        });
        for event in erx {
            if let EventType::KeyPress(key) = event.event_type {
                if let Some(name) = rdev_key_to_name(key) {
                    let _ = tx.send(name);
                    return;
                }
            }
        }
    });
    rx.recv_timeout(Duration::from_secs(10)).ok()
}

pub fn listen_for_key_sequence(duration: Duration) -> String {
    #[cfg(windows)]
    {
        return listen_for_key_sequence_windows(duration);
    }

    #[cfg(target_os = "linux")]
    {
        return listen_for_key_sequence_x11(duration);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        String::new()
    }
}

#[derive(Default)]
struct SequenceParts {
    last_key_at: Option<Instant>,
    parts: Vec<String>,
}

impl SequenceParts {
    fn push_key(&mut self, modifiers: ActiveModifiers, key_name: &str, event_at: Instant) {
        self.push_delay(event_at);
        let mut combo = Vec::new();
        if modifiers.ctrl_down {
            combo.push("ctrl");
        }
        if modifiers.alt_down {
            combo.push("alt");
        }
        if modifiers.shift_down {
            combo.push("shift");
        }
        if modifiers.meta_down {
            combo.push("meta");
        }
        combo.push(key_name);
        self.parts.push(combo.join("+"));
        self.last_key_at = Some(event_at);
    }

    fn push_delay(&mut self, event_at: Instant) {
        let Some(last_key_at) = self.last_key_at else {
            return;
        };
        let delay_ms = event_at.saturating_duration_since(last_key_at).as_millis();
        if delay_ms >= 30 {
            self.parts.push(delay_ms.to_string());
        }
    }

    fn finish(self) -> String {
        self.parts.join(";")
    }
}

#[derive(Default, Clone, Copy)]
struct ActiveModifiers {
    ctrl_down: bool,
    alt_down: bool,
    shift_down: bool,
    meta_down: bool,
}

#[cfg(target_os = "linux")]
fn listen_for_next_key_x11() -> Option<String> {
    let mut poller = LinuxKeyPoller::open()
        .map_err(|err| eprintln!("{err}"))
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut previous_down = vec![false; poller.keys.len()];

    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }

        let keys = poller.query_keymap();
        for (index, key) in poller.keys.iter().enumerate() {
            let down = key_is_down(&keys, key.keycode);
            if down && !previous_down[index] {
                return Some(key.name.to_string());
            }
            previous_down[index] = down;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn listen_for_key_sequence_x11(duration: Duration) -> String {
    let Ok(mut poller) = LinuxKeyPoller::open().map_err(|err| eprintln!("{err}")) else {
        return String::new();
    };
    let deadline = Instant::now() + duration;
    let mut previous_down = vec![false; poller.keys.len()];
    let mut parts = SequenceParts::default();

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let keys = poller.query_keymap();
        let modifiers = poller.active_modifiers(&keys);
        for (index, key) in poller.keys.iter().enumerate() {
            let down = key_is_down(&keys, key.keycode);
            if down && !previous_down[index] {
                parts.push_key(modifiers, key.name, now);
            }
            previous_down[index] = down;
        }
        thread::sleep(Duration::from_millis(10));
    }

    parts.finish()
}

#[cfg(target_os = "linux")]
struct LinuxKeyPoller {
    xlib: xlib::Xlib,
    display: *mut Display,
    keys: Vec<LinuxSequenceKey>,
    ctrl_keycodes: Vec<u32>,
    alt_keycodes: Vec<u32>,
    shift_keycodes: Vec<u32>,
    meta_keycodes: Vec<u32>,
}

#[cfg(target_os = "linux")]
impl LinuxKeyPoller {
    fn open() -> Result<Self, String> {
        if std::env::var_os("DISPLAY").is_none() {
            return Err(
                "Linux 按键监听需要 X11/XWayland 图形会话，请确认 DISPLAY 已设置".to_string(),
            );
        }

        let xlib = xlib::Xlib::open().map_err(|e| format!("加载 libX11 失败: {e:?}"))?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return Err("连接 X11 显示服务失败，无法监听按键".to_string());
            }

            let mut poller = Self {
                xlib,
                display,
                keys: Vec::new(),
                ctrl_keycodes: Vec::new(),
                alt_keycodes: Vec::new(),
                shift_keycodes: Vec::new(),
                meta_keycodes: Vec::new(),
            };
            poller.keys = linux_sequence_keys()
                .iter()
                .filter_map(|spec| poller.sequence_key(spec.name, spec.keysym_name))
                .collect();
            poller.ctrl_keycodes = poller.keycodes_for(&["Control_L", "Control_R"]);
            poller.alt_keycodes = poller.keycodes_for(&["Alt_L", "Alt_R"]);
            poller.shift_keycodes = poller.keycodes_for(&["Shift_L", "Shift_R"]);
            poller.meta_keycodes = poller.keycodes_for(&["Meta_L", "Meta_R", "Super_L", "Super_R"]);

            if poller.keys.is_empty() {
                Err("X11 没有返回可监听的按键映射".to_string())
            } else {
                Ok(poller)
            }
        }
    }

    fn query_keymap(&mut self) -> [u8; 32] {
        let mut keys = [0; 32];
        unsafe {
            (self.xlib.XQueryKeymap)(self.display, keys.as_mut_ptr() as *mut i8);
        }
        keys
    }

    fn active_modifiers(&self, keys: &[u8; 32]) -> ActiveModifiers {
        ActiveModifiers {
            ctrl_down: any_key_down(keys, &self.ctrl_keycodes),
            alt_down: any_key_down(keys, &self.alt_keycodes),
            shift_down: any_key_down(keys, &self.shift_keycodes),
            meta_down: any_key_down(keys, &self.meta_keycodes),
        }
    }

    fn sequence_key(&self, name: &'static str, keysym_name: &str) -> Option<LinuxSequenceKey> {
        let keycode = self.keycode_for(keysym_name)?;
        Some(LinuxSequenceKey { name, keycode })
    }

    fn keycodes_for(&self, names: &[&str]) -> Vec<u32> {
        names
            .iter()
            .filter_map(|name| self.keycode_for(name))
            .collect()
    }

    fn keycode_for(&self, keysym_name: &str) -> Option<u32> {
        let c_name = CString::new(keysym_name).ok()?;
        let keysym = unsafe { (self.xlib.XStringToKeysym)(c_name.as_ptr()) };
        if keysym == 0 {
            return None;
        }
        let keycode = keysym_to_keycode(&self.xlib, self.display, keysym);
        (keycode != 0).then_some(keycode)
    }
}

#[cfg(target_os = "linux")]
impl Drop for LinuxKeyPoller {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

#[cfg(target_os = "linux")]
struct LinuxSequenceKey {
    name: &'static str,
    keycode: u32,
}

#[cfg(target_os = "linux")]
struct LinuxSequenceKeySpec {
    name: &'static str,
    keysym_name: &'static str,
}

#[cfg(target_os = "linux")]
const fn linux_sequence_key(name: &'static str, keysym_name: &'static str) -> LinuxSequenceKeySpec {
    LinuxSequenceKeySpec { name, keysym_name }
}

#[cfg(target_os = "linux")]
fn linux_sequence_keys() -> &'static [LinuxSequenceKeySpec] {
    LINUX_SEQUENCE_KEYS
}

#[cfg(target_os = "linux")]
fn any_key_down(keys: &[u8; 32], keycodes: &[u32]) -> bool {
    keycodes.iter().any(|keycode| key_is_down(keys, *keycode))
}

#[cfg(target_os = "linux")]
static LINUX_SEQUENCE_KEYS: &[LinuxSequenceKeySpec] = &[
    linux_sequence_key("esc", "Escape"),
    linux_sequence_key("enter", "Return"),
    linux_sequence_key("tab", "Tab"),
    linux_sequence_key("backspace", "BackSpace"),
    linux_sequence_key("space", "space"),
    linux_sequence_key("delete", "Delete"),
    linux_sequence_key("insert", "Insert"),
    linux_sequence_key("home", "Home"),
    linux_sequence_key("end", "End"),
    linux_sequence_key("pageup", "Page_Up"),
    linux_sequence_key("pagedown", "Page_Down"),
    linux_sequence_key("up", "Up"),
    linux_sequence_key("down", "Down"),
    linux_sequence_key("left", "Left"),
    linux_sequence_key("right", "Right"),
    linux_sequence_key("f1", "F1"),
    linux_sequence_key("f2", "F2"),
    linux_sequence_key("f3", "F3"),
    linux_sequence_key("f4", "F4"),
    linux_sequence_key("f5", "F5"),
    linux_sequence_key("f6", "F6"),
    linux_sequence_key("f7", "F7"),
    linux_sequence_key("f8", "F8"),
    linux_sequence_key("f9", "F9"),
    linux_sequence_key("f10", "F10"),
    linux_sequence_key("f11", "F11"),
    linux_sequence_key("f12", "F12"),
    linux_sequence_key("printscreen", "Print"),
    linux_sequence_key("pause", "Pause"),
    linux_sequence_key("0", "0"),
    linux_sequence_key("1", "1"),
    linux_sequence_key("2", "2"),
    linux_sequence_key("3", "3"),
    linux_sequence_key("4", "4"),
    linux_sequence_key("5", "5"),
    linux_sequence_key("6", "6"),
    linux_sequence_key("7", "7"),
    linux_sequence_key("8", "8"),
    linux_sequence_key("9", "9"),
    linux_sequence_key("a", "a"),
    linux_sequence_key("b", "b"),
    linux_sequence_key("c", "c"),
    linux_sequence_key("d", "d"),
    linux_sequence_key("e", "e"),
    linux_sequence_key("f", "f"),
    linux_sequence_key("g", "g"),
    linux_sequence_key("h", "h"),
    linux_sequence_key("i", "i"),
    linux_sequence_key("j", "j"),
    linux_sequence_key("k", "k"),
    linux_sequence_key("l", "l"),
    linux_sequence_key("m", "m"),
    linux_sequence_key("n", "n"),
    linux_sequence_key("o", "o"),
    linux_sequence_key("p", "p"),
    linux_sequence_key("q", "q"),
    linux_sequence_key("r", "r"),
    linux_sequence_key("s", "s"),
    linux_sequence_key("t", "t"),
    linux_sequence_key("u", "u"),
    linux_sequence_key("v", "v"),
    linux_sequence_key("w", "w"),
    linux_sequence_key("x", "x"),
    linux_sequence_key("y", "y"),
    linux_sequence_key("z", "z"),
    linux_sequence_key("minus", "minus"),
    linux_sequence_key("equal", "equal"),
    linux_sequence_key("bracketleft", "bracketleft"),
    linux_sequence_key("bracketright", "bracketright"),
    linux_sequence_key("semicolon", "semicolon"),
    linux_sequence_key("apostrophe", "apostrophe"),
    linux_sequence_key("comma", "comma"),
    linux_sequence_key("period", "period"),
    linux_sequence_key("slash", "slash"),
    linux_sequence_key("backslash", "backslash"),
    linux_sequence_key("grave", "grave"),
];

#[cfg(windows)]
fn listen_for_key_sequence_windows(duration: Duration) -> String {
    let deadline = Instant::now() + duration;
    let keys = windows_sequence_keys();
    let mut previous_down = vec![false; keys.len()];
    let mut parts = SequenceParts::default();

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let modifiers = windows_active_modifiers();
        for (index, key) in keys.iter().enumerate() {
            let down = crate::platform::is_key_pressed(key.vk);
            if down && !previous_down[index] {
                parts.push_key(modifiers, key.name, now);
            }
            previous_down[index] = down;
        }
        thread::sleep(Duration::from_millis(10));
    }

    parts.finish()
}

#[cfg(windows)]
fn windows_active_modifiers() -> ActiveModifiers {
    ActiveModifiers {
        ctrl_down: crate::platform::is_key_pressed(0x11),
        alt_down: crate::platform::is_key_pressed(0x12),
        shift_down: crate::platform::is_key_pressed(0x10),
        meta_down: crate::platform::is_key_pressed(0x5B) || crate::platform::is_key_pressed(0x5C),
    }
}

#[cfg(windows)]
struct WindowsSequenceKey {
    name: &'static str,
    vk: i32,
}

#[cfg(windows)]
const fn windows_sequence_key(name: &'static str, vk: i32) -> WindowsSequenceKey {
    WindowsSequenceKey { name, vk }
}

#[cfg(windows)]
fn windows_sequence_keys() -> &'static [WindowsSequenceKey] {
    WINDOWS_SEQUENCE_KEYS
}

#[cfg(windows)]
static WINDOWS_SEQUENCE_KEYS: &[WindowsSequenceKey] = &[
    windows_sequence_key("esc", 0x1B),
    windows_sequence_key("enter", 0x0D),
    windows_sequence_key("tab", 0x09),
    windows_sequence_key("backspace", 0x08),
    windows_sequence_key("space", 0x20),
    windows_sequence_key("delete", 0x2E),
    windows_sequence_key("insert", 0x2D),
    windows_sequence_key("home", 0x24),
    windows_sequence_key("end", 0x23),
    windows_sequence_key("pageup", 0x21),
    windows_sequence_key("pagedown", 0x22),
    windows_sequence_key("up", 0x26),
    windows_sequence_key("down", 0x28),
    windows_sequence_key("left", 0x25),
    windows_sequence_key("right", 0x27),
    windows_sequence_key("f1", 0x70),
    windows_sequence_key("f2", 0x71),
    windows_sequence_key("f3", 0x72),
    windows_sequence_key("f4", 0x73),
    windows_sequence_key("f5", 0x74),
    windows_sequence_key("f6", 0x75),
    windows_sequence_key("f7", 0x76),
    windows_sequence_key("f8", 0x77),
    windows_sequence_key("f9", 0x78),
    windows_sequence_key("f10", 0x79),
    windows_sequence_key("f11", 0x7A),
    windows_sequence_key("f12", 0x7B),
    windows_sequence_key("printscreen", 0x2C),
    windows_sequence_key("pause", 0x13),
    windows_sequence_key("0", 0x30),
    windows_sequence_key("1", 0x31),
    windows_sequence_key("2", 0x32),
    windows_sequence_key("3", 0x33),
    windows_sequence_key("4", 0x34),
    windows_sequence_key("5", 0x35),
    windows_sequence_key("6", 0x36),
    windows_sequence_key("7", 0x37),
    windows_sequence_key("8", 0x38),
    windows_sequence_key("9", 0x39),
    windows_sequence_key("a", 0x41),
    windows_sequence_key("b", 0x42),
    windows_sequence_key("c", 0x43),
    windows_sequence_key("d", 0x44),
    windows_sequence_key("e", 0x45),
    windows_sequence_key("f", 0x46),
    windows_sequence_key("g", 0x47),
    windows_sequence_key("h", 0x48),
    windows_sequence_key("i", 0x49),
    windows_sequence_key("j", 0x4A),
    windows_sequence_key("k", 0x4B),
    windows_sequence_key("l", 0x4C),
    windows_sequence_key("m", 0x4D),
    windows_sequence_key("n", 0x4E),
    windows_sequence_key("o", 0x4F),
    windows_sequence_key("p", 0x50),
    windows_sequence_key("q", 0x51),
    windows_sequence_key("r", 0x52),
    windows_sequence_key("s", 0x53),
    windows_sequence_key("t", 0x54),
    windows_sequence_key("u", 0x55),
    windows_sequence_key("v", 0x56),
    windows_sequence_key("w", 0x57),
    windows_sequence_key("x", 0x58),
    windows_sequence_key("y", 0x59),
    windows_sequence_key("z", 0x5A),
    windows_sequence_key("minus", 0xBD),
    windows_sequence_key("equal", 0xBB),
    windows_sequence_key("bracketleft", 0xDB),
    windows_sequence_key("bracketright", 0xDD),
    windows_sequence_key("semicolon", 0xBA),
    windows_sequence_key("apostrophe", 0xDE),
    windows_sequence_key("comma", 0xBC),
    windows_sequence_key("period", 0xBE),
    windows_sequence_key("slash", 0xBF),
    windows_sequence_key("backslash", 0xDC),
    windows_sequence_key("grave", 0xC0),
];
