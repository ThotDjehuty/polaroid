#!/bin/bash
set -e

echo "🚀 Building Polaroid for Serverless Deployment"
echo "=============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check dependencies
echo -e "${YELLOW}📦 Checking dependencies...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Optional: Check for cloud-specific tools
CARGO_LAMBDA=$(command -v cargo-lambda &> /dev/null && echo "yes" || echo "no")
WASM_OPT=$(command -v wasm-opt &> /dev/null && echo "yes" || echo "no")

echo -e "${GREEN}✅ Cargo found${NC}"
echo "   cargo-lambda: $CARGO_LAMBDA"
echo "   wasm-opt: $WASM_OPT"

# Navigate to serverless directory
cd "$(dirname "$0")/.."
SERVERLESS_DIR="$(pwd)/serverless"

if [ ! -d "$SERVERLESS_DIR" ]; then
    echo -e "${RED}❌ Serverless directory not found: $SERVERLESS_DIR${NC}"
    exit 1
fi

cd "$SERVERLESS_DIR"

# Create output directory
mkdir -p dist

echo ""
echo -e "${YELLOW}📦 Building generic HTTP server (works everywhere)...${NC}"
cargo build --release --bin polaroid-http --features generic-http
if [ $? -eq 0 ]; then
    cp target/release/polaroid-http dist/
    SIZE=$(du -h dist/polaroid-http | cut -f1)
    echo -e "${GREEN}✅ Generic HTTP server built: dist/polaroid-http ($SIZE)${NC}"
else
    echo -e "${RED}❌ Generic HTTP server build failed${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}☁️  Skipping cloud-specific builds (dependency conflicts)...${NC}"
echo "   To enable:"
echo "   - Azure: Fix tower-service version conflicts"
echo "   - AWS: Install cargo-lambda and update dependencies"
echo "   - GCP: Update actix-web dependencies"
echo "   - Cloudflare: Install wasm32 target"

# echo ""
# echo -e "${YELLOW}☁️  Building for Azure Functions...${NC}"
# cargo build --release --bin polaroid-azure --features azure --target x86_64-unknown-linux-musl 2>/dev/null || \
# cargo build --release --bin polaroid-azure --features azure
# cargo build --release --bin polaroid-azure --features azure
if [ $? -eq 0 ]; then
    # cp target/*/release/polaroid-azure dist/ 2>/dev/null || cp target/release/polaroid-azure dist/
    SIZE=$(du -h dist/polaroid-azure | cut -f1)
    echo -e "${GREEN}✅ Azure Functions build: dist/polaroid-azure ($SIZE)${NC}"
# else
    # echo -e "${YELLOW}⚠️  Azure build failed (dependencies missing), skipping...${NC}"
fi

# Commented out until dependencies are fixed
# echo ""
# if [ "$CARGO_LAMBDA" = "yes" ]; then
# if [ "$CARGO_LAMBDA" = "yes" ]; then
    echo -e "${YELLOW}🚀 Building for AWS Lambda (with cargo-lambda)...${NC}"
    cargo lambda build --release --bin polaroid-lambda --features aws
    if [ $? -eq 0 ]; then
        cp target/lambda/polaroid-lambda/bootstrap.zip dist/polaroid-lambda.zip
        SIZE=$(du -h dist/polaroid-lambda.zip | cut -f1)
        echo -e "${GREEN}✅ AWS Lambda build: dist/polaroid-lambda.zip ($SIZE)${NC}"
    else
        echo -e "${RED}❌ AWS Lambda build failed${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  cargo-lambda not installed, skipping AWS build${NC}"
    echo "   Install: cargo install cargo-lambda"
fi

echo ""
echo -e "${YELLOW}🌐 Building for Google Cloud Functions...${NC}"
cargo build --release --bin polaroid-gcp --features gcp --target x86_64-unknown-linux-gnu 2>/dev/null || \
cargo build --release --bin polaroid-gcp --features gcp
if [ $? -eq 0 ]; then
    cp target/*/release/polaroid-gcp dist/ 2>/dev/null || cp target/release/polaroid-gcp dist/
    SIZE=$(du -h dist/polaroid-gcp | cut -f1)
    echo -e "${GREEN}✅ Google Cloud Functions build: dist/polaroid-gcp ($SIZE)${NC}"
else
    echo -e "${YELLOW}⚠️  GCP build failed (dependencies missing), skipping...${NC}"
fi

echo ""
echo -e "${YELLOW}⚡ Building for Cloudflare Workers (WASM)...${NC}"
cargo build --release --bin polaroid-cloudflare --features cloudflare --target wasm32-unknown-unknown 2>/dev/null
if [ $? -eq 0 ]; then
    cp target/wasm32-unknown-unknown/release/polaroid_cloudflare.wasm dist/polaroid.wasm
    
    if [ "$WASM_OPT" = "yes" ]; then
        echo "   Optimizing WASM with wasm-opt..."
        wasm-opt -Oz -o dist/polaroid-optimized.wasm dist/polaroid.wasm
        mv dist/polaroid-optimized.wasm dist/polaroid.wasm
    fi
    
    SIZE=$(du -h dist/polaroid.wasm | cut -f1)
    echo -e "${GREEN}✅ Cloudflare Workers build: dist/polaroid.wasm ($SIZE)${NC}"
else
    echo -e "${YELLOW}⚠️  Cloudflare Workers build failed (wasm32 target missing), skipping...${NC}"
    echo "   Install: rustup target add wasm32-unknown-unknown"
fi

echo ""
echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}✅ Serverless build complete!${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo "📁 Artifacts in dist/:"
ls -lh dist/ 2>/dev/null | grep -E "polaroid" || echo "   No artifacts found"
echo ""
echo "🚀 Deployment options:"
echo "   • Generic HTTP:       ./scripts/deploy-docker.sh"
echo "   • Azure Functions:    ./scripts/deploy-azure.sh"
echo "   • AWS Lambda:         ./scripts/deploy-aws.sh"
echo "   • Google Cloud:       ./scripts/deploy-gcp.sh"
echo "   • Cloudflare Workers: ./scripts/deploy-cloudflare.sh"
echo ""
echo "📚 Documentation: ../README.md"
