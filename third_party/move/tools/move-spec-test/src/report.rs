use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tabled::{builder::Builder, settings::Style};

/// This struct represents a report of the specification testing.
/// It contains the list of entries, where each entry is a file and the number of mutants tested
/// and killed in that file (in form of a `ReportEntry` structure).
#[derive(Debug, Serialize)]
pub struct Report {
    /// The list of entries in the report.
    files: HashMap<PathBuf, MutantStats>,
}

impl Report {
    /// Creates a new report.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Adds an entry to the report.
    pub fn add_entry(&mut self, path: PathBuf, entry: MutantStats) {
        self.files.insert(path, entry);
    }

    /// Returns `true` if the report contains an entry for the given path.
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.files.contains_key(path)
    }

    /// Increments the number of mutants tested for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested set to 1.
    pub fn increment_mutants_tested(&mut self, path: &PathBuf) {
        if self.contains(path) {
            let entry = self.files.get_mut(path).unwrap();
            entry.tested += 1;
        } else {
            self.add_entry(
                path.clone(),
                MutantStats {
                    tested: 1,
                    killed: 0,
                },
            );
        }
    }

    /// Increments the number of mutants killed for the given path.
    /// If the path is not in the report, it adds it with the number of mutants tested and killed set to 1.
    pub fn increment_mutants_killed(&mut self, path: &PathBuf) {
        if self.contains(path) {
            let entry = self.files.get_mut(path).unwrap();
            entry.killed += 1;
        } else {
            self.add_entry(
                path.clone(),
                MutantStats {
                    tested: 1,
                    killed: 1,
                },
            );
        }
    }

    /// Returns the number of mutants tested.
    pub fn mutants_tested(&self) -> u64 {
        self.files.values().map(|entry| entry.tested).sum()
    }

    /// Returns the number of mutants killed.
    pub fn mutants_killed(&self) -> u64 {
        self.files.values().map(|entry| entry.killed).sum()
    }

    /// Save the report to a JSON file.
    /// The file is created if it does not exist, otherwise it is overwritten.
    pub fn save_to_json_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    /// Returns the list of entries in the report.
    #[cfg(test)]
    pub fn entries(&self) -> &HashMap<PathBuf, MutantStats> {
        &self.files
    }

    /// Prints the report to stdout in a table format.
    pub fn print_table(&self) {
        let mut builder = Builder::new();

        builder.push_record(["File", "Mutants tested", "Mutants killed", "Percentage"]);

        for (path, stats) in self.files.iter() {
            builder.push_record([
                path.to_string_lossy().to_string(),
                stats.tested.to_string(),
                stats.killed.to_string(),
                format!(
                    "{:.2}%",
                    (stats.killed as f64 / stats.tested as f64) * 100.0
                ),
            ]);
        }

        let table = builder.build().with(Style::modern_rounded()).to_string();

        println!("{}", table);
    }
}

/// This struct represents an entry in the report.
/// It contains the number of mutants tested and killed.
#[derive(Default, Debug, Serialize)]
pub struct MutantStats {
    /// The number of mutants tested.
    pub tested: u64,
    /// The number of mutants killed.
    pub killed: u64,
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
    fn adding_entry_increases_count() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let stats = MutantStats {
            tested: 5,
            killed: 3,
        };
        report.add_entry(path, stats);
        assert_eq!(report.entries().len(), 1);
    }

    #[test]
    fn contains_returns_true_for_existing_entry() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        let stats = MutantStats {
            tested: 5,
            killed: 3,
        };
        report.add_entry(path.clone(), stats);
        assert!(report.contains(&path));
    }

    #[test]
    fn contains_returns_false_for_non_existing_entry() {
        let report = Report::new();
        let path = PathBuf::from("path/to/file");
        assert!(!report.contains(&path));
    }

    #[test]
    fn increment_mutants_tested_increases_tested_count() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        report.increment_mutants_tested(&path);
        assert_eq!(report.entries().get(&path).unwrap().tested, 1);
    }

    #[test]
    fn increment_mutants_killed_increases_killed_count() {
        let mut report = Report::new();
        let path = PathBuf::from("path/to/file");
        report.increment_mutants_killed(&path);
        assert_eq!(report.entries().get(&path).unwrap().killed, 1);
    }

    #[test]
    fn mutants_tested_returns_total_tested_count() {
        let mut report = Report::new();
        let path1 = PathBuf::from("path/to/file1");
        let path2 = PathBuf::from("path/to/file2");
        report.increment_mutants_tested(&path1);
        report.increment_mutants_tested(&path2);
        assert_eq!(report.mutants_tested(), 2);
    }

    #[test]
    fn mutants_killed_returns_total_killed_count() {
        let mut report = Report::new();
        let path1 = PathBuf::from("path/to/file1");
        let path2 = PathBuf::from("path/to/file2");
        report.increment_mutants_killed(&path1);
        report.increment_mutants_killed(&path2);
        assert_eq!(report.mutants_killed(), 2);
    }
}
