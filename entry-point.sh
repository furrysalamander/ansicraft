#!/bin/bash

export DISPLAY=:1

# Configure Minecraft options
echo "rawMouseInput:false" > /root/minecraft-launcher/profiles/docker-base-1.18.2/options.txt

# Start Xorg with dummy driver
echo "Starting Xorg with dummy video driver..."
Xorg "$DISPLAY" -noreset -logfile /tmp/xorg.log -config /etc/X11/xorg.conf.dummy &
sleep 2

# Start Minecraft
echo "Starting Minecraft..."
/root/minecraft-launcher/start 1.18.2 docker >/dev/null 2>&1 &
sleep 15

# Interact with Minecraft
echo "Sending input..."
xdotool mousemove 640 360 click 1
sleep 1
xdotool key Return
xdotool key F11

# Start terminal viewer
echo "Starting terminal-based Minecraft viewer..."
/root/termcast
clear
