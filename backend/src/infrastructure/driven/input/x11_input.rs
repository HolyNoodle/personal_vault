use anyhow::{Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use x11rb::connection::Connection;
use x11rb::protocol::xtest;
use x11rb::protocol::xproto;
use x11rb::rust_connection::RustConnection;

/// Manages input forwarding to X11 displays using direct X11 protocol
pub struct X11InputManager {
    connections: Arc<RwLock<HashMap<String, Arc<RustConnection>>>>,
}

impl X11InputManager {
        pub async fn handle_mouse_scroll(&self, session_id: &str, delta_y: f32) -> Result<()> {
            tracing::info!("Mouse scroll: session={}, delta_y={}", session_id, delta_y);
            let conn = {
                let connections = self.connections.read().await;
                connections.get(session_id).cloned()
            };

            if let Some(conn) = conn {
                // Normalize scroll direction and sensitivity
                let normalized_delta = -delta_y / 150.0; // Reverse and scale for less sensitivity
                let button = if normalized_delta > 0.0 { 4 } else { 5 }; // 4 = scroll up, 5 = scroll down
                let count = normalized_delta.abs().round() as u8;
                tokio::task::spawn_blocking(move || {
                    for _ in 0..count {
                        let _ = xtest::fake_input(&*conn, 4, button, 0, x11rb::NONE, 0, 0, 0);
                        let _ = xtest::fake_input(&*conn, 5, button, 0, x11rb::NONE, 0, 0, 0);
                    }
                    let _ = conn.flush();
                });
            } else {
                tracing::warn!("No X11 connection found for session: {}", session_id);
            }
            Ok(())
        }
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_session(&self, session_id: String, display: String) {
        // Connect to X11 display in a blocking task
        let display_str = display.clone();
        tracing::info!("Attempting to connect to X11 display: {}", display_str);
        let result = tokio::task::spawn_blocking(move || {
            RustConnection::connect(Some(&display))
                .map(|(conn, _)| Arc::new(conn))
                .ok()
        })
        .await;

        if let Ok(Some(conn)) = result {
            tracing::info!("Successfully connected to X11 display {} for session {}", display_str, session_id);
            let mut connections = self.connections.write().await;
            connections.insert(session_id, conn);
        } else {
            tracing::error!("Failed to connect to X11 display {} for session {}", display_str, session_id);
        }
    }

    pub async fn unregister_session(&self, session_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(session_id);
    }

    pub async fn handle_mouse_move(&self, session_id: &str, x: i32, y: i32) -> Result<()> {
        tracing::debug!("Mouse move: session={}, x={}, y={}", session_id, x, y);
        let conn = {
            let connections = self.connections.read().await;
            connections.get(session_id).cloned()
        };

        if let Some(conn) = conn {
            let conn_clone = conn.clone();
            tokio::task::spawn_blocking(move || {
                let screen = &conn_clone.setup().roots[0];
                match xproto::warp_pointer(
                    &*conn_clone,
                    x11rb::NONE,
                    screen.root,
                    0,
                    0,
                    0,
                    0,
                    x as i16,
                    y as i16,
                ) {
                    Ok(_) => {
                        if let Err(e) = conn_clone.flush() {
                            tracing::error!("Failed to flush warp_pointer: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to warp pointer: {}", e);
                    }
                }
            });
        } else {
            tracing::warn!("No X11 connection found for session: {}", session_id);
        }
        Ok(())
    }

    pub async fn handle_mouse_down(&self, session_id: &str, button: u8) -> Result<()> {
        tracing::info!("Mouse down: session={}, button={}", session_id, button);
        let conn = {
            let connections = self.connections.read().await;
            connections.get(session_id).cloned()
        };

        if let Some(conn) = conn {
            tokio::task::spawn_blocking(move || {
                match xtest::fake_input(
                    &*conn,
                    4, // ButtonPress (X11 event type 4)
                    button,
                    0, // timestamp (0 = current time)
                    x11rb::NONE,
                    0,
                    0,
                    0,
                ) {
                    Ok(cookie) => {
                        // Check the cookie to ensure request is sent
                        if let Err(e) = cookie.check() {
                            tracing::error!("Failed to check mouse_down cookie: {}", e);
                        }
                        if let Err(e) = conn.flush() {
                            tracing::error!("Failed to flush mouse_down: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to send mouse_down: {}", e);
                    }
                }
            });
        } else {
            tracing::warn!("No X11 connection found for session: {}", session_id);
        }
        Ok(())
    }

    pub async fn handle_mouse_up(&self, session_id: &str, button: u8) -> Result<()> {
        let conn = {
            let connections = self.connections.read().await;
            connections.get(session_id).cloned()
        };

        if let Some(conn) = conn {
            tokio::task::spawn_blocking(move || {
                let _ = xtest::fake_input(
                    &*conn,
                    5, // ButtonRelease (X11 event type 5)
                    button,
                    0, // timestamp (0 = current time)
                    x11rb::NONE,
                    0,
                    0,
                    0,
                );
                let _ = conn.flush();
            });
        }
        Ok(())
    }

    pub async fn handle_key_down(&self, session_id: &str, key: &str) -> Result<()> {
        tracing::info!("Key down: session={}, key={}", session_id, key);
        let conn = {
            let connections = self.connections.read().await;
            connections.get(session_id).cloned()
        };

        if let Some(conn) = conn {
            let keycode = map_key_to_keycode(key);
            tokio::task::spawn_blocking(move || {
                let _ = xtest::fake_input(
                    &*conn,
                    2, // KeyPress (X11 event type 2)
                    keycode,
                    0,
                    x11rb::NONE,
                    0,
                    0,
                    0,
                );
                let _ = conn.flush();
            });
        }
        Ok(())
    }

    pub async fn handle_key_up(&self, session_id: &str, key: &str) -> Result<()> {
        let conn = {
            let connections = self.connections.read().await;
            connections.get(session_id).cloned()
        };

        if let Some (conn) = conn {
            let keycode = map_key_to_keycode(key);
            tokio::task::spawn_blocking(move || {
                let _ = xtest::fake_input(
                    &*conn,
                    3, // KeyRelease (X11 event type 3)
                    keycode,
                    0,
                    x11rb::NONE,
                    0,
                    0,
                    0,
                );
                let _ = conn.flush();
            });
        }
        Ok(())
    }
}

// Map web key names to X11 keycodes (simplified mapping)
fn map_key_to_keycode(key: &str) -> u8 {
    match key {
        "Enter" => 36,
        "Escape" => 9,
        "Backspace" => 22,
        "Tab" => 23,
        "Shift" => 50,
        "Control" => 37,
        "Alt" => 64,
        "Meta" => 133,
        "ArrowUp" => 111,
        "ArrowDown" => 116,
        "ArrowLeft" => 113,
        "ArrowRight" => 114,
        "Delete" => 119,
        "Home" => 110,
        "End" => 115,
        "PageUp" => 112,
        "PageDown" => 117,
        " " => 65,
        "a" | "A" => 38,
        "b" | "B" => 56,
        "c" | "C" => 54,
        "d" | "D" => 40,
        "e" | "E" => 26,
        "f" | "F" => 41,
        "g" | "G" => 42,
        "h" | "H" => 43,
        "i" | "I" => 31,
        "j" | "J" => 44,
        "k" | "K" => 45,
        "l" | "L" => 46,
        "m" | "M" => 58,
        "n" | "N" => 57,
        "o" | "O" => 32,
        "p" | "P" => 33,
        "q" | "Q" => 24,
        "r" | "R" => 27,
        "s" | "S" => 39,
        "t" | "T" => 28,
        "u" | "U" => 30,
        "v" | "V" => 55,
        "w" | "W" => 25,
        "x" | "X" => 53,
        "y" | "Y" => 29,
        "z" | "Z" => 52,
        "0" => 19,
        "1" => 10,
        "2" => 11,
        "3" => 12,
        "4" => 13,
        "5" => 14,
        "6" => 15,
        "7" => 16,
        "8" => 17,
        "9" => 18,
        _ => 0,
    }
}
