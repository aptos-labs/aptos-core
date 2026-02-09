#!/usr/bin/env python3
"""
Comprehensive analysis script for interner benchmark results.
Parses Criterion output and generates statistical analysis.
"""

import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import re

import pandas as pd
import numpy as np
from scipy import stats


class CriterionParser:
    """Parser for Criterion benchmark results."""

    def __init__(self, results_dir: Path):
        self.results_dir = results_dir
        self.benchmarks = {}

    def parse_run(self, run_dir: Path) -> pd.DataFrame:
        """Parse a single benchmark run."""
        criterion_dir = run_dir / "criterion"

        if not criterion_dir.exists():
            print(f"Warning: No criterion directory found in {run_dir}")
            return pd.DataFrame()

        results = []

        # Walk through all benchmark groups
        for bench_group in criterion_dir.iterdir():
            if not bench_group.is_dir():
                continue

            # Skip report directories
            if bench_group.name in ["report", ".DS_Store"]:
                continue

            # Walk through all benchmarks in the group
            for bench_dir in bench_group.iterdir():
                if not bench_dir.is_dir():
                    continue

                # Parse estimates.json
                # Try 'new' first (comparison runs), fall back to 'base' (standalone runs)
                estimates_file = bench_dir / "new" / "estimates.json"
                if not estimates_file.exists():
                    estimates_file = bench_dir / "base" / "estimates.json"

                if estimates_file.exists():
                    result = self._parse_estimates(bench_group.name, bench_dir.name, estimates_file)
                    if result:
                        results.append(result)

        return pd.DataFrame(results)

    def _parse_estimates(self, group: str, bench_name: str, estimates_file: Path) -> Dict:
        """Parse Criterion estimates.json file."""
        try:
            with open(estimates_file) as f:
                data = json.load(f)

            # Parse benchmark name based on group
            # Formats:
            # - read_throughput/write_throughput: impl_cores (e.g., rwlock_btree_1)
            # - mixed_workload: impl_workload_cores (e.g., dashmap_chunked_50%read_16)
            # - latency: impl_read or impl_write (no cores)
            # - warmup: impl (no cores)

            parts = bench_name.split("_")

            if group in ["read_throughput", "write_throughput"]:
                # Format: impl_cores (last part is cores)
                if parts[-1].isdigit():
                    cores = int(parts[-1])
                    impl_name = "_".join(parts[:-1])
                else:
                    cores = 1
                    impl_name = bench_name
                workload = None

            elif group == "mixed_workload":
                # Format: impl_workload_cores (e.g., dashmap_chunked_50%read_16)
                # Last part is cores, second-to-last contains workload
                if parts[-1].isdigit():
                    cores = int(parts[-1])
                    # Find the workload part (contains %)
                    workload_idx = -1
                    for i, part in enumerate(parts):
                        if "%" in part:
                            workload_idx = i
                            break

                    if workload_idx >= 0:
                        impl_name = "_".join(parts[:workload_idx])
                        workload = "_".join(parts[workload_idx:-1])
                    else:
                        impl_name = "_".join(parts[:-1])
                        workload = None
                else:
                    cores = 1
                    impl_name = bench_name
                    workload = None

            elif group == "latency":
                # Format: impl_read or impl_write (no cores in name)
                # Use the last part (read/write) as workload
                if parts[-1] in ["read", "write"]:
                    workload = parts[-1]
                    impl_name = "_".join(parts[:-1])
                else:
                    workload = None
                    impl_name = bench_name
                cores = 1  # Latency is single operation

            elif group == "warmup":
                # Format: impl (no cores or workload)
                impl_name = bench_name
                cores = 1  # Default to 1 for warmup
                workload = None

            else:
                # Unknown group, use default parsing
                impl_name = bench_name
                cores = 1
                workload = None

            # Extract timing data (in nanoseconds)
            mean = data.get("mean", {}).get("point_estimate", 0) / 1e9  # Convert to seconds
            std_dev = data.get("std_dev", {}).get("point_estimate", 0) / 1e9
            median = data.get("median", {}).get("point_estimate", 0) / 1e9

            result = {
                "group": group,
                "implementation": impl_name,
                "cores": cores,
                "mean_seconds": mean,
                "std_dev_seconds": std_dev,
                "median_seconds": median,
                "cv": (std_dev / mean * 100) if mean > 0 else 0,  # Coefficient of variation
            }

            # Add workload if present
            if workload:
                result["workload"] = workload

            return result
        except Exception as e:
            print(f"Error parsing {estimates_file}: {e}")
            return None

    def parse_all_runs(self) -> pd.DataFrame:
        """Parse all benchmark runs in the results directory."""
        all_results = []

        run_dirs = sorted([d for d in self.results_dir.iterdir() if d.is_dir() and d.name.startswith("run_")])

        for i, run_dir in enumerate(run_dirs):
            print(f"Parsing run {i+1}/{len(run_dirs)}: {run_dir.name}")
            df = self.parse_run(run_dir)
            if not df.empty:
                df["run"] = i + 1
                all_results.append(df)

        if not all_results:
            print("No benchmark results found!")
            return pd.DataFrame()

        return pd.concat(all_results, ignore_index=True)


