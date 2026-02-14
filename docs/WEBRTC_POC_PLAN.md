# WebRTC Video Feed POC - Implementation Plan

## Project Context

This POC focuses on implementing the **WebRTC video streaming** component for **Client Users** who need to view files in a sandboxed environment without downloading them.

### System Overview
- **Purpose**: Secure file viewing via real-time video streaming
- **Architecture**: Rust backend (Axum) + React frontend (MUI)
- **Technology**: WebRTC for video + WebSocket for signaling
- **Security**: Passwordless auth (WebAuthn), Landlock LSM, namespaces

---

## Key Requirements from Documentation

### Security-First Principles
⚠️ **CRITICAL**: All decisions MUST prioritize security over convenience
- Default deny for all permissions
- No downloads, clipboard, or data exfiltration
- Encrypted video streams (DTLS-SRTP)
- Audit logging of all video sessions

### User Personas for POC
1. **User (Owner)** - Shares files, grants access to clients
2. **Client User** - Views files via video stream (our POC focus)

### Architecture Components Needed

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Browser                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   HTML/JS    │  │  WebRTC      │  │  WebSocket   │         │
│  │   UI (MUI)   │  │  Video Stream│  │  Signaling   │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Rust Server (Axum)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ WebSocket    │  │ WebRTC       │  │ Session      │         │
│  │ Signaling    │  │ Peer Conn    │  │ Manager      │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              Isolated Sandbox Environment                       │
│  ┌────────┐  ┌────────┐  ┌────────────────────┐               │
│  │ Xvfb   │→│ FFmpeg │→│ H.264 Video Stream │               │
│  │ :100   │  │ Encode │  │ (WebRTC Track)     │               │
│  └────────┘  └────────┘  └────────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

---

## POC Scope

### Phase 1: Minimal Working Demo (This POC)

**Goal**: Client user connects and sees a live video stream of a sandboxed application

**In Scope**:
1. ✅ WebSocket signaling server (SDP offer/answer exchange)
2. ✅ WebRTC peer connection (backend ↔ frontend)
3. ✅ Basic sandbox with Xvfb virtual display
4. ✅ FFmpeg video capture and H.264 encoding
5. ✅ React component to display video stream
6. ✅ Basic session management (start/stop)

**Out of Scope** (for Phase 1):
- ❌ Full authentication (use mock/hardcoded for POC)
- ❌ Permission system (assume client has access)
- ❌ Input forwarding (mouse/keyboard)
- ❌ Landlock policies (basic namespace only)
- ❌ Production-grade error handling
- ❌ Resource limits (cgroups)

### Phase 2: Production Features (Future)
- Full WebAuthn authentication
- Permission-based file access
- Input forwarding (xdotool)
- Complete sandbox isolation (Landlock + seccomp)
- Audit logging
- Multi-user support

---

## Technical Implementation Details

### Backend Stack

**Rust Libraries Needed**:
```toml
[dependencies]
# Web framework
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
tokio = { version = "1", features = ["full"] }

# WebSocket
axum-extra = { version = "0.9", features = ["typed-header"] }
tokio-tungstenite = "0.21"

# WebRTC
webrtc = "0.9"  # Pure Rust WebRTC implementation

# Video encoding (process spawning)
tokio-process = "0.2"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# UUID for session IDs
uuid = { version = "1.0", features = ["v4", "serde"] }
```

### Backend Architecture

```rust
backend/src/
├── main.rs                          # Server entry point
├── webrtc/
│   ├── mod.rs
│   ├── peer_connection.rs           # WebRTC peer connection
│   ├── signaling.rs                 # WebSocket signaling handler
│   └── video_track.rs               # Video media track
├── sandbox/
│   ├── mod.rs
│   ├── session.rs                   # Session management
│   ├── xvfb.rs                      # Virtual display
│   └── ffmpeg.rs                    # Video encoder
└── api/
    ├── mod.rs
    ├── routes.rs                    # HTTP routes
    └── websocket.rs                 # WebSocket endpoint
```

