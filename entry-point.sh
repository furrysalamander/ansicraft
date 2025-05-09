#!/bin/bash

# Start minecraft in the background using xvfb
echo "rawMouseInput:false" > /root/minecraft-launcher/profiles/docker-base-1.18.2/options.txt

xvfb-run --listen-tcp --server-num 44 --auth-file /tmp/xvfb.auth -s "-ac -screen 0 1280x720x24 +extension XTEST" minecraft-launcher/start 1.18.2 docker >/dev/null 2>&1 &
# Wait for minecraft to start up
sleep 30 # If your computer loads the world slower or faster, this can be adjusted.
DISPLAY=:44 xdotool click 1
sleep 1
DISPLAY=:44 xdotool key Return

# Start our terminal-based Minecraft viewer with direct X11 capture
echo "Starting terminal-based Minecraft viewer..."
cd /root
./termcast
clear
