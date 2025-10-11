#!/bin/bash
# Build script for WebAssembly UI

set -e

echo "🦊 Building Chrome to Firefox Converter WebAssembly UI"
echo "======================================================="
echo ""

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed!"
    echo ""
    echo "Install it with:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    echo ""
    exit 1
fi

echo "✅ wasm-pack found"
echo ""

# Build the WASM module
echo "📦 Building WASM module..."
wasm-pack build --target web --out-dir web/pkg --no-default-features

if [ $? -eq 0 ]; then
    echo "✅ WASM module built successfully!"
    echo ""
    echo "📁 Output location: ./web/pkg/"
    echo ""
    echo "🚀 To run the web UI:"
    echo ""
    echo "  Option 1 - Using Python:"
    echo "    cd web && python3 -m http.server 8080"
    echo ""
    echo "  Option 2 - Using Node.js (http-server):"
    echo "    npx http-server web -p 8080"
    echo ""
    echo "  Option 3 - Using Rust (miniserve):"
    echo "    cargo install miniserve"
    echo "    miniserve web -p 8080"
    echo ""
    echo "Then open: http://localhost:8080"
    echo ""
else
    echo "❌ Build failed!"
    exit 1
fi