### Frontend Stack

**Components Needed**:
```
frontend/web/src/
├── pages/
│   └── VideoSessionPage.tsx         # Main video viewing page
├── components/
│   ├── VideoPlayer.tsx              # WebRTC video component
│   ├── SessionControls.tsx          # Start/stop buttons
│   └── ConnectionStatus.tsx         # Connection indicator
├── services/
│   ├── webrtc.ts                    # WebRTC client
│   └── signaling.ts                 # WebSocket client
└── hooks/
    └── useWebRTCSession.ts          # React hook for session
```

---

## WebRTC Flow Diagram

### Session Initialization

```
┌─────────┐              ┌─────────┐              ┌─────────┐
│ Client  │              │ Server  │              │ Sandbox │
└────┬────┘              └────┬────┘              └────┬────┘
     │                        │                        │
     │ 1. POST /api/sessions  │                        │
     ├───────────────────────►│                        │
     │                        │ 2. Create session      │
     │                        ├───────────────────────►│
     │                        │ 3. Start Xvfb :100     │
     │                        │◄───────────────────────┤
     │                        │ 4. Start FFmpeg        │
     │                        │◄───────────────────────┤
     │ 5. Session ID          │                        │
     │◄───────────────────────┤                        │
     │                        │                        │
     │ 6. WS connect          │                        │
     ├───────────────────────►│                        │
     │                        │                        │
     │ 7. Request offer       │                        │
     ├───────────────────────►│                        │
     │                        │ 8. Create peer conn    │
     │                        │ 9. Add video track     │
     │                        │◄───────────────────────┤
     │                        │                        │
     │ 10. SDP Offer          │                        │
     │◄───────────────────────┤                        │
     │                        │                        │
     │ 11. SDP Answer         │                        │
     ├───────────────────────►│                        │
     │                        │                        │
     │ 12. ICE Candidates     │                        │
     │◄──────────────────────►│                        │
     │                        │                        │
     │ 13. Video streaming    │                        │
     │◄═══════════════════════╪════════════════════════┤
     │    (RTP/DTLS-SRTP)     │                        │
```

### Signaling Messages

**Client → Server**:
```json
{
  "type": "offer",
  "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\n..."
}

{
  "type": "ice-candidate",
  "candidate": {
    "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54321 typ host",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

**Server → Client**:
```json
{
  "type": "answer",
  "sdp": "v=0\r\no=- 987654321 2 IN IP4 127.0.0.1\r\n..."
}

