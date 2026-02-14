# WebRTC POC - Input Mapping & Video Streaming

## Overview

This POC demonstrates WebRTC video streaming with mouse/keyboard input forwarding from browser to server.

**Key Features:**
- ✅ WebRTC peer connection (no STUN/TURN for local testing)
- ✅ Video track: Test pattern stream (can replace with Xvfb capture)
- ✅ Data channel: Bidirectional input events (mouse, keyboard)
- ✅ Input validation: Coordinate clamping, key filtering, rate limiting
- ✅ Minimal dependencies: Pure webrtc-rs, no external services

## Architecture

```
┌──────────────────┐                          ┌──────────────────┐
│   Browser        │                          │   Rust Server    │
│                  │                          │                  │
│  ┌────────────┐  │  1. HTTP GET /          │  ┌────────────┐  │
│  │ HTML/JS    │──┼─────────────────────────┼─→│ Axum HTTP  │  │
│  │ Client     │  │  2. Return HTML+JS      │  │ Server     │  │
│  └────────────┘  │←────────────────────────┼──│            │  │
│                  │                          │  └────────────┘  │
│  ┌────────────┐  │  3. POST /offer         │  ┌────────────┐  │
│  │ WebRTC     │──┼─────────────────────────┼─→│ WebRTC     │  │
│  │ Peer       │  │     (SDP Offer)         │  │ Peer       │  │
│  │            │  │  4. SDP Answer          │  │            │  │
│  │            │←─┼─────────────────────────┼──│            │  │
│  └────────────┘  │                          │  └────────────┘  │
│        │         │  5. ICE Candidates      │         │         │
│        └─────────┼─────────────────────────┼─────────┘         │
│                  │                          │                  │
│  ┌────────────┐  │  6. Video RTP ────────→ │  ┌────────────┐  │
│  │ <video>    │←─┼─────────────────────────┼──│ Test Video │  │
│  │ element    │  │                          │  │ Source     │  │
│  └────────────┘  │                          │  └────────────┘  │
│                  │                          │                  │
│  ┌────────────┐  │  7. Data Channel        │  ┌────────────┐  │
│  │ Input      │──┼─────────────────────────┼─→│ Input      │  │
│  │ Capture    │  │  {type:"mouse",x,y}     │  │ Handler    │  │
│  │ (mouse/kbd)│  │  {type:"key",code}      │  │ (validate) │  │
│  └────────────┘  │                          │  └────────────┘  │
└──────────────────┘                          └──────────────────┘
```

## Running the POC

### Prerequisites

```bash
# Rust nightly (required for webrtc-rs)
rustup default nightly

# Install dependencies (if needed for video capture)
# sudo apt install xvfb ffmpeg  # Not needed for test pattern
```

### Build and Run

```bash
cd backend/examples/webrtc_poc

# Build
cargo build --release

# Run server
cargo run --release

# Open browser
# Navigate to: http://localhost:3030
```

### Expected Output

**Server Console:**
```
WebRTC POC Server starting on http://0.0.0.0:3030
Received SDP offer from client
Created SDP answer
WebRTC peer connection established
Data channel opened: input
Received input event: Mouse { x: 512, y: 384, button: Left, action: Click }
Received input event: Keyboard { key: "KeyA", action: Press }
```

**Browser:**
- Video stream displays (test pattern or black screen with timestamp)
- Mouse clicks are logged in browser console
- Keyboard presses are logged in browser console
- Server echoes events back via data channel

## Input Mapping Details

### Mouse Events

**Browser Capture:**
```javascript
videoElement.addEventListener('mousemove', (e) => {
  const rect = videoElement.getBoundingClientRect();
  const x = Math.round((e.clientX - rect.left) / rect.width * VIDEO_WIDTH);
  const y = Math.round((e.clientY - rect.top) / rect.height * VIDEO_HEIGHT);
  
  sendInput({ type: 'mouse', x, y, button: null, action: 'move' });
});
```

**Server Validation:**
```rust
fn validate_mouse_event(event: &MouseEvent, max_width: u16, max_height: u16) -> Result<()> {
    if event.x >= max_width || event.y >= max_height {
        return Err(Error::InvalidCoordinates);
    }
    Ok(())
}
```

### Keyboard Events

