# Application Behavior & User Flows

## Overview

This document describes how the Secure Sandbox Server application behaves from the user's perspective, including all workflows, UI views, and real-time permission enforcement.

---

## User Setup & File Management

### Super Admin: User Provisioning

**Workflow:**

```
1. Super Admin logs in with WebAuthn hardware key
2. Navigates to "User Management" dashboard
3. Creates new user account:
   - Email address
   - Role: Owner or Client (not Super Admin)
   - Storage quota (e.g., 10GB, 100GB, 1TB)
   - Local root folder path (server filesystem)
4. System creates:
   - User account in database
   - Dedicated folder: /data/users/{user_id}/
   - Encryption key for user's data
   - Initial permissions structure
5. System sends invitation email to user
6. User registers via WebAuthn passkey
```

**Local Root Folder Security:**

Each user gets an isolated directory on the server:

```
/data/users/
├── usr_550e8400.../              # User 1's root folder
│   ├── Documents/
│   ├── Images/
│   └── .metadata                 # Encryption metadata
├── usr_660f9511.../              # User 2's root folder
│   ├── Financial_Reports/
│   └── .metadata
```

**Isolation Enforced By:**
- Landlock LSM (kernel-level filesystem access control)
- User namespace (UID mapping per user)
- Database permissions (user_id foreign key constraints)
- Application-level authorization checks

---

## User (Owner) Interface

### View 1: File Explorer

**Purpose:** Manage files and folders that can be shared with clients.

**Features:**

```
┌────────────────────────────────────────────────────┐
│  📁 My Files                           [Upload] [+]│
├────────────────────────────────────────────────────┤
│                                                    │
│  🔙 /Documents/Legal/                             │
│                                                    │
│  ┌──────────────────────────────────────────────┐ │
│  │ 📁 Contracts/                    Modified     │ │
│  │ 📁 NDAs/                         2 days ago   │ │
│  │ 📄 Partnership_Agreement.pdf     5 MB         │ │
│  │ 📄 Client_Contract_v2.docx       1.2 MB       │ │
│  └──────────────────────────────────────────────┘ │
│                                                    │
│  Right-click menu:                                 │
│  • Preview                                         │
│  • Download                                        │
│  • Rename                                          │
│  • Move to...                                      │
│  • Delete                                          │
│  • Share with client...                            │
│  • Properties                                      │
│                                                    │
└────────────────────────────────────────────────────┘
```

**Operations:**

**1. Upload Files**
```
User clicks [Upload] → File picker → Select files
→ Client-side encryption (optional) → Upload via HTTPS
→ Server stores in /data/users/{user_id}/ → Encrypt at rest
→ File appears in explorer immediately
```

**2. Download Files**
```
User clicks file → [Download] → Decrypt if encrypted
→ Download via HTTPS (TLS 1.3) → Client receives file
```

**3. Preview Files**
```
User clicks file → [Preview]
→ Server renders file (PDF, images, text)
→ Preview shown in modal (no download)
→ For videos: stream via encrypted connection
```

**4. Create Folder**
```
User clicks [+] → "New Folder" → Enter name
→ mkdir in /data/users/{user_id}/{path}
→ Folder appears in explorer
```

**5. Move/Rename**
```
User drags file → Drop in folder → Server validates permissions
→ mv /data/users/{user_id}/old /data/users/{user_id}/new
→ Update database file_metadata table
→ Explorer updates in real-time
```

**6. Delete**
```
User selects file → Delete → Confirmation modal
→ "Are you sure? This will revoke all client access."
→ Mark as deleted in database
→ Move to .trash/ folder (30-day retention)
→ Notify active clients (file removed from their view)
```

**7. Share with Client**
```
User right-clicks file → "Share with client..."
→ Modal opens:
  - Select client user (or create invitation)
  - Set permissions: View only (default)
  - Set expiration: 1 hour, 1 day, 1 week, never
  - Click [Grant Access]
→ Permission created in database
→ Client instantly sees file in their explorer (if active)
→ Audit log entry created
```

**Technical Implementation:**

```rust
// File Explorer API
GET  /api/files?path=/Documents/Legal     // List directory
POST /api/files/upload                     // Upload file
GET  /api/files/download/{file_id}         // Download file
GET  /api/files/preview/{file_id}          // Preview file
POST /api/files/folder                     // Create folder
PUT  /api/files/{file_id}/move             // Move file
PUT  /api/files/{file_id}/rename           // Rename file
DELETE /api/files/{file_id}                // Delete file (soft delete)

// Real-time updates via WebSocket
WS /ws/files
→ Sends: { event: "file_created", path: "/Documents/report.pdf" }
→ Sends: { event: "file_deleted", path: "/Documents/old.pdf" }
→ Sends: { event: "file_moved", from: "/a/file", to: "/b/file" }
```

