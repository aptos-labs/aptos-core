// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Histogram generation for type size distribution analysis.

use crate::types::TypeInfo;
use std::fmt::Write;

/// A bucket in the histogram
#[derive(Debug, Clone)]
pub struct Bucket {
    /// Lower bound (inclusive)
    pub min: usize,
    /// Upper bound (inclusive), None means infinity
    pub max: Option<usize>,
    /// Number of types in this bucket
    pub count: usize,
}

impl Bucket {
    fn new(min: usize, max: Option<usize>) -> Self {
        Self { min, max, count: 0 }
    }

    fn contains(&self, value: usize) -> bool {
        match self.max {
            Some(max) => value >= self.min && value <= max,
            None => value >= self.min,
        }
    }

    fn label(&self) -> String {
        match self.max {
            Some(max) if self.min == max => format!("{}", self.min),
            Some(max) => format!("{}-{}", self.min, max),
            None => format!("{}+", self.min),
        }
    }
}

/// Histogram of type sizes
pub struct Histogram {
    buckets: Vec<Bucket>,
    total: usize,
    title: String,
}

impl Histogram {
    /// Create a histogram with power-of-2 bucket boundaries
    /// Buckets: [0], [1], [2-3], [4-7], [8-15], [16-31], [32-63], [64-127], [128-255], [256-511], [512-1023], [1024+]
    pub fn with_power_of_2_buckets() -> Self {
        let mut buckets = vec![
            Bucket::new(0, Some(0)), // exactly 0
            Bucket::new(1, Some(1)), // exactly 1
        ];

        // Powers of 2: [2-3], [4-7], [8-15], ..., [512-1023]
        for exp in 1..10 {
            let min = 1 << exp; // 2, 4, 8, 16, 32, 64, 128, 256, 512
            let max = (1 << (exp + 1)) - 1; // 3, 7, 15, 31, 63, 127, 255, 511, 1023
            buckets.push(Bucket::new(min, Some(max)));
        }

        // 1024+
        buckets.push(Bucket::new(1024, None));

        Self {
            buckets,
            total: 0,
            title: "Type Size Distribution".to_string(),
        }
    }

    /// Create a histogram with custom fixed-size buckets
    pub fn with_fixed_buckets(boundaries: &[usize]) -> Self {
        let mut buckets = Vec::new();
        let mut prev = 0;

        for &boundary in boundaries {
            buckets.push(Bucket::new(prev, Some(boundary.saturating_sub(1))));
            prev = boundary;
        }

        // Final bucket: prev+
        buckets.push(Bucket::new(prev, None));

        Self {
            buckets,
            total: 0,
            title: "Distribution".to_string(),
        }
    }

    /// Add a value to the histogram
    pub fn add(&mut self, value: usize) {
        self.total += 1;
        for bucket in &mut self.buckets {
            if bucket.contains(value) {
                bucket.count += 1;
                return;
            }
        }
    }

    /// Build histogram from type info results (stack sizes)
    pub fn from_type_results(results: &[(String, &TypeInfo)]) -> Self {
        let mut hist = Self::with_power_of_2_buckets();
        for (_, info) in results {
            hist.add(info.stack_size);
        }
        hist
    }

    /// Build histogram of nesting depths from type info results
    pub fn depth_from_type_results(results: &[(String, &TypeInfo)]) -> Self {
        // Depths are typically small (0-10), so use individual buckets
        let mut hist = Self::with_depth_buckets();
        for (_, info) in results {
            hist.add(info.nested_depth);
        }
        hist
    }

    /// Create a histogram with buckets for depth values (0, 1, 2, ..., 9, 10+)
    pub fn with_depth_buckets() -> Self {
        let mut buckets: Vec<Bucket> = (0..10).map(|i| Bucket::new(i, Some(i))).collect();
        buckets.push(Bucket::new(10, None)); // 10+

        Self {
            buckets,
            total: 0,
            title: "Nesting Depth Distribution".to_string(),
        }
    }

    /// Get the maximum count across all buckets (for scaling)
    fn max_count(&self) -> usize {
        self.buckets.iter().map(|b| b.count).max().unwrap_or(0)
    }

    /// Render as ASCII histogram
    pub fn render_ascii(&self, bar_width: usize) -> String {
        let mut s = String::new();
        let max_count = self.max_count();
        let max_label_len = self
            .buckets
            .iter()
            .map(|b| b.label().len())
            .max()
            .unwrap_or(0);

        writeln!(s, "{} (n={})", self.title, self.total).unwrap();
        writeln!(s, "{}", "=".repeat(max_label_len + bar_width + 20)).unwrap();

        for bucket in &self.buckets {
            let label = bucket.label();
            let bar_len = if max_count > 0 {
                (bucket.count as f64 / max_count as f64 * bar_width as f64) as usize
            } else {
                0
            };
            let pct = if self.total > 0 {
                bucket.count as f64 / self.total as f64 * 100.0
            } else {
                0.0
            };

            writeln!(
                s,
                "{:>width$} | {:bar$} {:>6} ({:>5.1}%)",
                label,
                "█".repeat(bar_len),
                bucket.count,
                pct,
                width = max_label_len,
                bar = bar_width
            )
            .unwrap();
        }

        s
    }

    /// Render as CSV
    pub fn render_csv(&self) -> String {
        let mut s = String::new();
        writeln!(s, "bucket,count,percentage").unwrap();

        for bucket in &self.buckets {
            let pct = if self.total > 0 {
                bucket.count as f64 / self.total as f64 * 100.0
            } else {
                0.0
            };
            writeln!(s, "\"{}\",{},{:.2}", bucket.label(), bucket.count, pct).unwrap();
        }

        s
    }

    /// Get buckets for inspection
    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }

    /// Get total count
    pub fn total(&self) -> usize {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_of_2_buckets() {
        let mut hist = Histogram::with_power_of_2_buckets();

        hist.add(0);
        hist.add(1);
        hist.add(2);
        hist.add(3);
        hist.add(4);
        hist.add(7);
        hist.add(8);
        hist.add(1024);
        hist.add(2000);

        assert_eq!(hist.buckets[0].count, 1); // 0
        assert_eq!(hist.buckets[1].count, 1); // 1
        assert_eq!(hist.buckets[2].count, 2); // 2-3
        assert_eq!(hist.buckets[3].count, 2); // 4-7
        assert_eq!(hist.buckets[4].count, 1); // 8-15
        assert_eq!(hist.buckets[11].count, 2); // 1024+
    }
}
