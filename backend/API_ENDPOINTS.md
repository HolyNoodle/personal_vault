# API Endpoints (2026-02-18)

This backend exposes the following HTTP and WebSocket endpoints:

## AUTH
- POST http://localhost:8080/api/auth/register
- POST http://localhost:8080/api/auth/login

## APPLICATION PLATFORM
- GET  http://localhost:8080/api/applications
- POST http://localhost:8080/api/applications/launch

## WEBSOCKET
- WS   ws://localhost:8080/ws  
  (Used for application signaling, not for video streaming)

## SYSTEM
- GET  http://localhost:8080/health

---

### Notes
- The legacy `/api/sessions` endpoint and video session APIs have been removed.
- The `/ws` route is now used for application signaling only.

---

## OpenAPI/Swagger

This project does not yet expose a live OpenAPI/Swagger spec. For future extensibility, consider documenting endpoints in OpenAPI 3.0 format. Example:

```yaml
openapi: 3.0.0
info:
  title: Secure Sandbox API
  version: 1.0.0
paths:
  /api/applications:
    get:
      summary: List available applications
      responses:
        '200':
          description: List of applications
  /api/applications/launch:
    post:
      summary: Launch an application session
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                app_id:
                  type: string
                user_id:
                  type: string
      responses:
        '200':
          description: Session info
```

For a full OpenAPI spec, see [OpenAPI documentation](https://swagger.io/specification/).
