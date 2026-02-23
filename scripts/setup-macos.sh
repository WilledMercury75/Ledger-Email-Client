#!/usr/bin/env bash
# Ledger — macOS Dependency Setup
set -euo pipefail

echo "=== Ledger Development Environment Setup (macOS) ==="

# Homebrew
if ! command -v brew &> /dev/null; then
    echo "Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
else
    echo "Homebrew already installed."
fi

# System dependencies
brew install openssl sqlite protobuf pkg-config

# Rust
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
    brew install dotnet-sdk
else
    echo ".NET already installed: $(dotnet --version)"
fi

# Java 21
if ! command -v java &> /dev/null; then
    echo "Installing Java 21..."
    brew install openjdk@21
    sudo ln -sfn "$(brew --prefix openjdk@21)/libexec/openjdk.jdk" /Library/Java/JavaVirtualMachines/openjdk-21.jdk
else
    echo "Java already installed: $(java --version | head -1)"
fi

# Maven
if ! command -v mvn &> /dev/null; then
    echo "Installing Maven..."
    brew install maven
else
    echo "Maven already installed."
fi

# Python 3
if ! command -v python3 &> /dev/null; then
    echo "Installing Python 3..."
    brew install python@3.12
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
