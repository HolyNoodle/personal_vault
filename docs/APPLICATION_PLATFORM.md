# Application Platform Architecture

## Overview

The Secure Sandbox Platform is a sandboxed application hosting system where all applications run server-side in extreme isolation and are delivered to users via WebRTC video. Different user roles (Owner vs Client) receive different file system permissions within the sandbox.

**Key Insight**: Video streaming is the sandboxing and data control mechanism. All users interact with a video feed (input forwarding), ensuring zero data exfiltration. Permissions are enforced server-side via Landlock LSM.

---

## Sandboxed Execution Model

### Architecture Overview

**All Users**: Both Owner and Client users access applications the same way — through video streaming

**How It Works**:
```
Browser → WebRTC (VP8) ← GStreamer ximagesrc :N ← Xvfb :N ← App (any X11 UI)
   ↓ input events (WS) → X11 XTEST (x11rb) :N
```

**Execution Flow**:
1. Backend creates sandbox (mount namespace, Landlock, cgroups, seccomp)
2. Backend starts `Xvfb :N` outside the sandbox; bind-mounts `/tmp/.X11-unix/XN` into the sandbox namespace
3. Backend starts GStreamer pipeline: `ximagesrc display=:N ! vp8enc ! webrtcbin`
4. Backend spawns app inside sandbox with `DISPLAY=:N`
5. App connects to Xvfb and renders with its chosen X11 UI framework
6. Video stream flows to browser via WebRTC; input events from browser → X11 XTEST (x11rb) → Xvfb

---

> **Status (WIP)**:
> - GStreamer VP8 + WebRTC video track: **implemented**
> - WebSocket signaling + input routing: **implemented**
> - Xvfb-per-session + ximagesrc capture: **implemented**
> - X11 XTEST input injection (x11rb): **implemented**
> - Native process sandbox (mount namespace + Landlock): **planned**
> - File-explorer native X11 binary (eframe/egui): **implemented — current production model**
> - `sandbox-app-sdk`: **planned** (business logic only, no rendering)
> - WebRTC security hardening (WSS, auth on `/ws`, encrypted TURN, input via data channel): **not yet implemented — see Security Considerations**

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

**Native binary** (`apps/file-explorer/`):

The file-explorer is a standard native Linux binary built with `eframe` (egui + winit, X11 feature). It reads `DISPLAY` from the environment variable set by the backend, connects to `Xvfb :N`, and renders using the normal X11 path. This is the current production model.

- Any X11-capable UI framework — GTK4, Iced, Qt, egui+winit, etc.
- No platform-specific rendering code; no framebuffer accessors; no exported C-ABI frame functions

---

## Native Process Execution Model

Apps on this platform are native Linux executables. Isolation is provided by the OS — not by a language runtime — making it equivalent in security model to containers, and as secure as the OS kernel and sandbox configuration.

### Sandbox setup (applied before exec)

The backend prepares the sandbox before spawning the app process:

1. **Mount namespace** — a new namespace is created; only the paths the session needs are bind-mounted into it. The app's process sees only those paths; the rest of the host filesystem does not exist from its perspective (`ENOENT`, not `EACCES`).
2. **X11 socket** — `/tmp/.X11-unix/XN` (the Xvfb socket) is bind-mounted into the namespace so the app can connect to its virtual display.
3. **Landlock LSM** — a ruleset is applied that restricts which of the mounted paths can be opened and with which operations (read, write, delete). This is belt-and-suspenders on top of the mount namespace.
4. **Network namespace** — no network interfaces; the app is fully offline.
5. **seccomp** — a syscall allowlist is applied; dangerous syscalls (e.g. `ptrace`, `mount`, `pivot_root`) are blocked.
6. **cgroups** — CPU, memory, and PID limits are applied to the app's cgroup.

The mount namespace gives the strongest filesystem isolation: paths outside the session's allowlist don't exist, so the app cannot enumerate, stat, or discover them. Landlock prevents access even in edge cases where a path might be visible.

`Xvfb :N` runs **outside** the sandbox namespace (managed by the backend), so the virtual display server is not part of the app's attack surface.

### Frame delivery

1. Backend starts `Xvfb :N` and a GStreamer pipeline: `ximagesrc display=:N fps=30 ! vp8enc ! webrtcbin`
2. Backend spawns the app inside the sandbox with `DISPLAY=:N`
3. App renders to the virtual display using its chosen X11 UI framework — no platform-specific rendering code required
4. GStreamer captures the display, encodes to VP8, and sends it as a WebRTC video track

No shared memory, no custom frame IPC, no framebuffer accessors.

### Input forwarding

