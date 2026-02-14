# Frontend Architecture

**Purpose**: Web-based user interface for secure file sharing and sandboxed viewing.

**Technology Stack**: Modern JavaScript framework (React/Vue/Svelte) + TypeScript

**Layer**: Presentation/UI Layer

---

## Technology Recommendations

### Option 1: React + TypeScript (Recommended)
**Pros**:
- Largest ecosystem and community
- Excellent TypeScript support
- Rich component libraries (Material-UI, Ant Design, Chakra UI)
- Strong file upload libraries (react-dropzone, uppy)
- WebRTC integration well-documented

**Cons**:
- Larger bundle size
- More boilerplate

### Option 2: Vue 3 + TypeScript
**Pros**:
- Gentler learning curve
- Smaller bundle size
- Great file upload support (vue-upload-component)
- Good TypeScript support

**Cons**:
- Smaller ecosystem than React
- Fewer enterprise-grade component libraries

### Option 3: Svelte + TypeScript
**Pros**:
- Smallest bundle size (compiled)
- Most performant
- Minimal boilerplate
- Built-in reactivity

**Cons**:
- Smallest ecosystem
- Fewer component libraries
- Less mature for enterprise apps

**Recommendation**: **React + TypeScript** for enterprise-grade stability and ecosystem.

---

## Application Structure

```
frontend/web/
├── src/
│   ├── components/
│   │   ├── auth/                 # Authentication components
│   │   │   ├── WebAuthnLogin.tsx
│   │   │   ├── WebAuthnRegister.tsx
│   │   │   └── CredentialList.tsx
│   │   │
│   │   ├── files/                # File management components
│   │   │   ├── FileExplorer.tsx           # Main file browser
│   │   │   ├── FileList.tsx               # File/folder list view
│   │   │   ├── FileUpload.tsx             # Upload component (drag-drop)
│   │   │   ├── FilePreview.tsx            # Preview modal
│   │   │   ├── FolderBreadcrumb.tsx       # Navigation breadcrumb
│   │   │   ├── FolderTree.tsx             # Sidebar folder tree
│   │   │   ├── StorageQuota.tsx           # Storage usage widget
│   │   │   └── DownloadProgress.tsx       # Download progress bar
│   │   │
│   │   ├── permissions/          # Permission management
│   │   │   ├── PermissionList.tsx
│   │   │   ├── GrantPermissionModal.tsx
│   │   │   ├── PermissionDetails.tsx
│   │   │   └── AccessRequestCard.tsx
│   │   │
│   │   ├── sessions/             # Session management
│   │   │   ├── ActiveSessionsList.tsx
│   │   │   ├── SessionViewer.tsx          # Sandboxed file viewer
│   │   │   ├── SessionControls.tsx        # Start/stop session
│   │   │   └── SessionTimerWidget.tsx     # Remaining time display
│   │   │
│   │   ├── invitations/          # Invitations
│   │   │   ├── InvitationList.tsx
│   │   │   ├── CreateInvitationModal.tsx
│   │   │   └── AcceptInvitation.tsx
│   │   │
│   │   ├── admin/                # SuperAdmin components
│   │   │   ├── UserManagement.tsx
│   │   │   ├── SystemStats.tsx
│   │   │   ├── AuditLogViewer.tsx
│   │   │   └── ActiveSessionsMonitor.tsx
│   │   │
│   │   └── common/               # Shared components
│   │       ├── Layout.tsx
│   │       ├── Navbar.tsx
│   │       ├── Sidebar.tsx
│   │       ├── Notification.tsx
│   │       └── LoadingSpinner.tsx
│   │
│   ├── services/                 # API integration
│   │   ├── api.ts                # Axios/fetch wrapper
│   │   ├── auth.ts               # WebAuthn client
│   │   ├── files.ts              # File API calls
│   │   ├── permissions.ts        # Permission API calls
│   │   ├── sessions.ts           # Session API calls
│   │   └── webrtc.ts             # WebRTC client
│   │
│   ├── stores/                   # State management (Zustand/Redux)
│   │   ├── authStore.ts
│   │   ├── fileStore.ts
│   │   └── sessionStore.ts
│   │
│   ├── hooks/                    # Custom React hooks
│   │   ├── useWebAuthn.ts
│   │   ├── useFileUpload.ts
│   │   ├── useSessionTimer.ts
│   │   └── usePolling.ts
│   │
│   ├── types/                    # TypeScript definitions
│   │   ├── api.ts
│   │   ├── domain.ts
│   │   └── components.ts
│   │
│   ├── utils/                    # Utility functions
│   │   ├── formatFileSize.ts
│   │   ├── formatDate.ts
│   │   ├── validateFile.ts
│   │   └── checksum.ts
│   │
│   ├── App.tsx                   # Root component
│   ├── main.tsx                  # Entry point
│   └── router.tsx                # React Router
│
├── public/
│   ├── index.html
│   └── assets/
│
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

---

## File Explorer Component

### Core Features

#### 1. File List View
```tsx
interface FileListProps {
  files: File[];
  folders: Folder[];
  currentFolder: string | null;
  onFileClick: (file: File) => void;
  onFolderClick: (folder: Folder) => void;
  onFileDelete: (fileId: string) => void;
  onFileDownload: (fileId: string) => void;
  onFileRename: (fileId: string, newName: string) => void;
  viewMode: 'list' | 'grid';
}

