#!/bin/bash

# Build script for Linux x86_64 cross-compilation
# This script builds the office reader MCP server for Linux

set -e

echo "Building Office Reader MCP Server for Linux x86_64..."
echo "======================================================"

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "Error: cross is not installed. Please run:"
    echo "cargo install cross --git https://github.com/cross-rs/cross"
    exit 1
fi

# Check if Docker is running (required for cross)
if ! docker info &> /dev/null; then
    echo "Error: Docker is not running. Please start Docker Desktop."
    exit 1
fi

# Clean previous builds
echo "Cleaning previous builds..."
cargo clean

# Build for Linux x86_64 with optimizations
echo "Building for x86_64-unknown-linux-gnu..."
cross build --target x86_64-unknown-linux-gnu --release --features pdfium

# Check if build was successful
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    echo "Binary location: target/x86_64-unknown-linux-gnu/release/office_reader_mcp"
    
    # Show binary info
    echo ""
    echo "Binary information:"
    ls -lh target/x86_64-unknown-linux-gnu/release/office_reader_mcp
    file target/x86_64-unknown-linux-gnu/release/office_reader_mcp || echo "file command not available"
    
    echo ""
    echo "To run on Linux:"
    echo "./target/x86_64-unknown-linux-gnu/release/office_reader_mcp"
else
    echo "❌ Build failed!"
    exit 1
fi
