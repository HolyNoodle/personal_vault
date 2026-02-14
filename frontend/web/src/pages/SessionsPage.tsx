export function SessionsPage() {
  return (
    <div>
      <h1 style={{ marginBottom: '2rem' }}>Active Sessions</h1>
      <div style={{ 
        background: '#1a1a1a', 
        padding: '2rem', 
        borderRadius: '8px',
        textAlign: 'center'
      }}>
        <p>Active viewing sessions will appear here</p>
        <p style={{ marginTop: '1rem', color: '#888' }}>
          Monitor and manage WebRTC sessions
        </p>
      </div>
    </div>
  )
}