The backend receives keyboard and mouse events from the browser over WebSocket and injects them into the Xvfb display using the X11 XTEST extension via the `x11rb` crate (`xtest_fake_input`). Mouse moves, button presses, and key events are all injected as synthetic X11 events directly over the existing x11rb connection to the display.

The app receives normal X11 input events — no special input handling code required.

### Capabilities available to the app

Because the app is a native binary inside a well-configured sandbox, it can use:
- `std::fs` directly — no host function protocol needed; Landlock enforces access
- SQLite (or any file-based database) — full native support, operates on files within the sandbox
- Any native library — no compilation-to-WASM required
- Threads, async runtimes, memory-mapped files — all standard Linux capabilities

The security boundary is the sandbox configuration, not the language runtime.

---

## Communication Contract

The interface between the platform backend and a running app instance uses two channels. There is no shared memory framebuffer and no control socket for input.

### Display (Xvfb X11 protocol)

The app connects to `Xvfb :N` via the `DISPLAY=:N` environment variable. Frame capture is handled entirely by GStreamer (`ximagesrc`) on the backend side. The app writes no special rendering code — it is a standard X11 application.

### Input (X11 XTEST → Xvfb)

Browser input events are received by the backend over WebSocket and injected into the Xvfb display via the X11 XTEST extension (`xtest_fake_input` from the `x11rb` crate). The app receives normal X11 input events with no special input handling required.

### Optional capability socket (app → backend)

If an app declares capabilities that require backend notifications (e.g. upload complete, download ready), it may communicate with the backend via a thin Unix socket or stdout. This is optional and declared in `manifest.json` `capabilities`. It is **not** a frame channel — only lightweight event messages.

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

No `exports` map. No framebuffer accessors. No render function signatures. The `permissions` block is what the backend uses to configure the Landlock ruleset and bind mounts for the session. `"path": "."` means the user's storage root; the backend resolves it to the actual session path before applying the policy.

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
│   └── file_explorer        # Required — native Linux x86_64 executable
├── my-custom-app/
│   ├── manifest.json
│   ├── my_custom_app        # Native Linux executable
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

2. **`Cargo.toml`**: standard `[[bin]]`; depend on any X11-capable UI framework:
   ```toml
   [[bin]]
   name = "my_app"

   [dependencies]
   # Pick any X11-capable UI framework, e.g.:
   # iced = { version = "0.12", features = ["winit"] }
   # gtk4 = "0.9"
   # egui + winit + glutin — standard desktop setup
   # rusqlite = "0.31"  — SQLite, or any other native library
   ```

3. **`manifest.json`**: declare `name`, `binary`, `permissions`, and `capabilities`. No `exports` map.
   ```json
   {
     "name": "My App",
     "version": "0.1.0",
     "type": "native",
     "binary": "my_app",
     "permissions": [
       { "path": ".", "access": ["read"] }
     ],
     "capabilities": ["preview"]
   }
   ```

4. **`src/main.rs`**: standard X11 application entry point. The backend sets `DISPLAY=:N` before exec — the app reads it from the environment automatically via the X11 client library. No special IPC code needed.

5. **Filesystem access**: use `std::fs` directly. Landlock and the mount namespace enforced by the backend guarantee the app can only access what `manifest.json` declared.

6. **Build**: `cargo build --release` (native target — no cross-compilation needed)

7. **Install**: copy binary + `manifest.json` into `$APPS_ROOT/<app-name>/`, then restart the backend.

---

## sandbox-app-sdk crate (planned)

The `sandbox-app-sdk` is a pure business logic library. It contains no rendering code — the rendering framework is the developer's choice.

### What the SDK provides

**1. Manifest types** — strongly-typed structs for parsing and validating `manifest.json`:

```rust
use sandbox_app_sdk::{AppManifest, Permission, Capability};
```

**2. Capability notification helpers** — optional helpers for sending structured events to the backend over stdout or a thin Unix socket:

```rust
sandbox_app_sdk::notify::upload_complete(path);
sandbox_app_sdk::notify::download_ready(path, filename);
```

### What the SDK does NOT provide

- Rendering, framebuffer management, or egui integration — apps use their chosen X11 framework directly
- Filesystem access — apps use `std::fs` directly; Landlock enforces the policy
- SQLite or database access — apps link whatever they need natively
- Input handling — apps receive normal X11 events from Xvfb via their UI framework

### Planned location: `crates/sandbox-app-sdk/`

**Status: Planned — not yet implemented.**

---

## Architecture Diagrams

