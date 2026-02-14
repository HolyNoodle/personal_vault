# Application Platform Architecture

## Overview

The Secure Sandbox Platform is a sandboxed application hosting system where all applications run server-side in extreme isolation and are streamed to users via WebRTC video. Different user roles (Owner vs Client) receive different file system permissions within the sandbox.

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
1. Application runs server-side in isolated sandbox (Landlock LSM, namespaces, network isolation)
2. Sandbox permissions configured based on user role (Owner = read/write, Client = read-only)
3. Application renders to virtual display (Xvfb)
4. FFmpeg captures display, encodes H.264/VP8 video
5. Video streamed to user browser via WebRTC (DTLS-SRTP encrypted)
6. User interacts with video feed - input events sent back to server
7. Input forwarded to sandboxed app (xdotool or similar)

---

## Role-Based Permissions

### Owner Users (Full Access)

**Sandbox Permissions**:
- ✅ **Read/Write access** - Full file system access within allowed paths
- ✅ **Create/Delete files** - Can manage file system
- ✅ **Network access** - Can make API calls (if enabled per-app)
- ✅ **Resource quota** - Higher CPU/memory limits

**Capabilities**:
- Full file management through sandboxed UI
- Upload new files (streamed through sandbox)
- Rename, delete, organize files
- Preview and edit files
- All actions visible in video stream

**Use Cases**:
- Data owner organizing their files
- Managing documents in file explorer
- Editing files through applications
- Full file system management

---

### Client Users (Read-Only)

**Sandbox Permissions**:
- ✅ **Read-only access** - Can view files but not modify
- ❌ **No file creation** - Cannot create/delete files
- ❌ **No network access** - Sandbox completely isolated
- ❌ **Lower resource quota** - Restricted CPU/memory
- ✅ **Watermarking** - Optional visual watermarks on video stream

**Capabilities**:
- View files and documents
- Navigate file system (read-only)
- Preview PDFs, images, videos
- Interact with UI (read-only operations)
- All actions visible to owner via audit log

**Security Guarantees**:
- ❌ **No downloads** - User only sees video pixels
- ❌ **No clipboard access** - Copy/paste disabled
- ❌ **No local file access** - Files stay server-side
- ❌ **No network access** - Sandbox has no internet
- ✅ **All actions logged** - Complete audit trail
- ✅ **Landlock enforcement** - Kernel-level permission control
- ✅ **Resource limits** - cgroups prevent abuse

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
- File preview:
  - **PDF**: Rendered inline (PDF.js in browser mode, poppler in sandbox mode)
  - **Images**: JPEG, PNG, GIF, WebP display
  - **Videos**: MP4, WebM playback with controls
- Search and filtering
- Sorting (name, date, size, type)

**Sandboxed Mode Features**:
- View-only interface
- Preview files without downloading
- Navigate directory structure
- Session time limits
- Watermarking (optional, configurable by owner)

**Browser Mode Features**:
- All sandboxed features PLUS:
- Download individual files or folders (ZIP)
- Upload new files (drag & drop)
- Create/rename/delete files and folders
- Move files between directories
- Bulk operations (multi-select)
- Sharing management UI

### Technical Implementation

#### Sandboxed Mode Stack

**Server-Side Application** (Rust):
```rust
// File explorer running in sandbox
// Uses: egui (immediate mode GUI) or web browser (headless Chrome/Firefox)

Components:
- Directory walker (read-only, Landlock-restricted paths)
- PDF renderer (poppler-rs)
- Image decoder (image crate)
- Video player (GStreamer or FFmpeg)
- GUI framework (egui on Xvfb OR headless browser rendering HTML)
```

**Sandbox Isolation**:
- Landlock policy: Read-only access to specific user directory
- Network namespace: No network interfaces (fully offline)
- Mount namespace: Minimal filesystem, read-only mounts
- PID namespace: Isolated process tree
- cgroups: CPU/memory/PID limits
- seccomp: Syscall filtering (block dangerous operations)

**Video Streaming Pipeline**:
```
Xvfb :100 (1920x1080) → FFmpeg capture → H.264/VP8 encode → WebRTC track → Browser
```

#### Browser Mode Stack

**Frontend Application** (TypeScript/React):
```typescript
// File explorer running directly in browser

Components:
- React components for file tree, list, preview
- PDF.js for PDF rendering
- Native <img> for images
- Native <video> for videos  
- File System Access API for downloads
- Backend API client for file operations
```

**Backend API** (Rust/Axum):
```rust
// RESTful API for file operations (owner users only)

Endpoints:
GET  /api/files?path=/folder        // List directory
GET  /api/files/download?path=/file // Download file
POST /api/files/upload               // Upload file
PUT  /api/files/move                 // Move/rename
DELETE /api/files                    // Delete file/folder
POST /api/files/share                // Create share link
```

---

## Application Framework

### Application Interface

All applications implement a common interface:

