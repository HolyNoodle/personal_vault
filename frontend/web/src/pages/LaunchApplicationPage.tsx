import { useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  Container,
  Typography,
  Card,
  CardContent,
  Button,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Box,
  Alert,
  Chip,
  IconButton,
} from '@mui/material';
import { Add, Delete, Launch } from '@mui/icons-material';
import { useAuthStore } from '../store/authStore';

export function LaunchApplicationPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { user } = useAuthStore();
  const appId = searchParams.get('appId') || 'file-explorer-v1';
  const appName = searchParams.get('name') || 'File Explorer';

  const [userRole, setUserRole] = useState('owner');
  const [allowedPaths, setAllowedPaths] = useState<string[]>(['/home']);
  const [newPath, setNewPath] = useState('');
  const [videoWidth, setVideoWidth] = useState(1920);
  const [videoHeight, setVideoHeight] = useState(1080);
  const [videoFramerate, setVideoFramerate] = useState(15);
  const [enableWatermarking, setEnableWatermarking] = useState(false);
  const [timeoutMinutes, setTimeoutMinutes] = useState(60);
  const [error, setError] = useState<string | null>(null);
  const [launching, setLaunching] = useState(false);

  const handleAddPath = () => {
    if (newPath && !allowedPaths.includes(newPath)) {
      setAllowedPaths([...allowedPaths, newPath]);
      setNewPath('');
    }
  };

  const handleRemovePath = (path: string) => {
    setAllowedPaths(allowedPaths.filter(p => p !== path));
  };

  const handleLaunch = async () => {
    if (allowedPaths.length === 0) {
      setError('At least one allowed path is required');
      return;
    }

    setLaunching(true);
    setError(null);

    try {
      const response = await fetch('http://localhost:8080/api/applications/launch', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          app_id: appId,
          user_id: user?.id || 'test-user',
          user_role: userRole,
          allowed_paths: allowedPaths,
          video_width: videoWidth,
          video_height: videoHeight,
          video_framerate: videoFramerate,
          enable_watermarking: enableWatermarking,
          timeout_minutes: timeoutMinutes,
        }),
      });

      if (!response.ok) {
        const errorData = await response.text();
        throw new Error(errorData || 'Failed to launch application');
      }

      const data = await response.json();
      console.log('Application launched:', data);
      
      // Navigate to video session page with the session ID
      navigate(`/video?sessionId=${data.session_id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLaunching(false);
    }
  };

  return (
    <Container maxWidth="md" sx={{ mt: 4, mb: 4 }}>
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" gutterBottom>
          Launch {appName}
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Configure sandboxed environment settings
        </Typography>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 3 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      <Card>
        <CardContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            {/* User Role */}
            <FormControl fullWidth>
              <InputLabel>User Role</InputLabel>
              <Select
                value={userRole}
                label="User Role"
                onChange={(e) => setUserRole(e.target.value)}
              >
                <MenuItem value="owner">Owner (Read/Write)</MenuItem>
                <MenuItem value="client">Client (Read-Only)</MenuItem>
              </Select>
            </FormControl>

            {/* Allowed Paths */}
            <Box>
              <Typography variant="subtitle1" gutterBottom>
                Allowed File System Paths
              </Typography>
              <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
                <TextField
                  fullWidth
                  size="small"
                  placeholder="/path/to/directory"
                  value={newPath}
                  onChange={(e) => setNewPath(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleAddPath()}
                />
                <Button
                  variant="contained"
                  startIcon={<Add />}
                  onClick={handleAddPath}
                >
                  Add
                </Button>
              </Box>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                {allowedPaths.map((path) => (
                  <Chip
                    key={path}
                    label={path}
                    onDelete={() => handleRemovePath(path)}
                    deleteIcon={<Delete />}
                  />
                ))}
              </Box>
            </Box>

            {/* Video Settings */}
            <Typography variant="h6" sx={{ mt: 2 }}>
              Video Stream Settings
            </Typography>

            <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 2 }}>
              <TextField
                label="Width"
                type="number"
                value={videoWidth}
                onChange={(e) => setVideoWidth(Number(e.target.value))}
                inputProps={{ min: 640, max: 3840 }}
              />
              <TextField
                label="Height"
                type="number"
                value={videoHeight}
                onChange={(e) => setVideoHeight(Number(e.target.value))}
                inputProps={{ min: 480, max: 2160 }}
              />
              <TextField
                label="Framerate"
                type="number"
                value={videoFramerate}
                onChange={(e) => setVideoFramerate(Number(e.target.value))}
                inputProps={{ min: 15, max: 60 }}
              />
            </Box>

            {/* Additional Settings */}
            <FormControl fullWidth>
              <InputLabel>Session Timeout (minutes)</InputLabel>
              <Select
                value={timeoutMinutes}
                label="Session Timeout (minutes)"
                onChange={(e) => setTimeoutMinutes(Number(e.target.value))}
              >
                <MenuItem value={30}>30 minutes</MenuItem>
                <MenuItem value={60}>1 hour</MenuItem>
                <MenuItem value={120}>2 hours</MenuItem>
                <MenuItem value={240}>4 hours</MenuItem>
              </Select>
            </FormControl>

            <FormControl fullWidth>
              <InputLabel>Watermarking</InputLabel>
              <Select
                value={enableWatermarking ? 'enabled' : 'disabled'}
                label="Watermarking"
                onChange={(e) => setEnableWatermarking(e.target.value === 'enabled')}
              >
                <MenuItem value="disabled">Disabled</MenuItem>
                <MenuItem value="enabled">Enabled (for client sessions)</MenuItem>
              </Select>
            </FormControl>

            {/* Launch Button */}
            <Box sx={{ display: 'flex', gap: 2, mt: 2 }}>
              <Button
                variant="outlined"
                onClick={() => navigate('/applications')}
                disabled={launching}
              >
                Cancel
              </Button>
              <Button
                variant="contained"
                size="large"
                startIcon={<Launch />}
                onClick={handleLaunch}
                disabled={launching || allowedPaths.length === 0}
                fullWidth
              >
                {launching ? 'Launching...' : 'Launch Application'}
              </Button>
            </Box>
          </Box>
        </CardContent>
      </Card>
    </Container>
  );
}
