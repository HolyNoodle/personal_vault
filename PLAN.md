# Plan: Fix WASM File Explorer + GStreamer Pipeline (Remove X11 Legacy)

## Context

The codebase evolved from an X11-native approach (apps on Xvfb, captured via `ximagesrc`) to a WASM-based approach (apps rendering to a framebuffer). But the backend pipeline was never updated — it still starts Xvfb, openbox, dbus, and tries to capture via X11. The WASM app renders to an internal buffer that nobody reads, while GStreamer captures an empty X11 display. All X11/FFmpeg dependencies are legacy and should be removed.

### Current broken flow
```
wasmtime CLI → renders to framebuffer internally → nobody reads it
Xvfb (empty) → ximagesrc → captures black → GStreamer → WebRTC
```

### Target flow
```
wasmtime (embedded via crate) → call render_frame() → read WASM linear memory
  → appsrc → videoconvert → vp8enc → appsink → WebRTC
```

---

## All Issues Found

### WASM App (`apps/file-explorer/`)
1. **main.rs:37** — Double tessellation: `ctx.tessellate(ctx.run(|_ctx| {}).shapes)` then `ctx.tessellate(shapes)` again — type mismatch, produces garbage
2. **main.rs:28** — `unsafe { std::mem::zeroed() }` for `eframe::Frame` — invalid vtable, UB
3. **main.rs:41-43** — Fills entire buffer black AFTER rendering, overwriting content
4. **main.rs:45** — Destructures `ClippedPrimitive` incorrectly (not a tuple)
5. **Cargo.toml** — `eframe` with `glow` feature not needed for headless; `autobins = false` but no `[bin]` target
6. **No input handling** — No exported functions for mouse/keyboard events from host

### Backend
7. **xvfb.rs:175-231** — Spawns `wasmtime` as CLI child process instead of embedding it; the WASM module exports functions meant to be called by a host, not run standalone (no `_start`)
8. **xvfb.rs:36-101** — Starts Xvfb + dbus + openbox for WASM apps that don't use X11
9. **gstreamer.rs:41-45** — Uses `ximagesrc` to capture from X11 display; WASM app has no X11 window → black frames
10. **gstreamer.rs:36** — `set_var("DISPLAY", ...)` is process-wide, not thread-safe for multiple sessions
11. **webrtc.rs:166-172** — Gets display string from XvfbManager, but WASM apps don't use displays
12. **webrtc.rs:174** — Registers X11 input session for WASM apps — X11 input won't work
13. **webrtc.rs:186-190** — IVF header skip logic is fragile; GStreamer `appsink` doesn't produce IVF format anyway (raw VP8 frames)
14. **main.rs:192-216** — `check_prerequisites()` requires Xvfb + FFmpeg at startup
15. **x11_input.rs** — Entire module is X11-only, won't work for WASM apps
16. **ffmpeg.rs** — Dead code, unused

### Dockerfile
17. Installs `xvfb`, `xterm`, `thunar`, `openbox`, `xdotool`, `wmctrl`, `dbus-x11`, `ffmpeg` — none needed
18. Missing GStreamer runtime plugins (`gstreamer1.0-plugins-base/good/ugly`)
19. Missing GStreamer build deps in builder stage
20. `wasmtime` installed to `/root/.wasmtime` but runtime user is `sandbox`
21. WASM binary never built or copied to runtime image

### Cargo dependencies to remove
22. `x11rb` — X11 protocol (backend/Cargo.toml:94)
23. `gstreamer-rtp`, `gstreamer-webrtc` — not used in current code
24. `libc` in backend — only used for `killpg` in cleanup (can keep or use `nix` crate)
25. `gstreamer`/`gstreamer-app` in file-explorer Cargo.toml — app shouldn't need GStreamer

---

## Implementation Plan

### Step 1: Fix WASM app rendering (`apps/file-explorer/src/main.rs`)

Rewrite the render function with correct egui software rendering:
- Use `ctx.run()` to run UI logic and collect paint output
- Call `ctx.tessellate()` once on the returned shapes
- Rasterize `ClippedPrimitive` meshes properly to the RGBA buffer (software rasterizer using `epaint`)
- Remove the fake `eframe::Frame` hack — use `egui::Context` directly without eframe
- Add exported functions for input: `handle_pointer_event(x, y, pressed)`, `handle_key_event(keycode, pressed)`

Update `Cargo.toml`:
- Remove `eframe` dependency (only need `egui` + `epaint` for headless rendering)
- Remove `glow` feature
- Remove `gstreamer`/`gstreamer-app` deps from the app
- Keep `egui`, `epaint`, `once_cell`, `serde`, `serde_json`

**Files**: `apps/file-explorer/src/main.rs`, `apps/file-explorer/src/app.rs`, `apps/file-explorer/Cargo.toml`

### Step 2: Create `WasmAppManager` (replaces XvfbManager)

