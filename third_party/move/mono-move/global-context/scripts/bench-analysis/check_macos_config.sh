#!/bin/bash
# macOS system configuration checker for benchmarking
# Helps identify potential issues before running benchmarks

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ISSUES=0
WARNINGS=0

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((ISSUES++))
}

check_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_header "macOS Benchmark Configuration Check"

# System Information
print_header "System Information"

# Get CPU info
CPU_BRAND=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Unknown")
check_info "CPU: $CPU_BRAND"

# Check if Intel or Apple Silicon
if echo "$CPU_BRAND" | grep -q "Intel"; then
    check_info "Architecture: Intel x86_64"
    IS_INTEL=true
elif echo "$CPU_BRAND" | grep -q "Apple"; then
    check_info "Architecture: Apple Silicon (ARM64)"
    IS_INTEL=false
else
    check_warn "Unknown CPU architecture"
    IS_INTEL=false
fi

# CPU cores
PHYS_CORES=$(sysctl -n hw.physicalcpu)
LOGICAL_CORES=$(sysctl -n hw.logicalcpu)
check_info "Physical cores: $PHYS_CORES"
check_info "Logical cores: $LOGICAL_CORES"

# Memory
MEMORY_GB=$(( $(sysctl -n hw.memsize) / 1024 / 1024 / 1024 ))
check_info "Memory: ${MEMORY_GB} GB"

# macOS Version
MACOS_VERSION=$(sw_vers -productVersion)
check_info "macOS Version: $MACOS_VERSION"

# Turbo Boost Check
print_header "Turbo Boost / Performance Settings"

if [ "$IS_INTEL" = true ]; then
    # Check turbo boost status (various methods)
    TURBO_STATUS="unknown"

    # Method 1: Check for Turbo Boost Switcher
    if pgrep -x "Turbo Boost Switcher" > /dev/null; then
        check_pass "Turbo Boost Switcher is running"
        check_warn "Check if Turbo Boost is disabled in the app menu"
    else
        check_warn "Turbo Boost Switcher not running (recommended for Intel Macs)"
        check_info "Install: brew install --cask turbo-boost-switcher"
    fi

    # Method 2: Check sysctl
    if sysctl machdep.xcpm.cpu_turbo &>/dev/null; then
        TURBO_VAL=$(sysctl -n machdep.xcpm.cpu_turbo 2>/dev/null || echo "unknown")
        if [ "$TURBO_VAL" = "0" ]; then
            check_pass "Turbo Boost disabled via sysctl"
        elif [ "$TURBO_VAL" = "1" ]; then
            check_fail "Turbo Boost is ENABLED - this will cause variance!"
            check_info "Disable: sudo sysctl -w machdep.xcpm.cpu_turbo=0"
        fi
    fi
else
    check_info "Apple Silicon detected - turbo boost control not applicable"
    check_info "Ensure 'High Power' mode is enabled in System Preferences → Battery"
fi

# Background Processes
print_header "Background Processes"

# Spotlight
if mdutil -s / 2>/dev/null | grep -q "Indexing disabled"; then
    check_pass "Spotlight indexing is disabled"
else
    check_fail "Spotlight indexing is ENABLED - will cause CPU noise"
    check_info "Disable: sudo mdutil -a -i off"
fi

# Time Machine
if tmutil status 2>/dev/null | grep -q "Running = 0"; then
    check_pass "Time Machine is not running"
else
    check_warn "Time Machine may be running"
    check_info "Disable: sudo tmutil disable"
fi

# Check for caffeinate
if pgrep -x caffeinate > /dev/null; then
    check_pass "caffeinate is running (system won't sleep)"
else
    check_warn "caffeinate not running - system may sleep during benchmarks"
    check_info "Run: caffeinate -i &"
fi

# App Nap
APP_NAP=$(defaults read NSGlobalDomain NSAppSleepDisabled 2>/dev/null || echo "0")
if [ "$APP_NAP" = "1" ]; then
    check_pass "App Nap is disabled"
else
    check_warn "App Nap is enabled - may affect performance"
    check_info "Disable: sudo defaults write NSGlobalDomain NSAppSleepDisabled -bool YES"
fi

# Development Tools
print_header "Development Tools"

# Xcode Command Line Tools
if xcode-select -p &>/dev/null; then
    check_pass "Xcode Command Line Tools installed"
else
    check_fail "Xcode Command Line Tools NOT installed"
    check_info "Install: xcode-select --install"
fi

