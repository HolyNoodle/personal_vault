import { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import CircularProgress from '@mui/material/CircularProgress'
import Box from '@mui/material/Box'
import { Layout } from './components/Layout'
import { LoginPage } from './pages/LoginPage'
import { ApplicationsPage } from './pages/ApplicationsPage'
import { LaunchApplicationPage } from './pages/LaunchApplicationPage'
import { SessionsPage } from './pages/SessionsPage'
import { VideoSessionPage } from './pages/VideoSessionPage'
import SetupPage from './pages/SetupPage'
import { useAuthStore } from './store/authStore'
import { theme } from './theme'

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAuthStore()
  
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />
  }
  
  return <Layout>{children}</Layout>
}

function App() {
  const [needsSetup, setNeedsSetup] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    checkSetupStatus();
  }, []);

  const checkSetupStatus = async () => {
    try {
      const response = await fetch('http://localhost:8080/api/setup/status');
      const data = await response.json();
      setNeedsSetup(data.needs_setup);
    } catch (error) {
      console.error('Failed to check setup status:', error);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <Box display="flex" justifyContent="center" alignItems="center" height="100vh">
          <CircularProgress />
        </Box>
      </ThemeProvider>
    );
  }

  if (needsSetup === true) {
    return (
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <SetupPage onSetupComplete={() => setNeedsSetup(false)} />
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/applications" element={<ProtectedRoute><ApplicationsPage /></ProtectedRoute>} />
          <Route path="/launch" element={<ProtectedRoute><LaunchApplicationPage /></ProtectedRoute>} />
          <Route path="/sessions" element={<ProtectedRoute><SessionsPage /></ProtectedRoute>} />
          <Route path="/video" element={<ProtectedRoute><VideoSessionPage /></ProtectedRoute>} />
          <Route path="/" element={<ProtectedRoute><ApplicationsPage /></ProtectedRoute>} />
        </Routes>
      </BrowserRouter>
    </ThemeProvider>
  )
}

export default App
