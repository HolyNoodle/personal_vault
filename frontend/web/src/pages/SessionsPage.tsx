import { useTranslation } from 'react-i18next'
import { Box, Typography, Paper } from '@mui/material'
import VideoCallIcon from '@mui/icons-material/VideoCall'

export function SessionsPage() {
  const { t } = useTranslation()

  return (
    <Box>
      <Typography variant="h4" component="h1" gutterBottom>
        {t('sessions.title')}
      </Typography>
      
      <Paper sx={{ p: 6, textAlign: 'center' }}>
        <VideoCallIcon sx={{ fontSize: 64, color: 'text.secondary', mb: 2 }} />
        <Typography variant="h6" color="text.secondary" gutterBottom>
          {t('sessions.noSessions')}
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Monitor and manage WebRTC sessions
        </Typography>
      </Paper>
    </Box>
  )
}
