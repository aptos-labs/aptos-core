# Interner Benchmark Analysis Suite

Comprehensive analysis tools for benchmarking concurrent interner implementations with rigorous statistical analysis and visualization.

## 📋 Overview

This suite provides automated tools to:
- **Run benchmarks** with proper system configuration
- **Parse Criterion output** and calculate statistics
- **Generate visualizations** (throughput, efficiency, scaling, heatmaps)
- **Compare multiple runs** for consistency validation
- **Produce HTML reports** with comprehensive analysis

## 🚀 Quick Start

### Prerequisites

```bash
# Install Python dependencies
pip install -r requirements.txt

# Install Rust benchmarking tools (if not already installed)
cargo install cargo-criterion

# Optional: Install flamegraph for profiling (Linux only)
cargo install flamegraph
```

### Running Benchmarks

```bash
# Basic usage (3 runs by default)
./run_benchmarks.sh

# Custom number of runs
./run_benchmarks.sh --runs 5

# With profiling
./run_benchmarks.sh --profile

# As root for full system configuration (Linux only)
sudo ./run_benchmarks.sh --runs 3
```

### Analyzing Results

After benchmarks complete, results are automatically analyzed and a report is generated. The report will be saved at:
```
target/bench-results/run_<timestamp>/report.html
```

## 📂 Directory Structure

```
scripts/bench-analysis/
├── run_benchmarks.sh          # Main benchmark execution script
├── analyze_results.py         # Parse and analyze Criterion output
├── visualize_results.py       # Generate plots and visualizations
├── generate_report.py         # Generate HTML report
├── compare_runs.py            # Compare multiple benchmark runs
├── requirements.txt           # Python dependencies
└── README.md                  # This file

target/bench-results/
└── run_<timestamp>/
    ├── system_info.txt        # System configuration
    ├── run_1/                 # First benchmark run
    │   ├── bench.log          # Benchmark output
    │   └── criterion/         # Criterion results
    ├── run_2/                 # Second benchmark run
    ├── run_3/                 # Third benchmark run
    ├── analysis/              # Aggregated analysis
    │   ├── aggregated.csv     # Aggregated statistics
    │   ├── efficiency.csv     # Efficiency metrics
    │   ├── best_performers.csv
    │   ├── statistical_tests.csv
    │   ├── summary.json
    │   └── plots/             # All visualizations
    ├── profiling/             # Profiling data (if enabled)
    │   ├── perf.data
    │   ├── perf_report.txt
    │   └── flamegraph.svg
    └── report.html            # Comprehensive HTML report
```

## 🔧 Individual Scripts

### 1. run_benchmarks.sh

Main orchestration script that runs benchmarks with proper system configuration.

**Features:**
- System configuration (CPU governor, turbo boost, ASLR)
- Multiple benchmark runs for statistical rigor
- Core pinning (Linux)
- Automated analysis and report generation
- Optional profiling

**Options:**
```bash
--runs N       Number of benchmark runs (default: 3)
--profile      Run profiling after benchmarks
--help         Show help message
```

**Environment Variables:**
```bash
NUM_RUNS=5 ./run_benchmarks.sh
PROFILE=1 ./run_benchmarks.sh
```

### 2. analyze_results.py

Parses Criterion benchmark output and performs statistical analysis.

**Usage:**
```bash
python3 analyze_results.py <results_dir>
```

**Outputs:**
- `aggregated.csv` - Aggregated statistics across runs
- `efficiency.csv` - Parallel efficiency metrics
- `best_performers.csv` - Best implementation per configuration
- `statistical_tests.csv` - Pairwise statistical comparisons
- `summary.json` - Summary statistics

**What it does:**
- Parses Criterion JSON output
- Calculates throughput from timing data
- Aggregates multiple runs (mean, std, median, min, max)
- Computes parallel efficiency and speedup
- Performs statistical tests (t-tests, effect sizes)
- Identifies best performers

### 3. visualize_results.py

Generates comprehensive visualizations.

**Usage:**
```bash
python3 visualize_results.py <results_dir>
```

**Plots Generated:**
- **Throughput scaling** - Throughput vs. core count (per benchmark group)
- **Efficiency** - Parallel efficiency vs. core count
- **Speedup** - Speedup vs. core count with ideal scaling reference
- **Coefficient of variation** - Measurement stability
- **Relative performance** - Bar charts normalized to best performer
- **Heatmaps** - Throughput across implementations and core counts
- **Best performers** - Best implementation for each configuration

All plots saved as high-resolution PNG (300 DPI).

### 4. generate_report.py

Creates a comprehensive HTML report with all analysis and visualizations.

**Usage:**
```bash
python3 generate_report.py <results_dir>
```

**Report Sections:**
- **Executive Summary** - Key findings and statistics
- **Methodology** - Benchmark setup and configuration
- **Detailed Results** - Tables of best performers
- **Throughput Analysis** - Plots and analysis
- **Efficiency Analysis** - Scaling analysis
- **Measurement Stability** - Variance analysis
- **Recommendations** - Implementation selection guide

### 5. compare_runs.py

Compares multiple benchmark runs to assess consistency.

