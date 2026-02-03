#!/bin/bash
# Record demo video
# Run from this directory: ./record.sh

set -e

DEMO_DIR="/tmp/dialogdetective-demo"

echo "==> Cleaning up previous demo environment..."
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR"

echo "==> Copying mock script..."
cp mock_dialog_detective.sh "$DEMO_DIR/"
chmod +x "$DEMO_DIR/mock_dialog_detective.sh"

echo "==> Recording demo..."
vhs demo.tape

echo "==> Moving outputs to pages assets..."
mv demo.webm demo.mp4 ../pages/assets/ 2>/dev/null || true

echo "==> Cleaning up temp directory..."
rm -rf "$DEMO_DIR"

echo "==> Done! Videos are in docs/pages/assets/"
