#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tracing::warn;

use super::PlatformApi;

static WAYLAND_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

pub(crate) struct LinuxPlatform {
    warned: AtomicBool,
    is_x11: bool,
    x11_session: Option<Mutex<X11Session>>,
}

impl LinuxPlatform {
    #[must_use]
    pub(crate) fn new() -> Self {
        let is_x11 = std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("x11"))
            .unwrap_or(false)
            || std::env::var("DISPLAY").is_ok();

        if !is_x11 && !WAYLAND_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
            warn!("Wayland detected: fullscreen detection unavailable, reminders will always show");
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
            warned: AtomicBool::new(false),
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
                if !self.warned.swap(true, Ordering::Relaxed) {
                    warn!("X11 fullscreen detection failed: {error}");
                }
                false
            }
        }
    }
}

struct X11Session {
    connection: x11rb::rust_connection::RustConnection,
    screen_num: usize,
    active_window_atom: u32,
    state_atom: u32,
    fullscreen_atom: u32,
}

impl X11Session {
    fn connect() -> Result<Self, String> {
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

        Ok(Self {
            connection,
            screen_num,
            active_window_atom,
            state_atom,
            fullscreen_atom,
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

    let is_fullscreen = state_reply
        .value32()
        .is_some_and(|atoms| atoms.into_iter().any(|atom| atom == session.fullscreen_atom));

    Ok(is_fullscreen)
}
