#!/bin/bash

# Ensure X11 directory exists
mkdir -p "/tmp/.X11-unix"

# Clean up any existing X server lock files for display :1
rm -f "/tmp/.X1-lock"
rm -f "/tmp/.X11-unix/X1"

# Start Xorg on display :1
echo "Starting Xorg on :1..."
Xorg :1 -noreset -logfile "/tmp/xorg.log" -config /etc/X11/xorg.conf &
sleep 2

# Set invisible cursor
echo "Setting blank cursor..."
DISPLAY=:1 xsetroot -cursor /root/blank_cursor.xbm /root/blank_cursor.xbm || true

# Start go2rtc RTSP server
echo "Starting go2rtc..."
/usr/local/bin/go2rtc -c /root/go2rtc.yaml &
sleep 2

# Start ONVIF camera server
echo "Starting ONVIF camera..."
RUST_BACKTRACE=full /usr/local/bin/onvif_camera
