#!/usr/bin/env python3
"""
Compare benchmark results across multiple runs to assess consistency
and identify performance regressions or improvements.
"""

import sys
from pathlib import Path
from typing import List

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt


class RunComparator:
    """Compares benchmark results across multiple runs."""

    def __init__(self, run_dirs: List[Path]):
        self.run_dirs = run_dirs
        self.run_names = [d.name for d in run_dirs]

    def load_run_data(self, run_dir: Path) -> pd.DataFrame:
        """Load aggregated data from a run."""
        analysis_dir = run_dir / "analysis"
        agg_file = analysis_dir / "aggregated.csv"

        if not agg_file.exists():
            print(f"Warning: No aggregated data found in {run_dir}")
            return pd.DataFrame()

        df = pd.read_csv(agg_file)
        df["run_name"] = run_dir.name
        return df

    def compare_runs(self) -> pd.DataFrame:
        """Compare all runs and generate comparison statistics."""
        all_data = []

        for run_dir in self.run_dirs:
            df = self.load_run_data(run_dir)
            if not df.empty:
                all_data.append(df)

        if not all_data:
            print("No data to compare!")
            return pd.DataFrame()

        combined = pd.concat(all_data, ignore_index=True)

        # Group by configuration and compare across runs
        groupby_cols = ["group", "implementation", "cores"]

        comparison = combined.groupby(groupby_cols).agg({
            "throughput_mean": ["mean", "std", "min", "max"],
            "cv_mean": ["mean"],
            "run_name": ["count"],
        }).reset_index()

        # Flatten column names
        comparison.columns = ["_".join(col).strip("_") if col[1] else col[0] for col in comparison.columns.values]

        # Calculate consistency metrics
        comparison["throughput_cv"] = (comparison["throughput_mean_std"] / comparison["throughput_mean_mean"]) * 100
        comparison["throughput_range"] = comparison["throughput_mean_max"] - comparison["throughput_mean_min"]
        comparison["throughput_range_pct"] = (comparison["throughput_range"] / comparison["throughput_mean_mean"]) * 100

        return comparison

    def identify_inconsistencies(self, comparison: pd.DataFrame, threshold: float = 10.0) -> pd.DataFrame:
        """Identify configurations with high variance across runs."""
        inconsistent = comparison[comparison["throughput_cv"] > threshold].copy()
        inconsistent = inconsistent.sort_values("throughput_cv", ascending=False)
        return inconsistent

    def compare_two_runs(self, run1_dir: Path, run2_dir: Path) -> pd.DataFrame:
        """Detailed comparison between two specific runs."""
        df1 = self.load_run_data(run1_dir)
        df2 = self.load_run_data(run2_dir)

        if df1.empty or df2.empty:
            print("One or both runs have no data!")
            return pd.DataFrame()

        # Merge on configuration
        merged = pd.merge(
            df1,
            df2,
            on=["group", "implementation", "cores"],
            suffixes=("_run1", "_run2"),
        )

        # Calculate differences
        merged["throughput_diff"] = merged["throughput_mean_run2"] - merged["throughput_mean_run1"]
        merged["throughput_pct_change"] = (merged["throughput_diff"] / merged["throughput_mean_run1"]) * 100

        # Identify significant changes (>5% difference)
        merged["significant"] = abs(merged["throughput_pct_change"]) > 5

        return merged

    def plot_run_comparison(self, comparison: pd.DataFrame, output_dir: Path):
        """Generate plots comparing runs."""
        output_dir.mkdir(exist_ok=True)

        # Plot throughput CV across configurations
        fig, ax = plt.subplots(figsize=(14, 8))

        implementations = comparison["implementation"].unique()
        colors = plt.cm.tab10.colors

        for i, impl in enumerate(implementations):
            df_impl = comparison[comparison["implementation"] == impl].sort_values("cores")

            if df_impl.empty:
                continue

            cores = df_impl["cores"]
            throughput_cv = df_impl["throughput_cv"]

            ax.scatter(cores, throughput_cv, label=impl, s=100, alpha=0.7, color=colors[i % len(colors)])

        ax.axhline(y=5, color="g", linestyle="--", alpha=0.5, linewidth=2, label="Good (CV < 5%)")
        ax.axhline(y=10, color="orange", linestyle="--", alpha=0.5, linewidth=2, label="Acceptable (CV < 10%)")

        ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
        ax.set_ylabel("Throughput CV Across Runs (%)", fontsize=14, fontweight="bold")
        ax.set_title("Run-to-Run Consistency", fontsize=16, fontweight="bold")
        ax.legend(loc="best", frameon=True, shadow=True)
        ax.grid(True, alpha=0.3)
        ax.set_xscale("log", base=2)

        plt.tight_layout()
        output_file = output_dir / "run_consistency.png"
        plt.savefig(output_file, dpi=300, bbox_inches="tight")
        print(f"  Saved: {output_file}")
        plt.close()

    def generate_comparison_report(self, comparison: pd.DataFrame, output_dir: Path):
        """Generate a text report of the comparison."""
        output_file = output_dir / "comparison_report.txt"

        with open(output_file, "w") as f:
            f.write("=" * 80 + "\n")
            f.write("Benchmark Run Comparison Report\n")
            f.write("=" * 80 + "\n\n")

            f.write(f"Number of runs compared: {comparison['run_name_count'].iloc[0]}\n\n")

            # Summary statistics
            f.write("Summary Statistics:\n")
            f.write("-" * 80 + "\n")
            f.write(f"Mean throughput CV: {comparison['throughput_cv'].mean():.2f}%\n")
            f.write(f"Max throughput CV: {comparison['throughput_cv'].max():.2f}%\n")
            f.write(f"Configurations with CV > 10%: {len(comparison[comparison['throughput_cv'] > 10])}\n\n")

            # Identify most consistent configurations
            most_consistent = comparison.nsmallest(10, "throughput_cv")
            f.write("Most Consistent Configurations (Top 10):\n")
            f.write("-" * 80 + "\n")
            for _, row in most_consistent.iterrows():
                f.write(f"  {row['implementation']:25s} @ {row['cores']:2d} cores: CV = {row['throughput_cv']:.2f}%\n")
            f.write("\n")

            # Identify least consistent configurations
            least_consistent = comparison.nlargest(10, "throughput_cv")
            f.write("Least Consistent Configurations (Top 10):\n")
            f.write("-" * 80 + "\n")
            for _, row in least_consistent.iterrows():
                f.write(f"  {row['implementation']:25s} @ {row['cores']:2d} cores: CV = {row['throughput_cv']:.2f}%\n")
            f.write("\n")

            # Overall assessment
            f.write("Overall Assessment:\n")
            f.write("-" * 80 + "\n")
            mean_cv = comparison['throughput_cv'].mean()
            if mean_cv < 5:
                f.write("✅ EXCELLENT: Benchmarks are highly consistent across runs (CV < 5%)\n")
            elif mean_cv < 10:
                f.write("✅ GOOD: Benchmarks show acceptable consistency (CV < 10%)\n")
            elif mean_cv < 15:
                f.write("⚠️  MODERATE: Some variability detected (CV < 15%)\n")
                f.write("   Consider investigating sources of variance\n")
            else:
                f.write("❌ POOR: High variability across runs (CV > 15%)\n")
                f.write("   Results may not be reliable - investigate system configuration\n")

            f.write("\n" + "=" * 80 + "\n")

        print(f"  Saved: {output_file}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python compare_runs.py <results_dir_1> [results_dir_2] [...]")
        print()
        print("If only one directory is provided, it should contain multiple run_* subdirectories")
        sys.exit(1)

    paths = [Path(p) for p in sys.argv[1:]]

    # Check if we got a single directory with multiple runs
    if len(paths) == 1 and paths[0].is_dir():
        run_dirs = sorted([d for d in paths[0].iterdir() if d.is_dir() and d.name.startswith("run_")])
        if len(run_dirs) < 2:
            print(f"Error: Need at least 2 runs to compare, found {len(run_dirs)}")
            sys.exit(1)
        parent_dir = paths[0]
    else:
        run_dirs = paths
        parent_dir = paths[0].parent

    print("=" * 80)
    print("Benchmark Run Comparison")
    print("=" * 80)
    print()
    print(f"Comparing {len(run_dirs)} runs:")
    for run_dir in run_dirs:
        print(f"  - {run_dir}")
    print()

    comparator = RunComparator(run_dirs)

    # Compare all runs
    print("Comparing runs...")
    comparison = comparator.compare_runs()

    if comparison.empty:
        print("No data to compare!")
        sys.exit(1)

    # Identify inconsistencies
    inconsistent = comparator.identify_inconsistencies(comparison, threshold=10.0)

    print(f"Found {len(inconsistent)} configurations with high variance (CV > 10%)")

    # Save results
    output_dir = parent_dir / "comparison"
    output_dir.mkdir(exist_ok=True)

    print()
    print("Saving results...")
    comparison.to_csv(output_dir / "run_comparison.csv", index=False)
    print(f"  Saved: {output_dir / 'run_comparison.csv'}")

    if not inconsistent.empty:
        inconsistent.to_csv(output_dir / "inconsistent_configs.csv", index=False)
        print(f"  Saved: {output_dir / 'inconsistent_configs.csv'}")

    # Generate plots
    print()
    print("Generating plots...")
    comparator.plot_run_comparison(comparison, output_dir)

    # Generate report
    print()
    print("Generating report...")
    comparator.generate_comparison_report(comparison, output_dir)

    # Print summary
    print()
    print("=" * 80)
    print("Comparison Summary")
    print("=" * 80)
    print(f"Mean CV: {comparison['throughput_cv'].mean():.2f}%")
    print(f"Max CV: {comparison['throughput_cv'].max():.2f}%")
    print(f"Configurations with CV > 10%: {len(inconsistent)}")

    if len(inconsistent) > 0:
        print()
        print("Most inconsistent configurations:")
        for _, row in inconsistent.head(5).iterrows():
            print(f"  - {row['implementation']} @ {row['cores']} cores: CV = {row['throughput_cv']:.2f}%")

    print()
    print("=" * 80)
    print(f"Results saved to: {output_dir}")
    print("=" * 80)


if __name__ == "__main__":
    main()
