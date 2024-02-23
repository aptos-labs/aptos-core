use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tabled::{builder::Builder, settings::Style};

/// This struct represents a report of the specification testing.
/// It contains the list of entries, where each entry is a file and the number of mutants tested
/// and killed in that file (in form of a `ReportEntry` structure).
#[derive(Debug, Serialize)]
pub struct Report {
    /// The list of entries in the report.
    files: BTreeMap<PathBuf, Vec<MutantStats>>,
}

impl Report {
    /// Creates a new report.
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    /// Increments the number of mutants tested for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested set to 1.
    pub fn increment_mutants_tested(&mut self, path: &Path, module_func: &str) {
        self.increment_stat(path, module_func, |stat| stat.tested += 1);
    }

    /// Increments the number of mutants killed for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested set to 0 and killed
    /// count set to 1.
    pub fn increment_mutants_killed(&mut self, path: &Path, module_func: &str) {
        self.increment_stat(path, module_func, |stat| stat.killed += 1);
    }

    /// Returns the number of mutants tested.
    pub fn mutants_tested(&self) -> u32 {
        self.total_count(|v| v.tested)
    }

    /// Returns the number of mutants killed.
    pub fn mutants_killed(&self) -> u32 {
        self.total_count(|v| v.killed)
    }

    /// Add a diff for a not killed mutant.
    pub fn add_mutants_alive_diff(&mut self, path: &Path, module_func: &str, diff: &str) {
        let entry = self
            .files
            .entry(path.to_path_buf())
            .or_insert(vec![MutantStats::new(module_func)]);

        if let Some(stat) = entry.iter_mut().find(|s| s.module_func == module_func) {
            stat.mutants_alive_diffs.push(diff.to_owned());
        } else {
            let mut new_entry = MutantStats::new(module_func);
            new_entry.mutants_alive_diffs.push(diff.to_owned());
            entry.push(new_entry);
        }
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
                let percentage = if stat.tested == 0 {
                    0.0
                } else {
                    f64::from(stat.killed) / f64::from(stat.tested) * 100.0
                };

                builder.push_record([
                    format!("{}::{}", path.to_string_lossy(), stat.module_func.clone()),
                    stat.tested.to_string(),
                    stat.killed.to_string(),
                    format!("{percentage:.2}%"),
                ]);
            }
        }

        let table = builder.build().with(Style::modern_rounded()).to_string();

        println!("{table}\n\n");
    }

    // Internal function to increment the chosen stat.
    fn increment_stat<F>(&mut self, path: &Path, module_func: &str, mut increment: F)
    where
        F: FnMut(&mut MutantStats),
    {
        let entry = self
            .files
            .entry(path.to_path_buf())
            .or_insert(vec![MutantStats::new(module_func)]);

        match entry.iter_mut().find(|s| s.module_func == module_func) {
            Some(stat) => increment(stat),
            None => {
                entry.push(MutantStats::new(module_func));
                increment(entry.last_mut().unwrap());
            },
        }
    }

    // Internal function to count the chosen stat.
    fn total_count<F>(&self, mut count: F) -> u32
    where
        F: FnMut(&MutantStats) -> u32,
    {
        self.files
            .values()
            .map(|entry| entry.iter().map(&mut count).sum::<u32>())
            .sum()
    }

    /// Returns the list of entries in the report.
    #[cfg(test)]
    pub fn entries(&self) -> &BTreeMap<PathBuf, Vec<MutantStats>> {
        &self.files
    }
}

/// This struct represents an entry in the report.
/// It contains the number of mutants tested and killed.
#[derive(Default, Debug, Serialize)]
pub struct MutantStats {
    /// Module::function where mutant resides.
    pub module_func: String,
    /// The number of mutants tested.
    pub tested: u32,
    /// The number of mutants killed.
    pub killed: u32,
    /// The list of not killed mutants.
    pub mutants_alive_diffs: Vec<String>,
}

impl MutantStats {
    /// Creates a new entry with the given number of mutants tested and killed.
    pub fn new(module_func: &str) -> Self {
        Self {
            module_func: module_func.to_string(),
            tested: 0,
            killed: 0,
            mutants_alive_diffs: vec![],
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
        assert!(entry.iter().any(|s| s.module_func == module_name));
    }

    #[test]
    fn increment_mutants_killed_adds_new_module_if_not_present() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "new_module";
        report.increment_mutants_killed(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        assert!(entry.iter().any(|s| s.module_func == module_name));
    }

    #[test]
    fn increment_mutants_tested_increases_tested_count_for_existing_module() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "existing_module";
        report.increment_mutants_tested(&path, module_name);
        report.increment_mutants_tested(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        let stat = entry.iter().find(|s| s.module_func == module_name).unwrap();
        assert_eq!(stat.tested, 2);
        assert_eq!(stat.killed, 0);
    }

    #[test]
    fn increment_mutants_killed_increases_killed_count_for_existing_module() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "existing_module";
        report.increment_mutants_killed(&path, module_name);
        report.increment_mutants_killed(&path, module_name);
        let entry = report.entries().get(&path).unwrap();
        let stat = entry.iter().find(|s| s.module_func == module_name).unwrap();
        assert_eq!(stat.killed, 2);
        assert_eq!(stat.tested, 0);
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

    #[test]
    fn add_mutants_alive_diff_adds_new_module_if_not_present() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "new_module";
        let diff = "diff";
        report.add_mutants_alive_diff(&path, module_name, diff);
        let entry = report.entries().get(&path).unwrap();
        assert!(entry
            .iter()
            .any(|s| s.module_func == module_name && s.mutants_alive_diffs.contains(&diff.to_owned())));
    }

    #[test]
    fn add_mutants_alive_diff_adds_diff_to_existing_module() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let module_name = "existing_module";
        let diff1 = "diff1";
        let diff2 = "diff2";
        report.add_mutants_alive_diff(&path, module_name, diff1);
        report.add_mutants_alive_diff(&path, module_name, diff2);
        let entry = report.entries().get(&path).unwrap();
        let stat = entry.iter().find(|s| s.module_func == module_name).unwrap();
        assert_eq!(stat.mutants_alive_diffs, vec![diff1, diff2]);
    }
}
