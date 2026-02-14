import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../store/authStore'

export function LoginPage() {
  const [email, setEmail] = useState('')
  const navigate = useNavigate()
  const login = useAuthStore((state) => state.login)

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    
    // TODO: Implement WebAuthn authentication
    // For now, mock login
    login(
      { id: '1', email, role: 'owner' },
      'mock-jwt-token'
    )
    
    navigate('/files')
  }

  return (
    <div style={{ 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center', 
      minHeight: '100vh',
      background: '#242424'
    }}>
      <div style={{ 
        background: '#1a1a1a', 
        padding: '2rem', 
        borderRadius: '8px',
        width: '400px'
      }}>
        <h1 style={{ marginBottom: '2rem', fontSize: '2rem' }}>Secure Sandbox</h1>
        <form onSubmit={handleLogin} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          <div>
            <label htmlFor="email" style={{ display: 'block', marginBottom: '0.5rem' }}>
              Email
            </label>
            <input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              style={{
                width: '100%',
                padding: '0.5rem',
                borderRadius: '4px',
                border: '1px solid #333',
                background: '#242424',
                color: '#fff'
              }}
            />
          </div>
          <button type="submit" style={{ marginTop: '1rem' }}>
            Login with WebAuthn
          </button>
        </form>
      </div>
    </div>
  )
}
