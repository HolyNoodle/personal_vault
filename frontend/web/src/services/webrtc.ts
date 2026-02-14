// WebRTC service for handling video sessions
export interface SessionConfig {
  width?: number
  height?: number
  framerate?: number
}

export interface CreateSessionResponse {
  session_id: string
  websocket_url: string
}

export class WebRTCService {
  private baseUrl: string

  constructor(baseUrl: string = 'http://localhost:8080') {
    this.baseUrl = baseUrl
  }

  async createSession(config?: SessionConfig): Promise<CreateSessionResponse> {
    const response = await fetch(`${this.baseUrl}/api/sessions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ config: config || {} }),
    })

    if (!response.ok) {
      throw new Error(`Failed to create session: ${response.statusText}`)
    }

    return response.json()
  }

  async terminateSession(sessionId: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/sessions/${sessionId}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      throw new Error(`Failed to terminate session: ${response.statusText}`)
    }
  }
}
