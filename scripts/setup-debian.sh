#!/usr/bin/env bash
# Ledger — Debian/Ubuntu Dependency Setup
set -euo pipefail

echo "=== Ledger Development Environment Setup (Debian/Ubuntu) ==="

# System packages
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    protobuf-compiler \
    curl \
    git \
    wget

# Rust (via rustup)
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust already installed: $(rustc --version)"
fi

# .NET 8 SDK
if ! command -v dotnet &> /dev/null; then
    echo "Installing .NET 8 SDK..."
    wget https://dot.net/v1/dotnet-install.sh -O /tmp/dotnet-install.sh
    chmod +x /tmp/dotnet-install.sh
    /tmp/dotnet-install.sh --channel 8.0
    export DOTNET_ROOT="$HOME/.dotnet"
    export PATH="$PATH:$DOTNET_ROOT:$DOTNET_ROOT/tools"
    echo 'export DOTNET_ROOT="$HOME/.dotnet"' >> ~/.bashrc
    echo 'export PATH="$PATH:$DOTNET_ROOT:$DOTNET_ROOT/tools"' >> ~/.bashrc
else
    echo ".NET already installed: $(dotnet --version)"
fi

# Java 21
if ! command -v java &> /dev/null; then
    echo "Installing Java 21..."
    sudo apt-get install -y openjdk-21-jdk
else
    echo "Java already installed: $(java --version | head -1)"
fi

# Maven
if ! command -v mvn &> /dev/null; then
    echo "Installing Maven..."
    sudo apt-get install -y maven
else
    echo "Maven already installed: $(mvn --version | head -1)"
fi

# Python 3 + pip
if ! command -v python3 &> /dev/null; then
    echo "Installing Python 3..."
    sudo apt-get install -y python3 python3-pip python3-venv
else
    echo "Python already installed: $(python3 --version)"
fi

echo ""
echo "=== Setup Complete ==="
echo "Next steps:"
echo "  cd ledger-core && cargo build"
echo "  cd ledger-ui && dotnet build"
echo "  cd ledger-plugins && mvn package"
echo "  cd ledger-cli && pip install -r requirements.txt"
