#!/bin/bash

set -e

# Directory to store minecraft data
MINECRAFT_DATA_DIR="$(pwd)/minecraft_data"

# Create minecraft data directory if it doesn't exist
mkdir -p "$MINECRAFT_DATA_DIR"

# echo "Building Docker container..."
docker build -t furrysalamander/minecraft-terminal .

# echo "Running Docker container..."
docker run -it --rm \
  -p 9867:2222 \
  -v "$MINECRAFT_DATA_DIR:/root/.minecraft" \
  furrysalamander/minecraft-terminal

echo "Container exited."