```rust
pub trait SandboxApplication {
    /// Unique application identifier
    fn app_id(&self) -> AppId;
    
    /// Application name and metadata
    fn metadata(&self) -> AppMetadata;
    
    /// Initialize application in sandbox
    fn initialize_sandbox(&self, ctx: SandboxContext) -> Result<SandboxInstance>;
    
    /// Initialize application in browser (returns WASM binary or JS bundle)
    fn initialize_browser(&self) -> Result<BrowserBundle>;
    
    /// Handle input event from WebRTC client
    fn handle_input(&mut self, event: InputEvent) -> Result<()>;
    
    /// Get required permissions for this app
    fn required_permissions(&self) -> Vec<Permission>;
    
    /// Landlock policy for sandboxed execution
    fn landlock_policy(&self) -> LandlockPolicy;
}
```

### Application Registry

```rust
pub struct ApplicationRegistry {
    apps: HashMap<AppId, Box<dyn SandboxApplication>>,
}

impl ApplicationRegistry {
    pub fn register(&mut self, app: Box<dyn SandboxApplication>) {
        self.apps.insert(app.app_id(), app);
    }
    
    pub fn launch_sandboxed(&self, app_id: AppId, user_ctx: UserContext) -> Result<SessionId>;
    pub fn launch_browser(&self, app_id: AppId, user_ctx: UserContext) -> Result<BrowserBundle>;
}
```

---

## Architecture Diagrams

### Sandboxed Mode Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                     CLIENT USER BROWSER                          │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────────────┐  │
│  │ App UI         │  │ WebRTC Player  │  │ Input Forwarder   │  │
│  │ (React)        │  │ (Video Stream) │  │ (Mouse/Keyboard)  │  │
│  └────────────────┘  └────────────────┘  └───────────────────┘  │
│         ↑                    ↑                      │            │
│         │ App State          │ H.264/VP8            │ Events     │
│         │ via API            │ over WebRTC          │ over WS    │
└─────────┼────────────────────┼──────────────────────┼────────────┘
          │                    │                      │
          ∨                    ∨                      ∨
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
└──────────────────────────────────┬───────────────────────────────┘
                                   │
                                   ∨
┌──────────────────────────────────────────────────────────────────┐
│                    ISOLATED SANDBOX CONTAINER                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Xvfb :100 (Virtual Display)                              │  │
│  │  ┌──────────────────────────────────────────────────────┐ │  │
│  │  │  File Explorer Application (Rust/egui or Browser)   │ │  │
│  │  │  - Directory tree                                    │ │  │
│  │  │  - File preview (PDF/Image/Video)                    │ │  │
│  │  │  - Read-only operations                              │ │  │
│  │  └──────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  FFmpeg (Screen Capture & Encode)                         │  │
│  │  xvfb :100 → H.264/VP8 → WebRTC track                     │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Input Handler (xdotool or similar)                       │  │
│  │  Receives events → Injects into Xvfb display              │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Mounted File System (Read-Only)                          │  │
│  │  /mnt/user_files → Landlock-restricted to specific paths  │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Security Constraints:                                           │
│  • Network: DISABLED (no network namespace interfaces)           │
│  • Syscalls: FILTERED (seccomp whitelist)                        │
│  • File Access: READ-ONLY via Landlock LSM                       │
│  • Resources: CPU 50%, Memory 512MB, PIDs 100                    │
└──────────────────────────────────────────────────────────────────┘
```

### Browser Mode Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                     OWNER USER BROWSER                           │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  File Explorer App (React/TypeScript)                     │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐ │  │
│  │  │ Directory    │  │ File Preview │  │ Download        │ │  │
│  │  │ Tree         │  │ (PDF.js)     │  │ Manager         │ │  │
│  │  └──────────────┘  └──────────────┘  └─────────────────┘ │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐ │  │
│  │  │ Upload UI    │  │ Share Mgmt   │  │ File Operations │ │  │
│  │  │ (Drag/Drop)  │  │              │  │ (Move/Delete)   │ │  │
│  │  └──────────────┘  └──────────────┘  └─────────────────┘ │  │
│  └────────────────────────────────────────────────────────────┘  │
│         │                                                         │
│         │ REST API (HTTPS)                                        │
│         │ - GET /api/files?path=/dir                              │
│         │ - GET /api/files/download?path=/file                    │
│         │ - POST /api/files/upload                                │
│         │ - PUT /api/files/move                                   │
└─────────┼─────────────────────────────────────────────────────────┘
          │
          ∨
┌──────────────────────────────────────────────────────────────────┐
│                    BACKEND API SERVER (Rust/Axum)                │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  File Operations API (Owner Users Only)                   │  │
│  │  - Authentication & Authorization checks                  │  │
│  │  - Permission validation                                  │  │
│  │  - Audit logging                                          │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  File System Abstraction Layer                            │  │
│  │  - Encrypted storage access                               │  │
│  │  - Quota enforcement                                       │  │
│  │  - Virus scanning (optional)                              │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────┬───────────────────────────────┘
                                   │
                                   ∨
┌──────────────────────────────────────────────────────────────────┐
│                      ENCRYPTED FILE STORAGE                      │
│  /storage/users/{user_id}/                                       │
│  - Documents/                                                    │
│  - Images/                                                       │
│  - Videos/                                                       │
│  (AES-256-GCM encrypted at rest)                                 │
└──────────────────────────────────────────────────────────────────┘
```