{
  "type": "ice-candidate",
  "candidate": {
    "candidate": "candidate:2 1 UDP 2130706431 10.0.0.5 12345 typ host",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

---

## FFmpeg Pipeline

### Command Structure

```bash
ffmpeg -f x11grab \
       -video_size 1920x1080 \
       -framerate 30 \
       -i :100 \
       -c:v libx264 \
       -preset ultrafast \
       -tune zerolatency \
       -pix_fmt yuv420p \
       -g 60 \
       -f rtp \
       rtp://127.0.0.1:5004
```

**Flags Explained**:
- `-f x11grab`: Capture X11 display
- `-video_size`: Resolution (1920x1080)
- `-framerate 30`: 30 FPS for smooth video
- `-i :100`: Xvfb display number
- `-c:v libx264`: H.264 codec
- `-preset ultrafast`: Low CPU encoding
- `-tune zerolatency`: Minimize latency
- `-pix_fmt yuv420p`: Browser-compatible pixel format
- `-g 60`: Keyframe every 60 frames (2 seconds at 30fps)
- `-f rtp`: Output to RTP stream

### Integration with WebRTC

**Option 1: Direct Pipe (Recommended for POC)**
```rust
// Spawn FFmpeg to output H.264 stream
// Read raw H.264 NAL units
// Feed to WebRTC video track
```

**Option 2: External RTP Server**
```rust
// FFmpeg outputs to RTP port
// WebRTC reads from RTP port
// More complex, better for production
```

---

## Frontend WebRTC Component

### VideoPlayer.tsx Example

```tsx
import React, { useEffect, useRef, useState } from 'react'
import { Box, Typography, CircularProgress } from '@mui/material'

interface VideoPlayerProps {
  sessionId: string
  onConnectionStateChange?: (state: RTCPeerConnectionState) => void
}

export const VideoPlayer: React.FC<VideoPlayerProps> = ({
  sessionId,
  onConnectionStateChange
}) => {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [pc, setPc] = useState<RTCPeerConnection | null>(null)
  const [ws, setWs] = useState<WebSocket | null>(null)
  const [connectionState, setConnectionState] = useState<string>('new')

  useEffect(() => {
    // 1. Create WebSocket connection
    const websocket = new WebSocket(`ws://localhost:8080/ws?session=${sessionId}`)
    setWs(websocket)

    // 2. Create RTCPeerConnection
    const peerConnection = new RTCPeerConnection({
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' }
      ]
    })
    setPc(peerConnection)

    // 3. Handle incoming video track
    peerConnection.ontrack = (event) => {
      if (videoRef.current) {
        videoRef.current.srcObject = event.streams[0]
      }
    }

    // 4. Handle ICE candidates
    peerConnection.onicecandidate = (event) => {
      if (event.candidate && websocket.readyState === WebSocket.OPEN) {
        websocket.send(JSON.stringify({
          type: 'ice-candidate',
          candidate: event.candidate
        }))
      }
    }

    // 5. Monitor connection state
    peerConnection.onconnectionstatechange = () => {
      const state = peerConnection.connectionState
      setConnectionState(state)
      onConnectionStateChange?.(state)
    }

    // 6. Handle signaling messages
    websocket.onmessage = async (event) => {
      const message = JSON.parse(event.data)

      switch (message.type) {
        case 'offer':
          // Server sends offer, client responds with answer
          await peerConnection.setRemoteDescription(
            new RTCSessionDescription(message)
          )
          const answer = await peerConnection.createAnswer()
          await peerConnection.setLocalDescription(answer)
          websocket.send(JSON.stringify({
            type: 'answer',
            sdp: answer.sdp
          }))
          break

        case 'ice-candidate':
          // Add server's ICE candidate
          await peerConnection.addIceCandidate(
            new RTCIceCandidate(message.candidate)
          )
          break
      }
    }

    // 7. Request offer from server
    websocket.onopen = () => {
      websocket.send(JSON.stringify({ type: 'request-offer' }))
    }

    // 8. Cleanup
    return () => {
      peerConnection.close()
      websocket.close()
    }
  }, [sessionId])

  return (
    <Box sx={{ position: 'relative', width: '100%', height: '100%' }}>
      <video
        ref={videoRef}
        autoPlay
        playsInline
        style={{
          width: '100%',
          height: '100%',
          backgroundColor: '#000'
        }}
      />
      
      {connectionState !== 'connected' && (
        <Box
          sx={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            backgroundColor: 'rgba(0,0,0,0.7)'
          }}
        >
          <CircularProgress />
          <Typography sx={{ mt: 2, color: 'white' }}>
            {connectionState === 'new' && 'Initializing...'}
            {connectionState === 'connecting' && 'Connecting...'}
            {connectionState === 'failed' && 'Connection failed'}
            {connectionState === 'disconnected' && 'Disconnected'}
          </Typography>
        </Box>
      )}
    </Box>
  )
}
```

---

## Testing Strategy

### Backend Tests

```rust
// Unit tests
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_create_session() {
        // Test session creation
    }

    #[tokio::test]
    async fn test_webrtc_offer() {
        // Test SDP offer generation
    }
}
```

### Frontend Tests

```tsx
// Component tests with React Testing Library
import { render, screen } from '@testing-library/react'
import { VideoPlayer } from './VideoPlayer'

