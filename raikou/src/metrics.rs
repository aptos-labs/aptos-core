use std::{cmp, cmp::min, ops, sync::Arc};
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Allows async concurrent *unordered* metric recording.
pub struct UnorderedBuilder<T> {
    data_shards: Vec<Arc<Mutex<Vec<T>>>>,
}

pub trait Sender<T> {
    fn push(&mut self, value: T);
}

pub struct UnorderedSender<T> {
    data: OwnedMutexGuard<Vec<T>>,
}

impl<T> Sender<T> for UnorderedSender<T> {
    fn push(&mut self, value: T) {
        self.data.push(value);
    }
}

impl<T, S: Sender<T>> Sender<T> for Option<S> {
    fn push(&mut self, value: T) {
        if let Some(sender) = self {
            sender.push(value);
        }
    }
}

impl<T: Copy> UnorderedBuilder<T> {
    pub fn new() -> Self {
        UnorderedBuilder {
            data_shards: Vec::new(),
        }
    }

    pub fn new_sender(&mut self) -> UnorderedSender<T> {
        let shard = Arc::new(Mutex::new(Vec::new()));
        self.data_shards.push(shard.clone());
        UnorderedSender {
            data: shard.try_lock_owned().unwrap(),
        }
    }

    pub async fn build(self) -> Metric<T> {
        let mut data = Vec::new();
        for shard in self.data_shards {
            // If lock succeeds, then the sender is dropped and `Arc::try_unwrap` will succeed.
            let _ = shard.lock().await;
            data.extend(Arc::try_unwrap(shard).ok().unwrap().into_inner());
        }
        Metric::from_vec(data)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metric<T> {
    data: Vec<T>,
    sorted: bool,
}

impl<T> Metric<T> {
    pub fn from_vec(data: Vec<T>) -> Self {
        Metric {
            data,
            sorted: false,
        }
    }

    pub fn filter<F>(self, f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Metric {
            data: self.data.into_iter().filter(f).collect(),
            sorted: self.sorted,
        }
    }

    pub fn map<F, R>(self, f: F) -> Metric<R>
    where
        F: FnMut(T) -> R,
    {
        Metric {
            data: self.data.into_iter().map(f).collect(),
            sorted: self.sorted,
        }
    }
}

impl<T> Metric<T>
where
    T: Copy + PartialOrd + ops::Sub<Output = T>,
{
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn sort(mut self) -> Self {
        self.data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        self.sorted = true;
        self
    }

    pub fn derivative(mut self) -> Self {
        self.data = self.data.windows(2).map(|w| w[1] - w[0]).collect();
        self.sorted = false;
        self
    }

    /// Removes first `n` measurements.
    pub fn drop_first(mut self, n: usize) -> Self {
        self.data = self.data.iter().skip(n).copied().collect();
        self
    }

    /// Removes last `n` measurements.
    pub fn drop_last(mut self, mut n: usize) -> Self {
        n = min(n, self.len());
        self.data.resize_with(self.len() - n, || unreachable!());
        self
    }

    /// Returns the quantile of the metric.
    pub fn quantile(&self, q: f64) -> T {
        if !self.sorted {
            return self.clone().sort().quantile(q);
        }

        if self.is_empty() {
            panic!("Cannot compute quantile of an empty metric");
        }
        if !(0.0..=1.0).contains(&q) {
            panic!("Quantile must be between 0.0 and 1.0");
        }
        self.data[min((q * (self.len() as f64)) as usize, self.len() - 1)]
    }

    pub fn median(&self) -> T {
        self.quantile(0.5)
    }

    pub fn min(&self) -> Option<T> {
        self.data
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
    }

    pub fn max(&self) -> Option<T> {
        self.data
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl Metric<f64> {
    pub fn mean(&self) -> f64 {
        self.data.iter().copied().sum::<f64>() / self.len() as f64
    }

    pub fn variance(&self) -> f64 {
        let mean = self.mean();
        self.data
            .iter()
            .copied()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / self.len() as f64
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn show_histogram_range(&self, n_bins: usize, n_lines: usize, min: f64, max: f64) {
        let mut hist = vec![0; n_bins];

        let bin_width = (max - min) / n_bins as f64;
        for &value in &self.data {
            let bin = cmp::min(((value - min) / bin_width) as usize, n_bins - 1);
            hist[bin] += 1;
        }

        // Draw a nice horizontal ascii histogram
        let max_count = hist.iter().copied().fold(0, i32::max);
        let line_range = max_count as f64 / n_lines as f64;

        // lines are numbered from bottom to top, but are traversed from top to bottom
        println!("{:>6.0}", max_count as f64);
        for line in (0..n_lines).rev() {
            let line_threshold = line as f64 * line_range;
            print!("{:>6.0}  ", line_threshold);
            for bin in 0..n_bins {
                print!(
                    "{}",
                    if hist[bin] as f64 > line_threshold {
                        '#'
                    } else {
                        ' '
                    }
                );
            }
            println!();
        }

        let min_str = format!("{:.2}", min);
        let max_str = format!("{:.2}", max);
        if min_str.len() + max_str.len() + 3 < n_bins {
            // 8 spaces
            let n_spaces = n_bins - min_str.len() - max_str.len();
            print!(
                "{:8}{}{:n_spaces$}{}",
                "",
                min_str,
                "",
                max_str,
                n_spaces = n_spaces
            );
        }
        println!();
    }

    pub fn show_histogram(&self, n_bins: usize, n_lines: usize) {
        let min = self.data.iter().copied().fold(f64::INFINITY, f64::min);
        let max = self.data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.show_histogram_range(n_bins, n_lines, min, max);
    }

    /// Prints the basic stats about the metric such as
    pub fn print_stats(&self) {
        println!(" #points: {:.2}", self.len());
        println!("    mean: {:.2}", self.mean());
        println!(" std dev: {:.2}", self.std_dev());
        println!("     min: {:.2}", self.min().unwrap());
        println!("     10%: {:.2}", self.quantile(0.10));
        println!("     50%: {:.2}", self.median());
        println!("     90%: {:.2}", self.quantile(0.90));
        println!("     max: {:.2}", self.max().unwrap());
    }
}