---

### View 2: Client Users Dashboard

**Purpose:** Monitor who has access, view activity, manage permissions in real-time.

**Interface:**

```
┌────────────────────────────────────────────────────────────┐
│  👥 Client Users                           [Invite Client]  │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Active Sessions (2)                                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🟢 Alice Johnson (alice@example.com)                 │  │
│  │    Session: ses_abc123                               │  │
│  │    Started: 15 minutes ago                           │  │
│  │    Viewing: /Documents/Contract.pdf                  │  │
│  │    IP: 192.168.1.100                                 │  │
│  │    [View Details] [Terminate Session]                │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🟢 Bob Smith (bob@company.com)                       │  │
│  │    Session: ses_def456                               │  │
│  │    Started: 2 hours ago                              │  │
│  │    Viewing: /Financial/Report_Q4.xlsx                │  │
│  │    IP: 203.0.113.50                                  │  │
│  │    [View Details] [Terminate Session]                │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Pending Access Requests (1)                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ⏳ Charlie Davis (charlie@partner.com)               │  │
│  │    Requested: 5 minutes ago                          │  │
│  │    Purpose: "Review partnership terms"               │  │
│  │    [Approve] [Deny]                                   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  All Clients (5)                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Alice Johnson        Last active: 15 min ago    [⚙️] │  │
│  │ Bob Smith            Last active: 2 hours ago   [⚙️] │  │
│  │ Charlie Davis        Never accessed             [⚙️] │  │
│  │ Dana White           Last active: 3 days ago    [⚙️] │  │
│  │ Eve Martinez         Access revoked             [⚙️] │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**Client Details Modal:**

```
┌────────────────────────────────────────────────────────────┐
│  👤 Client Details: Alice Johnson                      [×] │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Email: alice@example.com                                  │
│  Status: 🟢 Active Session                                 │
│  Session Started: 15 minutes ago                           │
│  Current File: /Documents/Contract.pdf                     │
│  IP Address: 192.168.1.100                                 │
│  Location: San Francisco, CA (approximate)                 │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  📁 Accessible Files & Folders                  [Add File] │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ✅ /Documents/Contract.pdf             [Remove]      │  │
│  │ ✅ /Documents/NDA.pdf                  [Remove]      │  │
│  │ ✅ /Financial/Report_Q4.xlsx           [Remove]      │  │
│  │ ✅ /Images/ (entire folder)            [Remove]      │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  ⚙️ Settings (Real-Time)                                   │
│                                                             │
│  Session Timeout: [30 minutes ▼]                           │
│  Watermark: [Enabled ✓]                                    │
│  Watermark Text: "Confidential - Alice Johnson"            │
│  Allow Copy/Paste: [Disabled ☐]                            │
│  Max Session Duration: [2 hours ▼]                         │
│  Auto-terminate after: [Never ▼]                           │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  📊 Activity Log (Last 7 Days)                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 2026-02-14 10:30  Session started                    │  │
│  │ 2026-02-14 10:31  Viewed /Documents/Contract.pdf     │  │
│  │ 2026-02-14 10:45  Viewed /Documents/NDA.pdf          │  │
│  │ 2026-02-13 14:20  Session ended (timeout)            │  │
│  │ 2026-02-13 14:00  Session started                    │  │
│  │ 2026-02-13 14:10  Viewed /Financial/Report_Q4.xlsx   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  [Export Activity Log] [Revoke All Access] [Close]         │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**Real-Time Permission Changes:**

**Scenario 1: Remove File Access**

**Security Enforcement (Server-Side):**
```
Owner clicks [Remove] next to /Documents/Contract.pdf
    ↓
Database: UPDATE permissions SET revoked=true WHERE file_id=...
    ↓
Domain Event: PermissionRevoked
    ↓
Sandbox Manager: Find session ses_abc123
    ↓
Landlock: Rebuild ruleset WITHOUT Contract.pdf
    ↓
Sandbox: Kill process, start new one with updated Landlock rules
    ↓
Kernel: Any attempt to open Contract.pdf returns EPERM
    ↓
✅ SECURITY ENFORCED (Client cannot bypass this)
```

