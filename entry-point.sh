#!/bin/bash

# Start minecraft server
# I gave up on this for now because this:
# https://gaming.stackexchange.com/a/348749
# would cause the game to throw a null string error and I couldn't figure out why.
# cd server
# java -jar server.jar nogui &
# cd ..

# Start minecraft
xvfb-run --listen-tcp --server-num 44 --auth-file /tmp/xvfb.auth -s "-ac -screen 0 1280x720x24" minecraft-launcher/start 1.18.2 docker &

rtspServer=127.0.0.1:rtsp://127.0.0.1:8554/minecraftStream ./rtsp-simple-server &

ffmpeg -f x11grab -video_size 1280x720 -i :44 -f rtsp -rtsp_transport tcp rtsp://127.0.0.1:8554/minecraftStream &

# This is a hack, I wish I could just load straight into a server, but the aforementioned issue is preventing that.
# A good solution would be to try different minecraft versions.  I think that maybe an older version wouldn't have issues.
# It may also be due to the fact that we're unauthenticated.  Using the --server flag seemed to work just fine on my laptop
# when not running in the container.
sleep 45 # If your computer loads the world slower or faster, this can be adjusted.
DISPLAY=:44 xdotool click 1
sleep 1
DISPLAY=:44 xdotool key Return

while true; do
    sleep 10
done