New module using the `wasmtime` crate to:
- Load `.wasm` modules from disk
- Instantiate with WASI support (for stdio/filesystem access inside sandbox)
- Call `render_file_explorer_frame()` in a render loop (~30fps)
- Read framebuffer from WASM linear memory via `get_framebuffer_ptr()` + `get_framebuffer_size()`
- Forward input events by calling exported WASM functions (input handling lives here, no separate input module)
- Send raw RGBA frames via `tokio::sync::mpsc` channel to GStreamer

**Files**: New `backend/src/infrastructure/driven/sandbox/wasm_runtime.rs`, update `backend/src/infrastructure/driven/sandbox/mod.rs`

### Step 3: Replace `ximagesrc` with `appsrc` in GStreamer

Rewrite `gstreamer.rs`:
- Replace pipeline: `appsrc → videoconvert → vp8enc → appsink`
- `appsrc` accepts raw `video/x-raw,format=RGBA` frames pushed from the WasmAppManager
- Accept a `tokio::sync::mpsc::Receiver<Vec<u8>>` for frame input
- Remove all X11/DISPLAY references
- Remove IVF header logic (appsink produces raw VP8 frames, not IVF)

**Files**: `backend/src/infrastructure/driven/sandbox/gstreamer.rs`

### Step 4: Update WebRTC adapter

Modify `webrtc.rs`:
- Remove `XvfbManager` dependency — use `WasmAppManager` instead
- Remove `X11InputManager` — route input through `WasmAppManager`
- Remove `ffmpeg_handles` field (dead code)
- Connect WasmAppManager frame channel → GStreamer appsrc → VP8 → WebRTC track
- Remove display_str lookups
- Remove `stream_vp8_to_track()` function (dead FFmpeg code at bottom)

**Files**: `backend/src/infrastructure/driving/webrtc.rs`

### Step 5: Remove X11/FFmpeg legacy code and dependencies

**Delete files**:
- `backend/src/infrastructure/driven/sandbox/xvfb.rs`
- `backend/src/infrastructure/driven/sandbox/ffmpeg.rs`
- `backend/src/infrastructure/driven/input/x11_input.rs`
- `backend/src/infrastructure/driven/input/xdotool.rs`
- `backend/src/infrastructure/driven/input/mod.rs` (delete entire input/ directory)

**Update `backend/Cargo.toml`** — remove:
- `x11rb`
- `gstreamer-rtp` (unused)
- `gstreamer-webrtc` (unused)

**Add to `backend/Cargo.toml`**:
- `wasmtime` crate with WASI support

**Update `backend/src/infrastructure/driven/sandbox/mod.rs`**:
- Remove `xvfb` and `ffmpeg` modules
- Add `wasm_runtime` module
- Export `WasmAppManager` instead of `XvfbManager`

**Update `backend/src/infrastructure/driven/mod.rs`**:
- Remove `XvfbManager` exports, remove `input` module
- Add `WasmAppManager` export

**Update `backend/src/main.rs`**:
- Replace `XvfbManager::new()` with `WasmAppManager::new()`
- Remove `check_prerequisites()` Xvfb/FFmpeg checks — add wasmtime + GStreamer checks
- Update `ApiState` and handler initialization

**Files**: Multiple (see above)

### Step 6: Fix Dockerfile

**Remove from runtime stage**: `xvfb`, `ffmpeg`, `xterm`, `thunar`, `dbus-x11`, `wmctrl`, `openbox`, `xdotool`

**Add to runtime stage**: `libgstreamer1.0-0`, `gstreamer1.0-plugins-base`, `gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-ugly` (VP8 codec), `gstreamer1.0-libav`

**Add to builder stage**: `libgstreamer1.0-dev`, `libgstreamer-plugins-base1.0-dev` (GStreamer build deps)

**Add WASM build stage**:
```dockerfile
FROM rust:1.93-slim AS wasm-builder
RUN rustup target add wasm32-unknown-unknown
COPY apps/file-explorer/ ./apps/file-explorer/
RUN cargo build --target wasm32-unknown-unknown --release -p file-explorer
```

**Copy WASM binary**: `COPY --from=wasm-builder ... /app/wasm/file_explorer.wasm`

**Remove wasmtime CLI install** (embedded via crate, CLI not needed)

**Files**: `backend/Dockerfile`

### Step 7: Update manifest and config

- `apps/file-explorer/manifest.json`: Remove `command` field (no longer launching as process), add `wasm_path` field for embedded loading
- Update `check_prerequisites()` in main.rs

**Files**: `apps/file-explorer/manifest.json`, `backend/src/main.rs`

---

## Verification

1. `cargo build -p file-explorer --target wasm32-unknown-unknown` — WASM app compiles
2. `cargo build -p sandbox-server` — backend compiles without X11 deps
3. Run backend → connect WebSocket → `request-offer`:
   - WasmAppManager loads WASM module
   - Render loop produces non-black RGBA frames
   - GStreamer appsrc receives frames → VP8 encoding
   - WebRTC peer receives video showing file explorer UI
4. Send mouse/key events via WebSocket → verify egui UI responds
5. `docker compose build` succeeds without X11 packages