> **Note**: The WebRTC signaling channel currently uses plain `ws://` and lacks authentication — production hardening is required (see Security Considerations). The file-explorer is a native X11 binary running under Xvfb — the current production model.

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
│  │             │  │ Signaling    │  │ (X11 XTEST via x11rb)  │  │
│  └─────────────┘  └──────────────┘  └────────────────────────┘  │
│         │                 │                      │               │
│         ∨                 ∨                      ∨               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │           Sandbox Orchestrator                          │    │
│  │  - Namespace creation                                   │    │
│  │  - Landlock policy application                          │    │
│  │  - Resource limit enforcement                           │    │
│  │  - Xvfb :N lifecycle (start/stop/display pool)         │    │
│  └─────────────────────────────────────────────────────────┘    │
│         │                 │                                      │
│         ∨                 ∨                                      │
│  ┌──────────────┐  ┌──────────────────────────────────────────┐ │
│  │  Xvfb :N     │  │  GStreamer pipeline                      │ │
│  │  (virtual    │  │  ximagesrc display=:N fps=30             │ │
│  │   display)   │  │  → vp8enc → webrtcbin (DTLS-SRTP)       │ │
│  └──────┬───────┘  └──────────────────────────────────────────┘ │
│         │ X11 socket bind-mounted into sandbox                   │
└─────────┼────────────────────────────────────────────────────────┘
          │
          ∨
┌──────────────────────────────────────────────────────────────────┐
│              ISOLATED SANDBOX (native process)                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Mount namespace — only session paths + X11 socket visible │  │
│  │  ┌──────────────────────────────────────────────────────┐ │  │
│  │  │  Native App binary (any X11 UI framework)           │ │  │
│  │  │  - DISPLAY=:N set by backend before exec            │ │  │
│  │  │  - Connects to Xvfb, renders with chosen framework  │ │  │
│  │  │  - std::fs for file access (SQLite, etc.)           │ │  │
│  │  │  - Receives normal X11 input events from Xvfb       │ │  │
│  │  └──────────────────────────────────────────────────────┘ │  │
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
│  • Xvfb runs outside sandbox — not in app's attack surface       │
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
4. Backend starts `Xvfb :N` (outside sandbox) and bind-mounts `/tmp/.X11-unix/XN` into the namespace
5. Backend starts GStreamer pipeline: `ximagesrc display=:N ! vp8enc ! webrtcbin`
6. Backend spawns the app binary inside the sandbox with `DISPLAY=:N`
7. Backend creates WebRTC peer connection, returns session ID + WebRTC offer
8. Client accepts offer, establishes WebRTC connection
9. Video stream flows to client; input events flow back over WebSocket → X11 XTEST (x11rb) → Xvfb → app

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
| **Xvfb display hijacking** | Xvfb runs outside sandbox; app sees only its own X11 socket via bind-mount | Planned |
| **Cross-session display access** | One Xvfb instance per session; display number recycled only after session end | Planned |

---

## Implementation Phases

### Phase 1: WebRTC pipeline (done, needs hardening)
- [x] GStreamer VP8 encode + WebRTC video track (DTLS-SRTP)
- [x] WebSocket signaling (offer/answer/ICE exchange)
- [x] Input forwarding over WebSocket signaling channel
- [x] File explorer native X11 binary (eframe/egui + winit, X11 feature) — current production model
- [ ] WebRTC security hardening (WSS, endpoint auth, encrypted TURN, data channel for input)

### Phase 2: Xvfb + ximagesrc integration
- [x] Xvfb-per-session lifecycle management (start/stop/display number pool)
- [x] GStreamer pipeline: `ximagesrc display=:N fps=30 ! vp8enc ! webrtcbin` (replaces appsrc)
- [x] X11 XTEST input injection via x11rb (keyboard + mouse → Xvfb display)
- [x] X11 socket bind-mount into sandbox namespace

### Phase 3: Native process sandbox
- [ ] Sandbox orchestrator: fork + mount namespace + Landlock + seccomp + cgroups applied before exec
- [ ] App discovery and registry from `APPS_ROOT`
- [ ] Session management

### Phase 4: sandbox-app-sdk
- [ ] `crates/sandbox-app-sdk/` — manifest types, capability notification helpers
- [x] File-explorer native X11 binary (implemented; eframe/egui + X11 feature)
- [ ] SDK documentation and example app

### Phase 5: Advanced features + additional applications
- [ ] Watermarking (sandboxed mode)
- [ ] Session recording
- [ ] Multiple video quality options
- [ ] Collaborative viewing
- [ ] Hot-reload of apps without backend restart
- [ ] Document viewer/editor, code viewer, media player, spreadsheet viewer

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

**Last Updated**: 2026-02-18