**Browser Capture:**
```javascript
document.addEventListener('keydown', (e) => {
  if (ALLOWED_KEYS.includes(e.code)) {
    sendInput({ type: 'keyboard', code: e.code, action: 'press' });
    e.preventDefault();
  }
});
```

**Server Validation:**
```rust
const ALLOWED_KEYS: &[&str] = &[
    "KeyA", "KeyB", ..., "KeyZ",
    "Digit0", ..., "Digit9",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
    // Blocked: F1-F12, Alt+F4, Ctrl+Alt+Del, etc.
];

fn validate_keyboard_event(event: &KeyboardEvent) -> Result<()> {
    if !ALLOWED_KEYS.contains(&event.code.as_str()) {
        return Err(Error::DisallowedKey);
    }
    Ok(())
}
```

### Rate Limiting

**Implementation:**
```rust
use std::time::{Duration, Instant};

struct RateLimiter {
    events: VecDeque<Instant>,
    max_events: usize,
    window: Duration,
}

impl RateLimiter {
    fn check(&mut self) -> Result<()> {
        let now = Instant::now();
        // Remove events outside window
        self.events.retain(|t| now.duration_since(*t) < self.window);
        
        if self.events.len() >= self.max_events {
            return Err(Error::RateLimitExceeded);
        }
        
        self.events.push_back(now);
        Ok(())
    }
}

// Usage: max 100 events per second
let mut limiter = RateLimiter::new(100, Duration::from_secs(1));
```

## Testing Scenarios

### 1. Basic Connectivity
- [ ] Server starts successfully
- [ ] Browser loads HTML page
- [ ] WebRTC connection establishes
- [ ] Video stream displays

### 2. Mouse Input
- [ ] Mouse move events captured with correct coordinates
- [ ] Mouse clicks (left/right/middle) detected
- [ ] Coordinates mapped correctly from canvas to video resolution
- [ ] Out-of-bounds coordinates rejected by server

### 3. Keyboard Input
- [ ] Alphanumeric keys captured
- [ ] Arrow keys captured
- [ ] Function keys (F1-F12) blocked
- [ ] System key combos (Alt+F4, Ctrl+Alt+Del) blocked

### 4. Rate Limiting
- [ ] Normal input (30 events/sec) accepted
- [ ] Burst input (150 events/sec) throttled
- [ ] Rate limiter resets after 1 second

### 5. Error Handling
- [ ] Invalid JSON rejected
- [ ] Unknown event types logged and ignored
- [ ] Malformed coordinates clamped or rejected
- [ ] Connection loss detected and cleaned up

## Next Steps (Integration)

Once POC is validated:

1. **Replace test video with Xvfb capture**
   - Use `ffmpeg` crate or `gstreamer-rs`
   - Capture from Xvfb :100 display
   - H.264 encode and send via RTP

2. **Inject input into sandboxed X11 session**
   - Use `uinput` kernel module
   - Create virtual mouse/keyboard devices
   - Inject events into namespace

3. **Add authentication**
   - Require JWT token in WebRTC offer
   - Validate session ownership
   - Enforce Landlock policies

4. **Production WebRTC config**
   - Add STUN/TURN servers for NAT traversal
   - Enable DTLS-SRTP encryption
   - Implement ICE candidate gathering

5. **Metrics and monitoring**
   - Log input event counts
   - Track latency (browser → server → X11)
   - Monitor bitrate and frame drops

## Known Limitations

- **Local only**: No STUN/TURN, works on localhost/LAN only
- **Test video**: Static pattern, not real Xvfb capture
- **No persistence**: Session state lost on server restart
- **Single peer**: Server handles one client at a time (can be extended)

## Dependencies

```toml
[dependencies]
webrtc = "0.11"  # Pure Rust WebRTC
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Files

- `Cargo.toml` - Dependencies and build config
- `src/main.rs` - WebRTC server implementation
- `client.html` - Browser client with input mapping
- `POC_RESULTS.md` - Test results and findings (created after testing)

## References

- [webrtc-rs documentation](https://github.com/webrtc-rs/webrtc)
- [WebRTC Signaling](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Signaling_and_video_calling)
- [DataChannel API](https://developer.mozilla.org/en-US/docs/Web/API/RTCDataChannel)
- [uinput kernel module](https://www.kernel.org/doc/html/latest/input/uinput.html)
