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

# Add and build Go webcam viewer
COPY webcam_go /root/webcam_go
WORKDIR /root/webcam_go
RUN go build -o /root/termcast

# Add entrypoint
WORKDIR /root
COPY --chmod=0755 entry-point.sh /root/entry-point.sh

ENTRYPOINT ["/root/entry-point.sh"]
