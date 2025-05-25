#!/bin/bash

# ensure that the x11 directory exists so the server doesn't print any errors
mkdir -p "/tmp/.X11-unix"

# Clean up any existing X server lock files
for i in {1..10}; do
    rm -f "/tmp/.X$i-lock"
    rm -f "/tmp/.X11-unix/X$i"
done

# Start multiple Xorg instances with dummy drivers for different displays
# We'll start X servers on :1 through :10 to support multiple seats
for i in {1..10}; do
    Xorg ":$i" -noreset -logfile "/tmp/xorg$i.log" -config /etc/X11/xorg.conf.dummy &
done
sleep 2

# Set invisible cursor for each display
for i in {1..10}; do
    DISPLAY=:$i xsetroot -cursor /root/blank_cursor.xbm /root/blank_cursor.xbm || true
done

# Start terminal viewer
RUST_BACKTRACE=full /root/termcast
