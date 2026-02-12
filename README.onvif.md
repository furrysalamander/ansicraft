# ONVIF Minecraft Camera

ONVIF-compliant mock cameras that stream Minecraft gameplay via RTSP. Perfect for testing NVR software (Frigate, Blue Iris, etc.) with unique, controllable video sources.

## Features

- **ONVIF Compliance**: Full Device, Media, and PTZ service support
- **WS-Discovery**: Automatic camera discovery on the network
- **RTSP Streaming**: H264 video via go2rtc (320x200@30fps)
- **PTZ Controls**: Pan/tilt/zoom mapped to Minecraft mouse and keyboard
- **RCON Integration**: Position each camera's player at specific spawn coordinates
- **Multi-Camera**: Run multiple cameras against a single Minecraft server
- **Library Architecture**: Reusable components for terminal SSH viewer and ONVIF cameras

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Docker Network                          │
│                                                              │
│  ┌──────────────────┐                                        │
│  │ Minecraft Server │                                        │
│  │  (Paper 1.21.6)  │                                        │
│  │   + RCON         │                                        │
│  └────────┬─────────┘                                        │
│           │                                                  │
│    ┌──────┴────────┬──────────────┬─────────────┐           │
│    │               │              │             │           │
│  ┌─▼─────────┐  ┌─▼─────────┐  ┌─▼─────────┐   ...         │
│  │ Camera 1  │  │ Camera 2  │  │ Camera 3  │               │
│  │           │  │           │  │           │               │
│  │ Xorg :1   │  │ Xorg :1   │  │ Xorg :1   │               │
│  │ Minecraft │  │ Minecraft │  │ Minecraft │               │
│  │ go2rtc    │  │ go2rtc    │  │ go2rtc    │               │
│  │ ONVIF     │  │ ONVIF     │  │ ONVIF     │               │
│  └───────────┘  └───────────┘  └───────────┘               │
│   RTSP:5541      RTSP:5542      RTSP:5543                   │
│   ONVIF:8081     ONVIF:8082     ONVIF:8083                  │
└─────────────────────────────────────────────────────────────┘
```

Each camera container:
- Runs X server on :1 (headless)
- Launches Minecraft client (Java Edition 1.21.6)
- Connects to Minecraft server
- Teleports player to spawn coordinates via RCON
- Captures X display with FFmpeg → go2rtc
- Serves ONVIF endpoints for discovery and control
- PTZ commands translate to xdotool input injection

## Quick Start

### Build and Run

```bash
# Clone the repository
git clone <repository-url>
cd ansicraft

# Start 3 cameras + Minecraft server
docker-compose -f docker-compose.onvif.yml up --build

# Wait ~30 seconds for cameras to initialize
```

### Test ONVIF Discovery

```bash
# Using ONVIF Device Manager (Windows)
# Cameras should auto-discover on the network

# Or test manually with Python
python3 <<EOF
from onvif import ONVIFCamera
cam = ONVIFCamera('172.28.0.101', 8080, '', '')
device = cam.create_devicemgmt_service()
print(device.GetDeviceInformation())
EOF
```

### View RTSP Stream

```bash
# VLC
vlc rtsp://172.28.0.101:554/stream

# FFplay
ffplay rtsp://172.28.0.101:554/stream

# Or add to Frigate/Blue Iris using the ONVIF discovered cameras
```

### Test PTZ Controls

```python
from onvif import ONVIFCamera

cam = ONVIFCamera('172.28.0.101', 8080, '', '')
ptz = cam.create_ptz_service()

# Pan right
ptz.ContinuousMove({
    'ProfileToken': 'Profile1',
    'Velocity': {
        'PanTilt': {'x': 0.5, 'y': 0},
        'Zoom': {'x': 0}
    }
})

