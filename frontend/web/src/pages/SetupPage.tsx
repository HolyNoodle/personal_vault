import { useState } from 'react';

interface SetupPageProps {
  onSetupComplete: () => void;
}

// Helper function to convert base64url to ArrayBuffer
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

// Helper function to convert array of numbers to ArrayBuffer
function arrayToArrayBuffer(arr: number[]): ArrayBuffer {
  return new Uint8Array(arr).buffer;
}

// Convert WebAuthn options from server format to browser format
function convertCredentialCreationOptions(options: any): PublicKeyCredentialCreationOptions {
  return {
    ...options,
    challenge: typeof options.challenge === 'string' 
      ? base64urlToArrayBuffer(options.challenge)
      : arrayToArrayBuffer(options.challenge),
    user: {
      ...options.user,
      id: typeof options.user.id === 'string'
        ? base64urlToArrayBuffer(options.user.id)
        : arrayToArrayBuffer(options.user.id),
    },
    excludeCredentials: options.excludeCredentials?.map((cred: any) => ({
      ...cred,
      id: typeof cred.id === 'string'
        ? base64urlToArrayBuffer(cred.id)
        : arrayToArrayBuffer(cred.id),
    })) || [],
  };
}

function SetupPage({ onSetupComplete }: SetupPageProps) {
  const [email, setEmail] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      // Step 1: Initiate registration
      const initiateRes = await fetch('http://localhost:8080/api/setup/initiate-registration', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, display_name: displayName }),
      });

      if (!initiateRes.ok) {
        const errData = await initiateRes.text();
        throw new Error(errData || 'Failed to initiate registration');
      }

      const { options, challenge_id } = await initiateRes.json();

      // Step 2: Convert options and create credential using WebAuthn
      const publicKeyOptions = convertCredentialCreationOptions(options.publicKey);
      const credential = await navigator.credentials.create({
        publicKey: publicKeyOptions,
      }) as PublicKeyCredential;

      if (!credential) {
        throw new Error('Failed to create credential');
      }

      // Step 3: Complete registration
      const completeRes = await fetch('http://localhost:8080/api/setup/complete-registration', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          challenge_id,
          credential: {
            id: credential.id,
            rawId: Array.from(new Uint8Array(credential.rawId)),
            response: {
              attestationObject: Array.from(
                new Uint8Array((credential.response as AuthenticatorAttestationResponse).attestationObject)
              ),
              clientDataJSON: Array.from(
                new Uint8Array(credential.response.clientDataJSON)
              ),
            },
            type: credential.type,
          },
          email,
          display_name: displayName,
        }),
      });

      if (!completeRes.ok) {
        const errData = await completeRes.text();
        throw new Error(errData || 'Failed to complete registration');
      }

      // Registration successful
      onSetupComplete();
    } catch (err) {
      console.error('Registration error:', err);
      setError(err instanceof Error ? err.message : 'Setup failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center', 
      minHeight: '100vh'
    }}>
      <div style={{
        backgroundColor: 'white',
        padding: '2rem',
        borderRadius: '8px',
        boxShadow: '0 2px 10px rgba(0,0,0,0.1)',
        width: '100%',
        maxWidth: '400px'
      }}>
        <h1 style={{ marginBottom: '0.5rem' }}>Initial Setup</h1>
        <p style={{ color: '#666', marginBottom: '2rem' }}>
          No super admin exists. Set up the first super admin account with a hardware security key.
        </p>

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

        <form onSubmit={handleSubmit}>
          <div style={{ marginBottom: '1rem' }}>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 500 }}>
              Email
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              style={{
                width: '100%',
                padding: '0.5rem',
                border: '1px solid #ddd',
                borderRadius: '4px',
                fontSize: '1rem'
              }}
              placeholder="admin@example.com"
            />
          </div>

          <div style={{ marginBottom: '1.5rem' }}>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 500 }}>
              Display Name
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              required
              style={{
                width: '100%',
                padding: '0.5rem',
                border: '1px solid #ddd',
                borderRadius: '4px',
                fontSize: '1rem'
              }}
              placeholder="System Administrator"
            />
          </div>

          <button
            type="submit"
            disabled={loading}
            style={{
              width: '100%',
              padding: '0.75rem',
              backgroundColor: '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              fontSize: '1rem',
              fontWeight: 500,
              cursor: loading ? 'not-allowed' : 'pointer',
              opacity: loading ? 0.6 : 1
            }}
          >
            {loading ? 'Registering...' : 'Register with Security Key'}
          </button>

          <div style={{
            marginTop: '1rem',
            padding: '0.75rem',
            backgroundColor: '#f0f8ff',
            borderRadius: '4px',
            fontSize: '0.875rem',
            color: '#666'
          }}>
            <strong>Note:</strong> You will need a WebAuthn-compatible security key (YubiKey, etc.) or platform authenticator (Touch ID, Windows Hello) to complete registration.
          </div>
        </form>
      </div>
    </div>
  );
}

export default SetupPage;
