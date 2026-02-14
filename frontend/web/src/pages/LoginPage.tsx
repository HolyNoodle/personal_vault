import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Formik, Form } from 'formik'
import * as Yup from 'yup'
import {
  Container,
  Paper,
  Typography,
  TextField,
  Button,
  Alert,
  Box,
} from '@mui/material'
import SecurityIcon from '@mui/icons-material/Security'
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
  const { t } = useTranslation()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const navigate = useNavigate()
  const login = useAuthStore((state) => state.login)

  const validationSchema = Yup.object({
    email: Yup.string().email('Invalid email').required('Email is required'),
  })

  const handleLogin = async (values: { email: string }) => {
    setLoading(true)
    setError('')

    try {
      // Step 1: Initiate login
      const initiateRes = await fetch('http://localhost:8080/api/auth/initiate-login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: values.email }),
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
          email: values.email,
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
    <Container maxWidth="sm">
      <Box
        display="flex"
        flexDirection="column"
        justifyContent="center"
        alignItems="center"
        minHeight="100vh"
      >
        <Paper elevation={3} sx={{ p: 4, width: '100%' }}>
          <Box display="flex" alignItems="center" gap={2} mb={3}>
            <SecurityIcon fontSize="large" color="primary" />
            <Typography variant="h4" component="h1">
              {t('auth.login.title')}
            </Typography>
          </Box>

          <Typography variant="body1" color="text.secondary" mb={3}>
            {t('auth.login.description')}
          </Typography>

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}

          <Formik
            initialValues={{ email: '' }}
            validationSchema={validationSchema}
            onSubmit={handleLogin}
          >
            {({ values, errors, touched, handleChange, handleBlur }) => (
              <Form>
                <TextField
                  fullWidth
                  id="email"
                  name="email"
                  label={t('auth.login.emailLabel')}
                  placeholder={t('auth.login.emailPlaceholder')}
                  value={values.email}
                  onChange={handleChange}
                  onBlur={handleBlur}
                  error={touched.email && Boolean(errors.email)}
                  helperText={touched.email && errors.email}
                  margin="normal"
                  type="email"
                />

                <Button
                  type="submit"
                  fullWidth
                  variant="contained"
                  size="large"
                  disabled={loading}
                  startIcon={<SecurityIcon />}
                  sx={{ mt: 3 }}
                >
                  {loading ? 'Authenticating...' : t('auth.login.loginButton')}
                </Button>
              </Form>
            )}
          </Formik>
        </Paper>
      </Box>
    </Container>
  )
}