const FileList: React.FC<FileListProps> = ({ files, folders, ... }) => {
  return (
    <div className="file-list">
      {/* Folder entries */}
      {folders.map(folder => (
        <FolderItem 
          key={folder.id}
          folder={folder}
          onClick={() => onFolderClick(folder)}
        />
      ))}
      
      {/* File entries */}
      {files.map(file => (
        <FileItem
          key={file.id}
          file={file}
          onClick={() => onFileClick(file)}
          onDelete={() => onFileDelete(file.id)}
          onDownload={() => onFileDownload(file.id)}
          onRename={(newName) => onFileRename(file.id, newName)}
        />
      ))}
    </div>
  );
};
```

#### 2. File Upload (Drag & Drop)
```tsx
import { useDropzone } from 'react-dropzone';

const FileUpload: React.FC<{ folderId: string | null }> = ({ folderId }) => {
  const onDrop = useCallback(async (acceptedFiles: File[]) => {
    for (const file of acceptedFiles) {
      // Calculate SHA-256 checksum client-side
      const checksum = await calculateChecksum(file);
      
      // Create FormData
      const formData = new FormData();
      formData.append('file', file);
      formData.append('parent_folder_id', folderId || '');
      formData.append('checksum_sha256', checksum);
      
      // Upload with progress tracking
      await uploadFile(formData, (progress) => {
        setUploadProgress(file.name, progress);
      });
    }
  }, [folderId]);
  
  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    maxSize: 10 * 1024 * 1024 * 1024, // 10GB
    multiple: true,
  });
  
  return (
    <div {...getRootProps()} className={`dropzone ${isDragActive ? 'active' : ''}`}>
      <input {...getInputProps()} />
      <p>Drag files here or click to browse</p>
    </div>
  );
};
```

#### 3. Folder Navigation
```tsx
const FolderBreadcrumb: React.FC<{ path: Folder[] }> = ({ path }) => {
  return (
    <nav className="breadcrumb">
      <Link to="/files">Home</Link>
      {path.map((folder, index) => (
        <React.Fragment key={folder.id}>
          <span className="separator">/</span>
          <Link to={`/files/${folder.id}`}>
            {folder.name}
          </Link>
        </React.Fragment>
      ))}
    </nav>
  );
};
```

#### 4. File Actions Menu
```tsx
const FileContextMenu: React.FC<{ file: File }> = ({ file }) => {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  
  return (
    <>
      <IconButton onClick={(e) => setAnchorEl(e.currentTarget)}>
        <MoreVertIcon />
      </IconButton>
      
      <Menu anchorEl={anchorEl} open={Boolean(anchorEl)}>
        <MenuItem onClick={() => handleDownload(file)}>
          <DownloadIcon /> Download
        </MenuItem>
        <MenuItem onClick={() => handleRename(file)}>
          <EditIcon /> Rename
        </MenuItem>
        <MenuItem onClick={() => handleMove(file)}>
          <FolderMoveIcon /> Move
        </MenuItem>
        <MenuItem onClick={() => handleGrantPermission(file)}>
          <ShareIcon /> Grant Permission
        </MenuItem>
        <MenuItem onClick={() => handleViewSessions(file)}>
          <VisibilityIcon /> View Active Sessions
        </MenuItem>
        <Divider />
        <MenuItem onClick={() => handleDelete(file)} className="danger">
          <DeleteIcon /> Delete
        </MenuItem>
      </Menu>
    </>
  );
};
```

#### 5. Storage Quota Widget
```tsx
const StorageQuota: React.FC<{ quota: number; used: number }> = ({ quota, used }) => {
  const percentage = (used / quota) * 100;
  const available = quota - used;
  
  return (
    <div className="storage-quota">
      <div className="quota-header">
        <span>Storage</span>
        <span>{formatFileSize(used)} / {formatFileSize(quota)}</span>
      </div>
      
      <div className="quota-bar">
        <div 
          className={`quota-fill ${percentage > 90 ? 'danger' : percentage > 75 ? 'warning' : ''}`}
          style={{ width: `${percentage}%` }}
        />
      </div>
      
      <div className="quota-footer">
        <span>{formatFileSize(available)} available</span>
      </div>
    </div>
  );
};
```

#### 6. File Preview Modal
```tsx
const FilePreview: React.FC<{ file: File; onClose: () => void }> = ({ file, onClose }) => {
  const canPreview = ['image/', 'application/pdf', 'text/'].some(type => 
    file.content_type.startsWith(type)
  );
  
  return (
    <Modal open onClose={onClose}>
      <div className="file-preview">
        <div className="preview-header">
          <h2>{file.file_name}</h2>
          <IconButton onClick={onClose}><CloseIcon /></IconButton>
        </div>
        
        <div className="preview-body">
          {canPreview ? (
            file.content_type.startsWith('image/') ? (
              <img src={`/api/owner/files/${file.id}/download`} alt={file.file_name} />
            ) : file.content_type === 'application/pdf' ? (
              <iframe src={`/api/owner/files/${file.id}/download`} />
            ) : (
              <pre>{/* Load text content */}</pre>
            )
          ) : (
            <div className="no-preview">
              <InfoIcon />
              <p>Preview not available for this file type</p>
              <Button onClick={() => handleDownload(file)}>Download</Button>
            </div>
          )}
        </div>
        
        <div className="preview-footer">
          <Button onClick={() => handleDownload(file)}>Download</Button>
          <Button onClick={() => handleGrantPermission(file)}>Share</Button>
        </div>
      </div>
    </Modal>
  );
};
```

---

## Session Viewer (Sandboxed File Viewing)

### WebRTC-Based Viewer

```tsx
const SessionViewer: React.FC<{ sessionId: string; fileId: string }> = ({ sessionId, fileId }) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [peerConnection, setPeerConnection] = useState<RTCPeerConnection | null>(null);
  const [sessionInfo, setSessionInfo] = useState<Session | null>(null);
  const [remainingTime, setRemainingTime] = useState<number>(0);
  
  useEffect(() => {
    // Initialize WebRTC connection
    const initWebRTC = async () => {
      const pc = new RTCPeerConnection({
        iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
      });
      
      // Handle incoming video stream
      pc.ontrack = (event) => {
        if (videoRef.current) {
          videoRef.current.srcObject = event.streams[0];
        }
      };
      
      // Get SDP offer from backend
      const response = await fetch(`/api/client/sessions/${sessionId}/webrtc-offer`);
      const { sdp_offer } = await response.json();
      
      await pc.setRemoteDescription(new RTCSessionDescription(sdp_offer));
      
      // Create answer
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      
      // Send answer to backend
      await fetch(`/api/client/sessions/${sessionId}/webrtc-answer`, {
        method: 'POST',
        body: JSON.stringify({ sdp_answer: answer }),
      });
      
      setPeerConnection(pc);
    };
    
    initWebRTC();
    
    // Cleanup on unmount
    return () => {
      peerConnection?.close();
    };
  }, [sessionId]);
  
  // Update remaining time every second
  useEffect(() => {
    const timer = setInterval(() => {
      if (sessionInfo) {
        const remaining = Math.max(0, 
          (new Date(sessionInfo.expires_at).getTime() - Date.now()) / 1000
        );
        setRemainingTime(remaining);
        
        if (remaining === 0) {
          handleSessionExpired();
        }
      }
    }, 1000);
    
    return () => clearInterval(timer);
  }, [sessionInfo]);
  
  return (
    <div className="session-viewer">
      <div className="viewer-header">
        <h2>{sessionInfo?.file_name}</h2>
        <div className="session-info">
          <Chip label={`${Math.floor(remainingTime / 60)}:${String(Math.floor(remainingTime % 60)).padStart(2, '0')} remaining`} />
          <Chip label="Read-only" color="primary" />
        </div>
        <Button onClick={handleTerminateSession} color="error">
          End Session
        </Button>
      </div>
      
      <div className="viewer-body">
        <video 
          ref={videoRef} 
          autoPlay 
          playsInline 
          className="webrtc-video"
        />
      </div>
      
      <div className="viewer-footer">
        <p className="security-notice">
          <SecurityIcon /> 
          This file is displayed in a secure sandbox. No data leaves the server.
        </p>
      </div>
    </div>
  );
};
```

---

## Polling for Updates

Since the application doesn't use WebSocket, updates are fetched via polling:

```tsx
const usePolling = <T,>(
  fetchFn: () => Promise<T>,
  interval: number = 5000, // 5 seconds
  enabled: boolean = true
) => {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<Error | null>(null);
  
  useEffect(() => {
    if (!enabled) return;
    
    const poll = async () => {
      try {
        const result = await fetchFn();
        setData(result);
        setError(null);
      } catch (err) {
        setError(err as Error);
      }
    };
    
    // Initial fetch
    poll();
    
    // Set up polling
    const intervalId = setInterval(poll, interval);
    
    return () => clearInterval(intervalId);
  }, [fetchFn, interval, enabled]);
  
  return { data, error };
};

