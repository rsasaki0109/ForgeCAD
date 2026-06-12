#!/usr/bin/env bash
# Install system OpenCASCADE (OCCT) development packages on Debian/Ubuntu.
# OpenCAD MVP uses cadrum's static OCCT by default; this script is for
# developers who want distro packages or a future direct C++ bridge.
set -euo pipefail

if ! command -v apt-get >/dev/null 2>&1; then
  echo "apt-get not found. Install OCCT manually for your distribution."
  echo "See docs/developer-guide/occt-install.md"
  exit 1
fi

echo "Installing OCCT 7.6 development packages..."
sudo apt-get update
sudo apt-get install -y \
  pkg-config \
  libocct-foundation-dev \
  libocct-modeling-data-dev \
  libocct-modeling-algorithms-dev \
  libocct-data-exchange-dev

echo "Done. Verify with: dpkg -l 'libocct-*-dev'"
