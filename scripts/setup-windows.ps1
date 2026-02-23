# Ledger — Windows Dependency Setup (PowerShell)
# Run as Administrator

Write-Host "=== Ledger Development Environment Setup (Windows) ===" -ForegroundColor Cyan

# Check for winget
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: winget not found. Please install App Installer from Microsoft Store." -ForegroundColor Red
    exit 1
}

# Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rust..."
    winget install -e --id Rustlang.Rustup
    Write-Host "Please restart your terminal after Rust installation, then re-run this script."
} else {
    Write-Host "Rust already installed: $(rustc --version)"
}

# .NET 8 SDK
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
    Write-Host "Installing .NET 8 SDK..."
    winget install -e --id Microsoft.DotNet.SDK.8
} else {
    Write-Host ".NET already installed: $(dotnet --version)"
}

# Java 21
if (-not (Get-Command java -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Java 21..."
    winget install -e --id Microsoft.OpenJDK.21
} else {
    Write-Host "Java already installed: $(java --version 2>&1 | Select-Object -First 1)"
}

# Maven
if (-not (Get-Command mvn -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Maven..."
    winget install -e --id Apache.Maven
} else {
    Write-Host "Maven already installed."
}

# Python 3
if (-not (Get-Command python3 -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Python 3..."
    winget install -e --id Python.Python.3.12
} else {
    Write-Host "Python already installed: $(python3 --version)"
}

# Visual Studio Build Tools (for Rust MSVC)
Write-Host ""
Write-Host "NOTE: Rust on Windows requires Visual Studio Build Tools with C++ workload." -ForegroundColor Yellow
Write-Host "If not installed, run: winget install -e --id Microsoft.VisualStudio.2022.BuildTools"

Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Green
Write-Host "Next steps:"
Write-Host "  cd ledger-core; cargo build"
Write-Host "  cd ledger-ui; dotnet build"
Write-Host "  cd ledger-plugins; mvn package"
Write-Host "  cd ledger-cli; pip install -r requirements.txt"
