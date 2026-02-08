#!/usr/bin/env python3
"""
Visualization script for interner benchmark results.
Generates comprehensive plots for throughput, efficiency, latency, etc.
"""

import sys
from pathlib import Path
from typing import List

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.gridspec import GridSpec


# Set style for publication-quality plots
plt.style.use("seaborn-v0_8-darkgrid" if "seaborn-v0_8-darkgrid" in plt.style.available else "ggplot")
plt.rcParams["figure.figsize"] = (12, 8)
plt.rcParams["font.size"] = 10
plt.rcParams["axes.labelsize"] = 12
plt.rcParams["axes.titlesize"] = 14
plt.rcParams["legend.fontsize"] = 10


class BenchmarkVisualizer:
    """Generates visualizations for benchmark results."""

    def __init__(self, analysis_dir: Path):
        self.analysis_dir = analysis_dir
        self.output_dir = analysis_dir / "plots"
        self.output_dir.mkdir(exist_ok=True)

        # Load data
        self.df_agg = pd.read_csv(analysis_dir / "aggregated.csv")
        self.df_efficiency = pd.read_csv(analysis_dir / "efficiency.csv")
        self.df_best = pd.read_csv(analysis_dir / "best_performers.csv")

        # Color palette
        self.colors = plt.cm.tab10.colors
        self.impl_colors = self._assign_colors()

    def _assign_colors(self):
        """Assign consistent colors to implementations."""
        implementations = self.df_agg["implementation"].unique()
        return {impl: self.colors[i % len(self.colors)] for i, impl in enumerate(implementations)}

    def plot_throughput_by_cores(self):
        """Plot bar charts comparing implementations at each core count."""
        for group in self.df_agg["group"].unique():
            df_group = self.df_agg[self.df_agg["group"] == group]

            # Get all unique core counts
            core_counts = sorted(df_group["cores"].unique())

            # Check if this group has workload column (mixed_workload)
            has_workload = "workload" in df_group.columns and df_group["workload"].notna().any()

            if has_workload:
                # For mixed_workload, group by both cores and workload
                workloads = sorted(df_group["workload"].dropna().unique())

                for cores in core_counts:
                    for workload in workloads:
                        df_subset = df_group[
                            (df_group["cores"] == cores) &
                            (df_group["workload"] == workload)
                        ].sort_values("throughput_mean", ascending=False)

                        if df_subset.empty:
                            continue

                        fig, ax = plt.subplots(figsize=(14, 8))

                        implementations = df_subset["implementation"].values
                        throughput = df_subset["throughput_mean"].values
                        throughput_std = df_subset["throughput_std"].values

                        # Create bar positions
                        x_pos = np.arange(len(implementations))
                        colors = [self.impl_colors[impl] for impl in implementations]

                        # Create bar chart
                        bars = ax.bar(x_pos, throughput, yerr=throughput_std,
                                     color=colors, capsize=5, alpha=0.8, edgecolor='black', linewidth=1.5)

                        # Customize plot
                        ax.set_xlabel("Implementation", fontsize=14, fontweight="bold")
                        ax.set_ylabel("Throughput (ops/sec)", fontsize=14, fontweight="bold")
                        ax.set_title(f"{group} - {workload} - {cores} Core{'s' if cores > 1 else ''}",
                                   fontsize=16, fontweight="bold")
                        ax.set_xticks(x_pos)
                        ax.set_xticklabels(implementations, rotation=45, ha='right')
                        ax.set_yscale("log")
                        ax.grid(True, alpha=0.3, axis='y')

                        # Add value labels on bars
                        for i, (bar, val) in enumerate(zip(bars, throughput)):
                            height = bar.get_height()
                            ax.text(bar.get_x() + bar.get_width()/2., height,
                                   f'{val:.2e}',
                                   ha='center', va='bottom', fontsize=8, rotation=0)

                        plt.tight_layout()
                        # Sanitize workload name for filename
                        workload_safe = workload.replace("%", "pct").replace("/", "_")
                        output_file = self.output_dir / f"throughput_{group}_{workload_safe}_{cores}cores.png"
                        plt.savefig(output_file, dpi=300, bbox_inches="tight")
                        print(f"  Saved: {output_file}")
                        plt.close()
            else:
                # For other groups, just group by cores
                for cores in core_counts:
                    df_cores = df_group[df_group["cores"] == cores].sort_values("throughput_mean", ascending=False)

                    if df_cores.empty:
                        continue

                    fig, ax = plt.subplots(figsize=(14, 8))

                    implementations = df_cores["implementation"].values
                    throughput = df_cores["throughput_mean"].values
                    throughput_std = df_cores["throughput_std"].values

                    # Create bar positions
                    x_pos = np.arange(len(implementations))
                    colors = [self.impl_colors[impl] for impl in implementations]

                    # Create bar chart
                    bars = ax.bar(x_pos, throughput, yerr=throughput_std,
                                 color=colors, capsize=5, alpha=0.8, edgecolor='black', linewidth=1.5)

                    # Customize plot
                    ax.set_xlabel("Implementation", fontsize=14, fontweight="bold")
                    ax.set_ylabel("Throughput (ops/sec)", fontsize=14, fontweight="bold")
                    ax.set_title(f"{group} - {cores} Core{'s' if cores > 1 else ''}", fontsize=16, fontweight="bold")
                    ax.set_xticks(x_pos)
                    ax.set_xticklabels(implementations, rotation=45, ha='right')
                    ax.set_yscale("log")
                    ax.grid(True, alpha=0.3, axis='y')

                    # Add value labels on bars
                    for i, (bar, val) in enumerate(zip(bars, throughput)):
                        height = bar.get_height()
                        ax.text(bar.get_x() + bar.get_width()/2., height,
                               f'{val:.2e}',
                               ha='center', va='bottom', fontsize=8, rotation=0)

                    plt.tight_layout()
                    output_file = self.output_dir / f"throughput_{group}_{cores}cores.png"
                    plt.savefig(output_file, dpi=300, bbox_inches="tight")
                    print(f"  Saved: {output_file}")
                    plt.close()

    def plot_throughput_scaling(self):
        """Plot throughput vs. core count for all implementations."""
        for group in self.df_agg["group"].unique():
            fig, ax = plt.subplots(figsize=(14, 8))

            df_group = self.df_agg[self.df_agg["group"] == group]

            for impl in df_group["implementation"].unique():
                df_impl = df_group[df_group["implementation"] == impl].sort_values("cores")

                if df_impl.empty:
                    continue

                cores = df_impl["cores"]
                throughput = df_impl["throughput_mean"]
                throughput_std = df_impl["throughput_std"]

                color = self.impl_colors[impl]

                # Plot with error bars
                ax.errorbar(
                    cores,
                    throughput,
                    yerr=throughput_std,
                    label=impl,
                    marker="o",
                    markersize=8,
                    linewidth=2,
                    capsize=5,
                    color=color,
                )

            ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
            ax.set_ylabel("Throughput (ops/sec)", fontsize=14, fontweight="bold")
            ax.set_title(f"Throughput Scaling: {group}", fontsize=16, fontweight="bold")
            ax.legend(loc="best", frameon=True, shadow=True)
            ax.grid(True, alpha=0.3)
            ax.set_xscale("log", base=2)
            ax.set_yscale("log")

            # Add ideal scaling reference line (from 1-core baseline)
            if not df_group[df_group["cores"] == 1].empty:
                baseline = df_group[df_group["cores"] == 1]["throughput_mean"].mean()
                max_cores = df_group["cores"].max()
                ideal_cores = [1, max_cores]
                ideal_throughput = [baseline, baseline * max_cores]
                ax.plot(ideal_cores, ideal_throughput, "k--", alpha=0.5, linewidth=1.5, label="Ideal Scaling")

            plt.tight_layout()
            output_file = self.output_dir / f"throughput_scaling_{group}.png"
            plt.savefig(output_file, dpi=300, bbox_inches="tight")
            print(f"  Saved: {output_file}")
            plt.close()

    def plot_efficiency(self):
        """Plot parallel efficiency metrics."""
        for group in self.df_efficiency["group"].unique():
            fig, ax = plt.subplots(figsize=(14, 8))

            df_group = self.df_efficiency[self.df_efficiency["group"] == group]

            for impl in df_group["implementation"].unique():
                df_impl = df_group[df_group["implementation"] == impl].sort_values("cores")

                if df_impl.empty:
                    continue

                cores = df_impl["cores"]
                efficiency = df_impl["efficiency"] * 100  # Convert to percentage

                color = self.impl_colors[impl]

                ax.plot(
                    cores,
                    efficiency,
                    label=impl,
                    marker="s",
                    markersize=8,
                    linewidth=2,
                    color=color,
                )

            # Reference lines
            ax.axhline(y=100, color="k", linestyle="--", alpha=0.5, linewidth=1.5, label="Ideal (100%)")
            ax.axhline(y=90, color="g", linestyle=":", alpha=0.5, linewidth=1.5, label="Excellent (90%)")
            ax.axhline(y=70, color="orange", linestyle=":", alpha=0.5, linewidth=1.5, label="Good (70%)")
            ax.axhline(y=50, color="r", linestyle=":", alpha=0.5, linewidth=1.5, label="Poor (50%)")

            ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
            ax.set_ylabel("Parallel Efficiency (%)", fontsize=14, fontweight="bold")
            ax.set_title(f"Parallel Efficiency: {group}", fontsize=16, fontweight="bold")
            ax.legend(loc="best", frameon=True, shadow=True)
            ax.grid(True, alpha=0.3)
            ax.set_xscale("log", base=2)
            ax.set_ylim(0, 110)

            plt.tight_layout()
            output_file = self.output_dir / f"efficiency_{group}.png"
            plt.savefig(output_file, dpi=300, bbox_inches="tight")
            print(f"  Saved: {output_file}")
            plt.close()

    def plot_speedup(self):
        """Plot speedup vs. core count."""
        for group in self.df_efficiency["group"].unique():
            fig, ax = plt.subplots(figsize=(14, 8))

            df_group = self.df_efficiency[self.df_efficiency["group"] == group]

            for impl in df_group["implementation"].unique():
                df_impl = df_group[df_group["implementation"] == impl].sort_values("cores")

                if df_impl.empty:
                    continue

                cores = df_impl["cores"]
                speedup = df_impl["speedup"]

                color = self.impl_colors[impl]

                ax.plot(
                    cores,
                    speedup,
                    label=impl,
                    marker="o",
                    markersize=8,
                    linewidth=2,
                    color=color,
                )

            # Ideal speedup line
            max_cores = df_group["cores"].max()
            ideal_cores = np.array([1, max_cores])
            ax.plot(ideal_cores, ideal_cores, "k--", alpha=0.5, linewidth=2, label="Ideal Speedup")

            ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
            ax.set_ylabel("Speedup", fontsize=14, fontweight="bold")
            ax.set_title(f"Speedup: {group}", fontsize=16, fontweight="bold")
            ax.legend(loc="best", frameon=True, shadow=True)
            ax.grid(True, alpha=0.3)
            ax.set_xscale("log", base=2)
            ax.set_yscale("log", base=2)

            plt.tight_layout()
            output_file = self.output_dir / f"speedup_{group}.png"
            plt.savefig(output_file, dpi=300, bbox_inches="tight")
            print(f"  Saved: {output_file}")
            plt.close()

    def plot_coefficient_of_variation(self):
        """Plot coefficient of variation to assess measurement stability."""
        fig, ax = plt.subplots(figsize=(14, 8))

        for impl in self.df_agg["implementation"].unique():
            df_impl = self.df_agg[self.df_agg["implementation"] == impl]

            cores = df_impl["cores"]
            cv = df_impl["cv_mean"]

            color = self.impl_colors[impl]

            ax.scatter(cores, cv, label=impl, s=100, alpha=0.7, color=color)

        # Reference lines for CV thresholds
        ax.axhline(y=5, color="g", linestyle="--", alpha=0.5, linewidth=2, label="Good (CV < 5%)")
        ax.axhline(y=10, color="orange", linestyle="--", alpha=0.5, linewidth=2, label="Acceptable (CV < 10%)")
        ax.axhline(y=15, color="r", linestyle="--", alpha=0.5, linewidth=2, label="Poor (CV > 10%)")

        ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
        ax.set_ylabel("Coefficient of Variation (%)", fontsize=14, fontweight="bold")
        ax.set_title("Measurement Stability (Coefficient of Variation)", fontsize=16, fontweight="bold")
        ax.legend(loc="best", frameon=True, shadow=True)
        ax.grid(True, alpha=0.3)
        ax.set_xscale("log", base=2)

        plt.tight_layout()
        output_file = self.output_dir / "coefficient_of_variation.png"
        plt.savefig(output_file, dpi=300, bbox_inches="tight")
        print(f"  Saved: {output_file}")
        plt.close()

    def plot_relative_performance(self):
        """Plot relative performance (normalized to best)."""
        for group in self.df_agg["group"].unique():
            for cores in sorted(self.df_agg["cores"].unique()):
                df_subset = self.df_agg[
                    (self.df_agg["group"] == group) & (self.df_agg["cores"] == cores)
                ]

                if df_subset.empty:
                    continue

                # Normalize to best performer
                max_throughput = df_subset["throughput_mean"].max()
                df_subset = df_subset.copy()
                df_subset["relative_perf"] = (df_subset["throughput_mean"] / max_throughput) * 100

                # Sort by performance
                df_subset = df_subset.sort_values("relative_perf", ascending=True)

                fig, ax = plt.subplots(figsize=(12, 8))

                implementations = df_subset["implementation"]
                relative_perf = df_subset["relative_perf"]

                bars = ax.barh(implementations, relative_perf, color=[self.impl_colors[impl] for impl in implementations])

                # Color bars by performance
                for i, bar in enumerate(bars):
                    perf = relative_perf.iloc[i]
                    if perf >= 95:
                        bar.set_color("green")
                        bar.set_alpha(0.8)
                    elif perf >= 80:
                        bar.set_color("orange")
                        bar.set_alpha(0.8)
                    else:
                        bar.set_color("red")
                        bar.set_alpha(0.8)

                ax.set_xlabel("Relative Performance (%)", fontsize=14, fontweight="bold")
                ax.set_ylabel("Implementation", fontsize=14, fontweight="bold")
                ax.set_title(f"Relative Performance: {group} @ {cores} cores", fontsize=16, fontweight="bold")
                ax.axvline(x=100, color="k", linestyle="--", alpha=0.5, linewidth=2)
                ax.set_xlim(0, 105)
                ax.grid(True, alpha=0.3, axis="x")

                # Add value labels
                for i, (impl, perf) in enumerate(zip(implementations, relative_perf)):
                    ax.text(perf + 1, i, f"{perf:.1f}%", va="center", fontsize=10)

                plt.tight_layout()
                output_file = self.output_dir / f"relative_perf_{group}_{cores}cores.png"
                plt.savefig(output_file, dpi=300, bbox_inches="tight")
                print(f"  Saved: {output_file}")
                plt.close()

    def plot_heatmap(self):
        """Plot heatmap of throughput across implementations and core counts."""
        for group in self.df_agg["group"].unique():
            df_group = self.df_agg[self.df_agg["group"] == group]

            # Pivot to create a matrix
            pivot = df_group.pivot(index="implementation", columns="cores", values="throughput_mean")

            if pivot.empty:
                continue

            fig, ax = plt.subplots(figsize=(14, 10))

            # Create heatmap
            im = ax.imshow(pivot.values, cmap="YlOrRd", aspect="auto")

            # Set ticks
            ax.set_xticks(np.arange(len(pivot.columns)))
            ax.set_yticks(np.arange(len(pivot.index)))
            ax.set_xticklabels(pivot.columns)
            ax.set_yticklabels(pivot.index)

            # Rotate x labels
            plt.setp(ax.get_xticklabels(), rotation=45, ha="right", rotation_mode="anchor")

            # Add colorbar
            cbar = plt.colorbar(im, ax=ax)
            cbar.set_label("Throughput (ops/sec)", rotation=270, labelpad=20, fontsize=12, fontweight="bold")

            # Add text annotations
            for i in range(len(pivot.index)):
                for j in range(len(pivot.columns)):
                    value = pivot.values[i, j]
                    if not np.isnan(value):
                        text = ax.text(j, i, f"{value:.2e}", ha="center", va="center", color="black", fontsize=8)

            ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
            ax.set_ylabel("Implementation", fontsize=14, fontweight="bold")
            ax.set_title(f"Throughput Heatmap: {group}", fontsize=16, fontweight="bold")

            plt.tight_layout()
            output_file = self.output_dir / f"heatmap_{group}.png"
            plt.savefig(output_file, dpi=300, bbox_inches="tight")
            print(f"  Saved: {output_file}")
            plt.close()

    def plot_best_performers(self):
        """Plot best performers for each configuration."""
        fig, ax = plt.subplots(figsize=(14, 8))

        groups = self.df_best["group"].unique()
        width = 0.2
        x = np.arange(len(self.df_best["cores"].unique()))

        for i, group in enumerate(groups):
            df_group = self.df_best[self.df_best["group"] == group].sort_values("cores")

            throughput = df_group["throughput_mean"]

            ax.bar(x + i * width, throughput, width, label=group, alpha=0.8)

        ax.set_xlabel("Core Count", fontsize=14, fontweight="bold")
        ax.set_ylabel("Throughput (ops/sec)", fontsize=14, fontweight="bold")
        ax.set_title("Best Performers by Benchmark Group", fontsize=16, fontweight="bold")
        ax.set_xticks(x + width * (len(groups) - 1) / 2)
        ax.set_xticklabels(sorted(self.df_best["cores"].unique()))
        ax.legend(loc="best", frameon=True, shadow=True)
        ax.grid(True, alpha=0.3, axis="y")
        ax.set_yscale("log")

        plt.tight_layout()
        output_file = self.output_dir / "best_performers.png"
        plt.savefig(output_file, dpi=300, bbox_inches="tight")
        print(f"  Saved: {output_file}")
        plt.close()

    def generate_all_plots(self):
        """Generate all visualization plots."""
        print("Generating visualizations...")

        print("  1. Throughput by core count (bar charts)...")
        self.plot_throughput_by_cores()

        print("  2. Throughput scaling plots...")
        self.plot_throughput_scaling()

        print("  3. Efficiency plots...")
        self.plot_efficiency()

        print("  4. Speedup plots...")
        self.plot_speedup()

        print("  5. Coefficient of variation plot...")
        self.plot_coefficient_of_variation()

        print("  6. Relative performance plots...")
        self.plot_relative_performance()

        print("  7. Heatmaps...")
        self.plot_heatmap()

        print("  8. Best performers plot...")
        self.plot_best_performers()

        print()
        print(f"All plots saved to: {self.output_dir}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python visualize_results.py <results_dir>")
        sys.exit(1)

    results_dir = Path(sys.argv[1])
    analysis_dir = results_dir / "analysis"

    if not analysis_dir.exists():
        print(f"Error: Analysis directory not found: {analysis_dir}")
        print("Run analyze_results.py first!")
        sys.exit(1)

    print("=" * 80)
    print("Interner Benchmark Visualization")
    print("=" * 80)
    print()

    visualizer = BenchmarkVisualizer(analysis_dir)
    visualizer.generate_all_plots()

    print()
    print("=" * 80)
    print("Visualization complete!")
    print("=" * 80)


if __name__ == "__main__":
    main()
