import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Container,
  Typography,
  Card,
  CardContent,
  CardActions,
  Button,
  Grid,
  Box,
  Chip,
  Alert,
  CircularProgress,
} from '@mui/material';
import { Launch, Folder } from '@mui/icons-material';

interface Application {
  app_id: string;
  name: string;
  description: string;
  version: string;
}

export function ApplicationsPage() {
  const navigate = useNavigate();
  const [applications, setApplications] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadApplications();
  }, []);

  const loadApplications = async () => {
    try {
      const response = await fetch('http://localhost:8080/api/applications');
      if (!response.ok) throw new Error('Failed to load applications');
      const data = await response.json();
      setApplications(Array.isArray(data) ? data : []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const handleLaunch = async (app: Application) => {
    navigate(`/launch?appId=${app.app_id}&name=${encodeURIComponent(app.name)}`);
  };

  if (loading) {
    return (
      <Container maxWidth="lg" sx={{ mt: 4, display: 'flex', justifyContent: 'center' }}>
        <CircularProgress />
      </Container>
    );
  }

  if (error) {
    return (
      <Container maxWidth="lg" sx={{ mt: 4 }}>
        <Alert severity="error">{error}</Alert>
      </Container>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" gutterBottom>
          Applications
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Launch applications in secure sandboxed environments
        </Typography>
      </Box>

      <Grid container spacing={3}>
        {applications.map((app) => (
          <Grid size={{ xs: 12, sm: 6, md: 4 }} key={app.app_id}>
            <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
              <CardContent sx={{ flexGrow: 1 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                  <Folder sx={{ fontSize: 40, mr: 2, color: 'primary.main' }} />
                  <Box>
                    <Typography variant="h6" component="h2">
                      {app.name}
                    </Typography>
                    <Chip label={`v${app.version}`} size="small" />
                  </Box>
                </Box>
                <Typography variant="body2" color="text.secondary">
                  {app.description}
                </Typography>
              </CardContent>
              <CardActions>
                <Button
                  size="small"
                  variant="contained"
                  startIcon={<Launch />}
                  onClick={() => handleLaunch(app)}
                  fullWidth
                >
                  Launch
                </Button>
              </CardActions>
            </Card>
          </Grid>
        ))}
      </Grid>

      {applications.length === 0 && (
        <Box sx={{ textAlign: 'center', mt: 8 }}>
          <Typography variant="h6" color="text.secondary">
            No applications available
          </Typography>
        </Box>
      )}
    </Container>
  );
}
