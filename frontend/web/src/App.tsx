import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Layout } from './components/Layout'
import { LoginPage } from './pages/LoginPage'
import { FilesPage } from './pages/FilesPage'
import { SessionsPage } from './pages/SessionsPage'
import { useAuthStore } from './store/authStore'

function App() {
  const { isAuthenticated } = useAuthStore()

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/"
          element={
            isAuthenticated ? (
              <Layout>
                <Routes>
                  <Route path="/files" element={<FilesPage />} />
                  <Route path="/sessions" element={<SessionsPage />} />
                  <Route path="/" element={<Navigate to="/files" replace />} />
                </Routes>
              </Layout>
            ) : (
              <Navigate to="/login" replace />
            )
          }
        />
      </Routes>
    </BrowserRouter>
  )
}

export default App