**UI Synchronization (Client-Side, Optional):**
```
(In parallel with security enforcement)
    ↓
WebSocket: Send { event: "permission_revoked", file_path: "..." }
    ↓
Client Browser: Receives message
    ↓
Client UI: Remove file from file explorer
    ↓
Client UI: If viewing file, show "Access Revoked" modal
    ↓
✅ GRACEFUL UX (But security already enforced server-side)
```

**Audit Trail (Always):**
```
(In parallel)
    ↓
Audit Logger: INSERT INTO audit_events
    ↓
✅ COMPLIANCE (Immutable log)
```

**If WebSocket Fails:**
- ✅ Security still enforced (Landlock in kernel)
- ✅ Audit still logged (database write)
- ❌ Client sees stale UI until they try to access file (gets error)
- ❌ Owner doesn't see live activity updates

**Scenario 2: Add File Access**
```
User clicks [Add File] → File picker → Selects /Documents/NewReport.pdf
→ Database: INSERT INTO permissions (user_id, file_id, granted_at) VALUES (...)
→ WebSocket message sent to active client session:
  {
    event: "permission_granted",
    file_path: "/Documents/NewReport.pdf"
  }
→ Client's sandbox: Landlock policy updated to include new file
→ Client's file explorer: New file appears immediately
→ Audit log: "User granted file access for alice@example.com"
```

**Scenario 3: Change Watermark Setting**
```
User toggles watermark → "Enable watermark? This will restart the client's session."
→ Database: UPDATE sessions SET watermark_enabled=true WHERE session_id=...
→ WebSocket message: { event: "settings_changed", setting: "watermark", value: true }
→ Client's video encoder: Restart with watermark overlay
→ Client sees: Brief reconnection, then video resumes with watermark
→ Audit log: "Watermark enabled for session ses_abc123"
```

**Scenario 4: Terminate Session**
```
User clicks [Terminate Session]
→ Confirmation: "End Alice's session immediately?"
→ Database: UPDATE sessions SET state='terminated' WHERE session_id=...
→ WebSocket message: { event: "session_terminated", reason: "Owner ended session" }
→ Client's sandbox: Graceful shutdown (save state if any)
→ Client's browser: Redirect to "Session ended by owner" page
→ Audit log: "Session terminated by owner"
```

**Technical Implementation:**

```rust
// Client Users API
GET    /api/clients                           // List all clients
GET    /api/clients/{client_id}               // Get client details
GET    /api/clients/{client_id}/activity      // Activity log
POST   /api/clients/{client_id}/permissions   // Grant file access
DELETE /api/clients/{client_id}/permissions/{permission_id}  // Revoke access
PUT    /api/clients/{client_id}/settings      // Update session settings
DELETE /api/sessions/{session_id}             // Terminate session

// Real-time updates via WebSocket
WS /ws/clients
→ Sends: { event: "session_started", client_id: "...", session_id: "..." }
→ Sends: { event: "session_ended", session_id: "..." }
→ Sends: { event: "file_accessed", client_id: "...", file_path: "..." }
→ Sends: { event: "access_request", client_id: "...", purpose: "..." }
```

---

### View 3: Invitations

**Purpose:** Invite new client users via link or email.

**Interface:**

```
┌────────────────────────────────────────────────────────────┐
│  📧 Invite Client User                                [×]  │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Method:                                                    │
│  ○ Email Invitation                                         │
│  ● Link Invitation                                          │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  Share Files/Folders:                         [Select...]  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ✅ /Documents/Contract.pdf                           │  │
│  │ ✅ /Financial/Report_Q4.xlsx                         │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Access Duration:                                           │
│  [○ 1 hour  ○ 1 day  ● 1 week  ○ Never expire]            │
│                                                             │
│  Maximum Session Duration:                                  │
│  [2 hours ▼]                                                │
│                                                             │
│  Require Approval:                                          │
│  [☑] Client must request access (I will approve manually)  │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  Invitation Link:                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ https://sandbox.example.com/invite/tk_a1b2c3d4e5f6  │  │
│  └──────────────────────────────────────────────────────┘  │
│  [Copy Link] [Send via Email]                              │
│                                                             │
│  This link expires in: 7 days                               │
│  Uses remaining: Unlimited                                  │
│                                                             │
│  [Create Invitation]                                        │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**Email Invitation Flow:**
```
User enters email → Selects files → Clicks [Send via Email]
→ Server generates invitation token
→ Sends email:
  Subject: "You've been invited to view secure documents"
  Body: 
    "John Doe has shared files with you.
     Click here to access: https://sandbox.example.com/invite/tk_...
     This link expires in 7 days."
