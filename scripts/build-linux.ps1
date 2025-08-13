# PowerShell script for Linux x86_64 cross-compilation
# This script builds the office reader MCP server for Linux from Windows

$ErrorActionPreference = "Stop"

Write-Host "Building Office Reader MCP Server for Linux x86_64..." -ForegroundColor Green
Write-Host "======================================================" -ForegroundColor Green

# Check if cross is installed
try {
    cross --version | Out-Null
} catch {
    Write-Host "Error: cross is not installed. Please run:" -ForegroundColor Red
    Write-Host "cargo install cross --git https://github.com/cross-rs/cross" -ForegroundColor Yellow
    exit 1
}

# Check if Docker is running (required for cross)
try {
    docker info | Out-Null
} catch {
    Write-Host "Error: Docker is not running. Please start Docker Desktop." -ForegroundColor Red
    exit 1
}

# Clean previous builds
Write-Host "Cleaning previous builds..." -ForegroundColor Yellow
cargo clean

# Build for Linux x86_64 with optimizations
Write-Host "Building for x86_64-unknown-linux-gnu..." -ForegroundColor Yellow
cross build --target x86_64-unknown-linux-gnu --release --features pdfium

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ Build successful!" -ForegroundColor Green
    Write-Host "Binary location: target/x86_64-unknown-linux-gnu/release/office_reader_mcp" -ForegroundColor Cyan
    
    # Show binary info
    Write-Host ""
    Write-Host "Binary information:" -ForegroundColor Yellow
    $binaryPath = "target/x86_64-unknown-linux-gnu/release/office_reader_mcp"
    if (Test-Path $binaryPath) {
        $fileInfo = Get-Item $binaryPath
        Write-Host "Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Cyan
        Write-Host "Modified: $($fileInfo.LastWriteTime)" -ForegroundColor Cyan
    }
    
    Write-Host ""
    Write-Host "To run on Linux:" -ForegroundColor Yellow
    Write-Host "./target/x86_64-unknown-linux-gnu/release/office_reader_mcp" -ForegroundColor Cyan
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
