# macOS Benchmarking Setup Guide

Complete guide for running rigorous benchmarks on macOS (both Intel and Apple Silicon).

## 🍎 System Configuration

### 1. Disable Turbo Boost (Intel Macs)

**Option A: Using Turbo Boost Switcher (Recommended)**

```bash
# Install Turbo Boost Switcher
# Download from: https://github.com/rugarciap/Turbo-Boost-Switcher
# Or via Homebrew:
brew install --cask turbo-boost-switcher

# Launch and disable turbo boost from the menu bar
```

**Option B: Using voltageshift (Advanced)**

```bash
# Install voltageshift
git clone https://github.com/sicreative/VoltageShift.git
cd VoltageShift
./voltageshift buildlaunchd

# Disable turbo boost
sudo ./voltageshift turbo 0

# To re-enable later:
sudo ./voltageshift turbo 1
```

**Option C: Programmatic (May require SIP disabled)**

Note: This method may not work on newer macOS versions due to security restrictions.

```bash
# Disable turbo boost (requires root)
sudo sysctl -w machdep.xcpm.cpu_turbo=0

# Re-enable
sudo sysctl -w machdep.xcpm.cpu_turbo=1
```

### 2. Disable Spotlight Indexing

```bash
# Disable Spotlight indexing (reduces background CPU usage)
sudo mdutil -a -i off

# Re-enable after benchmarks
sudo mdutil -a -i on
```

### 3. Disable Time Machine

```bash
# Disable Time Machine
sudo tmutil disable

# Re-enable after benchmarks
sudo tmutil enable
```

### 4. Set Energy Saver Settings

```bash
# Prevent sleep (run before benchmarks)
caffeinate -i &
CAFFEINATE_PID=$!

# Kill caffeinate after benchmarks
kill $CAFFEINATE_PID
```

Or manually:
- Go to **System Preferences → Energy Saver**
- Set "Turn display off after" to **Never**
- Set "Computer Sleep" to **Never** (if available)
- Disable "Put hard disks to sleep when possible"

### 5. Disable App Nap

```bash
# Disable App Nap globally (done automatically by run_benchmarks.sh)
sudo defaults write NSGlobalDomain NSAppSleepDisabled -bool YES

# Re-enable
sudo defaults write NSGlobalDomain NSAppSleepDisabled -bool NO
```

### 6. Disable Automatic Graphics Switching (MacBook Pro)

For MacBook Pro with dual GPUs:
- Go to **System Preferences → Battery → Battery**
- Uncheck "Automatic graphics switching"
- This forces the discrete GPU to stay active

### 7. Close Background Applications

```bash
# List running applications
osascript -e 'tell application "System Events" to get name of (processes where background only is false)'

# Close specific apps (example)
osascript -e 'quit app "Safari"'
osascript -e 'quit app "Chrome"'
osascript -e 'quit app "Slack"'
```

### 8. Set Performance Mode (Apple Silicon)

For M1/M2/M3 Macs:
- Go to **System Preferences → Battery**
- Set Energy Mode to **High Power** (if available)
- Uncheck "Optimize video streaming while on battery"
- Uncheck "Enable Power Nap"

## 🔧 Development Tools

### Install Profiling Tools

```bash
# Install Xcode Command Line Tools (includes Instruments)
xcode-select --install

# Install cargo profiling tools
cargo install cargo-instruments
cargo install flamegraph

# Verify Instruments is available
which instruments
# Should output: /usr/bin/instruments
```

### Configure DTrace (for flamegraph)

```bash
# Add current user to _developer group for DTrace access
sudo dscl . -append /Groups/_developer GroupMembership $(whoami)

# Log out and log back in for changes to take effect
```

## 🚀 Running Benchmarks

### Basic Usage

```bash
cd third_party/move/mono-move/global-context/scripts/bench-analysis

# Run benchmarks (with sudo for best results)
sudo ./run_benchmarks.sh --runs 3

# With profiling
sudo ./run_benchmarks.sh --runs 3 --profile
```

### Manual System Configuration

If you prefer to configure manually before running:

```bash
# 1. Disable turbo boost (Intel only)
# Use Turbo Boost Switcher or voltageshift

# 2. Disable Spotlight
sudo mdutil -a -i off

# 3. Disable Time Machine
sudo tmutil disable

# 4. Keep system awake
caffeinate -i &

# 5. Run benchmarks
sudo ./run_benchmarks.sh --runs 3

# 6. Restore system
sudo tmutil enable
sudo mdutil -a -i on
killall caffeinate
```

## 📊 Profiling on macOS

### Using Instruments (GUI)

```bash
# Build benchmark binary
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release --bench interner_bench

# Find the binary
BENCH_BINARY=$(find ./target/release/deps -name "interner_bench-*" -type f -perm +111 | head -1)

# Profile with Instruments Time Profiler
instruments -t "Time Profiler" "$BENCH_BINARY" --bench

# Or open Instruments app manually
open -a Instruments
```

### Using cargo-instruments

