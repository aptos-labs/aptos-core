#!/usr/bin/env python3
"""
Interner Frequency Analysis Tool

Analyzes frequency distributions of addresses, identifiers, and type arguments
from Aptos VM execution to guide interner cache sizing decisions.

Emphasis on per-block temporal analysis to understand how distributions evolve.
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
from typing import Dict, List, Tuple
import seaborn as sns

# Set plotting style
sns.set_style("whitegrid")
plt.rcParams['figure.figsize'] = (12, 8)
plt.rcParams['font.size'] = 10


def load_profile_data(csv_path: str) -> pd.DataFrame:
    """
    Load interner profiling data from CSV.

    Expected CSV format: block_version,item_type,item_value,count

    Args:
        csv_path: Path to the CSV file

    Returns:
        DataFrame with columns: block_version, item_type, item_value, count
    """
    df = pd.read_csv(csv_path)

    # Ensure block_version is numeric
    df['block_version'] = pd.to_numeric(df['block_version'])

    # Sort by block version and count
    df = df.sort_values(['block_version', 'count'], ascending=[True, False])

    print(f"Loaded {len(df)} records from {csv_path}")
    print(f"Block versions: {df['block_version'].min()} to {df['block_version'].max()}")
    print(f"Unique blocks: {df['block_version'].nunique()}")
    print(f"Item types: {df['item_type'].unique()}")

    return df


def analyze_per_block(df: pd.DataFrame, item_type: str, top_k: int = 20) -> Dict:
    """
    Compute statistics for EACH block individually.

    Args:
        df: Full dataframe
        item_type: Type of item to analyze (address, module_name, function_name, type_arg, type_arg_vec)
        top_k: Number of top items to track

    Returns:
        Dictionary mapping block_version to statistics dict
    """
    filtered = df[df['item_type'] == item_type]

    per_block_stats = {}

    for block_version in sorted(filtered['block_version'].unique()):
        block_data = filtered[filtered['block_version'] == block_version]

        counts = block_data['count'].values

        stats = {
            'block_version': block_version,
            'total_items': len(block_data),
            'total_count': counts.sum(),
            'mean': counts.mean(),
            'median': np.median(counts),
            'std': counts.std(),
            'min': counts.min(),
            'max': counts.max(),
            'p90': np.percentile(counts, 90),
            'p95': np.percentile(counts, 95),
            'p99': np.percentile(counts, 99),
            'top_k_items': block_data.nlargest(top_k, 'count')[['item_value', 'count']].to_dict('records')
        }

        per_block_stats[block_version] = stats

    return per_block_stats


def plot_per_block_distributions(df: pd.DataFrame, item_type: str, output_dir: Path, max_blocks: int = 10):
    """
    Generate separate frequency plots for each block showing temporal evolution.

    Args:
        df: Full dataframe
        item_type: Type of item to analyze
        output_dir: Directory to save plots
        max_blocks: Maximum number of blocks to plot (for readability)
    """
    filtered = df[df['item_type'] == item_type]
    blocks = sorted(filtered['block_version'].unique())

    if len(blocks) > max_blocks:
        print(f"Warning: {len(blocks)} blocks found, only plotting first {max_blocks}")
        blocks = blocks[:max_blocks]

    # Create subplots
    n_cols = 2
    n_rows = (len(blocks) + n_cols - 1) // n_cols

    fig, axes = plt.subplots(n_rows, n_cols, figsize=(16, 4 * n_rows))
    if n_rows == 1:
        axes = axes.reshape(1, -1)

    for idx, block_version in enumerate(blocks):
        row = idx // n_cols
        col = idx % n_cols
        ax = axes[row, col]

        block_data = filtered[filtered['block_version'] == block_version]
        counts = block_data['count'].values

        # Histogram with log scale
        ax.hist(counts, bins=50, edgecolor='black', alpha=0.7)
        ax.set_yscale('log')
        ax.set_xlabel('Frequency')
        ax.set_ylabel('Count (log scale)')
        ax.set_title(f'Block {block_version} - {len(counts)} items')
        ax.grid(True, alpha=0.3)

    # Remove empty subplots
    for idx in range(len(blocks), n_rows * n_cols):
        row = idx // n_cols
        col = idx % n_cols
        fig.delaxes(axes[row, col])

    plt.suptitle(f'Per-Block Frequency Distribution - {item_type}', fontsize=14, y=0.995)
    plt.tight_layout()

    output_file = output_dir / f'per_block_distribution_{item_type}.png'
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_file}")
    plt.close()


def plot_temporal_evolution(df: pd.DataFrame, item_type: str, top_k: int, output_file: Path):
    """
    Line plot showing how top-K items' frequencies change across blocks.

    Args:
        df: Full dataframe
        item_type: Type of item to analyze
        top_k: Number of top items to track
        output_file: Path to save plot
    """
    filtered = df[df['item_type'] == item_type]

    # Find globally most frequent items
    total_counts = filtered.groupby('item_value')['count'].max().sort_values(ascending=False)
    top_items = total_counts.head(top_k).index.tolist()

    # Track these items across blocks
    plt.figure(figsize=(14, 8))

    for item in top_items:
        item_data = filtered[filtered['item_value'] == item].sort_values('block_version')

        if len(item_data) > 0:
            # Truncate long item names for legend
            display_name = str(item)[:50] + '...' if len(str(item)) > 50 else str(item)
            plt.plot(item_data['block_version'], item_data['count'],
                    marker='o', label=display_name, linewidth=2, markersize=6)

    plt.xlabel('Block Version', fontsize=12)
    plt.ylabel('Cumulative Frequency', fontsize=12)
    plt.title(f'Temporal Evolution of Top-{top_k} {item_type} Items', fontsize=14)
    plt.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
    plt.grid(True, alpha=0.3)
    plt.tight_layout()

    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_file}")
    plt.close()


def plot_cumulative(df: pd.DataFrame, item_type: str, output_file: Path):
    """
    Cumulative distribution plot (Zipf analysis) across all blocks.

    Args:
        df: Full dataframe
        item_type: Type of item to analyze
        output_file: Path to save plot
    """
    filtered = df[df['item_type'] == item_type]

    # Get max count per item (cumulative across blocks)
    max_counts = filtered.groupby('item_value')['count'].max().sort_values(ascending=False)

    # Calculate cumulative percentage
    total = max_counts.sum()
    cumulative_pct = (max_counts.cumsum() / total * 100).values
    ranks = np.arange(1, len(cumulative_pct) + 1)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))

    # Plot 1: Cumulative distribution
    ax1.plot(ranks, cumulative_pct, linewidth=2, color='blue')
    ax1.axhline(y=80, color='r', linestyle='--', label='80% coverage')
    ax1.axhline(y=90, color='orange', linestyle='--', label='90% coverage')
    ax1.axhline(y=95, color='green', linestyle='--', label='95% coverage')
    ax1.set_xlabel('Number of Items (Rank)', fontsize=12)
    ax1.set_ylabel('Cumulative Percentage (%)', fontsize=12)
    ax1.set_title(f'Cumulative Distribution - {item_type}', fontsize=14)
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    # Plot 2: Log-log plot (Zipf's law)
    ax2.loglog(ranks, max_counts.values, linewidth=2, color='blue', marker='.')
    ax2.set_xlabel('Rank (log scale)', fontsize=12)
    ax2.set_ylabel('Frequency (log scale)', fontsize=12)
    ax2.set_title(f'Zipf Distribution - {item_type}', fontsize=14)
    ax2.grid(True, alpha=0.3, which='both')

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_file}")
    plt.close()


def recommend_interner_size(df: pd.DataFrame, item_type: str, coverage: float = 0.95) -> Tuple[int, float]:
    """
    Calculate interner cache size needed for X% coverage.

    Args:
        df: Full dataframe
        item_type: Type of item to analyze
        coverage: Desired coverage percentage (0.0 to 1.0)

    Returns:
        Tuple of (cache_size, actual_coverage)
    """
    filtered = df[df['item_type'] == item_type]

    # Get max count per item
    max_counts = filtered.groupby('item_value')['count'].max().sort_values(ascending=False)

    total = max_counts.sum()
    cumsum = max_counts.cumsum()

    # Find cache size for desired coverage
    cache_size = (cumsum >= total * coverage).idxmax()
    cache_size = max_counts.index.get_loc(cache_size) + 1

    actual_coverage = cumsum.iloc[cache_size - 1] / total

    return cache_size, actual_coverage


def generate_summary_report(df: pd.DataFrame, output_dir: Path, top_k: int = 20):
    """
    Generate a text summary report with key statistics.

    Args:
        df: Full dataframe
        output_dir: Directory to save report
        top_k: Number of top items to include
    """
    report_path = output_dir / 'summary_report.txt'

    with open(report_path, 'w') as f:
        f.write("=" * 80 + "\n")
        f.write("INTERNER PROFILING ANALYSIS REPORT\n")
        f.write("=" * 80 + "\n\n")

        # Overall statistics
        f.write(f"Total records: {len(df)}\n")
        f.write(f"Block versions: {df['block_version'].min()} to {df['block_version'].max()}\n")
        f.write(f"Number of blocks: {df['block_version'].nunique()}\n\n")

        # Per item type analysis
        for item_type in df['item_type'].unique():
            f.write("\n" + "-" * 80 + "\n")
            f.write(f"Item Type: {item_type}\n")
            f.write("-" * 80 + "\n")

            filtered = df[df['item_type'] == item_type]

            # Latest block statistics
            latest_block = filtered['block_version'].max()
            latest_data = filtered[filtered['block_version'] == latest_block]

            f.write(f"\nLatest Block ({latest_block}) Statistics:\n")
            f.write(f"  Unique items: {len(latest_data)}\n")
            f.write(f"  Total frequency: {latest_data['count'].sum()}\n")
            f.write(f"  Mean frequency: {latest_data['count'].mean():.2f}\n")
            f.write(f"  Median frequency: {latest_data['count'].median():.2f}\n")
            f.write(f"  P90 frequency: {latest_data['count'].quantile(0.90):.2f}\n")
            f.write(f"  P95 frequency: {latest_data['count'].quantile(0.95):.2f}\n")
            f.write(f"  P99 frequency: {latest_data['count'].quantile(0.99):.2f}\n")

            # Cache sizing recommendations
            f.write(f"\nCache Sizing Recommendations:\n")
            for coverage in [0.80, 0.90, 0.95, 0.99]:
                cache_size, actual = recommend_interner_size(df, item_type, coverage)
                f.write(f"  {coverage*100:.0f}% coverage: {cache_size} items (actual: {actual*100:.2f}%)\n")

            # Top-K items
            f.write(f"\nTop-{top_k} Most Frequent Items (Latest Block):\n")
            top_items = latest_data.nlargest(top_k, 'count')
            for idx, row in enumerate(top_items.itertuples(), 1):
                item_str = str(row.item_value)[:60]
                f.write(f"  {idx:2d}. {item_str:60s} : {row.count:10d}\n")

    print(f"Saved: {report_path}")


def main():
    parser = argparse.ArgumentParser(description='Analyze interner profiling data')
    parser.add_argument('csv_path', help='Path to interner_profile.csv')
    parser.add_argument('--output-dir', default='./interner_analysis_results',
                       help='Output directory for plots and reports')
    parser.add_argument('--top-k', type=int, default=20,
                       help='Number of top items to track (default: 20)')
    parser.add_argument('--coverage', type=float, default=0.95,
                       help='Target cache coverage for sizing (default: 0.95)')
    parser.add_argument('--per-block', action='store_true',
                       help='Enable per-block distribution plots')
    parser.add_argument('--max-blocks-plot', type=int, default=10,
                       help='Maximum blocks to plot in per-block view (default: 10)')

    args = parser.parse_args()

    # Create output directory
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading data from {args.csv_path}...")
    df = load_profile_data(args.csv_path)

    print("\nGenerating analysis...")

    # Analyze each item type
    for item_type in df['item_type'].unique():
        print(f"\nAnalyzing {item_type}...")

        # Per-block analysis
        print(f"  Computing per-block statistics...")
        per_block_stats = analyze_per_block(df, item_type, args.top_k)

        # Temporal evolution
        print(f"  Plotting temporal evolution...")
        plot_temporal_evolution(
            df, item_type, args.top_k,
            output_dir / f'temporal_evolution_{item_type}.png'
        )

        # Cumulative distribution
        print(f"  Plotting cumulative distribution...")
        plot_cumulative(df, item_type, output_dir / f'cumulative_{item_type}.png')

        # Per-block distributions (if enabled)
        if args.per_block:
            print(f"  Plotting per-block distributions...")
            plot_per_block_distributions(df, item_type, output_dir, args.max_blocks_plot)

        # Cache sizing
        cache_size, actual_coverage = recommend_interner_size(df, item_type, args.coverage)
        print(f"  Cache size recommendation for {args.coverage*100:.0f}% coverage: {cache_size} items (actual: {actual_coverage*100:.2f}%)")

    # Generate summary report
    print("\nGenerating summary report...")
    generate_summary_report(df, output_dir, args.top_k)

    print(f"\n{'='*80}")
    print(f"Analysis complete! Results saved to: {output_dir}")
    print(f"{'='*80}")
    print(f"\nTo view results:")
    print(f"  - Summary report: {output_dir / 'summary_report.txt'}")
    print(f"  - Plots: {output_dir}/*.png")


if __name__ == '__main__':
    main()
