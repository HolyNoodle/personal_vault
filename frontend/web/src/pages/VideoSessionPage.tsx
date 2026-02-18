import React, { useState, useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import {
  Container,
  Paper,
  Typography,
  Button,
  Box,
  Alert,
  CircularProgress,
  Card,
  CardContent
} from '@mui/material'
import VideoCallIcon from '@mui/icons-material/VideoCall'
import StopCircleIcon from '@mui/icons-material/StopCircle'
import { VideoPlayer } from '../components/VideoPlayer'
import { WebRTCService } from '../services/webrtc'
import { useAuthStore } from '../store/authStore'

export const VideoSessionPage: React.FC = () => {
  const [searchParams] = useSearchParams()
  const launchedSessionId = searchParams.get('sessionId')
  
  const [sessionId, setSessionId] = useState<string | null>(launchedSessionId)
  const [websocketUrl, setWebsocketUrl] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [connectionState, setConnectionState] = useState<string>('disconnected')
  const { user } = useAuthStore()

  const webrtcService = new WebRTCService()

  // If we have a session ID from the launch page, set up the WebSocket URL
  useEffect(() => {
    if (launchedSessionId && !websocketUrl) {
      // For launched applications, we use the WebSocket URL directly
      // The WebRTC offer is already handled by the backend
      setWebsocketUrl(`ws://localhost:8080/ws?session=${launchedSessionId}`)
    }
  }, [launchedSessionId, websocketUrl])

  const handleStartSession = async () => {
    if (!user?.id) {
      setError('User not authenticated')
      return
    }

    setLoading(true)
    setError(null)

    try {
      // Calculate optimal resolution based on viewport
      const width = Math.min(window.innerWidth - 100, 1920)
      const height = Math.min(window.innerHeight - 200, 1080)
      const finalWidth = Math.max(640, width)
      const finalHeight = Math.max(480, height)
      
      const response = await webrtcService.createSession({
        user_id: user.id,
        width: finalWidth,
        height: finalHeight,
        framerate: 60,
        application: 'xterm',
      })

      console.log('Session created:', response)
      setSessionId(response.session_id)
      setWebsocketUrl(response.websocket_url)
    } catch (err) {
      console.error('Failed to create session:', err)
      setError(err instanceof Error ? err.message : 'Failed to create session')
    } finally {
      setLoading(false)
    }
  }

  const handleStopSession = async () => {
    if (!sessionId) return

    setLoading(true)
    try {
      await webrtcService.terminateSession(sessionId)
      setSessionId(null)
      setWebsocketUrl(null)
      setConnectionState('disconnected')
    } catch (err) {
      console.error('Failed to terminate session:', err)
      setError(err instanceof Error ? err.message : 'Failed to terminate session')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Box sx={{ 
      width: '100%',
      height: '100%', 
      display: 'flex', 
      flexDirection: 'column',
      bgcolor: '#000',
      overflow: 'hidden',
      position: 'absolute',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0
    }}>
      {/* Error display */}
      {error && (
        <Alert 
          severity="error" 
          onClose={() => setError(null)}
          sx={{ position: 'absolute', top: 8, left: 8, right: 8, zIndex: 1000 }}
        >
          {error}
        </Alert>
      )}

      {/* Full-height video container */}
      <Box sx={{ flex: 1, display: 'flex', overflow: 'hidden', bgcolor: '#000' }}>
        {websocketUrl ? (
          <VideoPlayer
            websocketUrl={websocketUrl}
            onConnectionStateChange={(state) => setConnectionState(state)}
            onError={(err) => setError(err)}
          />
        ) : (
          <Box sx={{ 
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center', 
            justifyContent: 'center',
            bgcolor: 'background.paper'
          }}>
            <VideoCallIcon sx={{ fontSize: 64, color: 'text.secondary', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No active session
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Click "Start Session" to begin
            </Typography>
          </Box>
        )}
      </Box>
    </Box>
  )
}