```bash
# Install
cargo install cargo-instruments

# Profile specific benchmark
cargo instruments --release --bench interner_bench -t time

# Available templates:
cargo instruments --list-templates
```

### Using cargo-flamegraph (DTrace)

```bash
# Install
cargo install flamegraph

# Generate flamegraph
sudo cargo flamegraph --bench interner_bench

# Output: flamegraph.svg
```

## 🍏 Apple Silicon Specific Notes

### Performance Cores vs. Efficiency Cores

Apple Silicon Macs have two types of cores:
- **Performance cores (P-cores)**: High performance, higher power
- **Efficiency cores (E-cores)**: Lower performance, lower power

The system automatically schedules tasks. For benchmarks:

```bash
# Check core configuration
sysctl hw.perflevel0.physicalcpu  # P-cores
sysctl hw.perflevel1.physicalcpu  # E-cores

# Example output for M1 Pro:
# hw.perflevel0.physicalcpu: 8  (8 P-cores)
# hw.perflevel1.physicalcpu: 2  (2 E-cores)
```

**Note**: You cannot directly control which cores are used on Apple Silicon. The OS scheduler handles this. Running with High Power mode and disabling background tasks helps ensure P-cores are used.

### Rosetta 2 Detection

If accidentally running under Rosetta:

```bash
# Check if running under Rosetta
sysctl sysctl.proc_translated
# Returns 1 if under Rosetta, 0 if native

# Ensure you're using native ARM64 Rust toolchain
rustc --version --verbose | grep host
# Should show: host: aarch64-apple-darwin
```

## 📈 Expected Performance

### Intel Mac (Example: 2019 16" MacBook Pro, 8-core i9)

```
Read throughput:  100-200M ops/sec @ 8 cores
Write throughput: 10-20M ops/sec @ 8 cores
Efficiency:       70-85% @ 8 cores
```

### Apple Silicon (Example: M1 Max, 10 cores)

```
Read throughput:  200-400M ops/sec @ 10 cores
Write throughput: 20-40M ops/sec @ 10 cores
Efficiency:       75-90% @ 10 cores
```

Note: Performance varies by specific chip and system configuration.

## 🔍 Troubleshooting

### High Variance (CV > 10%)

**Common causes on macOS:**
1. **Turbo Boost enabled** - Disable using methods above
2. **Background processes** - Close all apps, disable Spotlight
3. **Thermal throttling** - Ensure good ventilation, consider cooling pad
4. **Time Machine** - Disable during benchmarks
5. **Graphics switching** - Disable automatic switching
6. **Battery mode** - Connect to power, set to High Power mode

### Permission Issues

```bash
# DTrace permission denied
sudo dscl . -append /Groups/_developer GroupMembership $(whoami)
# Then log out and log back in

# Instruments permission denied
# Grant Full Disk Access in System Preferences → Security & Privacy → Privacy
```

### Benchmark Binary Not Found

```bash
# Clean and rebuild
cargo clean
cargo build --release --bench interner_bench

# Check for binary
ls -la target/release/deps/interner_bench-*
```

## 📋 Pre-Benchmark Checklist

Before running benchmarks, verify:

- [ ] Turbo Boost disabled (Intel) or High Power mode set (Apple Silicon)
- [ ] Spotlight indexing disabled
- [ ] Time Machine disabled
- [ ] System sleep disabled (caffeinate running)
- [ ] All background applications closed
- [ ] Connected to power (for laptops)
- [ ] Good ventilation / cooling
- [ ] No other intensive tasks running

## 🔄 Post-Benchmark Cleanup

```bash
# Re-enable system features
sudo tmutil enable                  # Time Machine
sudo mdutil -a -i on               # Spotlight
killall caffeinate                 # Allow sleep

# Re-enable turbo boost (Intel)
# Use Turbo Boost Switcher or voltageshift

# Re-enable App Nap
sudo defaults write NSGlobalDomain NSAppSleepDisabled -bool NO
```

## 📚 Additional Resources

- [Xcode Instruments Documentation](https://help.apple.com/instruments/)
- [cargo-instruments](https://github.com/cmyr/cargo-instruments)
- [DTrace Guide](http://dtrace.org/guide/preface.html)
- [Apple Silicon Performance](https://developer.apple.com/documentation/apple-silicon)

## 💡 Tips

1. **Run benchmarks multiple times** - At least 3 runs to assess consistency
2. **Monitor temperature** - Use iStat Menus or similar to watch CPU temp
3. **Check for throttling** - If CV is high, thermal throttling may be occurring
4. **Use Activity Monitor** - Watch for unexpected CPU usage during benchmarks
5. **Benchmark at night** - Fewer background tasks, cooler ambient temperature
6. **Compare with Linux** - If possible, run same benchmarks on Linux for validation

## 🆘 Getting Help

If benchmarks show inconsistent results (CV > 10%):

1. Check all configuration steps above
2. Review system logs for errors: `log show --predicate 'eventMessage contains "CPU"' --last 5m`
3. Monitor with Activity Monitor during benchmark
4. Try increasing measurement time in benchmark code
5. Consider external factors (ambient temperature, power supply)
