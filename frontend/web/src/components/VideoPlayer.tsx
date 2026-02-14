import React, { useEffect, useRef, useState } from 'react'
import { Box, Typography, CircularProgress, Alert } from '@mui/material'

export interface SignalingMessage {
  type: string
  sdp?: string
  candidate?: string
}

interface VideoPlayerProps {
  websocketUrl: string
  onConnectionStateChange?: (state: RTCPeerConnectionState) => void
  onError?: (error: string) => void
}

export const VideoPlayer: React.FC<VideoPlayerProps> = ({
  websocketUrl,
  onConnectionStateChange,
  onError
}) => {
  const videoRef = useRef<HTMLVideoElement>(null)
  const pcRef = useRef<RTCPeerConnection | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const mountedRef = useRef(true)
  const connectionInitializedRef = useRef(false)
  const [connectionState, setConnectionState] = useState<string>('new')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    mountedRef.current = true
    
    // Prevent multiple initializations
    if (connectionInitializedRef.current) {
      return
    }
    connectionInitializedRef.current = true

    const setupConnection = async () => {
      try {
        // Create WebSocket connection
        const websocket = new WebSocket(websocketUrl)
        wsRef.current = websocket

        // Create RTCPeerConnection
        const peerConnection = new RTCPeerConnection({
          iceServers: [
            { urls: 'stun:stun.l.google.com:19302' }
          ]
        })
        pcRef.current = peerConnection

        // Handle incoming video track
        peerConnection.ontrack = (event) => {
          console.log('✅ Received video track:', event)
          console.log('Track details:', {
            kind: event.track.kind,
            id: event.track.id,
            label: event.track.label,
            enabled: event.track.enabled,
            muted: event.track.muted,
            readyState: event.track.readyState
          })
          console.log('Stream details:', event.streams[0])
          
          if (videoRef.current && mountedRef.current) {
            videoRef.current.srcObject = event.streams[0]
            console.log('✅ Set srcObject to video element')
            
            // Log video element state
            setTimeout(() => {
              if (videoRef.current) {
                console.log('Video element state:', {
                  readyState: videoRef.current.readyState,
                  networkState: videoRef.current.networkState,
                  videoWidth: videoRef.current.videoWidth,
                  videoHeight: videoRef.current.videoHeight,
                  paused: videoRef.current.paused,
                  duration: videoRef.current.duration
                })
              }
            }, 1000)
          }
        }

        // Handle ICE candidates
        peerConnection.onicecandidate = (event) => {
          if (event.candidate && websocket.readyState === WebSocket.OPEN) {
            websocket.send(JSON.stringify({
              type: 'ice-candidate',
              candidate: event.candidate.candidate
            }))
          }
        }

        // Monitor connection state
        peerConnection.onconnectionstatechange = () => {
          const state = peerConnection.connectionState
          console.log('Connection state:', state)
          if (mountedRef.current) {
            setConnectionState(state)
            onConnectionStateChange?.(state)
            
            if (state === 'failed') {
              setError('WebRTC connection failed')
              onError?.('WebRTC connection failed')
            }
          }
        }

        // Handle signaling messages
        websocket.onmessage = async (event) => {
          try {
            const message: SignalingMessage = JSON.parse(event.data)
            console.log('Received signaling message:', message.type)

            switch (message.type) {
              case 'offer':
                // Server sends offer, client responds with answer
                if (message.sdp) {
                  await peerConnection.setRemoteDescription(
                    new RTCSessionDescription({ type: 'offer', sdp: message.sdp })
                  )
                  const answer = await peerConnection.createAnswer()
                  await peerConnection.setLocalDescription(answer)
                  websocket.send(JSON.stringify({
                    type: 'answer',
                    sdp: answer.sdp
                  }))
                  console.log('Sent answer to server')
                }
                break

              case 'ice-candidate':
                // Add server's ICE candidate
                if (message.candidate) {
                  await peerConnection.addIceCandidate(
                    new RTCIceCandidate({ candidate: message.candidate })
                  )
                }
                break

              case 'error':
                console.error('Signaling error:', message)
                if (mountedRef.current) {
                  setError('Signaling error')
                  onError?.('Signaling error')
                }
                break
            }
          } catch (err) {
            console.error('Error handling signaling message:', err)
            if (mountedRef.current) {
              setError('Error processing signaling message')
              onError?.('Error processing signaling message')
            }
          }
        }

        // Request offer from server
        websocket.onopen = () => {
          console.log('WebSocket connected, requesting offer...')
          websocket.send(JSON.stringify({ type: 'request-offer' }))
        }

        websocket.onerror = (err) => {
          console.error('WebSocket error:', err)
          if (mountedRef.current) {
            setError('WebSocket connection error')
            onError?.('WebSocket connection error')
          }
        }

        websocket.onclose = () => {
          console.log('WebSocket closed')
          if (mountedRef.current) {
            setConnectionState('disconnected')
          }
        }

      } catch (err) {
        console.error('Error setting up connection:', err)
        if (mountedRef.current) {
          setError('Failed to initialize connection')
          onError?.('Failed to initialize connection')
        }
      }
    }

    setupConnection()

    // Cleanup
    return () => {
      mountedRef.current = false
      connectionInitializedRef.current = false
      if (pcRef.current) {
        pcRef.current.close()
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [websocketUrl]) // Only re-run if websocketUrl changes

  return (
    <Box sx={{ position: 'relative', width: '100%', height: '100%', minHeight: 400 }}>
      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}
      
      <video
        ref={videoRef}
        autoPlay
        playsInline
        onLoadedMetadata={(e) => console.log('✅ Video metadata loaded', { 
          width: e.currentTarget.videoWidth, 
          height: e.currentTarget.videoHeight,
          duration: e.currentTarget.duration
        })}
        onLoadedData={() => console.log('✅ Video data loaded')}
        onCanPlay={() => console.log('✅ Video can play')}
        onPlaying={() => console.log('✅ Video is playing')}
        onError={(e) => console.error('❌ Video error:', e.currentTarget.error)}
        onStalled={() => console.warn('⚠️ Video stalled')}
        onWaiting={() => console.warn('⚠️ Video waiting')}
        style={{
          width: '100%',
          height: '100%',
          backgroundColor: '#000',
          borderRadius: 4
        }}
      />
      
      {connectionState !== 'connected' && !error && (
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
            backgroundColor: 'rgba(0,0,0,0.7)',
            borderRadius: 1
          }}
        >
          <CircularProgress sx={{ mb: 2 }} />
          <Typography sx={{ color: 'white' }}>
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
