import React, { useState } from 'react'
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

export const VideoSessionPage: React.FC = () => {
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [websocketUrl, setWebsocketUrl] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [connectionState, setConnectionState] = useState<string>('disconnected')

  const webrtcService = new WebRTCService()

  const handleStartSession = async () => {
    setLoading(true)
    setError(null)

    try {
      // Get window size (or use a reasonable default)
      const width = Math.min(window.innerWidth - 100, 1920)
      const height = Math.min(window.innerHeight - 200, 1080)
      const finalWidth = Math.max(640, width)
      const finalHeight = Math.max(480, height)
      
      console.log(`Window size: ${window.innerWidth}x${window.innerHeight}`)
      console.log(`Requesting resolution: ${finalWidth}x${finalHeight}`)
      
      const response = await webrtcService.createSession({
        width: finalWidth,
        height: finalHeight,
        framerate: 30
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
    <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
      <Paper sx={{ p: 3 }}>
        <Typography variant="h4" gutterBottom>
          WebRTC Video Session POC
        </Typography>
        <Typography variant="body2" color="text.secondary" paragraph>
          Test the WebRTC video streaming functionality
        </Typography>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
            {error}
          </Alert>
        )}

        <Box sx={{ mb: 3, display: 'flex', gap: 2, alignItems: 'center' }}>
          {!sessionId ? (
            <Button
              variant="contained"
              color="primary"
              startIcon={loading ? <CircularProgress size={20} /> : <VideoCallIcon />}
              onClick={handleStartSession}
              disabled={loading}
              size="large"
            >
              {loading ? 'Starting...' : 'Start Video Session'}
            </Button>
          ) : (
            <Button
              variant="contained"
              color="error"
              startIcon={<StopCircleIcon />}
              onClick={handleStopSession}
              disabled={loading}
              size="large"
            >
              Stop Session
            </Button>
          )}

          {sessionId && (
            <Card variant="outlined" sx={{ flex: 1 }}>
              <CardContent>
                <Typography variant="caption" color="text.secondary">
                  Session ID
                </Typography>
                <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
                  {sessionId}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                  Connection: {connectionState}
                </Typography>
              </CardContent>
            </Card>
          )}
        </Box>

        {websocketUrl && (
          <Paper elevation={3} sx={{ p: 0, overflow: 'hidden' }}>
            <VideoPlayer
              websocketUrl={websocketUrl}
              onConnectionStateChange={(state) => setConnectionState(state)}
              onError={(err) => setError(err)}
            />
          </Paper>
        )}

        {!sessionId && !loading && (
          <Box sx={{ textAlign: 'center', py: 8 }}>
            <VideoCallIcon sx={{ fontSize: 64, color: 'text.secondary', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No active session
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Click "Start Video Session" to begin
            </Typography>
          </Box>
        )}
      </Paper>
    </Container>
  )
}
