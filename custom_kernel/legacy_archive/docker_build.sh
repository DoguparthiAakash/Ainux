#!/bin/bash
set -e

# Build the Docker image
echo "Building Docker image..."
docker build -t ainux-builder .

# Run the build inside the container
echo "Running build..."
docker run --rm -v "$(pwd):/root/env" ainux-builder make os.iso

echo "Build complete. os.iso is ready."
