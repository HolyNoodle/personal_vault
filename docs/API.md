# API Reference

## Overview

The Secure Sandbox Server exposes a RESTful HTTP API for authentication and session management, and a WebSocket API for WebRTC signaling and real-time communication.

**Base URL:** `http://localhost:8080/api`

**Authentication:** JWT Bearer tokens (except login/register endpoints)

## Authentication

### Register User

Create a new user account.

**Endpoint:** `POST /api/auth/register`

**Request:**
```json
{
  "username": "john.doe",
  "password": "SecurePass123!",
  "email": "john@example.com"
}
```

**Response:** `201 Created`
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john.doe",
  "created_at": "2026-02-13T10:30:00Z"
}
```

**Errors:**
- `400 Bad Request`: Invalid input (weak password, invalid email)
- `409 Conflict`: Username already exists

---

### Login

Authenticate and receive JWT tokens.

**Endpoint:** `POST /api/auth/login`

**Request:**
```json
{
  "username": "john.doe",
  "password": "SecurePass123!"
}
```

**Response:** `200 OK`
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 900,
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "john.doe",
    "roles": ["user"]
  }
}
```

**Cookies Set:**
- `refresh_token`: HttpOnly, Secure, SameSite=Strict, Max-Age=604800

**Errors:**
- `401 Unauthorized`: Invalid credentials
- `429 Too Many Requests`: Rate limit exceeded (5 attempts per 5 minutes)

---

### Refresh Token

Obtain a new access token using refresh token.

**Endpoint:** `POST /api/auth/refresh`

**Headers:**
- Cookie: `refresh_token=<token>`

**Response:** `200 OK`
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

**Errors:**
- `401 Unauthorized`: Invalid or expired refresh token

---

### Logout

Invalidate current session and tokens.

**Endpoint:** `POST /api/auth/logout`

**Headers:**
- `Authorization: Bearer <access_token>`

**Response:** `204 No Content`

---

## Sessions

### Create Session

Initialize a new sandbox session.

**Endpoint:** `POST /api/sessions`

**Headers:**
- `Authorization: Bearer <access_token>`

**Request:**
```json
{
  "resolution": "1920x1080",
  "applications": ["evince", "eog"],
  "file_permissions": {
    "/documents/report.pdf": "read",
    "/workspace/notes.txt": "write"
  }
}
```

**Response:** `201 Created`
```json
{
  "session_id": "abc123def456",
  "websocket_url": "ws://localhost:8080/ws?session=abc123def456",
  "status": "initializing",
  "created_at": "2026-02-13T10:35:00Z",
  "expires_at": "2026-02-13T11:05:00Z"
}
```

**Errors:**
- `401 Unauthorized`: Invalid token
- `403 Forbidden`: Insufficient permissions for requested files
- `507 Insufficient Storage`: Server capacity reached

---

### Get Session

Retrieve session details.

**Endpoint:** `GET /api/sessions/{session_id}`

**Headers:**
- `Authorization: Bearer <access_token>`

**Response:** `200 OK`
```json
{
  "session_id": "abc123def456",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "active",
  "created_at": "2026-02-13T10:35:00Z",
  "last_activity": "2026-02-13T10:40:00Z",
  "expires_at": "2026-02-13T11:05:00Z",
  "resources": {
    "cpu_percent": 25,
    "memory_mb": 320,
    "pid_count": 15
  }
}
```

**Errors:**
- `404 Not Found`: Session doesn't exist or belongs to different user

---

### List Sessions

Get all active sessions for current user.

**Endpoint:** `GET /api/sessions`

**Headers:**
- `Authorization: Bearer <access_token>`

**Response:** `200 OK`
```json
{
  "sessions": [
    {
      "session_id": "abc123def456",
      "status": "active",
      "created_at": "2026-02-13T10:35:00Z",
      "last_activity": "2026-02-13T10:40:00Z"
    }
  ],
  "total": 1
}
```

---

### Terminate Session

Stop and clean up a session.

**Endpoint:** `DELETE /api/sessions/{session_id}`

**Headers:**
- `Authorization: Bearer <access_token>`

**Response:** `204 No Content`

**Errors:**
- `404 Not Found`: Session doesn't exist

---

## Files & Permissions

### List Files

Get files accessible to current user.

**Endpoint:** `GET /api/files`

**Headers:**
- `Authorization: Bearer <access_token>`

**Query Parameters:**
- `path` (optional): Filter by path prefix
- `permission` (optional): Filter by permission level (read, write, execute)

