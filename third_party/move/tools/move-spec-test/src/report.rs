use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tabled::{builder::Builder, settings::Style};

/// This struct represents a report of the specification testing.
/// It contains the list of entries, where each entry is a file and the number of mutants tested
/// and killed in that file (in form of a `ReportEntry` structure).
#[derive(Debug, Serialize)]
pub struct Report {
    /// The list of entries in the report.
    files: HashMap<PathBuf, Vec<MutantStats>>,
}

impl Report {
    /// Creates a new report.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Increments the number of mutants tested for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested set to 1.
    pub fn increment_mutants_tested(&mut self, path: &Path, module_name: &str) {
        self.increment_stat(path, module_name, |stat| stat.tested += 1);
    }

    /// Increments the number of mutants killed for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested and killed
    /// set to the default (0) and then increases only killed count!
    pub fn increment_mutants_killed(&mut self, path: &Path, module_name: &str) {
        self.increment_stat(path, module_name, |stat| stat.killed += 1);
    }

    /// Returns the number of mutants tested.
    pub fn mutants_tested(&self) -> u64 {
        self.total_count(|v| v.tested)
    }

    /// Returns the number of mutants killed.
    pub fn mutants_killed(&self) -> u64 {
        self.total_count(|v| v.killed)
    }

    /// Save the report to a JSON file.
    /// The file is created if it does not exist, otherwise it is overwritten.
    pub fn save_to_json_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        Ok(serde_json::to_writer_pretty(file, self)?)
    }

    /// Prints the report to stdout in a table format.
    pub fn print_table(&self) {
        let mut builder = Builder::new();
        builder.push_record(["Module", "Mutants tested", "Mutants killed", "Percentage"]);

        for (path, stats) in &self.files {
            for stat in stats {
                builder.push_record([
                    format!("{}::{}", path.to_string_lossy(), stat.module.clone()),
                    stat.tested.to_string(),
                    stat.killed.to_string(),
                    format!("{:.2}%", (stat.killed as f64 / stat.tested as f64) * 100.0),
                ]);
            }
        }

        let table = builder.build().with(Style::modern_rounded()).to_string();

        println!("{table}\n\n");
    }

    // Internal function to increment the chosen stat.
    fn increment_stat<F>(&mut self, path: &Path, module_name: &str, mut increment: F)
    where
        F: FnMut(&mut MutantStats),
    {
        let entry = self
            .files
            .entry(path.to_path_buf())
            .or_insert(vec![MutantStats::new(module_name)]);

        match entry.iter_mut().find(|s| s.module == module_name) {
            Some(stat) => increment(stat),
            None => {
                entry.push(MutantStats::new(module_name));
            },
        }
    }

    // Internal function to count the chosen stat.
    fn total_count<F>(&self, mut count: F) -> u64
    where
        F: FnMut(&MutantStats) -> u64,
    {
        self.files
            .values()
            .map(|entry| entry.iter().map(&mut count).sum::<u64>())
            .sum()
    }

    /// Returns the list of entries in the report.
    #[cfg(test)]
    pub fn entries(&self) -> &HashMap<PathBuf, Vec<MutantStats>> {
        &self.files
    }
}

/// This struct represents an entry in the report.
/// It contains the number of mutants tested and killed.
#[derive(Default, Debug, Serialize)]
pub struct MutantStats {
    /// Module name.
    pub module: String,
    /// The number of mutants tested.
    pub tested: u64,
    /// The number of mutants killed.
    pub killed: u64,
}

impl MutantStats {
    /// Creates a new entry with the given number of mutants tested and killed.
    pub fn new(module: &str) -> Self {
        Self {
            module: module.to_string(),
            tested: 0,
            killed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn report_starts_empty() {
        let report = Report::new();
        assert_eq!(report.entries().len(), 0);
    }

    #[test]
    fn increment_mutants_tested_adds_new_module_if_not_present() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "new_module";
        report.increment_mutants_tested(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        assert!(entry.iter().any(|s| s.module == module_name));
    }

    #[test]
    fn increment_mutants_killed_adds_new_module_if_not_present() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "new_module";
        report.increment_mutants_killed(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        assert!(entry.iter().any(|s| s.module == module_name));
    }

    #[test]
    fn increment_mutants_tested_increases_tested_count_for_existing_module() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "existing_module";
        report.increment_mutants_tested(&path, module_name);
        report.increment_mutants_tested(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        let stat = entry.iter().find(|s| s.module == module_name).unwrap();
        assert_eq!(stat.tested, 2);
    }

    #[test]
    fn increment_mutants_killed_increases_killed_count_for_existing_module() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "existing_module";
        report.increment_mutants_killed(&path, module_name);
        report.increment_mutants_killed(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        let stat = entry.iter().find(|s| s.module == module_name).unwrap();
        assert_eq!(stat.killed, 2);
    }

    #[test]
    fn mutants_tested_returns_correct_total_tested_count() {
        let mut report = Report::new();
        let path1 = PathBuf::from("path/to/file1");
        let path2 = PathBuf::from("path/to/file2");
        let module_name = "module";
        report.increment_mutants_tested(&path1, module_name);
        report.increment_mutants_tested(&path2, module_name);
        assert_eq!(report.mutants_tested(), 2);
        assert_eq!(report.mutants_killed(), 0);
    }

    #[test]
    fn mutants_killed_returns_correct_total_killed_count() {
        let mut report = Report::new();
        let path1 = PathBuf::from("path/to/file1");
        let path2 = PathBuf::from("path/to/file2");
        let module_name = "module";
        report.increment_mutants_killed(&path1, module_name);
        report.increment_mutants_killed(&path2, module_name);
        assert_eq!(report.mutants_killed(), 2);
        assert_eq!(report.mutants_tested(), 0);
    }
}
