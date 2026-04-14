# WebSH Tunnel Node

WebSocket-to-TCP proxy for WebSH. Bridges browser-based terminal sessions to remote SSH servers.

---

## Configuration

| Variable        | Description                                  | Required | Default        |
| :-------------- | :------------------------------------------- | :------: | :------------- |
| `TUNNEL_SECRET` | Shared secret for authentication.            | **Yes**  | -              |
| `BIND_ADDR`     | Server bind address and port.                |    No    | `0.0.0.0:5152` |

> [!IMPORTANT]
> Run behind a reverse proxy with **TLS (WSS)** enabled in production.

---

## Deployment

### Docker Compose

1. Create `.env`:
   ```env
   TUNNEL_SECRET=your_secret
   ```

2. Run:
   ```bash
   docker compose up -d
   ```

---

## API

### `GET /tunnel`

Upgrades to WebSocket and proxies data to target.

**Parameters:**
- `host`: Target host.
- `port`: Target TCP port.
- `secret`: Must match `TUNNEL_SECRET`.

**Example:**
`ws://localhost:5152/tunnel?host=1.2.3.4&port=22&secret=mysecret`

### `GET /health`

Returns `200 OK`.

---

## Security

- **TLS/WSS**: Required for production.
- **Secret**: Use a strong random string.
