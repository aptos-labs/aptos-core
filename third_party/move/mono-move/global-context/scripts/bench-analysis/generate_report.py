#!/usr/bin/env python3
"""
Automated report generation for interner benchmarks.
Creates a comprehensive HTML report with all analysis and visualizations.
"""

import sys
import json
from pathlib import Path
from datetime import datetime
from typing import Dict, List

import pandas as pd
import numpy as np


HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Interner Benchmark Report</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            background-color: #f5f5f5;
            padding: 20px;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background-color: white;
            padding: 40px;
            box-shadow: 0 0 20px rgba(0,0,0,0.1);
            border-radius: 8px;
        }}

        h1 {{
            color: #2c3e50;
            border-bottom: 4px solid #3498db;
            padding-bottom: 15px;
            margin-bottom: 30px;
            font-size: 2.5em;
        }}

        h2 {{
            color: #2c3e50;
            margin-top: 40px;
            margin-bottom: 20px;
            font-size: 1.8em;
            border-left: 5px solid #3498db;
            padding-left: 15px;
        }}

        h3 {{
            color: #34495e;
            margin-top: 30px;
            margin-bottom: 15px;
            font-size: 1.4em;
        }}

        .metadata {{
            background-color: #ecf0f1;
            padding: 20px;
            border-radius: 5px;
            margin-bottom: 30px;
        }}

        .metadata p {{
            margin: 5px 0;
            font-size: 0.95em;
        }}

        .summary-stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }}

        .stat-card {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 25px;
            border-radius: 8px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}

        .stat-card h4 {{
            font-size: 0.9em;
            opacity: 0.9;
            margin-bottom: 10px;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}

        .stat-card .value {{
            font-size: 2em;
            font-weight: bold;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            background-color: white;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        th {{
            background-color: #3498db;
            color: white;
            padding: 12px;
            text-align: left;
            font-weight: 600;
        }}

        td {{
            padding: 12px;
            border-bottom: 1px solid #ddd;
        }}

        tr:hover {{
            background-color: #f5f5f5;
        }}

        .best {{
            background-color: #d4edda;
            font-weight: bold;
        }}

        .good {{
            background-color: #fff3cd;
        }}

        .poor {{
            background-color: #f8d7da;
        }}

        .plot-container {{
            margin: 30px 0;
            text-align: center;
        }}

        .plot-container img {{
            max-width: 100%;
            height: auto;
            border: 1px solid #ddd;
            border-radius: 5px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}

        .recommendation {{
            background-color: #d1ecf1;
            border-left: 5px solid #0c5460;
            padding: 20px;
            margin: 30px 0;
            border-radius: 5px;
        }}

        .recommendation h3 {{
            color: #0c5460;
            margin-top: 0;
        }}

        .warning {{
            background-color: #fff3cd;
            border-left: 5px solid #856404;
            padding: 20px;
            margin: 30px 0;
            border-radius: 5px;
        }}

        .warning h3 {{
            color: #856404;
            margin-top: 0;
        }}

        .footer {{
            margin-top: 50px;
            padding-top: 20px;
            border-top: 2px solid #ddd;
            text-align: center;
            color: #7f8c8d;
            font-size: 0.9em;
        }}

        code {{
            background-color: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }}

        .toc {{
            background-color: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 20px;
            margin: 30px 0;
        }}

        .toc ul {{
            list-style-type: none;
            padding-left: 20px;
        }}

        .toc li {{
            margin: 8px 0;
        }}

        .toc a {{
            color: #3498db;
            text-decoration: none;
        }}

        .toc a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Interner Benchmark Report</h1>

        <div class="metadata">
            <p><strong>Generated:</strong> {timestamp}</p>
            <p><strong>Results Directory:</strong> <code>{results_dir}</code></p>
            <p><strong>Number of Runs:</strong> {num_runs}</p>
        </div>

        <div class="toc">
            <h3>📋 Table of Contents</h3>
            <ul>
                <li><a href="#summary">Executive Summary</a></li>
                <li><a href="#methodology">Methodology</a></li>
                <li><a href="#results">Detailed Results</a></li>
                <li><a href="#throughput">Throughput Analysis</a></li>
                <li><a href="#efficiency">Efficiency Analysis</a></li>
                <li><a href="#stability">Measurement Stability</a></li>
                <li><a href="#recommendations">Recommendations</a></li>
            </ul>
        </div>

        <h2 id="summary">📊 Executive Summary</h2>
        {executive_summary}

        <h2 id="methodology">🔬 Methodology</h2>
        {methodology}

        <h2 id="results">📈 Detailed Results</h2>
        {detailed_results}

        <h2 id="throughput">⚡ Throughput Analysis</h2>
        {throughput_analysis}

        <h2 id="efficiency">📉 Efficiency Analysis</h2>
        {efficiency_analysis}

        <h2 id="stability">🎯 Measurement Stability</h2>
        {stability_analysis}

        <h2 id="recommendations">💡 Recommendations</h2>
        {recommendations}

        <div class="footer">
            <p>Generated by Interner Benchmark Analysis Suite</p>
            <p>{timestamp}</p>
        </div>
    </div>
</body>
</html>
"""


class ReportGenerator:
    """Generates comprehensive HTML reports for benchmark results."""

    def __init__(self, results_dir: Path):
        self.results_dir = results_dir
        self.analysis_dir = results_dir / "analysis"
        self.plots_dir = self.analysis_dir / "plots"

        # Load data
        self.df_agg = pd.read_csv(self.analysis_dir / "aggregated.csv")
        self.df_efficiency = pd.read_csv(self.analysis_dir / "efficiency.csv")
        self.df_best = pd.read_csv(self.analysis_dir / "best_performers.csv")

        with open(self.analysis_dir / "summary.json") as f:
            self.summary = json.load(f)

    def generate_executive_summary(self) -> str:
        """Generate executive summary section."""
        html = '<div class="summary-stats">'

        # Total benchmarks
        html += f'''
        <div class="stat-card">
            <h4>Total Benchmarks</h4>
            <div class="value">{self.summary['total_benchmarks']}</div>
        </div>
        '''

        # Number of implementations
        html += f'''
        <div class="stat-card">
            <h4>Implementations Tested</h4>
            <div class="value">{len(self.summary['implementations'])}</div>
        </div>
        '''

        # Core counts tested
        html += f'''
        <div class="stat-card">
            <h4>Core Counts</h4>
            <div class="value">{len(self.summary['core_counts'])}</div>
        </div>
        '''

        # Performance range
        ratio = self.summary['throughput_range']['ratio']
        html += f'''
        <div class="stat-card">
            <h4>Performance Range</h4>
            <div class="value">{ratio:.1f}x</div>
        </div>
        '''

        html += '</div>'

        # Key findings
        html += '<h3>Key Findings</h3>'
        html += '<ul>'

        # Best overall performer
        best_overall = self.df_best.loc[self.df_best['throughput_mean'].idxmax()]
        html += f'<li><strong>Best Overall:</strong> {best_overall["implementation"]} @ {best_overall["cores"]} cores ({best_overall["throughput_mean"]:.2e} ops/sec)</li>'

        # Best single-core
        best_single = self.df_agg[self.df_agg['cores'] == 1].loc[
            self.df_agg[self.df_agg['cores'] == 1]['throughput_mean'].idxmax()
        ]
        html += f'<li><strong>Best Single-Core:</strong> {best_single["implementation"]} ({best_single["throughput_mean"]:.2e} ops/sec)</li>'

        # Best scaling
        best_scaling = self.df_efficiency.loc[
            self.df_efficiency[self.df_efficiency['cores'] == self.df_efficiency['cores'].max()]['efficiency'].idxmax()
        ]
        html += f'<li><strong>Best Scaling:</strong> {best_scaling["implementation"]} ({best_scaling["efficiency"]*100:.1f}% efficiency @ {best_scaling["cores"]} cores)</li>'

        # Measurement quality
        mean_cv = self.summary['mean_cv']
        quality = "excellent" if mean_cv < 5 else "good" if mean_cv < 10 else "acceptable"
        html += f'<li><strong>Measurement Quality:</strong> {quality} (mean CV: {mean_cv:.2f}%)</li>'

        html += '</ul>'

        return html

    def generate_methodology(self) -> str:
        """Generate methodology section."""
        html = '<p>This benchmark suite systematically evaluates concurrent interner implementations across multiple dimensions:</p>'
        html += '<ul>'
        html += '<li><strong>Implementations:</strong> ' + ', '.join(self.summary['implementations']) + '</li>'
        html += '<li><strong>Benchmark Groups:</strong> ' + ', '.join(self.summary['groups']) + '</li>'
        html += '<li><strong>Core Counts:</strong> ' + ', '.join(map(str, self.summary['core_counts'])) + '</li>'
        html += '<li><strong>Statistical Rigor:</strong> Multiple runs with variance analysis (target CV &lt; 10%)</li>'
        html += '<li><strong>System Configuration:</strong> CPU frequency scaling disabled, cores pinned, elevated priority</li>'
        html += '</ul>'

        return html

    def generate_detailed_results(self) -> str:
        """Generate detailed results tables."""
        html = '<h3>Best Performers by Configuration</h3>'

        # Create table
        html += '<table>'
        html += '<tr><th>Group</th><th>Cores</th><th>Implementation</th><th>Throughput (ops/sec)</th><th>Efficiency</th></tr>'

        for _, row in self.df_best.sort_values(['group', 'cores']).iterrows():
            # Get efficiency for this configuration
            eff_row = self.df_efficiency[
                (self.df_efficiency['group'] == row['group']) &
                (self.df_efficiency['implementation'] == row['implementation']) &
                (self.df_efficiency['cores'] == row['cores'])
            ]

            efficiency = eff_row['efficiency'].values[0] * 100 if not eff_row.empty else 0

            # Color code by efficiency
            row_class = 'best' if efficiency >= 90 else 'good' if efficiency >= 70 else ''

            html += f'<tr class="{row_class}">'
            html += f'<td>{row["group"]}</td>'
            html += f'<td>{row["cores"]}</td>'
            html += f'<td><strong>{row["implementation"]}</strong></td>'
            html += f'<td>{row["throughput_mean"]:.2e}</td>'
            html += f'<td>{efficiency:.1f}%</td>'
            html += '</tr>'

        html += '</table>'

        return html

    def generate_throughput_analysis(self) -> str:
        """Generate throughput analysis section with plots."""
        html = '<p>Throughput scaling analysis across all implementations and core counts.</p>'

        # Add plots
        for group in self.df_agg['group'].unique():
            plot_file = self.plots_dir / f"throughput_scaling_{group}.png"
            if plot_file.exists():
                html += f'<div class="plot-container">'
                html += f'<h3>{group}</h3>'
                html += f'<img src="plots/throughput_scaling_{group}.png" alt="Throughput Scaling - {group}">'
                html += '</div>'

        # Add heatmaps
        html += '<h3>Throughput Heatmaps</h3>'
        for group in self.df_agg['group'].unique():
            plot_file = self.plots_dir / f"heatmap_{group}.png"
            if plot_file.exists():
                html += f'<div class="plot-container">'
                html += f'<img src="plots/heatmap_{group}.png" alt="Heatmap - {group}">'
                html += '</div>'

        return html

    def generate_efficiency_analysis(self) -> str:
        """Generate efficiency analysis section."""
        html = '<p>Parallel efficiency metrics showing how well each implementation scales with core count.</p>'

        # Add efficiency plots
        for group in self.df_efficiency['group'].unique():
            plot_file = self.plots_dir / f"efficiency_{group}.png"
            if plot_file.exists():
                html += f'<div class="plot-container">'
                html += f'<h3>{group}</h3>'
                html += f'<img src="plots/efficiency_{group}.png" alt="Efficiency - {group}">'
                html += '</div>'

        # Add speedup plots
        html += '<h3>Speedup Analysis</h3>'
        for group in self.df_efficiency['group'].unique():
            plot_file = self.plots_dir / f"speedup_{group}.png"
            if plot_file.exists():
                html += f'<div class="plot-container">'
                html += f'<img src="plots/speedup_{group}.png" alt="Speedup - {group}">'
                html += '</div>'

        return html

    def generate_stability_analysis(self) -> str:
        """Generate measurement stability analysis."""
        html = '<p>Coefficient of variation (CV) analysis showing measurement stability across runs.</p>'

        plot_file = self.plots_dir / "coefficient_of_variation.png"
        if plot_file.exists():
            html += '<div class="plot-container">'
            html += '<img src="plots/coefficient_of_variation.png" alt="Coefficient of Variation">'
            html += '</div>'

        # Identify problematic measurements
        high_cv = self.df_agg[self.df_agg['cv_mean'] > 10]

        if not high_cv.empty:
            html += '<div class="warning">'
            html += '<h3>⚠️ High Variance Measurements</h3>'
            html += '<p>The following configurations showed high variance (CV &gt; 10%):</p>'
            html += '<ul>'
            for _, row in high_cv.iterrows():
                html += f'<li>{row["implementation"]} @ {row["cores"]} cores (CV: {row["cv_mean"]:.2f}%)</li>'
            html += '</ul>'
            html += '<p>Consider re-running these benchmarks or investigating sources of variability.</p>'
            html += '</div>'

        return html

    def generate_recommendations(self) -> str:
        """Generate recommendations based on results."""
        html = ''

        # Find best implementation for different scenarios
        best_single_core = self.df_agg[self.df_agg['cores'] == 1].loc[
            self.df_agg[self.df_agg['cores'] == 1]['throughput_mean'].idxmax()
        ]

        max_cores = self.df_agg['cores'].max()
        best_multi_core = self.df_agg[self.df_agg['cores'] == max_cores].loc[
            self.df_agg[self.df_agg['cores'] == max_cores]['throughput_mean'].idxmax()
        ]

        html += '<div class="recommendation">'
        html += '<h3>🎯 Implementation Selection Guide</h3>'
        html += '<ul>'
        html += f'<li><strong>Single-threaded workloads:</strong> Use <code>{best_single_core["implementation"]}</code> ({best_single_core["throughput_mean"]:.2e} ops/sec)</li>'
        html += f'<li><strong>Highly parallel workloads ({max_cores}+ cores):</strong> Use <code>{best_multi_core["implementation"]}</code> ({best_multi_core["throughput_mean"]:.2e} ops/sec)</li>'

        # Best scaling
        best_scaling = self.df_efficiency.loc[
            self.df_efficiency[self.df_efficiency['cores'] == max_cores]['efficiency'].idxmax()
        ]
        html += f'<li><strong>Best scalability:</strong> <code>{best_scaling["implementation"]}</code> ({best_scaling["efficiency"]*100:.1f}% efficiency @ {max_cores} cores)</li>'

        html += '</ul>'
        html += '</div>'

        # Success criteria check
        html += '<h3>Success Criteria Validation</h3>'
        html += '<table>'
        html += '<tr><th>Criterion</th><th>Target</th><th>Actual</th><th>Status</th></tr>'

        # Check read throughput target (200M ops/sec @ 32 cores)
        read_32 = self.df_agg[(self.df_agg['group'] == 'read_throughput') & (self.df_agg['cores'] == 32)]
        if not read_32.empty:
            max_read = read_32['throughput_mean'].max()
            status = '✅ Pass' if max_read >= 200e6 else '❌ Fail'
            html += f'<tr><td>Read throughput @ 32 cores</td><td>&gt;200M ops/sec</td><td>{max_read:.2e} ops/sec</td><td>{status}</td></tr>'

        # Check mean CV
        mean_cv = self.summary['mean_cv']
        status = '✅ Pass' if mean_cv < 10 else '❌ Fail'
        html += f'<tr><td>Mean CV</td><td>&lt;10%</td><td>{mean_cv:.2f}%</td><td>{status}</td></tr>'

        html += '</table>'

        return html

    def generate_report(self) -> str:
        """Generate complete HTML report."""
        return HTML_TEMPLATE.format(
            timestamp=datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            results_dir=str(self.results_dir),
            num_runs=self.df_agg['run'].max() if 'run' in self.df_agg.columns else "N/A",
            executive_summary=self.generate_executive_summary(),
            methodology=self.generate_methodology(),
            detailed_results=self.generate_detailed_results(),
            throughput_analysis=self.generate_throughput_analysis(),
            efficiency_analysis=self.generate_efficiency_analysis(),
            stability_analysis=self.generate_stability_analysis(),
            recommendations=self.generate_recommendations(),
        )


def main():
    if len(sys.argv) < 2:
        print("Usage: python generate_report.py <results_dir>")
        sys.exit(1)

    results_dir = Path(sys.argv[1])

    if not results_dir.exists():
        print(f"Error: Results directory not found: {results_dir}")
        sys.exit(1)

    print("=" * 80)
    print("Interner Benchmark Report Generation")
    print("=" * 80)
    print()

    print("Generating HTML report...")

    generator = ReportGenerator(results_dir)
    html = generator.generate_report()

    # Save report
    output_file = results_dir / "report.html"
    with open(output_file, "w") as f:
        f.write(html)

    print(f"Report saved to: {output_file}")
    print()
    print("=" * 80)
    print("Report generation complete!")
    print("=" * 80)
    print()
    print(f"To view the report: open {output_file}")


if __name__ == "__main__":
    main()
