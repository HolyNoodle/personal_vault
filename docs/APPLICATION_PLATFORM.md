# Application Platform Architecture

## Overview

The Secure Sandbox Platform is a sandboxed application hosting system where all applications run server-side in extreme isolation and are delivered to users via WebRTC video. Different user roles (Owner vs Client) receive different file system permissions within the sandbox.

**Key Insight**: Video streaming is the sandboxing and data control mechanism. All users interact with a video feed (input forwarding), ensuring zero data exfiltration. Permissions are enforced server-side via Landlock LSM.

---

## Sandboxed Execution Model

### Architecture Overview

**All Users**: Both Owner and Client users access applications the same way - through video streaming

**How It Works**:
```
User Browser → WebRTC Video Stream ← Server-Side App ← Isolated Sandbox
      ↓                                      ↑              (Landlock LSM)
   Input Events (mouse/keyboard) ────────────┘              Role-based permissions
```

**Execution Flow**:
1. Backend creates an isolated sandbox: new mount namespace (only selected paths bind-mounted), network namespace, Landlock LSM policy, seccomp filter, cgroups
2. Sandbox permissions configured based on user role and owner-configured share settings — applied before exec
3. App binary (native Linux executable) is spawned inside the sandbox; it has no visibility of the host filesystem beyond what was bind-mounted
4. App renders to an RGBA framebuffer via a shared memory region the backend also maps
5. Framebuffer pixels are fed into a GStreamer appsrc pipeline, encoded to VP8, and sent as a WebRTC video track (DTLS-SRTP encrypted)
6. User interacts with video feed — input events are sent back to server over the WebSocket signaling channel and forwarded into the app via a Unix socket

---

> **Status (WIP)**:
> - GStreamer VP8 encode + WebRTC video track: **implemented**
> - WebSocket signaling (offer/answer/ICE + input events): **implemented**
> - Native process sandbox (mount namespace + Landlock + seccomp): **planned** — current file-explorer is a WASM prototype; production model is native processes
> - Shared memory framebuffer IPC between app and backend: **planned**
> - Unix socket input event channel: **planned**
> - WebRTC security hardening (WSS, auth on `/ws`, encrypted TURN, input via data channel): **not yet implemented — see Security Considerations**
> - `sandbox-app-sdk` crate (rendering loop + input handling for native apps): **planned**

---

## Role-Based Permissions

### Owner Users (Full Access)

**Sandbox Permissions**:
- ✅ **Read/Write/Delete access** — Full file system access within their storage root (Landlock grants read/write/delete on their root directory)
- ✅ **Can configure client access** — Owner sets per-share permissions for client sessions
- ✅ **Network access** — Can make API calls (if enabled per-app)
- ✅ **Resource quota** — Higher CPU/memory limits

**Capabilities**:
- Full file management through sandboxed UI
- Upload new files (streamed through sandbox)
- Rename, delete, organize files
- Preview and edit files
- Configure which paths and operations to expose to clients via share settings

---

### Client Users (Owner-Configured Access)

Client permissions are **not a fixed platform-wide policy**. Instead, they are configured by the owner per share. The owner grants specific access — read on certain folders/files, write on others, or none — and these per-path rules are translated into the Landlock policy applied to the client's sandboxed session.

**Examples**:
- Owner shares `Reports/` → client gets read-only on `Reports/` only
- Owner shares `Collaboration/` with write → client can read+write there only
- Owner shares a single file → client gets read access to that file path only

**Security Guarantees** (always enforced regardless of share config):
- ❌ **No access outside shared paths** — Landlock enforces kernel-level path restrictions
- ❌ **No clipboard access** — Copy/paste disabled
- ❌ **No local file access** — Files stay server-side
- ❌ **No network access** — Sandbox has no internet
- ✅ **All actions logged** — Complete audit trail
- ✅ **Landlock enforcement** — Kernel-level permission control
- ✅ **Resource limits** — cgroups prevent abuse

