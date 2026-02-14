import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Formik, Form } from 'formik';
import * as Yup from 'yup';
import {
  Container,
  Paper,
  Typography,
  TextField,
  Button,
  Alert,
  Box,
} from '@mui/material';
import AdminPanelSettingsIcon from '@mui/icons-material/AdminPanelSettings';
import SecurityIcon from '@mui/icons-material/Security';

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
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const validationSchema = Yup.object({
    email: Yup.string().email('Invalid email').required('Email is required'),
    displayName: Yup.string().required('Display name is required'),
  });

  const handleSubmit = async (values: { email: string; displayName: string }) => {
    setLoading(true);
    setError('');

    try {
      // Step 1: Initiate registration
      const initiateRes = await fetch('http://localhost:8080/api/setup/initiate-registration', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: values.email, display_name: values.displayName }),
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
          email: values.email,
          display_name: values.displayName,
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
    <Container maxWidth="sm">
      <Box
        display="flex"
        flexDirection="column"
        justifyContent="center"
        alignItems="center"
        minHeight="100vh"
      >
        <Paper elevation={3} sx={{ p: 4, width: '100%' }}>
          <Box display="flex" alignItems="center" gap={2} mb={2}>
            <AdminPanelSettingsIcon fontSize="large" color="primary" />
            <Typography variant="h4" component="h1">
              {t('auth.setup.title')}
            </Typography>
          </Box>

          <Typography variant="body1" color="text.secondary" mb={3}>
            {t('auth.setup.description')}
          </Typography>

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}

          <Formik
            initialValues={{ email: '', displayName: '' }}
            validationSchema={validationSchema}
            onSubmit={handleSubmit}
          >
            {({ values, errors, touched, handleChange, handleBlur }) => (
              <Form>
                <TextField
                  fullWidth
                  id="email"
                  name="email"
                  label={t('auth.setup.emailLabel')}
                  placeholder={t('auth.setup.emailPlaceholder')}
                  value={values.email}
                  onChange={handleChange}
                  onBlur={handleBlur}
                  error={touched.email && Boolean(errors.email)}
                  helperText={touched.email && errors.email}
                  margin="normal"
                  type="email"
                />

                <TextField
                  fullWidth
                  id="displayName"
                  name="displayName"
                  label={t('auth.setup.displayNameLabel')}
                  placeholder={t('auth.setup.displayNamePlaceholder')}
                  value={values.displayName}
                  onChange={handleChange}
                  onBlur={handleBlur}
                  error={touched.displayName && Boolean(errors.displayName)}
                  helperText={touched.displayName && errors.displayName}
                  margin="normal"
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
                  {loading ? 'Registering...' : t('auth.setup.registerButton')}
                </Button>

                <Alert severity="info" sx={{ mt: 2 }}>
                  <strong>Note:</strong> You will need a WebAuthn-compatible security key (YubiKey, etc.) or platform authenticator (Touch ID, Windows Hello) to complete registration.
                </Alert>
              </Form>
            )}
          </Formik>
        </Paper>
      </Box>
    </Container>
  );
}

export default SetupPage;