# Stop movement
ptz.Stop({'ProfileToken': 'Profile1', 'PanTilt': True, 'Zoom': True})
```

## Environment Variables

### Minecraft Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `USERNAME` | `camera_player` | Minecraft player username |
| `MINECRAFT_SERVER` | _(empty)_ | Server address (e.g., `172.28.0.2:25565`) |
| `MINECRAFT_VERSION` | `1.21.6` | Minecraft version to launch |
| `DISPLAY` | `:1` | X display number |

### RCON Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `RCON_HOST` | `localhost` | RCON server host |
| `RCON_PORT` | `25575` | RCON port |
| `RCON_PASSWORD` | `minecraft` | RCON password |

### Spawn Position

| Variable | Default | Description |
|----------|---------|-------------|
| `SPAWN_X` | `0` | X coordinate for player teleport |
| `SPAWN_Y` | `70` | Y coordinate (height) |
| `SPAWN_Z` | `0` | Z coordinate |

### ONVIF Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST_IP` | `127.0.0.1` | Camera IP for ONVIF XAddr |
| `DEVICE_NAME` | `Minecraft Camera` | Camera display name |
| `DEVICE_UUID` | _(generated)_ | Unique device UUID |
| `ONVIF_PORT` | `8080` | ONVIF service port |
| `RTSP_PORT` | `554` | RTSP stream port |

### Streaming Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `VIDEO_WIDTH` | `320` | Video width |
| `VIDEO_HEIGHT` | `200` | Video height |
| `FRAMERATE` | `30` | Capture framerate |

## PTZ Control Mapping

### Pan/Tilt (Mouse Movement)

- **Pan**: -1.0 (left) to 1.0 (right)
- **Tilt**: -1.0 (down) to 1.0 (up)
- **Scaling**: 500 pixels per PTZ unit

Example:
- Pan=0.5, Tilt=0 → Mouse moves right 250 pixels
- Pan=0, Tilt=-0.5 → Mouse moves down 250 pixels

### Zoom (Mouse Scroll)

- **Zoom**: 0.0 to 1.0
- **Mapping**: Scroll wheel up/down

Example:
- Zoom=0.5 → Scroll up
- Zoom=-0.5 → Scroll down

### Continuous Move

Updates mouse position at 20Hz while active. Call `Stop` to halt movement.

## Docker Compose Configuration

### Minimal Setup (Single Camera)

```yaml
services:
  minecraft-server:
    image: itzg/minecraft-server
    environment:
      - EULA=TRUE
      - TYPE=PAPER
      - VERSION=1.21.6
      - ENABLE_RCON=true
      - RCON_PASSWORD=minecraft
    ports:
      - "25565:25565"
      - "25575:25575"

  camera:
    build:
      context: .
      dockerfile: Dockerfile.onvif
    environment:
      - USERNAME=camera_player
      - HOST_IP=172.28.0.101
      - MINECRAFT_SERVER=172.28.0.2:25565
      - RCON_HOST=172.28.0.2
      - RCON_PASSWORD=minecraft
    ports:
      - "8081:8080"  # ONVIF
      - "5541:554"   # RTSP
    privileged: true
    depends_on:
      - minecraft-server
```

### Multi-Camera Setup

See `docker-compose.onvif.yml` for a full 3-camera configuration with different spawn positions.

## Development

### Project Structure

```
ansicraft/
├── minecraft_terminal_viewer/    # Library + SSH terminal viewer
│   ├── src/
│   │   ├── lib.rs                # Public API
│   │   ├── minecraft.rs          # Minecraft process management
│   │   ├── xdo.rs                # Input injection via xdotool
│   │   ├── render.rs             # Terminal rendering
│   │   └── bin/
│   │       └── termcast.rs       # SSH binary
│   └── Cargo.toml
│
├── onvif_camera/                 # ONVIF camera binary
│   ├── src/
│   │   ├── main.rs               # Camera entrypoint
│   │   ├── lib.rs                # ONVIF server
│   │   ├── rcon_client.rs        # RCON integration
│   │   ├── ptz_controller.rs    # PTZ → input mapping
│   │   ├── discovery.rs          # WS-Discovery
│   │   ├── models.rs             # ONVIF data structures
│   │   ├── soap.rs               # SOAP envelope handling
│   │   └── services/
│   │       ├── device.rs         # Device service
│   │       ├── media.rs          # Media service
│   │       └── ptz.rs            # PTZ service
│   ├── tests/
│   │   └── integration_test.rs   # ONVIF integration tests
│   └── Cargo.toml
│
├── docker-compose.yml            # Original SSH terminal viewer
├── docker-compose.onvif.yml      # ONVIF cameras + server
├── Dockerfile.onvif              # Camera container image
├── entrypoint-onvif.sh           # Camera startup script
└── go2rtc.yaml                   # RTSP server config
```

