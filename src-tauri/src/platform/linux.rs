#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tracing::warn;

use super::{normalize_process_name, PlatformApi};

static WAYLAND_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

pub(crate) struct LinuxPlatform {
    fullscreen_warned: AtomicBool,
    idle_warned: AtomicBool,
    foreground_warned: AtomicBool,
    is_x11: bool,
    x11_session: Option<Mutex<X11Session>>,
}

impl LinuxPlatform {
    #[must_use]
    pub(crate) fn new() -> Self {
        let is_x11 = std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("x11"))
            || std::env::var("DISPLAY").is_ok();

        if !is_x11 && !WAYLAND_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
            warn!(
                "Wayland detected: fullscreen, idle, and foreground-process detection unavailable; reminders will always show"
            );
        }

        let x11_session = if is_x11 {
            match X11Session::connect() {
                Ok(session) => Some(Mutex::new(session)),
                Err(error) => {
                    warn!("X11 fullscreen detection unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };

        Self {
            fullscreen_warned: AtomicBool::new(false),
            idle_warned: AtomicBool::new(false),
            foreground_warned: AtomicBool::new(false),
            is_x11,
            x11_session,
        }
    }
}

impl PlatformApi for LinuxPlatform {
    fn is_fullscreen_app_active(&self) -> bool {
        if !self.is_x11 {
            return false;
        }

        let Some(session) = self.x11_session.as_ref() else {
            return false;
        };

        match detect_fullscreen_x11(session) {
            Ok(result) => result,
            Err(error) => {
                if !self.fullscreen_warned.swap(true, Ordering::Relaxed) {
                    warn!("X11 fullscreen detection failed: {error}");
                }
                false
            }
        }
    }

    fn idle_duration(&self) -> Option<Duration> {
        if !self.supports_idle_detection() {
            return None;
        }

        let session = self.x11_session.as_ref()?;

        match detect_idle_duration_x11(session) {
            Ok(duration) => Some(duration),
            Err(error) => {
                if !self.idle_warned.swap(true, Ordering::Relaxed) {
                    warn!("X11 idle detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_idle_detection(&self) -> bool {
        self.is_x11
            && self.x11_session.as_ref().is_some_and(|session| {
                session
                    .lock()
                    .is_ok_and(|guard| guard.screensaver_available)
            })
    }

    fn get_foreground_process_name(&self) -> Option<String> {
        if !self.is_x11 {
            return None;
        }
        let session = self.x11_session.as_ref()?;

        match detect_foreground_process_name_x11(session) {
            Ok(Some(name)) => normalize_process_name(&name),
            Ok(None) => None,
            Err(error) => {
                if !self.foreground_warned.swap(true, Ordering::Relaxed) {
                    warn!("X11 foreground process detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_foreground_process_detection(&self) -> bool {
        self.is_x11 && self.x11_session.is_some()
    }
}

struct X11Session {
    connection: x11rb::rust_connection::RustConnection,
    screen_num: usize,
    active_window_atom: u32,
    state_atom: u32,
    fullscreen_atom: u32,
    wm_pid_atom: u32,
    screensaver_available: bool,
}

impl X11Session {
    fn connect() -> Result<Self, String> {
        use x11rb::protocol::screensaver::ConnectionExt as ScreenSaverConnectionExt;
        use x11rb::protocol::xproto::ConnectionExt;

        let (connection, screen_num) =
            x11rb::connect(None).map_err(|error| format!("X11 connect failed: {error}"))?;

        let active_window_atom = connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .map_err(|error| format!("intern _NET_ACTIVE_WINDOW: {error}"))?
            .reply()
            .map_err(|error| format!("reply _NET_ACTIVE_WINDOW: {error}"))?
            .atom;

        let state_atom = connection
            .intern_atom(false, b"_NET_WM_STATE")
            .map_err(|error| format!("intern _NET_WM_STATE: {error}"))?
            .reply()
            .map_err(|error| format!("reply _NET_WM_STATE: {error}"))?
            .atom;

        let fullscreen_atom = connection
            .intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")
            .map_err(|error| format!("intern _NET_WM_STATE_FULLSCREEN: {error}"))?
            .reply()
            .map_err(|error| format!("reply _NET_WM_STATE_FULLSCREEN: {error}"))?
            .atom;

        let wm_pid_atom = connection
            .intern_atom(false, b"_NET_WM_PID")
            .map_err(|error| format!("intern _NET_WM_PID: {error}"))?
            .reply()
            .map_err(|error| format!("reply _NET_WM_PID: {error}"))?
            .atom;

        let screensaver_available = connection
            .screensaver_query_version(1, 1)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some();

        Ok(Self {
            connection,
            screen_num,
            active_window_atom,
            state_atom,
            fullscreen_atom,
            wm_pid_atom,
            screensaver_available,
        })
    }
}

fn detect_fullscreen_x11(session: &Mutex<X11Session>) -> Result<bool, String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let session = session
        .lock()
        .map_err(|_| "X11 session lock poisoned".to_string())?;
    let screen = &session.connection.setup().roots[session.screen_num];

    let active_window_reply = session
        .connection
        .get_property(
            false,
            screen.root,
            session.active_window_atom,
            AtomEnum::WINDOW,
            0,
            1,
        )
        .map_err(|error| format!("get _NET_ACTIVE_WINDOW: {error}"))?
        .reply()
        .map_err(|error| format!("reply active window property: {error}"))?;

    let active_window = active_window_reply
        .value32()
        .and_then(|mut values| values.next())
        .unwrap_or(0);

    if active_window == 0 {
        return Ok(false);
    }

    let state_reply = session
        .connection
        .get_property(
            false,
            active_window,
            session.state_atom,
            AtomEnum::ATOM,
            0,
            32,
        )
        .map_err(|error| format!("get _NET_WM_STATE: {error}"))?
        .reply()
        .map_err(|error| format!("reply state property: {error}"))?;

    let is_fullscreen = state_reply.value32().is_some_and(|atoms| {
        atoms
            .into_iter()
            .any(|atom| atom == session.fullscreen_atom)
    });

    Ok(is_fullscreen)
}

fn detect_idle_duration_x11(session: &Mutex<X11Session>) -> Result<Duration, String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::screensaver::ConnectionExt as ScreenSaverConnectionExt;

    let session = session
        .lock()
        .map_err(|_| "X11 session lock poisoned".to_string())?;

    if !session.screensaver_available {
        return Err("XScreenSaver extension unavailable".to_string());
    }

    let screen = &session.connection.setup().roots[session.screen_num];
    let reply = session
        .connection
        .screensaver_query_info(screen.root)
        .map_err(|error| format!("screensaver query info: {error}"))?
        .reply()
        .map_err(|error| format!("reply screensaver query info: {error}"))?;

    Ok(Duration::from_millis(u64::from(reply.ms_since_user_input)))
}

fn detect_foreground_process_name_x11(
    session: &Mutex<X11Session>,
) -> Result<Option<String>, String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let session = session
        .lock()
        .map_err(|_| "X11 session lock poisoned".to_string())?;
    let screen = &session.connection.setup().roots[session.screen_num];

    let active_window_reply = session
        .connection
        .get_property(
            false,
            screen.root,
            session.active_window_atom,
            AtomEnum::WINDOW,
            0,
            1,
        )
        .map_err(|error| format!("get _NET_ACTIVE_WINDOW: {error}"))?
        .reply()
        .map_err(|error| format!("reply active window property: {error}"))?;

    let active_window = active_window_reply
        .value32()
        .and_then(|mut values| values.next())
        .unwrap_or(0);

    if active_window == 0 {
        return Ok(None);
    }

    let pid_reply = session
        .connection
        .get_property(
            false,
            active_window,
            session.wm_pid_atom,
            AtomEnum::CARDINAL,
            0,
            1,
        )
        .map_err(|error| format!("get _NET_WM_PID: {error}"))?
        .reply()
        .map_err(|error| format!("reply pid property: {error}"))?;

    let pid = pid_reply
        .value32()
        .and_then(|mut values| values.next())
        .unwrap_or(0);

    if pid == 0 {
        return Ok(None);
    }

    let exe_link = format!("/proc/{pid}/exe");
    let Ok(exe_path) = std::fs::read_link(&exe_link) else {
        return Ok(None);
    };

    Ok(exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_owned))
}