→ Recipient clicks link → Registers with WebAuthn → Access granted
```

**Link Invitation Flow:**
```
User clicks [Create Invitation] → Copies link → Shares via any channel (Slack, text, etc.)
→ Recipient clicks link → Registers/logs in → Requests access (if approval required)
→ User approves → Recipient gains access
```

---

## Client User Interface

### View 1: Access Request

**Purpose:** Request access to a user's shared files.

**Interface:**

```
┌────────────────────────────────────────────────────────────┐
│  🔐 Access Request                                          │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  You've been invited by: John Doe (john@example.com)       │
│                                                             │
│  Shared files:                                              │
│  • Contract.pdf                                             │
│  • Financial Report Q4.xlsx                                 │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  To proceed, please provide:                                │
│                                                             │
│  Purpose of Access (required):                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Review partnership contract terms                    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Your Organization (optional):                              │
│  [Acme Corp                              ]                  │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│                                                             │
│  By requesting access, you agree to:                        │
│  ☑ Not download or copy any files                          │
│  ☑ Not take screenshots                                     │
│  ☑ All activity will be logged                             │
│                                                             │
│  [Request Access]                                           │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**After Approval:**

```
┌────────────────────────────────────────────────────────────┐
│  ✅ Access Granted                                          │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Your access request has been approved by John Doe.         │
│                                                             │
│  Access expires in: 6 days, 23 hours                        │
│  Maximum session duration: 2 hours                          │
│                                                             │
│  [Start Session]                                            │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

### View 2: Sandbox Video Feed

**Purpose:** View files via server-side application rendering (no client-side download).

**Interface:**

```
┌────────────────────────────────────────────────────────────┐
│  Secure Document Viewer - John Doe's Files            [×]  │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Session expires in: 1h 45m                                 │
│  Viewing: Contract.pdf                                      │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                                                      │  │
│  │         [VIDEO FEED OF SANDBOX]                     │  │
│  │                                                      │  │
│  │   Shows PDF viewer (evince) rendering Contract.pdf  │  │
│  │   User can scroll, zoom via mouse/keyboard          │  │
│  │   Input is forwarded to sandbox, video streamed back│  │
│  │                                                      │  │
│  │   Watermark: "Confidential - Alice Johnson"         │  │
│  │   (overlaid on video stream)                        │  │
│  │                                                      │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  [🖱️ Controls: Mouse and keyboard enabled]                 │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**How Video Feed Works:**

```
Client Browser ←→ WebRTC ←→ Rust Server ←→ Sandbox (isolated X11)
                                               ↓
                                          Application
                                         (evince, libreoffice, etc.)
                                               ↓
                                         Video Capture
                                    (GStreamer: ximagesrc + VP8)
                                               ↓
                                         VP8 Stream
                                               ↓
                                         WebRTC Video Track
                                               ↓
                                         Client Browser
```

**Real-Time Permission Enforcement in Video Feed:**

**Scenario: Owner removes file access while client is viewing**

```
Client is viewing /Documents/Contract.pdf in sandbox
↓
Owner clicks [Remove] on that file
↓
WebSocket message: { event: "permission_revoked", file_path: "/Documents/Contract.pdf" }
↓
Sandbox receives message
↓
Landlock policy updated: remove /Documents/Contract.pdf from allowed paths
↓
Video feed shows error:
  ┌──────────────────────────────────────┐
  │  ⚠️ Access Revoked                   │
  │                                      │
  │  The owner has removed your access   │
  │  to this file.                       │
  │                                      │
  │  [Return to File Explorer]           │
  └──────────────────────────────────────┘
```

---

### View 3: Limited File Explorer (Client)

**Purpose:** Browse accessible files (read-only, limited to granted permissions).

**Interface:**

```
┌────────────────────────────────────────────────────────────┐
│  📁 Accessible Files                             [Refresh] │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Shared by: John Doe                                        │
│  Access expires: 6 days, 23 hours                           │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 📄 Contract.pdf                  Last modified        │  │
│  │ 📄 NDA.pdf                       2 days ago           │  │
│  │ 📄 Report_Q4.xlsx                1 week ago           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Right-click menu:                                          │
│  • View (in sandbox)                                        │
│  • Properties                                               │
│                                                             │
│  ⚠️ Download disabled (view-only access)                    │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

**Client Clicks "View":**
```
→ Open file in sandbox
→ Start video feed
→ Application (evince/libreoffice) opens file
→ Client sees rendered output via WebRTC
→ Client can interact (scroll, zoom) via mouse/keyboard
→ No download happens on client side
```

---

## Real-Time Architecture

### Permission Enforcement vs UI Synchronization

**CRITICAL DISTINCTION:**

There are **two separate update mechanisms** with different purposes:

#### 1. Security Enforcement (Server-Side, Mandatory)

```
Owner Action → Database → Sandbox Manager → Landlock Policy Update
                                              (Kernel-Level, Immediate)
