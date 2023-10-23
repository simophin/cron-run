#!/bin/bash

# Exit on error
set -e

# Detect CPU arch
ARCH=$(uname -m)

# Download latest version of the binary from Github
curl -s https://api.github.com/repos/alexellis/faas/releases/latest \