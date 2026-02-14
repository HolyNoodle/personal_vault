import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../store/authStore'

// Helper functions for WebAuthn data conversion
function base64urlToArrayBuffer(base64url: string): ArrayBuffer {
  const base64 = base64url.replace(/-/g, '+').replace(/_/g, '/');
  const paddedBase64 = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=');
  const binaryString = atob(paddedBase64);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes.buffer;
}

function arrayToArrayBuffer(arr: number[]): ArrayBuffer {
  return new Uint8Array(arr).buffer;
}

function convertCredentialRequestOptions(options: any): PublicKeyCredentialRequestOptions {
  return {
    ...options,
    challenge: typeof options.challenge === 'string'
      ? base64urlToArrayBuffer(options.challenge)
      : arrayToArrayBuffer(options.challenge),
    allowCredentials: options.allowCredentials?.map((cred: any) => ({
      ...cred,
      id: typeof cred.id === 'string'
        ? base64urlToArrayBuffer(cred.id)
        : arrayToArrayBuffer(cred.id),
    })) || [],
  };
}

export function LoginPage() {
  const [email, setEmail] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const navigate = useNavigate()
  const login = useAuthStore((state) => state.login)

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')

    try {
      // Step 1: Initiate login
      const initiateRes = await fetch('http://localhost:8080/api/auth/initiate-login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
      });

      if (!initiateRes.ok) {
        const errData = await initiateRes.text();
        throw new Error(errData || 'Failed to initiate login');
      }

      const { options, challenge_id } = await initiateRes.json();

      // Step 2: Get credential using WebAuthn
      const publicKeyOptions = convertCredentialRequestOptions(options.publicKey);
      const credential = await navigator.credentials.get({
        publicKey: publicKeyOptions,
      }) as PublicKeyCredential;

      if (!credential) {
        throw new Error('Failed to get credential');
      }

      // Step 3: Complete login
      const completeRes = await fetch('http://localhost:8080/api/auth/complete-login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          challenge_id,
          credential: {
            id: credential.id,
            rawId: Array.from(new Uint8Array(credential.rawId)),
            response: {
              authenticatorData: Array.from(
                new Uint8Array((credential.response as AuthenticatorAssertionResponse).authenticatorData)
              ),
              clientDataJSON: Array.from(
                new Uint8Array(credential.response.clientDataJSON)
              ),
              signature: Array.from(
                new Uint8Array((credential.response as AuthenticatorAssertionResponse).signature)
              ),
              userHandle: (credential.response as AuthenticatorAssertionResponse).userHandle
                ? Array.from(new Uint8Array((credential.response as AuthenticatorAssertionResponse).userHandle!))
                : null,
            },
            type: credential.type,
          },
          email,
        }),
      });

      if (!completeRes.ok) {
        const errData = await completeRes.text();
        throw new Error(errData || 'Failed to complete login');
      }

      const { token, user } = await completeRes.json();

      // Store auth state and navigate
      login(user, token);
      navigate('/files');
    } catch (err) {
      console.error('Login error:', err);
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
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
        
        {error && (
          <div style={{
            backgroundColor: '#fee',
            color: '#c00',
            padding: '0.75rem',
            borderRadius: '4px',
            marginBottom: '1rem'
          }}>
            {error}
          </div>
        )}
        
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
          <button
            type="submit"
            disabled={loading}
            style={{
              marginTop: '1rem',
              padding: '0.75rem',
              backgroundColor: loading ? '#666' : '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              fontSize: '1rem',
              cursor: loading ? 'not-allowed' : 'pointer',
            }}
          >
            {loading ? 'Authenticating...' : 'Login with Security Key'}
          </button>
        </form>
      </div>
    </div>
  )
}