```

**This happens entirely on the server.** The client cannot bypass this.

When owner removes file access:
1. Permission revoked in database
2. Sandbox manager detects change (polling or event-driven)
3. Landlock LSM rules updated in kernel
4. File access blocked immediately at syscall level

**The client has NO involvement in enforcement.** Even if client's browser crashes or is malicious, the kernel blocks access.

#### 2. UI Synchronization (Client-Side, Optional UX)

```
Owner Action → WebSocket → Client Browser → UI Update
```

**This is purely for user experience.** It tells the client's UI to update so they don't see stale information.

When owner removes file access:
1. WebSocket message sent to client browser
2. Client's file explorer removes the file from view
3. If client is viewing the file, show "Access Revoked" message
4. Graceful UX instead of cryptic errors

**Why WebSocket for UI?**
- Owner sees live activity (who's viewing what, when)
- Client sees their file list update without page refresh
- Client gets graceful error messages instead of permission denied errors
- Session expiration warnings ("5 minutes remaining")

**If WebSocket fails:** Security is NOT compromised. Client just sees stale UI until they refresh or try to access the file (which fails at kernel level).

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Owner Browser                           │
│  User clicks [Remove] on /Documents/Contract.pdf            │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
                    POST /api/permissions/{id}/revoke
                             │
┌────────────────────────────▼────────────────────────────────┐
│                      Rust Server                            │
│                                                             │
│  [Application Layer]                                        │
│    RevokePermissionCommand handler                          │
│         │                                                   │
│         ├─► 1. UPDATE permissions SET revoked=true         │
│         │      (PostgreSQL - source of truth)              │
│         │                                                   │
│         ├─► 2. Emit PermissionRevoked event                │
│         │      (Domain event)                               │
│         │                                                   │
│         └─► 3. Call SandboxManager.update_permissions()    │
│                    │                                        │
│                    ▼                                        │
│  [Sandbox Manager] ◄────────────────────────┐              │
│    Polls database for permission changes    │              │
│    OR listens to domain events              │              │
│         │                                    │              │
│         ▼                                    │              │
│    Find active sessions for this file       │              │
│         │                                    │              │
│         ▼                                    │              │
│    session_id: ses_abc123 (Alice's session) │              │
│         │                                    │              │
│         ├─► A. Update Landlock rules ────────┼──────────┐  │
│         │   (CRITICAL: Security enforcement) │          │  │
│         │                                    │          │  │
│         └─► B. Send WebSocket message ───────┼────────┐ │  │
│             (OPTIONAL: UI sync)              │        │ │  │
│                                              │        │ │  │
└──────────────────────────────────────────────┼────────┼─┼──┘
                                               │        │ │
                    ┌──────────────────────────┘        │ │
                    ▼                                   │ │
         ┌──────────────────────┐                       │ │
         │   Sandbox (Kernel)   │                       │ │
         │   ses_abc123         │                       │ │
         │                      │                       │ │
         │  [Landlock LSM]      │                       │ │
         │   Allowed paths:     │                       │ │
         │   - /data/users/.../NDA.pdf                  │ │
         │   - /data/users/.../Report.xlsx              │ │
         │   ✗ Contract.pdf REMOVED ◄───────────────────┘ │
         │                      │                         │
         │  Next file access:   │                         │
         │  open("/Contract.pdf")                        │
         │       ↓              │                         │
         │   EPERM (kernel)     │                         │
         │   Access denied      │                         │
         └──────────────────────┘                         │
                                                          │
         ┌────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│          Client Browser (Alice)                 │
│                                                 │
│  WebSocket receives:                            │
│  {                                              │
│    event: "permission_revoked",                 │
│    file_path: "/Documents/Contract.pdf"         │
│  }                                              │
│         │                                       │
│         ▼                                       │
│  [UI Update]                                    │
│   - Remove file from file explorer              │
│   - If viewing: Show "Access Revoked" modal     │
│   - Disable "View" button                       │
│                                                 │
│  ⚠️ This is COSMETIC - security already         │
│     enforced in kernel                          │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Sandbox Permission Update Mechanisms

**Option 1: Event-Driven (Recommended)**

```rust
// When permission is revoked, emit event
impl RevokePermissionCommandHandler {
    async fn handle(&self, cmd: RevokePermissionCommand) -> Result<()> {
        // 1. Database update
        self.permission_repository.revoke(&cmd.permission_id).await?;
        
        // 2. Emit domain event
        let event = DomainEvent::PermissionRevoked {
            permission_id: cmd.permission_id,
            user_id: cmd.client_user_id,
            file_path: cmd.file_path,
        };
        self.event_publisher.publish(event).await?;
        
        Ok(())
    }
}

