use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};

/// The `Report` struct represents a report of mutations.
/// It contains a vector of `MutationReport` instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// The vector of `ReportEntry` instances.
    mutants: Vec<MutationReport>,
}

impl Report {
    /// Creates a new `Report` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mutants: Vec::new(),
        }
    }

    /// Adds a new `MutationReport` to the report.
    pub fn add_entry(&mut self, entry: MutationReport) {
        trace!("Adding a mutant to the report: {entry:?}");
        self.mutants.push(entry);
    }

    /// Saves the `Report` as a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn save_to_json_file(&self, path: &Path) -> Result<()> {
        let file = std::fs::File::create(path)?;

        info!("Saving report to {}", path.display());

        serde_json::to_writer_pretty(file, &self).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    /// Loads the `Report` from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or read from.
    pub fn load_from_json_file(path: &Path) -> Result<Self> {
        info!("Reading report from {}", path.display());

        let file = std::fs::File::open(path)?;

        serde_json::from_reader(file).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    /// Saves the `Report` as a text file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written to.
    pub fn save_to_text_file(&self, path: &Path) -> Result<()> {
        let mut file = std::fs::File::create(path)?;

        info!("Saving report to {}", path.display());

        for entry in &self.mutants {
            writeln!(file, "Mutant path: {}", entry.mutant_path.display())?;
            writeln!(file, "Original file: {}", entry.original_file.display())?;
            writeln!(file, "Module name: {}", entry.module_name)?;
            writeln!(file, "Mutations:")?;
            for modification in &entry.mutations {
                writeln!(file, "  Operator: {}", modification.operator_name)?;
                writeln!(file, "  Old value: {}", modification.old_value)?;
                writeln!(file, "  New value: {}", modification.new_value)?;
                writeln!(
                    file,
                    "  Changed place: {}-{}",
                    modification.changed_place.start, modification.changed_place.end
                )?;
            }
            writeln!(file, "Diff:")?;
            writeln!(file, "{}", entry.diff)?;
            writeln!(file, "----------------------------------------")?;
        }

        debug!("Report saved to {}", path.display());

        Ok(())
    }

    /// Returns the vector of `MutationReport` instances.
    #[must_use]
    pub fn get_mutants(&self) -> &Vec<MutationReport> {
        &self.mutants
    }

    /// Converts the `Report` to a JSON string.
    #[cfg(test)]
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self)
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new()
    }
}

/// The `Range` struct represents a range with a start and end.
/// It is used to represent the location of a mutation inside the source file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Range {
    /// The start of the range.
    start: usize,
    /// The end of the range.
    end: usize,
}

impl Range {
    /// Creates a new `Range` instance.
    ///
    /// # Panics
    /// The start must be smaller or equal to the end.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end);
        Self { start, end }
    }
}

/// The `Mutation` struct represents a modification that was applied to a file.
/// It contains the location of the modification, the name of the mutation operator, the old value and the new value.
/// It is used to represent a single modification inside a `ReportEntry`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mutation {
    /// The location of the modification.
    changed_place: Range,
    /// The name of the mutation operator.
    operator_name: String,
    /// The old operator value.
    old_value: String,
    /// The new operator value.
    new_value: String,
}

impl Mutation {
    /// Creates a new `Mutation` instance.
    #[must_use]
    pub fn new(
        changed_place: Range,
        operator_name: String,
        old_value: String,
        new_value: String,
    ) -> Self {
        Self {
            changed_place,
            operator_name,
            old_value,
            new_value,
        }
    }

    /// Returns the operator name.
    #[must_use]
    pub fn get_operator_name(&self) -> &str {
        &self.operator_name
    }
}

/// The `MutationReport` struct represents an entry in a report.
/// It contains information about a mutation that was applied to a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct MutationReport {
    /// The path to the mutated file.
    mutant_path: PathBuf,
    /// The path to the original file.
    original_file: PathBuf,
    /// The name of the module that the mutation is in.
    module_name: String,
    /// The modifications that were applied to the file.
    mutations: Vec<Mutation>,
    /// The diff between the original and mutated file.
    diff: String,
}