# Rust
if command -v rustc &>/dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    check_pass "Rust installed: $RUST_VERSION"

    # Check if native architecture
    RUST_HOST=$(rustc --version --verbose | grep host | awk '{print $2}')
    if [ "$IS_INTEL" = true ] && echo "$RUST_HOST" | grep -q "x86_64-apple-darwin"; then
        check_pass "Rust toolchain: native x86_64"
    elif [ "$IS_INTEL" = false ] && echo "$RUST_HOST" | grep -q "aarch64-apple-darwin"; then
        check_pass "Rust toolchain: native ARM64"
    else
        check_warn "Rust toolchain architecture mismatch"
        check_info "Host: $RUST_HOST"
    fi
else
    check_fail "Rust NOT installed"
    check_info "Install: https://rustup.rs/"
fi

# Cargo
if command -v cargo &>/dev/null; then
    CARGO_VERSION=$(cargo --version | awk '{print $2}')
    check_pass "Cargo installed: $CARGO_VERSION"
else
    check_fail "Cargo NOT installed"
fi

# Profiling Tools
print_header "Profiling Tools"

# Instruments
if command -v instruments &>/dev/null; then
    check_pass "Instruments available"
else
    check_warn "Instruments not found"
    check_info "Should be installed with Xcode Command Line Tools"
fi

# cargo-instruments
if command -v cargo-instruments &>/dev/null; then
    check_pass "cargo-instruments installed"
else
    check_info "cargo-instruments not installed (optional)"
    check_info "Install: cargo install cargo-instruments"
fi

# cargo-flamegraph
if command -v cargo-flamegraph &>/dev/null; then
    check_pass "cargo-flamegraph installed"
else
    check_info "cargo-flamegraph not installed (optional)"
    check_info "Install: cargo install flamegraph"
fi

# DTrace permissions
print_header "DTrace Permissions"

if groups | grep -q _developer; then
    check_pass "User is in _developer group (DTrace access)"
else
    check_warn "User not in _developer group - DTrace may not work"
    check_info "Add user: sudo dscl . -append /Groups/_developer GroupMembership \$(whoami)"
    check_info "Then log out and log back in"
fi

# Running Processes
print_header "Resource Usage"

# CPU usage
CPU_USAGE=$(ps aux | awk '{sum += $3} END {print sum}')
if (( $(echo "$CPU_USAGE < 10" | bc -l) )); then
    check_pass "Low CPU usage: ${CPU_USAGE}%"
elif (( $(echo "$CPU_USAGE < 30" | bc -l) )); then
    check_warn "Moderate CPU usage: ${CPU_USAGE}%"
else
    check_fail "High CPU usage: ${CPU_USAGE}% - close background apps!"
fi

# Top CPU processes
check_info "Top 3 CPU consumers:"
ps aux | sort -rk 3,3 | head -n 4 | tail -n 3 | awk '{printf "  - %s: %.1f%%\n", $11, $3}'

# Memory pressure
MEMORY_PRESSURE=$(memory_pressure | grep "System-wide memory free percentage" | awk '{print $5}' | tr -d '%')
if [ -n "$MEMORY_PRESSURE" ]; then
    if (( $(echo "$MEMORY_PRESSURE > 50" | bc -l) )); then
        check_pass "Memory free: ${MEMORY_PRESSURE}%"
    else
        check_warn "Low memory: ${MEMORY_PRESSURE}% free"
    fi
fi

# Python Dependencies
print_header "Python Dependencies"

if command -v python3 &>/dev/null; then
    PYTHON_VERSION=$(python3 --version | awk '{print $2}')
    check_pass "Python 3 installed: $PYTHON_VERSION"

    # Check for required packages
    for pkg in pandas matplotlib scipy numpy; do
        if python3 -c "import $pkg" 2>/dev/null; then
            check_pass "Python package '$pkg' installed"
        else
            check_warn "Python package '$pkg' NOT installed"
            check_info "Install: pip3 install $pkg"
        fi
    done
else
    check_fail "Python 3 NOT installed"
    check_info "Install: brew install python3"
fi

# Summary
print_header "Summary"

if [ $ISSUES -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✓ System is optimally configured for benchmarking!${NC}"
elif [ $ISSUES -eq 0 ]; then
    echo -e "${YELLOW}⚠ System is good but has $WARNINGS warnings${NC}"
    echo -e "${YELLOW}  Consider addressing warnings for best results${NC}"
else
    echo -e "${RED}✗ System has $ISSUES critical issues and $WARNINGS warnings${NC}"
    echo -e "${RED}  Fix issues before running benchmarks!${NC}"
fi

echo ""
echo -e "${BLUE}For detailed setup instructions, see: MACOS_SETUP.md${NC}"
echo ""

exit $ISSUES
