use rdev::EventType;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use super::tracker::MouseState;

#[derive(Clone, Default)]
struct RdevState {
    x: i32,
    y: i32,
    right_pressed: bool,
    left_just_pressed: bool,
    left_just_released: bool,
    space_pressed: bool,
    ctrl_alt_space: bool,
    ctrl_alt_z_pressed: bool,
    ctrl_m_pressed: bool,
    ctrl_down: bool,
    alt_down: bool,
}

pub(super) struct RdevBackend {
    state: Arc<Mutex<RdevState>>,
}

impl RdevBackend {
    pub(super) fn start() -> Self {
        let state = Arc::new(Mutex::new(RdevState::default()));
        let state_clone = state.clone();
        thread::spawn(move || listen_events(state_clone));
        Self { state }
    }

    pub(super) fn read(&self) -> MouseState {
        let mut s = self.state.lock().unwrap();
        let result = MouseState {
            x: s.x,
            y: s.y,
            right_pressed: s.right_pressed,
            left_just_pressed: s.left_just_pressed,
            left_just_released: s.left_just_released,
            space_pressed: s.space_pressed,
            ctrl_alt_space: s.ctrl_alt_space,
            ctrl_alt_z_pressed: s.ctrl_alt_z_pressed,
            ctrl_m_pressed: s.ctrl_m_pressed,
        };
        s.clear_transient_flags();
        result
    }
}

impl RdevState {
    fn clear_transient_flags(&mut self) {
        self.right_pressed = false;
        self.left_just_pressed = false;
        self.left_just_released = false;
        self.space_pressed = false;
        self.ctrl_alt_space = false;
        self.ctrl_alt_z_pressed = false;
        self.ctrl_m_pressed = false;
    }
}

fn listen_events(state: Arc<Mutex<RdevState>>) {
    let (tx, rx) = mpsc::channel::<rdev::Event>();
    thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event| {
            let _ = tx.send(event);
        }) {
            eprintln!("rdev listen error: {:?}", e);
        }
    });

    for event in rx {
        if let Ok(mut state) = state.lock() {
            apply_event(&mut state, event.event_type);
        }
    }
}

fn apply_event(state: &mut RdevState, event_type: EventType) {
    match event_type {
        EventType::MouseMove { x, y } => {
            state.x = x as i32;
            state.y = y as i32;
        }
        EventType::ButtonPress(rdev::Button::Right) => state.right_pressed = true,
        EventType::ButtonPress(rdev::Button::Left) => state.left_just_pressed = true,
        EventType::ButtonRelease(rdev::Button::Left) => state.left_just_released = true,
        EventType::KeyPress(rdev::Key::Space) => handle_space_press(state),
        EventType::KeyPress(rdev::Key::ControlLeft | rdev::Key::ControlRight) => {
            state.ctrl_down = true
        }
        EventType::KeyRelease(rdev::Key::ControlLeft | rdev::Key::ControlRight) => {
            state.ctrl_down = false
        }
        EventType::KeyPress(rdev::Key::Alt | rdev::Key::AltGr) => state.alt_down = true,
        EventType::KeyRelease(rdev::Key::Alt | rdev::Key::AltGr) => state.alt_down = false,
        EventType::KeyPress(rdev::Key::KeyZ) if state.ctrl_down && state.alt_down => {
            state.ctrl_alt_z_pressed = true
        }
        EventType::KeyPress(rdev::Key::KeyM) if state.ctrl_down => state.ctrl_m_pressed = true,
        _ => {}
    }
}

fn handle_space_press(state: &mut RdevState) {
    state.space_pressed = true;
    if state.ctrl_down && state.alt_down {
        state.ctrl_alt_space = true;
    }
}
