FROM debian:latest

# Install dependencies
RUN apt-get update && apt-get install -y \
    xserver-xorg-core \
    xserver-xorg-video-dummy \
    x11-xserver-utils \
    openjdk-17-jre \
    git unzip jq curl ffmpeg golang wget xdotool

# Clone Minecraft launcher
WORKDIR /root
RUN git clone https://github.com/alexivkin/minecraft-launcher
RUN minecraft-launcher/start 1.18.2 docker || true

# Add dummy xorg.conf
COPY xorg.conf /etc/X11/xorg.conf.dummy

# Install Rust and build Rust webcam viewer
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy and build Rust webcam viewer
COPY minecraft_terminal_viewer /root/minecraft_terminal_viewer
WORKDIR /root/minecraft_terminal_viewer
RUN cargo build --release
RUN cp /root/minecraft_terminal_viewer/target/release/minecraft_terminal_viewer /root/termcast

# Add entrypoint
WORKDIR /root
COPY --chmod=0755 entry-point.sh /root/entry-point.sh

ENTRYPOINT ["/root/entry-point.sh"]