### Building

```bash
# Build library + binaries
cargo build

# Build only ONVIF camera
cd onvif_camera && cargo build

# Run tests
cargo test

# Run integration tests
cd onvif_camera && cargo test --test integration_test
```

### Running Locally (Without Docker)

```bash
# Start Xorg on :1
Xorg :1 -config xorg.conf &

# Start Minecraft
export DISPLAY=:1
export USERNAME=testplayer
export MINECRAFT_SERVER=localhost:25565
cd onvif_camera && cargo run
```

## Troubleshooting

### Camera Not Discovered

- Check network connectivity: `ping 172.28.0.101`
- Verify ONVIF port is accessible: `curl http://172.28.0.101:8080/onvif/device_service`
- Check Docker network: `docker network inspect ansicraft_camera-net`

### RTSP Stream Not Available

- Check go2rtc logs: `docker logs <container_id> | grep go2rtc`
- Verify X server is running: `docker exec <container_id> ps aux | grep Xorg`
- Test RTSP URL directly: `ffprobe rtsp://172.28.0.101:554/stream`

### Player Not Spawning at Correct Position

- Check RCON is enabled on server: `docker logs onvif-minecraft-server`
- Verify RCON credentials in camera environment
- Check camera logs for teleport messages: `docker logs <container_id> | grep teleport`
- Increase wait time: Player may join after initial teleport attempts

### PTZ Controls Not Working

- PTZ implementation is currently stubbed
- Full xdo integration pending
- Verify xdotool is available: `docker exec <container_id> which xdotool`

### Performance Issues

- Reduce resolution: Set `VIDEO_WIDTH=240 VIDEO_HEIGHT=150`
- Lower framerate: Set `FRAMERATE=20`
- Use hardware encoding if available (requires GPU passthrough)
- Limit number of cameras

### Minecraft Fails to Launch

- Check Java version: `docker exec <container_id> java -version`
- Verify minecraft-launcher-lib: `docker exec <container_id> pip3 list | grep minecraft`
- Check Python script: `docker exec <container_id> cat /root/launch_minecraft.py`
- View full logs: `docker logs <container_id>`

## Integration Examples

### Frigate Configuration

```yaml
cameras:
  minecraft_camera_1:
    enabled: true
    ffmpeg:
      inputs:
        - path: rtsp://172.28.0.101:554/stream
          roles:
            - detect
            - record
    detect:
      width: 320
      height: 200
```

### Blue Iris Setup

1. Right-click → Add new camera
2. Select "ONVIF/RTSP"
3. Camera should auto-discover as "Minecraft Camera 1"
4. Or manually:
   - IP: 172.28.0.101
   - RTSP Port: 554
   - Path: /stream
   - Username/Password: (leave empty)

### Home Assistant

```yaml
camera:
  - platform: onvif
    host: 172.28.0.101
    port: 8080
    name: Minecraft Camera 1
```

## Future Enhancements

- [ ] Complete PTZ xdo integration
- [ ] WS-Discovery service startup
- [ ] Lazy Minecraft startup (launch on first RTSP connection)
- [ ] Idle timeout (stop Minecraft after N minutes)
- [ ] Higher resolutions (720p, 1080p)
- [ ] Hardware encoding (VAAPI/NVENC)
- [ ] Multiple worlds / servers
- [ ] PTZ preset positions
- [ ] Web UI for management
- [ ] Wayland support (requires Minecraft Wayland backend)

## License

MIT

## Contributing

Contributions welcome! Please open an issue to discuss major changes.

## Credits

- Built on [ansicraft](https://github.com/user/ansicraft) - Minecraft terminal viewer
- Uses [go2rtc](https://github.com/AlexxIT/go2rtc) for RTSP streaming
- ONVIF protocol implementation with [yaserde](https://github.com/media-io/yaserde)
- Minecraft server via [itzg/minecraft-server](https://github.com/itzg/docker-minecraft-server)
