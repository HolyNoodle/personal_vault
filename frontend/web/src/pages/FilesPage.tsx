import { useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Box,
  Typography,
  Button,
  Alert,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  IconButton,
} from '@mui/material'
import UploadFileIcon from '@mui/icons-material/UploadFile'
import CreateNewFolderIcon from '@mui/icons-material/CreateNewFolder'
import DownloadIcon from '@mui/icons-material/Download'
import FolderIcon from '@mui/icons-material/Folder'
import InsertDriveFileIcon from '@mui/icons-material/InsertDriveFile'
import { useAuthStore } from '../store/authStore'

interface FileItem {
  id: string
  name: string
  type: 'file' | 'folder'
  size?: number
  created_at: string
  path: string
}

export function FilesPage() {
  const { t } = useTranslation()
  const { token } = useAuthStore()
  const [files, setFiles] = useState<FileItem[]>([])
  const [currentPath, setCurrentPath] = useState('/')
  const [uploading, setUploading] = useState(false)
  const [error, setError] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setUploading(true)
    setError('')

    const formData = new FormData()
    formData.append('file', file)
    formData.append('path', currentPath)

    try {
      const response = await fetch('http://localhost:8080/api/files/upload', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
        },
        body: formData,
      })

      if (!response.ok) {
        const data = await response.json()
        throw new Error(data.error || 'Upload failed')
      }

      // Refresh file list after upload
      await loadFiles()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed')
    } finally {
      setUploading(false)
      if (fileInputRef.current) {
        fileInputRef.current.value = ''
      }
    }
  }

  const loadFiles = async () => {
    try {
      const response = await fetch(`http://localhost:8080/api/files?path=${encodeURIComponent(currentPath)}`, {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      })

      if (!response.ok) {
        throw new Error('Failed to load files')
      }

      const data = await response.json()
      setFiles(data.files || [])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load files')
    }
  }

  const formatFileSize = (bytes?: number) => {
    if (!bytes) return '-'
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  return (
    <Box>
      <Box display="flex" justifyContent="space-between" alignItems="center" mb={3}>
        <Typography variant="h4" component="h1">
          {t('files.title')}
        </Typography>
        <Box display="flex" gap={2}>
          <Button
            variant="outlined"
            startIcon={<CreateNewFolderIcon />}
            disabled
          >
            {t('files.createFolderButton')}
          </Button>
          <Button
            variant="contained"
            startIcon={<UploadFileIcon />}
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
          >
            {uploading ? 'Uploading...' : t('files.uploadButton')}
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            onChange={handleFileUpload}
            style={{ display: 'none' }}
          />
        </Box>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      <TableContainer component={Paper}>
        {files.length === 0 ? (
          <Box p={6} textAlign="center">
            <Typography variant="h6" color="text.secondary" gutterBottom>
              {t('files.noFiles')}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Upload your first file to get started
            </Typography>
          </Box>
        ) : (
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>{t('files.columns.name')}</TableCell>
                <TableCell>{t('files.columns.size')}</TableCell>
                <TableCell>{t('files.columns.modified')}</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {files.map((file) => (
                <TableRow key={file.id} hover>
                  <TableCell>
                    <Box display="flex" alignItems="center" gap={1}>
                      {file.type === 'folder' ? (
                        <FolderIcon color="primary" />
                      ) : (
                        <InsertDriveFileIcon color="action" />
                      )}
                      <Typography>{file.name}</Typography>
                    </Box>
                  </TableCell>
                  <TableCell>{formatFileSize(file.size)}</TableCell>
                  <TableCell>
                    {new Date(file.created_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell align="right">
                    <IconButton size="small" disabled>
                      <DownloadIcon />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </TableContainer>
    </Box>
  )
}