// Sandbox manager subscribes to events
impl EventHandler<DomainEvent> for SandboxManager {
    async fn handle(&self, event: DomainEvent) -> Result<()> {
        match event {
            DomainEvent::PermissionRevoked { user_id, file_path, .. } => {
                // Find active sessions for this user
                let sessions = self.find_active_sessions(&user_id).await?;
                
                for session in sessions {
                    // Update Landlock rules (SECURITY CRITICAL)
                    self.update_landlock_policy(&session.id, |policy| {
                        policy.remove_path(&file_path)
                    }).await?;
                    
                    // Notify client UI (OPTIONAL UX)
                    self.websocket.send(&session.id, WsMessage::PermissionRevoked {
                        file_path: file_path.clone(),
                    }).await.ok(); // Non-critical, ignore errors
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

**Option 2: Database Polling (Fallback)**

```rust
// Sandbox manager polls for permission changes
impl SandboxManager {
    async fn poll_permission_changes(&self) -> Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            // Query database for recent permission changes
            let changes = self.permission_repository
                .find_changes_since(self.last_check)
                .await?;
            
            for change in changes {
                if change.revoked {
                    // Update Landlock (SECURITY)
                    self.update_landlock_policy(&change.session_id, |policy| {
                        policy.remove_path(&change.file_path)
                    }).await?;
                    
                    // Notify UI (UX)
                    self.websocket.send(&change.session_id, ...).await.ok();
                }
            }
            
            self.last_check = Utc::now();
        }
    }
}
```

**Landlock Update Implementation:**

```rust
// CRITICAL: Landlock rules cannot be modified after sandbox starts
// Must restart sandbox with new rules

impl SandboxManager {
    async fn update_landlock_policy(
        &self,
        session_id: &SessionId,
        file_path: &Path,
        action: PolicyAction,
    ) -> Result<()> {
        // Get current session
        let session = self.sessions.get(session_id)?;
        
        match action {
            PolicyAction::RemovePath(path) => {
                // 1. Get current permissions from database (source of truth)
                let permissions = self.permission_repository
                    .find_active_for_user(&session.user_id)
                    .await?;
                
                // 2. Kill current sandbox process
                session.sandbox_process.kill().await?;
                
                // 3. Create new Landlock ruleset
                let ruleset = Landlock::new()
                    .allow_read(&[])
                    .allow_write(&[]);
                
                for perm in permissions {
                    if !perm.is_revoked() {
                        ruleset.allow_read(&perm.file_path)?;
                    }
                }
                
                // 4. Start new sandbox with updated rules
                let new_sandbox = self.spawn_sandbox(
                    &session.id,
                    ruleset,
                ).await?;
                
                // 5. Restore session state (reopen last file if possible)
                if let Some(current_file) = &session.current_file {
                    if permissions.iter().any(|p| p.file_path == current_file && !p.is_revoked()) {
                        new_sandbox.open_file(current_file).await?;
                    }
                }
                
                // Client experiences ~2 second interruption
                // but security is enforced at kernel level
            }
        }
        
        Ok(())
    }
}
```

### WebSocket Event Flow

**Server → Client (Owner) Events:**
```javascript
// New access request
{
  event: "access_request_received",
  client_id: "usr_123",
  client_email: "alice@example.com",
  purpose: "Review contract",
  timestamp: "2026-02-14T10:30:00Z"
}

// Client started session
{
  event: "session_started",
  session_id: "ses_abc123",
  client_id: "usr_123",
  client_email: "alice@example.com",
  ip_address: "192.168.1.100"
}

// Client accessing file
{
  event: "file_accessed",
  session_id: "ses_abc123",
  file_path: "/Documents/Contract.pdf",
  timestamp: "2026-02-14T10:31:00Z"
}

// Client session ended
{
  event: "session_ended",
  session_id: "ses_abc123",
  reason: "timeout",
  duration_seconds: 1800
}
```

**Server → Client (Client User) Events:**
```javascript
// Permission granted
{
  event: "permission_granted",
  file_path: "/Documents/NewReport.pdf",
  file_id: "fil_789"
}

// Permission revoked
{
  event: "permission_revoked",
  file_path: "/Documents/Contract.pdf",
  message: "Access has been revoked by the owner."
}

// Session settings changed
{
  event: "settings_changed",
  setting: "watermark",
  value: true,
  message: "Watermark has been enabled. Your session will restart."
}

// Session termination warning
{
  event: "session_expiring",
  seconds_remaining: 300,  // 5 minutes
  message: "Your session will expire in 5 minutes."
}

// Session terminated
{
  event: "session_terminated",
  reason: "owner_action",
  message: "The owner has ended your session."
}
```

### Permission Enforcement Pipeline

**TWO PARALLEL PATHS:**

```
Owner Action (UI)
    ↓
[API] POST /api/permissions/{id}/revoke
    ↓
[Application Layer] RevokePermissionCommand
    ↓
[Domain Layer] Permission.revoke() 
    ↓
Database: UPDATE permissions SET revoked=true
    ↓
Domain Event: PermissionRevoked emitted
    ↓
    ├─────────────────────────────────────────┬──────────────────────────────┐
    │                                         │                              │
    ▼ PATH 1: SECURITY (Critical)             ▼ PATH 2: UI SYNC (Optional)   ▼ PATH 3: AUDIT (Mandatory)
    │                                         │                              │
[Sandbox Manager]                      [WebSocket Service]          [Audit Logger]
    │                                         │                              │
Find active sessions                   Find connected clients        Log event to database
    │                                         │                              │
    ▼                                         ▼                              ▼
Update Landlock ruleset              Send WS message               INSERT audit_events
    │                                         │                        
Restart sandbox process              { event: "permission_revoked" }
    │                                         │                        
    ▼                                         ▼                        
Kernel blocks file access            Client UI updates              
(EPERM on open())                    (file disappears)              
```

**Timeline:**
- **PATH 1 (Security):** ~2 seconds (sandbox restart required)
- **PATH 2 (UI Sync):** ~100ms (WebSocket message)
- **PATH 3 (Audit):** ~10ms (database write)

**Important:** PATH 1 is the actual security enforcement. PATH 2 is purely cosmetic. Even if PATH 2 fails completely, security is maintained.

**Why WebSocket Exists:**

NOT for security - for these UX improvements:
1. **Owner dashboard**: See live activity (who's viewing what)
2. **Client warnings**: "Session expires in 5 minutes"
3. **Graceful errors**: "Access revoked" instead of cryptic EPERM
4. **Real-time UI**: File appears/disappears without refresh

If WebSocket fails, the worst outcome is stale UI. The kernel still enforces access control.

### Timeline: ~100ms from owner action to client enforcement

---

## Security Considerations

### Real-Time Permission Validation

**CRITICAL: Never trust client state**

Every file access MUST be validated server-side in real-time:

```rust
// WRONG: Trust client's cached permissions
async fn open_file(session_id: &SessionId, file_path: &str) -> Result<()> {
    // Client says they have access, just open it
    sandbox.open_file(file_path).await  // ❌ VULNERABLE
}

// CORRECT: Validate permission on every access
async fn open_file(
    session_id: &SessionId, 
    file_path: &str,
    permission_repository: &dyn PermissionRepository,
) -> Result<()> {
    // Get current session
    let session = session_repository.find_by_id(session_id).await?;
    
    // Check permission exists and is not revoked
    let permission = permission_repository
        .find_for_user_and_file(&session.user_id, file_path)
        .await?;
    
    if permission.is_revoked() || permission.is_expired() {
        return Err(DomainError::AccessDenied);
    }
    
    // Additional checks
    if session.is_terminated() {
        return Err(DomainError::SessionTerminated);
    }
    
    // All checks passed, open file
    sandbox.open_file(file_path).await  // ✅ SAFE
}
```

### Landlock Real-Time Updates

**Challenge:** Landlock policies are set at sandbox creation, cannot be modified after.

**Solution:** Session restart on permission changes

```rust
async fn revoke_permission(
    permission_id: &PermissionId,
    session_id: &SessionId,
) -> Result<()> {
    // 1. Revoke permission in database
    permission_repository.revoke(permission_id).await?;
    
    // 2. Send WebSocket notification
    websocket.send(session_id, Event::PermissionRevoked { ... }).await?;
    
    // 3. Restart sandbox with updated Landlock rules
    sandbox_manager.restart_session(session_id).await?;
    
    // Client experiences brief reconnection (~2 seconds)
    // but enforcement is immediate and kernel-level
    
    Ok(())
}
```

**Alternative (Faster):** Application-level file access gate

```rust
// Don't rely solely on Landlock, add application-level check
// This allows instant enforcement without sandbox restart

impl SandboxFileSystem {
    fn open(&self, path: &Path) -> Result<File> {
        // Real-time permission check
        if !self.check_permission_realtime(path)? {
            return Err(Error::AccessDenied);
        }
        
        // Landlock provides defense-in-depth
        // Even if application check is bypassed, kernel blocks access
        File::open(path)  // Landlock enforces at kernel level
    }
}
```

---

## Technical Implementation Summary

### Owner Workflows

| Feature | API Endpoint | Real-Time Event | Database Tables |
|---------|-------------|-----------------|-----------------|
| File Explorer | GET /api/files | file_created, file_deleted | files, file_metadata |
| Upload File | POST /api/files/upload | file_created | files |
| Delete File | DELETE /api/files/{id} | file_deleted, permission_revoked | files, permissions |
| Grant Access | POST /api/permissions | permission_granted | permissions |
| Revoke Access | DELETE /api/permissions/{id} | permission_revoked | permissions |
| Terminate Session | DELETE /api/sessions/{id} | session_terminated | sessions |
| View Activity | GET /api/clients/{id}/activity | - | audit_events |

### Client User Workflows

| Feature | API Endpoint | Real-Time Event | Database Tables |
|---------|-------------|-----------------|-----------------|
| Request Access | POST /api/access-requests | access_request_received | access_requests |
| View Files | GET /api/files | permission_granted | permissions, files |
| Start Session | POST /api/sessions | session_started | sessions |
| Open File | POST /api/sessions/{id}/open | file_accessed | audit_events |

### WebSocket Topics

```
/ws/owner/{user_id}       - Owner receives updates about their clients
/ws/client/{user_id}      - Client receives permission/session updates
/ws/session/{session_id}  - Sandbox receives control commands
```

---

## UI/UX Mockup Tools

**For Implementation:**
- **Owner Dashboard:** React + TypeScript + shadcn/ui components
- **File Explorer:** React Virtual (for large directories) + drag-and-drop
- **Video Feed:** WebRTC (RTCPeerConnection) + Canvas for watermark overlay
- **Real-Time UI Updates:** WebSocket (for cosmetic UI sync only, NOT security enforcement)
- **Security Enforcement:** Server-side Landlock LSM (kernel-level, client cannot bypass)

---

## Frequently Asked Questions

### Q: Why WebSocket if security is enforced server-side?

**A:** WebSocket is purely for **user experience**, not security.

- **Without WebSocket:** Client sees stale UI, gets cryptic errors, owner doesn't see live activity
- **With WebSocket:** Client sees graceful messages, owner sees real-time dashboard, better UX
- **Security:** Identical in both cases (Landlock in kernel enforces access)

### Q: What happens if WebSocket connection drops?

**A:** Security is NOT compromised. Only UX degrades.

- ✅ File access still blocked at kernel level (Landlock)
- ✅ Audit logs still written (database)
- ❌ Client UI shows stale file list
- ❌ Owner dashboard shows outdated activity
- **Fix:** Client refreshes page, reconnects WebSocket

### Q: Can malicious client bypass permission check?

**A:** No. Security is enforced in the Linux kernel, not client code.

Even if client:
- Modifies browser JavaScript
- Intercepts WebSocket messages
- Sends fake API requests
- Runs custom sandbox code

...the kernel's Landlock LSM will still block unauthorized file access with `EPERM`.

The client has NO involvement in security decisions. They are purely a display device for video feed.

### Q: Why restart sandbox instead of hot-reload Landlock rules?

**A:** Landlock policies are immutable after creation (kernel design).

Once a Landlock ruleset is applied to a process via `landlock_restrict_self()`, it cannot be modified. To change permissions, we must:
1. Kill sandbox process
2. Create new process with updated Landlock ruleset
3. Restart application (evince, etc.)

This takes ~2 seconds but provides absolute security guarantees.

**Alternative considered:** Application-level gatekeeper without Landlock restart
- Faster (instant permission updates)
- BUT: Vulnerable to application bugs
- Landlock provides defense-in-depth at kernel level

**Recommendation:** Use both:
- Application-level check for instant feedback
- Landlock restart for kernel-enforced security

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-14  
**Related Documents:** [PERSONAS.md](PERSONAS.md), [ARCHITECTURE.md](ARCHITECTURE.md), [COMMANDS.md](COMMANDS.md), [QUERIES.md](QUERIES.md)