**Use Cases**:
- Healthcare: Patient viewing medical records (can't download HIPAA data)
- Legal: Client reviewing case files (can't copy confidential documents)
- Finance: Customer viewing financial reports (can't export sensitive data)

---

## First Application: File Explorer

### Features

**Core Functionality**:
- Directory tree navigation
- File listing with metadata (name, size, modified date, type)
- File preview (image formats)
- Search and filtering
- Sorting (name, date, size, type)

**Owner Session**:
- Full file management (upload, rename, delete, organize)
- Preview files without downloading
- Navigate directory structure
- Share management: configure which paths/operations to expose to clients

**Client Session** (permissions configured by owner):
- Access limited to the paths and operations the owner shared
- All actions visible to owner via audit log

### Technical Implementation

**Current Implementation — WASM + egui + wasmtime**:

The file explorer is a WASM binary (`cdylib`) built with egui for UI logic. It runs inside a wasmtime runtime on the backend and renders to an RGBA framebuffer that the backend reads each frame.

```
crate-type = ["cdylib"]   (Cargo.toml)

egui 0.29 — immediate-mode UI logic (layout, widgets, input)
epaint 0.29 — tessellation + software rasterizer (triangle meshes → RGBA pixels)
```

Key implementation facts:
- Fonts (DejaVuSans, LiberationSans) are embedded in the WASM binary at compile time via `include_bytes!`
- The backend calls exported C-ABI functions each frame — no OS threads, no host syscalls unless explicitly imported
- Input (mouse position, click state, keyboard events) is forwarded from the backend into the app via `handle_pointer_event` / `handle_key_event` exports
- Framebuffer accessors (`get_framebuffer_ptr`, `get_framebuffer_size`, `get_width`, `get_height`) let the backend read rendered pixels

Reference: `apps/file-explorer/` — `Cargo.toml`, `src/app.rs`, `src/renderer.rs`, `manifest.json`.

---

## Native Process Execution Model

Apps on this platform are native Linux executables. Isolation is provided by the OS — not by a language runtime — making it equivalent in security model to containers, and as secure as the OS kernel and sandbox configuration.

### Sandbox setup (applied before exec)

The backend prepares the sandbox before spawning the app process:

1. **Mount namespace** — a new namespace is created; only the paths the session needs are bind-mounted into it. The app's process sees only those paths; the rest of the host filesystem does not exist from its perspective (`ENOENT`, not `EACCES`).
2. **Landlock LSM** — a ruleset is applied that restricts which of the mounted paths can be opened and with which operations (read, write, delete). This is belt-and-suspenders on top of the mount namespace.
3. **Network namespace** — no network interfaces; the app is fully offline.
4. **seccomp** — a syscall allowlist is applied; dangerous syscalls (e.g. `ptrace`, `mount`, `pivot_root`) are blocked.
5. **cgroups** — CPU, memory, and PID limits are applied to the app's cgroup.

The mount namespace gives the strongest filesystem isolation: paths outside the session's allowlist don't exist, so the app cannot enumerate, stat, or discover them. Landlock prevents access even in edge cases where a path might be visible.

### Per-frame rendering loop

1. Backend maps a shared memory region (e.g. `memfd`) as the framebuffer
2. App maps the same region and renders pixels into it each frame (egui → RGBA)
3. Backend reads the framebuffer and feeds it into GStreamer → VP8 → WebRTC

The shared memory approach is zero-copy — no data is transferred between processes, the backend simply reads what the app wrote.

### Input forwarding

The backend forwards mouse and keyboard events to the app via a **Unix socket** (or pipe). The app reads events from this channel on each iteration of its render loop and feeds them as input to egui.

### Capabilities available to the app

Because the app is a native binary inside a well-configured sandbox, it can use:
- `std::fs` directly — no host function protocol needed; Landlock enforces access
- SQLite (or any file-based database) — full native support, operates on files within the sandbox
- Any native library — no compilation-to-WASM required
- Threads, async runtimes, memory-mapped files — all standard Linux capabilities

The security boundary is the sandbox configuration, not the language runtime.

---

## Communication Contract

The interface between the platform backend and a running app instance uses two IPC channels and a shared memory framebuffer.

### Framebuffer (backend ← app)

A `memfd` shared memory region is created by the backend before exec and passed to the app as an open file descriptor. The app maps it and writes RGBA pixels each frame. The backend maps the same region and reads it on its render tick. No data is copied between processes.

The framebuffer dimensions are sent to the app at startup (and on resize) via the control socket.

### Control socket (backend → app)

A Unix socket is created before exec and passed to the app as an open file descriptor. The backend sends length-prefixed messages; the app reads them on each loop iteration.

**Messages the backend sends to the app:**

| Message | Payload |
|---------|---------|
| `Init` | width, height, framerate, session metadata |
| `Resize` | new width, new height |
| `PointerMove` | x, y |
| `PointerButton` | x, y, button, pressed |
| `KeyEvent` | key string, pressed, modifiers |
| `Shutdown` | graceful termination signal |

The app may also write messages back on the same socket (e.g. to request a file dialog, notify the backend of an upload, etc.) — this is defined per-app in `manifest.json` capabilities.

### What the app declares in `manifest.json`

- **Identity**: `name`, `version`, `description`, `type` (`"native"`)
- **Binary**: filename of the executable within the app directory
- **Permissions**: which filesystem paths the app needs and with what access (`read`, `write`, `delete`)
- **Capabilities**: logical operations the app exposes (`upload`, `download`, `preview`, …)

Example:
```json
{
  "name": "File Explorer",
  "version": "0.1.0",
  "type": "native",
  "binary": "file_explorer",
  "permissions": [
    { "path": ".", "access": ["read", "write", "delete"] }
  ],
  "capabilities": ["upload", "download", "delete", "preview"]
}
```

The `permissions` block is what the backend uses to configure the Landlock ruleset and bind mounts for the session. `"path": "."` means the user's storage root; the backend resolves it to the actual session path before applying the policy.

### Permission tokens (current)

| Token | Meaning |
|-------|---------|
| `filesystem:read` | Read files within allowed paths |
| `filesystem:write` | Write/create files within allowed paths |
| `filesystem:delete` | Delete files within allowed paths |
| `ipc:send` | Send messages to backend |
| `ipc:receive` | Receive messages from backend |

---

## App Installation & Discovery

Apps are dynamically installed by placing them in a configured directory. The backend scans this directory on startup to discover available apps.

### Apps directory layout

Configured via the `APPS_ROOT` environment variable:

```
$APPS_ROOT/
├── file-explorer/
│   ├── manifest.json        # Required
│   └── file_explorer        # Required — native Linux executable
├── my-custom-app/
│   ├── manifest.json
│   ├── my_custom_app        # Native executable
│   └── data.db              # Any supporting files the app needs at startup
```

### Discovery rules

- Each subdirectory in `$APPS_ROOT` is a candidate app
- Must contain a `manifest.json` with a `binary` field pointing to an existing executable file
- Invalid or incomplete apps are skipped with a warning; they do not block other apps from loading
- The app's ID in the registry is the subdirectory name

### Install procedure

1. **Build**: `cargo build --release` (native target — no cross-compilation needed)
2. **Copy** the binary + `manifest.json` (+ any static assets) into `$APPS_ROOT/<app-name>/`
3. **Restart** the backend (hot-reload is planned but not yet implemented)

---

## Building a New App — Step by Step

1. **Create** `apps/my-app/` directory and add it as a workspace member in the root `Cargo.toml`

2. **`Cargo.toml`**:
   ```toml
   [[bin]]
   name = "my_app"

   [dependencies]
   egui = "0.29"
   epaint = "0.29"
   # sandbox-app-sdk = { path = "../../crates/sandbox-app-sdk" }  # once available
   # rusqlite = "0.31"   # SQLite, or any other native library — works normally
   ```

3. **`manifest.json`**: declare identity, binary filename, required permissions (which paths and access modes), and capabilities.

4. **`src/main.rs`**:
   - Read the framebuffer fd and control socket fd from the environment (set by the backend before exec)
   - Map the shared memory framebuffer
   - Enter the render loop: read input events from the control socket → run egui → write pixels to framebuffer
   - The `sandbox-app-sdk` will handle all of this once available; for now, see the SDK section for the planned interface

5. **`src/app.rs`**: define your app state and a `show(&mut self, ctx: &egui::Context)` method — identical pattern to the current file-explorer.

6. **Filesystem access**: use `std::fs` directly. Landlock and the mount namespace enforced by the backend guarantee the app can only access what `manifest.json` declared.

7. **Build, install, start backend** — see "App Installation & Discovery" above.

---

## sandbox-app-sdk crate (planned)

Because apps are native binaries, the SDK is a straightforward Rust library — no host function ABI, no serialization protocol, no WASM-specific complexity. App authors link against it and implement one trait.

### What the SDK provides

**1. Rendering + event loop** — maps the shared framebuffer fd, reads the control socket, drives egui, writes pixels. App authors never touch IPC or framebuffer management directly:

```rust
use sandbox_app_sdk::SandboxApp;

struct MyApp {
    items: Vec<std::fs::DirEntry>,
}

impl SandboxApp for MyApp {
    fn new() -> Self {
        // std::fs works directly — Landlock + mount namespace enforce the boundary
        let items = std::fs::read_dir("/").map(|d| d.flatten().collect()).unwrap_or_default();
        MyApp { items }
    }

    fn show(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            for entry in &self.items {
                ui.label(entry.file_name().to_string_lossy());
            }
        });
    }
}

fn main() {
    sandbox_app_sdk::run::<MyApp>();
}
```

**2. Backend message types** — typed wrappers over the control socket protocol so apps can send capability notifications (e.g. upload complete, request file dialog):

```rust
sandbox_app_sdk::notify::upload_complete(path);
sandbox_app_sdk::notify::download_ready(path, filename);
```

### What the SDK does NOT provide

- Filesystem access — apps use `std::fs` directly; Landlock enforces the policy
- SQLite or database access — apps link whatever they need natively
- Any abstraction over what the sandbox allows — that is configured by the backend, not the SDK

### Planned location: `crates/sandbox-app-sdk/`

**Status: Planned — not yet implemented.**

---

## Architecture Diagrams

> **Note**: The WebRTC signaling channel currently uses plain `ws://` and lacks authentication — production hardening is required (see Security Considerations). The native process sandbox is planned; the current file-explorer is a WASM prototype.

### Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                     CLIENT USER BROWSER                          │
│  ┌────────────────────────────┐  ┌───────────────────┐          │
│  │ WebRTC Player (VP8 video)  │  │ Input Forwarder   │          │
│  └────────────────────────────┘  │ (Mouse/Keyboard)  │          │
│                ↑                  └───────────────────┘          │
│                │ VP8 over WebRTC          │ Events over WS       │
└───────────────────────────────────────────┼──────────────────────┘
                │                           │
                ∨                           ∨
┌──────────────────────────────────────────────────────────────────┐
│                    BACKEND SERVER (Rust/Axum)                    │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │ App Manager │  │ WebRTC       │  │ Input Event Router     │  │
│  │             │  │ Signaling    │  │                        │  │
│  └─────────────┘  └──────────────┘  └────────────────────────┘  │
│         │                 │                      │               │
│         ∨                 ∨                      ∨               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │           Sandbox Orchestrator                          │    │
│  │  - Namespace creation                                   │    │
│  │  - Landlock policy application                          │    │
│  │  - Resource limit enforcement                           │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────┬───────────────────────────────────────┘
                           │
                           ∨
┌──────────────────────────────────────────────────────────────────┐
│              ISOLATED SANDBOX (native process)                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Mount namespace — only session paths are visible         │  │
│  │  ┌──────────────────────────────────────────────────────┐ │  │
│  │  │  Native App binary (e.g. file_explorer)             │ │  │
│  │  │  - egui UI → RGBA pixels → shared memory framebuffer│ │  │
│  │  │  - std::fs for file access (SQLite, etc.)           │ │  │
│  │  │  - reads input events from Unix socket              │ │  │
│  │  └──────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  GStreamer appsrc → VP8 encode → WebRTC track (DTLS-SRTP) │  │
│  │  Signaling: WebSocket (⚠ plain ws://, auth missing)       │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Filesystem view (mount namespace)                        │  │
│  │  Only owner-configured paths bind-mounted into namespace  │  │
│  │  Landlock applied as additional enforcement layer         │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Security Constraints:                                           │
│  • Network: DISABLED (network namespace — no interfaces)         │
│  • Syscalls: FILTERED (seccomp allowlist)                        │
│  • File Access: mount namespace (paths don't exist) + Landlock   │
│  • Resources: CPU 50%, Memory 512MB, PIDs 100 (cgroups)         │
└──────────────────────────────────────────────────────────────────┘
```

---

## Future Applications

The platform is extensible. Future applications could include:

1. **Document Editor** — sandboxed document viewer/editor (e.g. LibreOffice, or a native egui editor)
2. **Code Viewer** — syntax-highlighted code browser with optional editing
3. **Media Player** — video/audio playback (no client download possible)
4. **Spreadsheet Viewer** — read-only or editable spreadsheet
5. **Image Gallery** — image viewer with zoom/pan
6. **Database UI** — SQLite browser, runs a local SQLite file within the sandbox

---

## Session Management

### Session Lifecycle

```rust
pub struct ApplicationSession {
    pub session_id: SessionId,
    pub app_id: AppId,
    pub user_id: UserId,
    pub sandbox_id: SandboxId,
    pub webrtc_connection: WebRTCPeerConnection,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub enum SessionState {
    Initializing,
    Ready,
    Active,
    Idle,
    Terminating,
    Terminated,
}
```

### Session Creation Flow

1. Client requests app launch → `POST /api/sessions/launch`
2. Backend validates permissions and resolves the owner-configured share policy
3. Backend creates the sandbox: mount namespace, Landlock, seccomp, cgroups
4. Backend spawns the app binary inside the sandbox; passes framebuffer fd + control socket fd
5. Backend starts reading the shared framebuffer, feeding frames into GStreamer → VP8
6. Backend creates WebRTC peer connection, returns session ID + WebRTC offer
7. Client accepts offer, establishes WebRTC connection
8. Video stream flows to client; input events flow back over WebSocket → Unix socket → app

---

## Security Considerations

### Sandboxed Mode Threats & Mitigations

| Threat | Mitigation | Status |
|--------|------------|--------|
| **Data exfiltration via video** | Watermarking, session recording, time limits | Planned |
| **Sandbox escape** | Mount namespace (paths don't exist outside allowlist), Landlock, seccomp, cgroups | Partial |
| **Resource exhaustion** | cgroups limits, automatic termination on limit breach | Planned |
| **Input injection attacks** | Input validation, rate limiting, sanitization | Partial (30 fps throttle client-side) |
| **WebRTC media MITM** | DTLS-SRTP (default in `webrtc` crate — enabled) | ✅ Done |
| **Signaling interception / MITM** | Upgrade signaling to `wss://` (TLS); currently plain `ws://` | ⚠ Not done |
| **Unauthenticated `/ws` endpoint** | Validate JWT/session token before WebSocket upgrade | ⚠ Not done |
| **Session hijacking via UUID** | Require auth token alongside session ID | ⚠ Not done |
| **Hardcoded TURN credentials** | Move to env-only config; rotate credentials; use `turns://` | ⚠ Not done |
| **IP/topology leak via ICE** | Filter host candidates; use mDNS obfuscation | ⚠ Not done |
| **Input events over plaintext WS** | Move input to WebRTC encrypted data channel | ⚠ Not done |
| **Malicious app code** | App code signing, audited app registry | Planned |

---

## Implementation Phases

### Phase 1: WebRTC pipeline (done, needs hardening)
- [x] GStreamer VP8 encode + WebRTC video track (DTLS-SRTP)
- [x] WebSocket signaling (offer/answer/ICE exchange)
- [x] Input forwarding over WebSocket signaling channel
- [x] File explorer WASM prototype (egui, software rasterizer) — reference only, not production model
- [ ] WebRTC security hardening (WSS, endpoint auth, encrypted TURN, data channel for input)

### Phase 2: Native process sandbox
- [ ] Sandbox orchestrator: fork + mount namespace + Landlock + seccomp + cgroups applied before exec
- [ ] Shared memory framebuffer IPC (memfd, mapped by both backend and app)
- [ ] Unix socket control channel (input events backend → app; notifications app → backend)
- [ ] App manifest v2 format (`type: "native"`, permission paths block)
- [ ] App discovery and registry from `APPS_ROOT`
- [ ] Session management

### Phase 3: sandbox-app-sdk
- [ ] `crates/sandbox-app-sdk/` — `SandboxApp` trait + `run()` entry point
- [ ] Framebuffer and control socket setup abstracted by SDK
- [ ] Typed backend message types
- [ ] Port file-explorer from WASM prototype to native binary using SDK
- [ ] SDK documentation and example app

### Phase 4: Advanced Features
- [ ] Watermarking (sandboxed mode)
- [ ] Session recording
- [ ] Multiple video quality options
- [ ] Collaborative viewing
- [ ] Hot-reload of apps without backend restart

### Phase 5: Additional Applications
- [ ] Document viewer/editor
- [ ] Code viewer
- [ ] Media player
- [ ] Spreadsheet viewer

---

## Configuration

### Application Configuration

```yaml
applications:
  file_explorer:
    app_id: "file-explorer"
    name: "File Explorer"
    description: "Browse and preview files"
    binary: "file_explorer"
    permissions:
      - path: "."         # user's storage root, resolved per session
        access: ["read", "write", "delete"]
    resource_limits:
      cpu_percent: 50
      memory_mb: 512
      max_pids: 100
    video_config:
      width: 1920
      height: 1080
      framerate: 30
      codec: "vp8"
    features:
      watermarking: true
      session_recording: false
      max_session_duration_minutes: 120
```

---

**Last Updated**: 2026-02-17
