#!/bin/bash

set -e

# Directory to store minecraft data
MINECRAFT_DATA_DIR="$(pwd)/.minecraft"
MINECRAFT_SERVER_DATA_DIR="$(pwd)/minecraft_server_data"

# Create minecraft data directories if they don't exist
mkdir -p "$MINECRAFT_DATA_DIR" "$MINECRAFT_SERVER_DATA_DIR"

# Optional server address parameter
# If provided, use it as the server address instead of the container name
if [ $# -gt 0 ]; then
  export MINECRAFT_SERVER_ADDRESS="$1"
  echo "Using custom Minecraft server address: $MINECRAFT_SERVER_ADDRESS"
else
  echo "Using default Minecraft server address (container name)"
fi

echo "Cleaning up any previous Docker resources..."
docker compose down --remove-orphans

# Ensure the minecraft-network exists
if ! docker network ls | grep -q minecraft-network; then
  echo "Creating minecraft-network..."
  docker network create minecraft-network
fi

echo "Building and starting Docker containers..."
docker compose up --build

echo "Containers exited."