// Usage example
const ActiveSessionsList: React.FC = () => {
  const { data: sessions } = usePolling(
    () => fetch('/api/owner/sessions/active').then(r => r.json()),
    5000 // Poll every 5 seconds
  );
  
  return (
    <div>
      {sessions?.map(session => (
        <SessionCard key={session.id} session={session} />
      ))}
    </div>
  );
};
```

---

## WebAuthn Integration

```tsx
const useWebAuthn = () => {
  const login = async (email: string) => {
    // 1. Initiate authentication
    const { challenge_id, publicKey } = await fetch('/api/auth/webauthn/login/initiate', {
      method: 'POST',
      body: JSON.stringify({ email }),
    }).then(r => r.json());
    
    // 2. Get credential from authenticator
    const credential = await navigator.credentials.get({ publicKey });
    
    // 3. Complete authentication
    const { token, user } = await fetch('/api/auth/webauthn/login/complete', {
      method: 'POST',
      body: JSON.stringify({
        challenge_id,
        credential: {
          id: credential.id,
          rawId: arrayBufferToBase64(credential.rawId),
          response: {
            authenticatorData: arrayBufferToBase64(credential.response.authenticatorData),
            clientDataJSON: arrayBufferToBase64(credential.response.clientDataJSON),
            signature: arrayBufferToBase64(credential.response.signature),
            userHandle: arrayBufferToBase64(credential.response.userHandle),
          },
          type: credential.type,
        },
      }),
    }).then(r => r.json());
    
    // 4. Store JWT token
    localStorage.setItem('jwt_token', token);
    
    return user;
  };
  
  const register = async (credentialName: string) => {
    // Similar flow for registration
    // ...
  };
  
  return { login, register };
};
```

---

## Technology Stack

```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.20.0",
    "typescript": "^5.3.0",
    
    "@mui/material": "^5.15.0",
    "@mui/icons-material": "^5.15.0",
    "@emotion/react": "^11.11.0",
    "@emotion/styled": "^11.11.0",
    
    "axios": "^1.6.0",
    "zustand": "^4.4.0",
    "react-dropzone": "^14.2.0",
    "react-query": "^3.39.0",
    
    "@simplewebauthn/browser": "^9.0.0",
    
    "date-fns": "^3.0.0",
    "clsx": "^2.1.0"
  },
  "devDependencies": {
    "vite": "^5.0.0",
    "@vitejs/plugin-react": "^4.2.0",
    "eslint": "^8.56.0",
    "prettier": "^3.1.0"
  }
}
```

---

**Document Version**: 1.0  
**Last Updated**: 2026-02-14  
**Related**: [../../backend/docs/adapters/http.md](../../backend/docs/adapters/http.md), [../../backend/docs/adapters/webauthn.md](../../backend/docs/adapters/webauthn.md)
