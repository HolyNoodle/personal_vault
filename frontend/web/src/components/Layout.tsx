import { ReactNode } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import {
  AppBar,
  Box,
  Toolbar,
  Typography,
  Button,
  IconButton,
  Container,
} from '@mui/material'
import FolderIcon from '@mui/icons-material/Folder'
import VideoCallIcon from '@mui/icons-material/VideoCall'
import LogoutIcon from '@mui/icons-material/Logout'
import { useAuthStore } from '../store/authStore'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  const { t } = useTranslation()
  const { user, logout } = useAuthStore()
  const navigate = useNavigate()
  const location = useLocation()

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: '100vh' }}>
      <AppBar position="static">
        <Toolbar>
          <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
            {t('app.title')}
          </Typography>
          
          <Button
            color="inherit"
            startIcon={<FolderIcon />}
            onClick={() => navigate('/files')}
            sx={{ 
              mx: 1,
              backgroundColor: location.pathname === '/files' ? 'rgba(255,255,255,0.1)' : 'transparent'
            }}
          >
            {t('nav.files')}
          </Button>
          
          <Button
            color="inherit"
            startIcon={<VideoCallIcon />}
            onClick={() => navigate('/sessions')}
            sx={{ 
              mx: 1,
              backgroundColor: location.pathname === '/sessions' ? 'rgba(255,255,255,0.1)' : 'transparent'
            }}
          >
            {t('nav.sessions')}
          </Button>

          <Box sx={{ ml: 2, display: 'flex', alignItems: 'center', gap: 2 }}>
            <Typography variant="body2">
              {user?.email} ({user?.role})
            </Typography>
            <IconButton color="inherit" onClick={handleLogout} title={t('nav.logout')}>
              <LogoutIcon />
            </IconButton>
          </Box>
        </Toolbar>
      </AppBar>
      
      <Container component="main" sx={{ flex: 1, py: 4 }} maxWidth="xl">
        {children}
      </Container>
    </Box>
  )
}
