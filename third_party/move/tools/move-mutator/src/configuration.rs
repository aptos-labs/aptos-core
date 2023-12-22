use crate::cli::Options;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration file type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FileType {
    JSON,
    TOML,
}

/// Mutator configuration for the Move project.
#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    /// Main project options. It's the same as the CLI options.
    pub project: Options,
    /// Path to the project.
    pub project_path: Option<PathBuf>,
    /// Configuration for the mutation operators (project-wide).
    pub mutation: Option<MutationConfig>,
    /// Configuration for the individual files. (optional)
    pub individual: Option<Vec<FileConfiguration>>,
}

impl Configuration {
    /// Creates a new configuration using command line options.
    pub fn new(project: Options, project_path: Option<PathBuf>) -> Self {
        Self {
            project,
            project_path,
            mutation: None,
            individual: None,
        }
    }

    /// Recognizes the file type based on the file extension.
    /// Currently supported file types are JSON and TOML.
    fn get_file_type(file_path: &Path) -> anyhow::Result<FileType> {
        match file_path.extension().and_then(|s| s.to_str()) {
            Some("json") => Ok(FileType::JSON),
            Some("toml") => Ok(FileType::TOML),
            _ => Err(anyhow::anyhow!("Unsupported file type")),
        }
    }

    /// Reads configuration from the configuration file recognizing its type.
    pub fn from_file(file_path: &Path) -> anyhow::Result<Configuration> {
        let file_type = Configuration::get_file_type(file_path)?;
        match file_type {
            FileType::JSON => Configuration::from_json_file(file_path),
            FileType::TOML => Configuration::from_toml_file(file_path),
        }
    }

    /// Reads configuration from the TOML configuration file.
    pub fn from_toml_file(toml_file: &Path) -> anyhow::Result<Configuration> {
        let toml_source = std::fs::read_to_string(toml_file)?;
        Ok(toml::from_str(toml_source.as_str())?)
    }

    /// Reads configuration from the JSON configuration source.
    pub fn from_json_file(json_file: &Path) -> anyhow::Result<Configuration> {
        Ok(serde_json::from_str(&std::fs::read_to_string(json_file)?)?)
    }
}

/// Configuration of the mutation operators.
#[derive(Debug, Serialize, Deserialize)]
pub struct MutationConfig {
    /// Names of the mutation operators to use. If not provided, all operators will be used.
    pub operators: Vec<String>,
    /// Names of the mutation categories to be used.
    pub categories: Vec<String>,
}

