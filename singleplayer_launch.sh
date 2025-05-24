#!/bin/bash

set -e

# Directory to store minecraft data
MINECRAFT_DATA_DIR="$(pwd)/.minecraft"

# Build the Docker image
echo "Building Docker image..."
docker build -t minecraft-terminal .

echo "Starting Docker container..."
docker run --rm -it -v "$MINECRAFT_DATA_DIR:/root/.minecraft" minecraft-terminal