impl MutationReport {
    /// Creates a new `MutationReport` instance.
    /// Generates diff (patch) between the original and mutated source.
    #[must_use]
    pub fn new(
        mutant_path: &Path,
        original_file: &Path,
        module_name: &str,
        mutated_source: &str,
        original_source: &str,
    ) -> Self {
        let patch = diffy::create_patch(original_source, mutated_source);
        Self {
            mutant_path: mutant_path.to_path_buf(),
            original_file: original_file.to_path_buf(),
            module_name: module_name.to_string(),
            mutations: vec![],
            diff: patch.to_string(),
        }
    }

    /// Adds a `Mutation` to the `MutationReport`.
    pub fn add_modification(&mut self, modification: Mutation) {
        trace!("Adding modification to report: {modification:?}");
        self.mutations.push(modification);
    }

    /// Return the mutant path.
    #[must_use]
    pub fn mutant_path(&self) -> &PathBuf {
        &self.mutant_path
    }

    /// Return the original file path.
    #[must_use]
    pub fn original_file_path(&self) -> &PathBuf {
        &self.original_file
    }

    /// Return the module name.
    #[must_use]
    pub fn get_module_name(&self) -> &str {
        &self.module_name
    }

    /// Return the diff.
    #[must_use]
    pub fn get_diff(&self) -> &str {
        &self.diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_report() {
        let mut report = Report::new();
        assert_eq!(report.to_json().unwrap(), "{\n  \"mutants\": []\n}");

        let range = Range::new(0, 10);
        let modification = Mutation::new(
            range,
            "operator".to_string(),
            "old".to_string(),
            "new".to_string(),
        );
        let mut report_entry = MutationReport::new(
            Path::new("file"),
            Path::new("original_file"),
            "module",
            "\n",
            "diff\n",
        );
        report_entry.add_modification(modification);

        report.add_entry(report_entry.clone());
        assert_eq!(
            report.to_json().unwrap(),
            "{\n  \"mutants\": [\n    {\n      \"mutant_path\": \"file\",\n      \"original_file\": \"original_file\",\n      \"module_name\": \"module\",\n      \"mutations\": [\n        {\n          \"changed_place\": {\n            \"start\": 0,\n            \"end\": 10\n          },\n          \"operator_name\": \"operator\",\n          \"old_value\": \"old\",\n          \"new_value\": \"new\"\n        }\n      ],\n      \"diff\": \"--- original\\n+++ modified\\n@@ -1 +1 @@\\n-diff\\n+\\n\"\n    }\n  ]\n}"
        );
    }

    #[test]
    fn test_range() {
        let range = Range::new(0, 10);
        assert_eq!(
            serde_json::to_string(&range).unwrap(),
            "{\"start\":0,\"end\":10}"
        );
    }

    #[test]
    fn test_modification() {
        let range = Range::new(0, 10);
        let modification = Mutation::new(
            range,
            "operator".to_string(),
            "old".to_string(),
            "new".to_string(),
        );
        assert_eq!(serde_json::to_string(&modification).unwrap(), "{\"changed_place\":{\"start\":0,\"end\":10},\"operator_name\":\"operator\",\"old_value\":\"old\",\"new_value\":\"new\"}");
    }

    #[test]
    fn saves_report_as_text_file_successfully() {
        let mut report = Report::new();
        let range = Range::new(0, 10);
        let modification = Mutation::new(
            range,
            "operator".to_string(),
            "old".to_string(),
            "new".to_string(),
        );
        let mut report_entry = MutationReport::new(
            Path::new("file"),
            Path::new("original_file"),
            "module",
            "\n",
            "diff\n",
        );
        report_entry.add_modification(modification);
        report.add_entry(report_entry);

        let path = Path::new("test_report.txt");
        report.save_to_text_file(path).unwrap();

        let mut file = fs::File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("Mutant path: file"));
        assert!(contents.contains("Original file: original_file"));
        assert!(contents.contains("Module name: module"));
        assert!(contents.contains("Mutations:"));
        assert!(contents.contains("Operator: operator"));
        assert!(contents.contains("Old value: old"));
        assert!(contents.contains("New value: new"));
        assert!(contents.contains("Changed place: 0-10"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    #[should_panic(expected = "No such file or directory")]
    fn fails_to_save_report_to_non_existent_directory() {
        let report = Report::new();
        let path = Path::new("non_existent_directory/test_report.txt");
        report.save_to_text_file(path).unwrap();
    }
}
