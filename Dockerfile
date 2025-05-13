# Build stage
FROM debian:sid-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev curl

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a new empty project
WORKDIR /root/minecraft_terminal_viewer
# First, copy just the Cargo files
COPY minecraft_terminal_viewer/Cargo.toml minecraft_terminal_viewer/Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() { println!(\"Dummy build\"); }" > src/main.rs && \
    cargo build --release && \
    rm -rf src/

# Now copy the actual source code
COPY minecraft_terminal_viewer/src ./src

# Build the application (dependencies are now cached)
RUN cargo clean --release --package minecraft_terminal_viewer && cargo build --release

# Runtime stage
FROM debian:sid-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    xserver-xorg-core \
    xserver-xorg-video-dummy \
    x11-xserver-utils \
    openjdk-21-jre \
    ffmpeg xdotool git python3 python3-pip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install minecraft-launcher-lib
RUN pip3 install --break-system-packages minecraft-launcher-lib

# Create Minecraft directory
WORKDIR /root/.minecraft
RUN mkdir -p /root/.minecraft

# Add dummy xorg.conf
COPY xorg.conf /etc/X11/xorg.conf.dummy

# Copy built binary from builder stage
COPY --from=builder /root/minecraft_terminal_viewer/target/release/minecraft_terminal_viewer /root/termcast

# Copy launcher script
COPY launch_minecraft.py /root/launch_minecraft.py

# Add entrypoint
COPY --chmod=0755 entry-point.sh /root/entry-point.sh

# IDK why, but this is needed to make the launcher work.

ENTRYPOINT ["/root/entry-point.sh"]
