FROM debian:latest

RUN apt-get update && apt-get install -y xvfb ffmpeg
RUN apt-get update && apt-get install -y openjdk-17-jre git unzip jq curl

WORKDIR /root
RUN git clone https://github.com/alexivkin/minecraft-launcher
RUN minecraft-launcher/start 1.18.2 docker; exit 0

# I couldn't get the game to automatically connect to a server without crashing.
# WORKDIR /root/server
# ADD https://launcher.mojang.com/v1/objects/c8f83c5655308435b3dcf03c06d9fe8740a77469/server.jar /root/server
# RUN java -jar server.jar nogui
# RUN sed -i 's/false/true/g' eula.txt
# RUN sed -i 's/online-mode=true/online-mode=false/g' server.properties

WORKDIR /root
RUN apt-get install -y wget xdotool
RUN wget https://github.com/aler9/rtsp-simple-server/releases/download/v0.21.2/rtsp-simple-server_v0.21.2_linux_amd64.tar.gz -O rtsp-simple-server.tar.gz

# Setup the RTSP server
RUN tar -xzvf rtsp-simple-server.tar.gz
RUN sed -i '$s/$/ --fullscreen/' minecraft-launcher/start

COPY --chmod=0755 entry-point.sh /root/entry-point.sh
ENTRYPOINT /root/entry-point.sh


# I had initially tried these launchers, but it wasn't successful.
# RUN wget https://launcher.mojang.com/download/Minecraft.deb -O /tmp/Minecraft.deb
# RUN apt-get install -y /tmp/Minecraft.deb

# ADD https://files.multimc.org/downloads/multimc_1.6-1.deb /root/
# RUN apt-get install -y /root/multimc_1.6-1.deb
# Neither of these worked