**Response:** `200 OK`
```json
{
  "files": [
    {
      "path": "/documents/report.pdf",
      "size": 2048576,
      "permissions": ["read"],
      "created_at": "2026-02-10T08:00:00Z",
      "modified_at": "2026-02-12T14:30:00Z"
    },
    {
      "path": "/workspace/notes.txt",
      "size": 4096,
      "permissions": ["read", "write"],
      "created_at": "2026-02-13T09:00:00Z",
      "modified_at": "2026-02-13T10:30:00Z"
    }
  ],
  "total": 2
}
```

---

### Get File Metadata

Retrieve metadata for a specific file.

**Endpoint:** `GET /api/files/metadata`

**Headers:**
- `Authorization: Bearer <access_token>`

**Query Parameters:**
- `path` (required): File path

**Response:** `200 OK`
```json
{
  "path": "/documents/report.pdf",
  "size": 2048576,
  "mime_type": "application/pdf",
  "permissions": ["read"],
  "checksum": "sha256:a3b2c1...",
  "created_at": "2026-02-10T08:00:00Z",
  "modified_at": "2026-02-12T14:30:00Z"
}
```

**Errors:**
- `403 Forbidden`: No permission to access file
- `404 Not Found`: File doesn't exist

---

## Audit Logs

### Get Access Logs

Retrieve audit trail for current user.

**Endpoint:** `GET /api/audit/logs`

**Headers:**
- `Authorization: Bearer <access_token>`

**Query Parameters:**
- `start_date` (optional): ISO 8601 timestamp
- `end_date` (optional): ISO 8601 timestamp
- `event_type` (optional): Filter by type (login, file_access, session_created)
- `limit` (optional, default 100): Max results
- `offset` (optional, default 0): Pagination offset

**Response:** `200 OK`
```json
{
  "logs": [
    {
      "id": "log-123",
      "timestamp": "2026-02-13T10:35:00Z",
      "event_type": "file_access",
      "resource": "/documents/report.pdf",
      "action": "read",
      "result": "allowed",
      "session_id": "abc123def456",
      "ip_address": "192.168.1.100"
    },
    {
      "id": "log-124",
      "timestamp": "2026-02-13T10:30:00Z",
      "event_type": "login",
      "action": "authenticate",
      "result": "success",
      "ip_address": "192.168.1.100"
    }
  ],
  "total": 2,
  "limit": 100,
  "offset": 0
}
```

---

## WebSocket API

### Connection

**Endpoint:** `ws://localhost:8080/ws`

**Query Parameters:**
- `session={session_id}`: Session identifier from POST /api/sessions
- `token={access_token}`: JWT access token

**Example:**
```
ws://localhost:8080/ws?session=abc123def456&token=eyJhbGci...
```

---

### Message Format

All messages are JSON-encoded.

**Client → Server:**
```json
{
  "type": "message_type",
  "payload": { /* type-specific data */ }
}
```

**Server → Client:**
```json
{
  "type": "message_type",
  "payload": { /* type-specific data */ }
}
```

---

### WebRTC Signaling

#### Offer (Server → Client)

Server sends SDP offer to initiate WebRTC connection.

```json
{
  "type": "offer",
  "payload": {
    "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\ns=-\r\n..."
  }
}
```

#### Answer (Client → Server)

Client responds with SDP answer.

```json
{
  "type": "answer",
  "payload": {
    "sdp": "v=0\r\no=- 987654321 2 IN IP4 127.0.0.1\r\ns=-\r\n..."
  }
}
```

#### ICE Candidate (Bidirectional)

Exchange ICE candidates for NAT traversal.

