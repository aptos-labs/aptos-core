#!/bin/bash
# Setup script for benchmark analysis suite

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Setting up Interner Benchmark Analysis Suite"
echo "=============================================="
echo ""

# Check Python
if ! command -v python3 &> /dev/null; then
    echo -e "${YELLOW}Warning: python3 not found${NC}"
    echo "Please install Python 3.8 or later"
    exit 1
fi

echo -e "${GREEN}✓${NC} Python 3 found: $(python3 --version)"

# Check pip
if ! command -v pip3 &> /dev/null; then
    echo -e "${YELLOW}Warning: pip3 not found${NC}"
    echo "Please install pip"
    exit 1
fi

echo -e "${GREEN}✓${NC} pip3 found"

# Install Python dependencies
echo ""
echo "Installing Python dependencies..."
pip3 install -r requirements.txt || {
    echo -e "${YELLOW}Warning: Some packages failed to install${NC}"
    echo "You may need to install system dependencies first"
    exit 1
}

echo -e "${GREEN}✓${NC} Python dependencies installed"

# Check Rust/Cargo
echo ""
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Warning: cargo not found${NC}"
    echo "Please install Rust: https://rustup.rs/"
else
    echo -e "${GREEN}✓${NC} Cargo found: $(cargo --version)"
fi

# Make scripts executable
echo ""
echo "Making scripts executable..."
chmod +x run_benchmarks.sh
chmod +x *.py

echo -e "${GREEN}✓${NC} Scripts are executable"

# Optional tools
echo ""
echo "Checking optional tools..."

if command -v perf &> /dev/null; then
    echo -e "${GREEN}✓${NC} perf found (profiling available)"
else
    echo -e "${YELLOW}○${NC} perf not found (profiling will be limited)"
fi

if command -v cpupower &> /dev/null; then
    echo -e "${GREEN}✓${NC} cpupower found (CPU tuning available)"
else
    echo -e "${YELLOW}○${NC} cpupower not found (CPU tuning will be skipped)"
fi

if command -v flamegraph &> /dev/null; then
    echo -e "${GREEN}✓${NC} flamegraph found"
else
    echo -e "${YELLOW}○${NC} flamegraph not found (install: cargo install flamegraph)"
fi

echo ""
echo "=============================================="
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "To run benchmarks:"
echo "  ./run_benchmarks.sh"
echo ""
echo "For help:"
echo "  ./run_benchmarks.sh --help"
echo "  cat README.md"
echo ""
