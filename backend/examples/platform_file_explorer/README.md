# Platform File Explorer

A GTK3-based file manager designed to run in the sandboxed platform with full platform integration.

## Features

- Browse files and directories
- Preview images (jpg, png, gif, bmp)
- Preview text files (txt, md, py, rs, js, json, xml, html)
- Upload files to current directory via platform
- Download files via platform
- Delete files and directories

## IPC Protocol

### Messages from App to Platform (stdout)

```json
{
  "type": "state",
  "path": "/home/user/documents",
  "selected": "/home/user/documents/file.txt",
  "actions": ["upload", "download", "delete"]
}
```

```json
{
  "type": "download_data",
  "filename": "document.pdf",
  "data": "base64encodeddata..."
}
```

```json
{
  "type": "error",
  "message": "Permission denied"
}
```

### Messages from Platform to App (stdin)

```json
{
  "type": "upload",
  "filename": "newfile.txt",
  "data": "base64encodeddata..."
}
```

```json
{
  "type": "download_request"
}
```

```json
{
  "type": "delete"
}
```

## Dependencies

- Python 3
- GTK 3
- PyGObject

## Usage

```bash
python3 main.py
```

The app communicates with the platform via stdin/stdout using JSON messages.
