#!/bin/bash

# For some stupid reason, the python script can't resolve DNS unless we override what docker sets.
echo "nameserver 8.8.8.8" > /etc/resolv.conf

# Start multiple Xorg instances with dummy drivers for different displays
# We'll start X servers on :1 through :10 to support multiple seats
for i in {1..10}; do
    Xorg ":$i" -noreset -logfile "/tmp/xorg$i.log" -config /etc/X11/xorg.conf.dummy &
done
sleep 2

# Start terminal viewer
RUST_BACKTRACE=full /root/termcast
