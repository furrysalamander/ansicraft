#!/bin/bash

export DISPLAY=:1

# For some stupid reason, the python script can't resolve DNS unless we override what docker sets.
echo "nameserver 8.8.8.8" > /etc/resolv.conf

# Start Xorg with dummy driver
Xorg "$DISPLAY" -noreset -logfile /tmp/xorg.log -config /etc/X11/xorg.conf.dummy &
sleep 2

# python3 /root/launch_minecraft.py &

# Start terminal viewer
RUST_BACKTRACE=full /root/termcast