class BenchmarkAnalyzer:
    """Statistical analyzer for benchmark results."""

    def __init__(self, df: pd.DataFrame):
        self.df = df

    def calculate_throughput(self) -> pd.DataFrame:
        """Calculate throughput (ops/sec) from timing data."""
        df = self.df.copy()
        # Assume each benchmark runs 1000 operations per iteration (from the plan)
        df["throughput"] = 1000 / df["mean_seconds"]
        return df

    def aggregate_runs(self) -> pd.DataFrame:
        """Aggregate statistics across multiple runs."""
        groupby_cols = ["group", "implementation", "cores"]

        # Add workload column if it exists
        if "workload" in self.df.columns:
            groupby_cols.append("workload")

        agg_dict = {
            "mean_seconds": ["mean", "std", "median", "min", "max"],
            "std_dev_seconds": ["mean"],
            "cv": ["mean", "std"],
        }

        aggregated = self.df.groupby(groupby_cols).agg(agg_dict).reset_index()

        # Flatten column names
        aggregated.columns = [
            "_".join(col).strip("_") if col[1] else col[0]
            for col in aggregated.columns.values
        ]

        # Calculate throughput from aggregated mean
        aggregated["throughput_mean"] = 1000 / aggregated["mean_seconds_mean"]
        aggregated["throughput_std"] = aggregated["mean_seconds_std"] / (aggregated["mean_seconds_mean"] ** 2) * 1000

        return aggregated

    def calculate_efficiency(self, df: pd.DataFrame) -> pd.DataFrame:
        """Calculate parallel efficiency metrics."""
        result = []

        # Check if workload column exists
        has_workload = "workload" in df.columns

        for group in df["group"].unique():
            # Get unique workloads for this group if they exist
            if has_workload:
                workloads = df[df["group"] == group]["workload"].dropna().unique()
                if len(workloads) == 0:
                    workloads = [None]
            else:
                workloads = [None]

            for workload in workloads:
                for impl in df["implementation"].unique():
                    # Filter by group, implementation, and workload if applicable
                    if workload is not None:
                        impl_data = df[
                            (df["group"] == group) &
                            (df["implementation"] == impl) &
                            (df["workload"] == workload)
                        ].copy()
                    else:
                        impl_data = df[
                            (df["group"] == group) &
                            (df["implementation"] == impl)
                        ].copy()

                    if impl_data.empty:
                        continue

                    # Get baseline (1 core) throughput
                    baseline = impl_data[impl_data["cores"] == 1]
                    if baseline.empty:
                        continue

                    baseline_throughput = baseline["throughput_mean"].values[0]

                    for _, row in impl_data.iterrows():
                        cores = row["cores"]
                        throughput = row["throughput_mean"]

                        # Calculate efficiency
                        ideal_throughput = baseline_throughput * cores
                        efficiency = (throughput / ideal_throughput) if ideal_throughput > 0 else 0

                        result_dict = {
                            "group": group,
                            "implementation": impl,
                            "cores": cores,
                            "throughput": throughput,
                            "baseline_throughput": baseline_throughput,
                            "ideal_throughput": ideal_throughput,
                            "efficiency": efficiency,
                            "speedup": throughput / baseline_throughput if baseline_throughput > 0 else 0,
                        }

                        # Add workload if present
                        if workload is not None:
                            result_dict["workload"] = workload

                        result.append(result_dict)

        return pd.DataFrame(result)

    def identify_best_performers(self, df: pd.DataFrame) -> pd.DataFrame:
        """Identify best performing implementation for each configuration."""
        best = []

        for group in df["group"].unique():
            for cores in df["cores"].unique():
                subset = df[(df["group"] == group) & (df["cores"] == cores)]

                if subset.empty:
                    continue

                # Find implementation with highest throughput
                best_idx = subset["throughput_mean"].idxmax()
                best_row = subset.loc[best_idx].copy()
                best_row["rank"] = 1

                best.append(best_row)

        return pd.DataFrame(best)

    def statistical_comparison(self, df: pd.DataFrame) -> pd.DataFrame:
        """Perform statistical tests to compare implementations."""
        results = []

        for group in df["group"].unique():
            for cores in df["cores"].unique():
                subset = self.df[(self.df["group"] == group) & (self.df["cores"] == cores)]

                if subset.empty:
                    continue

                implementations = subset["implementation"].unique()

                # Pairwise comparisons
                for i, impl1 in enumerate(implementations):
                    for impl2 in implementations[i+1:]:
                        data1 = subset[subset["implementation"] == impl1]["mean_seconds"].values
                        data2 = subset[subset["implementation"] == impl2]["mean_seconds"].values

                        if len(data1) < 2 or len(data2) < 2:
                            continue

                        # Perform t-test
                        t_stat, p_value = stats.ttest_ind(data1, data2)

                        # Calculate effect size (Cohen's d)
                        pooled_std = np.sqrt((np.var(data1) + np.var(data2)) / 2)
                        cohens_d = (np.mean(data1) - np.mean(data2)) / pooled_std if pooled_std > 0 else 0

                        results.append({
                            "group": group,
                            "cores": cores,
                            "impl1": impl1,
                            "impl2": impl2,
                            "mean1": np.mean(data1),
                            "mean2": np.mean(data2),
                            "t_stat": t_stat,
                            "p_value": p_value,
                            "significant": p_value < 0.05,
                            "cohens_d": cohens_d,
                            "effect_size": self._interpret_effect_size(abs(cohens_d)),
                        })

        return pd.DataFrame(results)

    @staticmethod
    def _interpret_effect_size(d: float) -> str:
        """Interpret Cohen's d effect size."""
        if d < 0.2:
            return "negligible"
        elif d < 0.5:
            return "small"
        elif d < 0.8:
            return "medium"
        else:
            return "large"

    def generate_summary_stats(self, df: pd.DataFrame) -> Dict:
        """Generate summary statistics."""
        return {
            "total_benchmarks": len(df),
            "implementations": df["implementation"].unique().tolist(),
            "groups": df["group"].unique().tolist(),
            "core_counts": sorted(df["cores"].unique().tolist()),
            "mean_cv": df["cv_mean"].mean(),
            "max_cv": df["cv_mean"].max(),
            "throughput_range": {
                "min": df["throughput_mean"].min(),
                "max": df["throughput_mean"].max(),
                "ratio": df["throughput_mean"].max() / df["throughput_mean"].min() if df["throughput_mean"].min() > 0 else float("inf"),
            },
        }


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_results.py <results_dir>")
        sys.exit(1)

    results_dir = Path(sys.argv[1])

    if not results_dir.exists():
        print(f"Error: Results directory not found: {results_dir}")
        sys.exit(1)

    print("=" * 80)
    print("Interner Benchmark Analysis")
    print("=" * 80)
    print()

    # Parse results
    print("Step 1: Parsing Criterion results...")
    parser = CriterionParser(results_dir)
    df_raw = parser.parse_all_runs()

    if df_raw.empty:
        print("No results to analyze!")
        sys.exit(1)

    print(f"  Parsed {len(df_raw)} benchmark results")
    print()

    # Analyze
    print("Step 2: Calculating statistics...")
    analyzer = BenchmarkAnalyzer(df_raw)

    # Calculate throughput
    df_throughput = analyzer.calculate_throughput()

    # Aggregate across runs
    df_agg = analyzer.aggregate_runs()
    print(f"  Aggregated across {df_raw['run'].max()} runs")

    # Calculate efficiency
    df_efficiency = analyzer.calculate_efficiency(df_agg)
    print(f"  Calculated efficiency metrics")

    # Identify best performers
    df_best = analyzer.identify_best_performers(df_agg)
    print(f"  Identified best performers")

    # Statistical comparison
    print("  Running statistical tests...")
    df_stats = analyzer.statistical_comparison(df_raw)

    # Generate summary
    summary = analyzer.generate_summary_stats(df_agg)
    print()

    # Save results
    output_dir = results_dir / "analysis"
    output_dir.mkdir(exist_ok=True)

    print("Step 3: Saving results...")
    df_agg.to_csv(output_dir / "aggregated.csv", index=False)
    print(f"  Saved: {output_dir / 'aggregated.csv'}")

    df_efficiency.to_csv(output_dir / "efficiency.csv", index=False)
    print(f"  Saved: {output_dir / 'efficiency.csv'}")

    df_best.to_csv(output_dir / "best_performers.csv", index=False)
    print(f"  Saved: {output_dir / 'best_performers.csv'}")

    df_stats.to_csv(output_dir / "statistical_tests.csv", index=False)
    print(f"  Saved: {output_dir / 'statistical_tests.csv'}")

    with open(output_dir / "summary.json", "w") as f:
        json.dump(summary, f, indent=2)
    print(f"  Saved: {output_dir / 'summary.json'}")

    # Print summary
    print()
    print("=" * 80)
    print("Summary Statistics")
    print("=" * 80)
    print(f"Total benchmarks: {summary['total_benchmarks']}")
    print(f"Implementations: {', '.join(summary['implementations'])}")
    print(f"Benchmark groups: {', '.join(summary['groups'])}")
    print(f"Core counts: {summary['core_counts']}")
    print(f"Mean CV: {summary['mean_cv']:.2f}%")
    print(f"Max CV: {summary['max_cv']:.2f}%")
    print(f"Throughput range: {summary['throughput_range']['min']:.2e} - {summary['throughput_range']['max']:.2e} ops/sec")
    print(f"Throughput ratio: {summary['throughput_range']['ratio']:.2f}x")

    print()
    print("Best Performers by Core Count:")
    print("-" * 80)
    for _, row in df_best.sort_values(["group", "cores"]).iterrows():
        print(f"  {row['group']:20s} @ {row['cores']:2d} cores: {row['implementation']:25s} ({row['throughput_mean']:.2e} ops/sec)")

    print()
    print("=" * 80)
    print("Analysis complete! Results saved to:", output_dir)
    print("=" * 80)


if __name__ == "__main__":
    main()
