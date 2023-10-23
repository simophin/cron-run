#!/bin/bash

# Exit on error
set -e

if [[ $(uname -s) != "Linux" ]]; then
    echo "$0 is only supported on Linux"
    exit 1
fi

# Detect CPU arch
ARCH=$(uname -m)

get_latest_release() {
  curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
    grep '"tag_name":' |                                            # Get tag line
    sed -E 's/.*"([^"]+)".*/\1/'                                    # Pluck JSON value
}

LATEST_VERSION="`get_latest_release simophin/cron-run`"

# Download latest version of the binary from Github
echo "Downloading latest version of cron-run for $ARCH..."

if [[ -w /usr/local/bin ]]; then
    echo "Installing to /usr/local/bin..."
    curl -fL "https://github.com/simophin/cron-run/releases/download/$LATEST_VERSION/cron-run.$ARCH" -o /usr/local/bin/cron-run
    chmod +x /usr/local/bin/cron-run
else
    echo "Installing to /usr/local/bin (requires sudo)..."
    sudo "curl -fL "https://github.com/simophin/cron-run/releases/download/$LATEST_VERSION/cron-run.$ARCH" -o /usr/local/bin/cron-run && \
            chmod +x /usr/local/bin/cron-run"
fi

echo "Done installing cron-run on $ARCH"