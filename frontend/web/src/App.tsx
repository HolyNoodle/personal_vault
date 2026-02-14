import { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import CircularProgress from '@mui/material/CircularProgress'
import Box from '@mui/material/Box'
import { Layout } from './components/Layout'
import { LoginPage } from './pages/LoginPage'
import { FilesPage } from './pages/FilesPage'
import { SessionsPage } from './pages/SessionsPage'
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
  const [isInitialized, setIsInitialized] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    checkSetupStatus();
  }, []);

  const checkSetupStatus = async () => {
    try {
      const response = await fetch('http://localhost:8080/api/setup/status');
      const data = await response.json();
      setIsInitialized(data.initialized);
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

  if (isInitialized === false) {
    return (
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <SetupPage onSetupComplete={() => setIsInitialized(true)} />
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/files" element={<ProtectedRoute><FilesPage /></ProtectedRoute>} />
          <Route path="/sessions" element={<ProtectedRoute><SessionsPage /></ProtectedRoute>} />
          <Route path="/" element={<Navigate to="/files" replace />} />
        </Routes>
      </BrowserRouter>
    </ThemeProvider>
  )
}

export default App