---

## Future Applications

The platform is extensible. Future applications could include:

1. **Document Editor**
   - Sandboxed: View-only document viewer (LibreOffice in sandbox)
   - Browser: Full collaborative editor (WASM-based)

2. **Code Viewer**
   - Sandboxed: Syntax-highlighted code browser
   - Browser: Full IDE (Monaco editor)

3. **Media Player**
   - Sandboxed: Video player with streaming (no download)
   - Browser: Full media player with download

4. **Spreadsheet Viewer**
   - Sandboxed: Read-only spreadsheet (LibreOffice Calc)
   - Browser: Full spreadsheet editor

5. **Image Gallery**
   - Sandboxed: Image viewer with zoom/pan
   - Browser: Image organizer with editing

---

## Session Management

### Session Lifecycle

```rust
pub struct ApplicationSession {
    pub session_id: SessionId,
    pub app_id: AppId,
    pub user_id: UserId,
    pub execution_mode: ExecutionMode,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub enum ExecutionMode {
    Sandboxed {
        sandbox_id: SandboxId,
        webrtc_connection: WebRTCPeerConnection,
        video_config: VideoConfig,
    },
    Browser {
        jwt_token: String,
        api_endpoint: Url,
    },
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

**Sandboxed Mode**:
1. Client requests app launch → `POST /api/sessions/launch`
2. Backend validates permissions
3. Backend creates sandbox (namespaces, Landlock, cgroups)
4. Backend starts Xvfb + application
5. Backend starts FFmpeg video capture
6. Backend creates WebRTC peer connection
7. Backend returns session ID + WebRTC offer
8. Client accepts offer, establishes WebRTC connection
9. Video stream flows to client
10. Input events flow from client to sandbox

**Browser Mode**:
1. Client requests app launch → `POST /api/sessions/launch`
2. Backend validates permissions
3. Backend generates JWT token with file access scope
4. Backend returns app bundle URL + JWT token
5. Client loads app bundle (JS/WASM)
6. App authenticates with JWT
7. App makes API calls for file operations

---

## Security Considerations

### Sandboxed Mode Threats & Mitigations

| Threat | Mitigation |
|--------|------------|
| **Data exfiltration via video** | Watermarking, session recording, time limits |
| **Sandbox escape** | Multiple isolation layers (Landlock, namespaces, seccomp, cgroups) |
| **Resource exhaustion** | cgroups limits, automatic termination on limit breach |
| **Input injection attacks** | Input validation, rate limiting, sanitization |
| **WebRTC MITM** | DTLS-SRTP encryption, certificate pinning |
| **Malicious app code** | App code signing, audited app registry |

### Browser Mode Threats & Mitigations

| Threat | Mitigation |
|--------|------------|
| **Unauthorized file access** | JWT token with scoped permissions, server-side validation |
| **Token theft** | Short expiry (15 min), refresh token rotation, HTTPS only |
| **XSS in app** | Content Security Policy, sanitized rendering |
| **CSRF** | CSRF tokens, SameSite cookies |
| **Malicious uploads** | File type validation, size limits, virus scanning |

---

## Implementation Phases

### Phase 1: Sandboxed File Explorer (MVP)
- [x] Video streaming POC completed
- [ ] File explorer application (Rust + egui)
- [ ] PDF preview (poppler-rs)
- [ ] Image preview (image crate)
- [ ] Video preview (GStreamer)
- [ ] Input forwarding (xdotool)
- [ ] Session management

### Phase 2: Browser File Explorer
- [ ] React file explorer UI
- [ ] Backend API for file operations
- [ ] File System Access API integration
- [ ] Upload/download functionality
- [ ] Share management UI

### Phase 3: Advanced Features
- [ ] Watermarking (sandboxed mode)
- [ ] Session recording
- [ ] Multiple video quality options
- [ ] Collaborative viewing
- [ ] Application plugin system

### Phase 4: Additional Applications
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
    app_id: "file-explorer-v1"
    name: "File Explorer"
    description: "Browse and preview files"
    
    sandboxed_mode:
      enabled: true
      binary: "/apps/file_explorer_sandbox"
      landlock_policy:
        - path: "/mnt/user_files"
          access: "read_only"
      resource_limits:
        cpu_percent: 50
        memory_mb: 512
        max_pids: 100
      video_config:
        width: 1920
        height: 1080
        framerate: 30
        codec: "h264"
      features:
        watermarking: true
        session_recording: false
        max_session_duration_minutes: 120
    
    browser_mode:
      enabled: true
      bundle_path: "/apps/file_explorer_browser/bundle.js"
      api_scopes:
        - "files:read"
        - "files:download"
      features:
        upload: true
        download: true
        share_management: false
```

---

**Last Updated**: 2026-02-14