**Server → Client:**
```json
{
  "type": "ice_candidate",
  "payload": {
    "candidate": "candidate:1 1 UDP 2130706431 192.168.1.10 54321 typ host",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

**Client → Server:**
```json
{
  "type": "ice_candidate",
  "payload": {
    "candidate": "candidate:2 1 UDP 2130706431 192.168.1.100 54322 typ host",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

---

### Input Events

#### Mouse Event (Client → Server)

Sent via WebRTC Data Channel (not WebSocket).

```json
{
  "type": "mouse",
  "x": 800,
  "y": 600,
  "button": "left",
  "action": "click"
}
```

**Fields:**
- `x`, `y`: Coordinates relative to video stream (0-based)
- `button`: `left`, `right`, `middle`, `scroll_up`, `scroll_down`
- `action`: `click`, `down`, `up`, `move`, `double_click`

#### Keyboard Event (Client → Server)

```json
{
  "type": "keyboard",
  "key": "a",
  "action": "press",
  "modifiers": ["ctrl"]
}
```

**Fields:**
- `key`: Key code or character
- `action`: `press`, `release`, `type`
- `modifiers`: Array of `ctrl`, `shift`, `alt`, `meta`

---

### Status Updates

#### Session Status (Server → Client)

```json
{
  "type": "session_status",
  "payload": {
    "status": "ready",
    "message": "Sandbox initialized successfully"
  }
}
```

**Status Values:**
- `initializing`: Creating sandbox environment
- `ready`: WebRTC connection established, video streaming
- `error`: Fatal error occurred
- `terminated`: Session ended

#### Error (Server → Client)

```json
{
  "type": "error",
  "payload": {
    "code": "SANDBOX_FAILED",
    "message": "Failed to create namespace: operation not permitted",
    "recoverable": false
  }
}
```

---

## Error Codes

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request successful |
| 201 | Created | Resource created successfully |
| 204 | No Content | Request successful, no response body |
| 400 | Bad Request | Invalid request format or parameters |
| 401 | Unauthorized | Missing or invalid authentication |
| 403 | Forbidden | Authenticated but insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Resource already exists |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server-side error |
| 503 | Service Unavailable | Server overloaded or maintenance |
| 507 | Insufficient Storage | Server capacity reached |

### Application Error Codes

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable description",
    "details": { /* optional additional context */ }
  }
}
```

**Common Codes:**

- `INVALID_CREDENTIALS`: Username or password incorrect
- `TOKEN_EXPIRED`: JWT token expired, refresh required
- `TOKEN_INVALID`: JWT token malformed or signature invalid
- `PERMISSION_DENIED`: Insufficient permissions for operation
- `USER_EXISTS`: Username already registered
- `SESSION_NOT_FOUND`: Session ID doesn't exist
- `SESSION_EXPIRED`: Session timeout reached
- `SANDBOX_FAILED`: Unable to create sandbox environment
- `FILE_NOT_FOUND`: Requested file doesn't exist
- `CAPACITY_EXCEEDED`: Server at maximum concurrent sessions
- `RATE_LIMITED`: Too many requests from this IP

---

## Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| POST /api/auth/login | 5 requests | 5 minutes |
| POST /api/auth/register | 3 requests | 1 hour |
| POST /api/sessions | 10 requests | 1 minute |
| All other endpoints | 100 requests | 1 minute |

**Rate Limit Headers:**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1676280600
```

---

## Examples

### Full Session Workflow

```javascript
// 1. Login
const loginResp = await fetch('http://localhost:8080/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ username: 'john.doe', password: 'SecurePass123!' })
});
const { access_token } = await loginResp.json();

// 2. Create session
const sessionResp = await fetch('http://localhost:8080/api/sessions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${access_token}`
  },
  body: JSON.stringify({
    resolution: '1920x1080',
    applications: ['evince'],
    file_permissions: {
      '/documents/report.pdf': 'read'
    }
  })
});
const { session_id, websocket_url } = await sessionResp.json();

// 3. Connect WebSocket
const ws = new WebSocket(`${websocket_url}&token=${access_token}`);

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  
  if (msg.type === 'offer') {
    // Handle WebRTC offer (see WebRTC example below)
  }
};

// 4. WebRTC connection (simplified)
const pc = new RTCPeerConnection({
  iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
});

pc.ontrack = (event) => {
  document.getElementById('video').srcObject = event.streams[0];
};

pc.ondatachannel = (event) => {
  const dataChannel = event.channel;
  
  // Send mouse events
  document.addEventListener('click', (e) => {
    dataChannel.send(JSON.stringify({
      type: 'mouse',
      x: e.clientX,
      y: e.clientY,
      button: 'left',
      action: 'click'
    }));
  });
};

// 5. Handle SDP offer from server
ws.onmessage = async (event) => {
  const msg = JSON.parse(event.data);
  
  if (msg.type === 'offer') {
    await pc.setRemoteDescription(new RTCSessionDescription({
      type: 'offer',
      sdp: msg.payload.sdp
    }));
    
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);
    
    ws.send(JSON.stringify({
      type: 'answer',
      payload: { sdp: answer.sdp }
    }));
  }
  
  if (msg.type === 'ice_candidate') {
    await pc.addIceCandidate(new RTCIceCandidate(msg.payload));
  }
};

// 6. Send ICE candidates
pc.onicecandidate = (event) => {
  if (event.candidate) {
    ws.send(JSON.stringify({
      type: 'ice_candidate',
      payload: {
        candidate: event.candidate.candidate,
        sdpMid: event.candidate.sdpMid,
        sdpMLineIndex: event.candidate.sdpMLineIndex
      }
    }));
  }
};
```

---

## Versioning

API version is included in the URL path (future):

- v1: `/api/v1/...`
- Current (unversioned): `/api/...`

Breaking changes will be released as new versions with 6-month deprecation period.