**Usage:**
```bash
# Compare all runs in a results directory
python3 compare_runs.py target/bench-results/run_20250208_120000

# Compare specific runs
python3 compare_runs.py run_1/ run_2/ run_3/
```

**Outputs:**
- `run_comparison.csv` - Comparison statistics
- `inconsistent_configs.csv` - High-variance configurations
- `run_consistency.png` - Visualization of run-to-run variance
- `comparison_report.txt` - Text report

**Use Cases:**
- Validate measurement consistency
- Identify unstable configurations
- Compare before/after changes

## 📊 Statistical Rigor

### Variance Analysis

The suite calculates **Coefficient of Variation (CV)** for all measurements:

```
CV = (std_dev / mean) × 100
```

**Quality Thresholds:**
- CV < 5%: Excellent
- CV < 10%: Good
- CV > 10%: Problematic (investigate)

### Statistical Tests

Pairwise t-tests with effect sizes (Cohen's d):
- **p < 0.05**: Statistically significant
- **Cohen's d**:
  - < 0.2: Negligible
  - 0.2-0.5: Small
  - 0.5-0.8: Medium
  - > 0.8: Large

### Efficiency Metrics

Parallel efficiency calculated as:
```
efficiency = actual_throughput / (baseline_throughput × cores)
```

**Efficiency Ratings:**
- \> 0.9: Excellent (near-linear scaling)
- 0.7-0.9: Good
- 0.5-0.7: Moderate
- < 0.5: Poor (contention issues)

## 🎯 System Configuration

### Linux

For best results, run with proper system configuration:

```bash
# Disable CPU frequency scaling
sudo cpupower frequency-set -g performance

# Disable turbo boost
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# Run benchmarks with elevated priority
sudo nice -n -20 ./run_benchmarks.sh
```

The `run_benchmarks.sh` script will attempt these configurations automatically when run as root.

### macOS

**See [MACOS_SETUP.md](MACOS_SETUP.md) for complete macOS configuration guide!**

Quick setup for macOS:

```bash
# Disable Turbo Boost (Intel Macs) - use Turbo Boost Switcher or voltageshift
# Disable Spotlight indexing
sudo mdutil -a -i off

# Disable Time Machine
sudo tmutil disable

# Keep system awake
caffeinate -i &

# Run benchmarks
sudo ./run_benchmarks.sh --runs 3

# Cleanup after benchmarks
sudo tmutil enable
sudo mdutil -a -i on
killall caffeinate
```

The `run_benchmarks.sh` script handles most macOS-specific configurations automatically.

## 📈 Interpreting Results

### Throughput Scaling

Look for:
- **Linear scaling** - Throughput increases proportionally with cores
- **Sublinear scaling** - Throughput increases but not proportionally (common)
- **Plateau** - Throughput stops increasing (bottleneck)
- **Degradation** - Throughput decreases with more cores (excessive contention)

### Efficiency

- **High efficiency (>90%)** - Implementation scales well
- **Moderate efficiency (70-90%)** - Acceptable scaling with some overhead
- **Low efficiency (<70%)** - Significant contention or synchronization overhead

### Best Performers

The report identifies best performers for:
- Single-threaded workloads (1 core)
- Highly parallel workloads (max cores)
- Best scalability (highest efficiency at high core counts)

## 🔍 Troubleshooting

### High Variance (CV > 10%)

**Possible causes:**
- CPU frequency scaling enabled
- Turbo boost enabled
- Background processes interfering
- Thermal throttling
- Hyperthreading/SMT enabled

**Solutions:**
- Run with proper system configuration (see above)
- Close background applications
- Run multiple times and use median
- Increase measurement time

### Missing Results

If analysis fails:
1. Check that benchmarks completed successfully
2. Verify Criterion output exists in `target/criterion`
3. Check for errors in `bench.log`

### Python Dependencies

If scripts fail with import errors:
```bash
pip install pandas matplotlib scipy numpy
```

## 📚 References

- [Criterion.rs](https://github.com/bheisler/criterion.rs) - Rust benchmarking framework
- [Statistical Analysis Best Practices](https://easyperf.net/blog/2019/08/02/Perf-measurement-environment-on-Linux)
- [Parallel Efficiency](https://en.wikipedia.org/wiki/Parallel_efficiency)

## 💡 Tips

1. **Run multiple iterations** - At least 3 runs to assess consistency
2. **Check system configuration** - Ensure CPU frequency scaling is disabled
3. **Review CV values** - High CV indicates unreliable measurements
4. **Compare runs** - Use `compare_runs.py` to validate consistency
5. **Profile the winner** - Use `--profile` to understand bottlenecks

## 📝 Example Workflow

```bash
# 1. Run benchmarks (5 runs with profiling)
sudo ./run_benchmarks.sh --runs 5 --profile

# 2. View the report
open target/bench-results/run_<timestamp>/report.html

# 3. Compare consistency across runs
python3 compare_runs.py target/bench-results/run_<timestamp>

# 4. Generate additional visualizations if needed
python3 visualize_results.py target/bench-results/run_<timestamp>
```

## 🤝 Contributing

To add new benchmark groups or implementations:

1. Add benchmark to `benches/interner_bench.rs`
2. Run benchmarks using this suite
3. Analysis and visualization will automatically include new benchmarks

## 📄 License

Part of the Aptos Core project.