test('displays loading state initially', () => {
  render(<VideoPlayer sessionId="test-123" />)
  expect(screen.getByText(/initializing/i)).toBeInTheDocument()
})
```

### Manual Testing Checklist

- [ ] WebSocket connects successfully
- [ ] SDP offer/answer exchange completes
- [ ] ICE candidates are exchanged
- [ ] Video element receives stream
- [ ] Video plays smoothly (30 FPS)
- [ ] Connection state updates correctly
- [ ] Graceful handling of disconnects
- [ ] Session cleanup on browser close

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| **Latency** | <200ms | Glass-to-glass delay |
| **FPS** | 30 FPS | Smooth video |
| **Bitrate** | 2-5 Mbps | Balance quality/bandwidth |
| **CPU (per session)** | <30% | Single core utilization |
| **Memory (per session)** | <500MB | Including Xvfb + FFmpeg |
| **Startup time** | <3 seconds | Session initialization |

---

## Security Considerations for POC

Even in POC, maintain minimum security:

1. **Network**:
   - ✅ Use WSS (secure WebSocket) in production
   - ✅ DTLS-SRTP encrypts video (WebRTC default)
   - ⚠️ For POC: localhost is OK

2. **Sandbox**:
   - ✅ Run Xvfb in isolated directory
   - ✅ No network access from sandbox
   - ⚠️ For POC: basic namespace isolation

3. **Session**:
   - ✅ UUID-based session IDs (not sequential)
   - ✅ Session timeout after inactivity
   - ⚠️ For POC: no authentication required

---

## Next Steps

### Immediate (POC Implementation)

1. **Backend Setup**:
   ```bash
   cd backend
   cargo add webrtc tokio-tungstenite
   ```

2. **Create WebRTC Module**:
   - `src/webrtc/mod.rs` - Module definition
   - `src/webrtc/peer_connection.rs` - Peer management
   - `src/webrtc/signaling.rs` - WebSocket handler

3. **Create Sandbox Module**:
   - `src/sandbox/xvfb.rs` - Virtual display
   - `src/sandbox/ffmpeg.rs` - Video encoding

4. **Frontend Setup**:
   ```bash
   cd frontend/web
   npm install
   ```

5. **Create Components**:
   - `src/pages/VideoSessionPage.tsx`
   - `src/components/VideoPlayer.tsx`
   - `src/services/webrtc.ts`

### After POC

1. **Add Authentication** (WebAuthn)
2. **Implement Permissions** (Database + RBAC)
3. **Add Input Forwarding** (Mouse/keyboard)
4. **Complete Sandbox Isolation** (Landlock + seccomp)
5. **Production Deployment** (Docker Compose)

---

## Resources

### WebRTC Documentation
- [MDN WebRTC API](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API)
- [webrtc-rs crate docs](https://docs.rs/webrtc/latest/webrtc/)

### FFmpeg
- [FFmpeg x11grab](https://trac.ffmpeg.org/wiki/Capture/Desktop)
- [H.264 encoding guide](https://trac.ffmpeg.org/wiki/Encode/H.264)

### Rust Async
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
- [Axum examples](https://github.com/tokio-rs/axum/tree/main/examples)

---

## Summary

This POC will demonstrate the **core technical feasibility** of secure file viewing via WebRTC:

✅ **Video streaming works** - Client sees live video of sandbox  
✅ **WebRTC is viable** - Low latency, encrypted by default  
✅ **Architecture is sound** - Clear separation of concerns  
✅ **Foundation for production** - Can build full system on this  

**Estimated POC Timeline**: 2-3 days for experienced Rust/React developer

**Critical Success Factors**:
1. WebSocket signaling works reliably
2. FFmpeg outputs compatible H.264 stream
3. Browser receives and plays video smoothly
4. Connection is stable and recovers from drops

Good luck with the implementation! 🚀