/// Configuration for the individual file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileConfiguration {
    /// The path to the Move source.
    pub file: PathBuf,
    /// Indicates if the mutants should be verified
    pub verify_mutants: Option<bool>,
    /// Names of the mutation operators to use. If not provided, all operators will be used.
    pub mutation_operators: Option<MutationConfig>,
    /// Mutate only the functions with the given names.
    pub include_functions: Option<Vec<String>>,
    /// Mutate all functions except the ones with the given names.
    pub exclude_functions: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn configuration_from_toml_file_loads_correctly() {
        let toml_content = r#"
            [project]
            move_sources = ["/path/to/move/source"]
            [mutation]
            operators = ["operator1", "operator2"]
            categories = ["category1", "category2"]
            [[individual]]
            file = "/path/to/file"
            verify_mutants = true
            include_functions = ["function1", "function2"]
            exclude_functions = ["function3", "function4"]
        "#;
        fs::write("test.toml", toml_content).unwrap();
        let config = Configuration::from_toml_file(Path::new("test.toml")).unwrap();
        assert_eq!(config.project.move_sources, vec!["/path/to/move/source"]);
        assert_eq!(
            config.mutation.unwrap().operators,
            vec!["operator1", "operator2"]
        );
        fs::remove_file("test.toml").unwrap();
    }

    #[test]
    fn configuration_from_non_existent_toml_file_fails() {
        let result = Configuration::from_toml_file(Path::new("non_existent.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn configuration_from_invalid_toml_file_fails() {
        let toml_content = r#"
            [project]
            move_sources = "/path/to/move/source"
        "#;
        fs::write("test.toml", toml_content).unwrap();
        let result = Configuration::from_toml_file(Path::new("test.toml"));
        assert!(result.is_err());
        fs::remove_file("test.toml").unwrap();
    }

    #[test]
    fn configuration_from_json_file_loads_correctly() {
        let json_content = r#"
            {
                "project": {
                    "move_sources": ["/path/to/move/source"],
                    "include_only_files": ["/path/to/include/file"],
                    "exclude_files": ["/path/to/exclude/file"],
                    "output_dir": "/path/to/output",
                    "verify_mutants": true,
                    "no_overwrite": false,
                    "downsample_filter": "filter",
                    "configuration_file": "/path/to/configuration"
                },
                "project_path": "/path/to/project",
                "mutation": {
                    "operators": ["operator1", "operator2"],
                    "categories": ["category1", "category2"]
                },
                "individual": [
                    {
                        "file": "/path/to/file",
                        "verify_mutants": true,
                        "mutation_operators": {
                            "operators": ["operator3", "operator4"],
                            "categories": ["category3", "category4"]
                        },
                        "include_functions": ["function1", "function2"],
                        "exclude_functions": ["function3", "function4"]
                    }
                ]
            }
        "#;
        fs::write("test.json", json_content).unwrap();
        let config = Configuration::from_json_file(Path::new("test.json")).unwrap();
        assert_eq!(config.project.move_sources, vec!["/path/to/move/source"]);
        assert_eq!(
            config.project.include_only_files.unwrap(),
            vec![Path::new("/path/to/include/file")]
        );
        assert_eq!(
            config.project.exclude_files.unwrap(),
            vec![Path::new("/path/to/exclude/file")]
        );
        assert_eq!(
            config.project.output_dir.unwrap(),
            Path::new("/path/to/output")
        );
        assert_eq!(config.project.verify_mutants.unwrap(), true);
        assert_eq!(config.project.no_overwrite.unwrap(), false);
        assert_eq!(config.project.downsample_filter.unwrap(), "filter");
        assert_eq!(
            config.project.configuration_file.unwrap(),
            Path::new("/path/to/configuration")
        );
        assert_eq!(config.project_path.unwrap(), Path::new("/path/to/project"));
        assert_eq!(
            config.mutation.unwrap().operators,
            vec!["operator1", "operator2"]
        );
        fs::remove_file("test.json").unwrap();
    }

    #[test]
    fn configuration_from_non_existent_json_file_fails() {
        let result = Configuration::from_json_file(Path::new("non_existent.json"));
        assert!(result.is_err());
    }

    #[test]
    fn configuration_from_invalid_json_file_fails() {
        let json_content = r#"
            {
                "project": {
                    "move_sources": "/path/to/move/source"
                }
            }
        "#;
        fs::write("test.json", json_content).unwrap();
        let result = Configuration::from_json_file(Path::new("test.json"));
        assert!(result.is_err());
        fs::remove_file("test.json").unwrap();
    }

    #[test]
    fn recognizes_json_file_type_correctly() {
        assert_eq!(
            Configuration::get_file_type(Path::new("test.json")).unwrap(),
            FileType::JSON
        );
    }

    #[test]
    fn recognizes_toml_file_type_correctly() {
        assert_eq!(
            Configuration::get_file_type(Path::new("test.toml")).unwrap(),
            FileType::TOML
        );
    }

    #[test]
    fn configuration_from_file_loads_json_correctly() {
        let json_content = r#"
            {
                "project": {
                    "move_sources": ["/path/to/move/source"]
                }
            }
        "#;
        fs::write("test.json", json_content).unwrap();
        let config = Configuration::from_file(Path::new("test.json")).unwrap();
        assert_eq!(config.project.move_sources, vec!["/path/to/move/source"]);
        fs::remove_file("test.json").unwrap();
    }

    #[test]
    fn configuration_from_file_loads_toml_correctly() {
        let toml_content = r#"
            [project]
            move_sources = ["/path/to/move/source"]
        "#;
        fs::write("test.toml", toml_content).unwrap();
        let config = Configuration::from_file(Path::new("test.toml")).unwrap();
        assert_eq!(config.project.move_sources, vec!["/path/to/move/source"]);
        fs::remove_file("test.toml").unwrap();
    }

    #[test]
    fn configuration_from_file_fails_for_unknown_file_type() {
        let result = Configuration::from_file(Path::new("test.unknown"));
        assert!(result.is_err());
    }
}
