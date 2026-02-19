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
import AppsIcon from '@mui/icons-material/Apps'
import VideoCallIcon from '@mui/icons-material/VideoCall'
import LogoutIcon from '@mui/icons-material/Logout'
import { useAuthStore } from '../store/authStore'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  const { t } = useTranslation()
  const { user, logout, hasRole } = useAuthStore()
  const navigate = useNavigate()
  const location = useLocation()

  const isVideoPage = location.pathname === '/video'

  const getPageTitle = () => {
    switch (location.pathname) {
      case '/applications':
      case '/':
        return 'Applications'
      case '/sessions':
        return t('nav.sessions')
      case '/video':
        return 'Application Session'
      case '/launch':
        return 'Launch Application'
      default:
        return t('app.title')
    }
  }

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: '100vh' }}>
      <AppBar position="static">
        <Toolbar>
          <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
            {getPageTitle()}
          </Typography>
          

          {/* SuperAdmin section */}
          {hasRole && hasRole('super_admin') && (
            <Button color="inherit" onClick={() => navigate('/admin/users')} sx={{ mx: 1 }}>
              Users
            </Button>
          )}

          {/* Owner section */}
          {hasRole && hasRole('owner') && (
            <>
              <Button color="inherit" onClick={() => navigate('/owner/invitations')} sx={{ mx: 1 }}>
                Invitations
              </Button>
              <Button color="inherit" onClick={() => navigate('/owner/permissions')} sx={{ mx: 1 }}>
                Permissions
              </Button>
              <Button color="inherit" onClick={() => navigate('/owner/clients')} sx={{ mx: 1 }}>
                Clients
              </Button>
            </>
          )}

          {/* Client section */}
          {hasRole && hasRole('client') && (
            <Button color="inherit" onClick={() => navigate('/my-permissions')} sx={{ mx: 1 }}>
              My Access
            </Button>
          )}

          {/* Common navigation */}
          <Button
            color="inherit"
            startIcon={<AppsIcon />}
            onClick={() => navigate('/applications')}
            sx={{ 
              mx: 1,
              backgroundColor: location.pathname === '/applications' || location.pathname === '/' ? 'rgba(255,255,255,0.1)' : 'transparent'
            }}
          >
            Applications
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
              {user?.email} {user?.roles && `[${user.roles.join(', ')}]`}
            </Typography>
            <IconButton color="inherit" onClick={handleLogout} title={t('nav.logout')}>
              <LogoutIcon />
            </IconButton>
          </Box>
        </Toolbar>
      </AppBar>
      
      {isVideoPage ? (
        <Box sx={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
          {children}
        </Box>
      ) : (
        <Container component="main" sx={{ flex: 1, py: 4 }} maxWidth="xl">
          {children}
        </Container>
      )}
    </Box>
  )
}
