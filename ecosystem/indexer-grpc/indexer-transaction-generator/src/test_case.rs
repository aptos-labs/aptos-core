// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};

const TEST_CONFIG_FILE: &str = "config.json";

/// Regex pattern to match the setup folder.
const SETUP_FOLDER_PATTERN: &str = r"^setup_(\d+)(?:_[a-zA-Z0-9-]*)?$";
const ACTION_FOLDER_PATTERN: &str = r"^action_(\d+)(?:_[a-zA-Z0-9-]*)?$";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct TestCaseConfig {
    pub fund_amount: Option<u64>,
}

/// TestCase is to validate the test case and store the test case metadata.
/// Each test case is a folder containing:
///  - folder: setup_1, setup_2, ...: the setup __folders__ to prepare the test environment.
///  - action_1, action_2, ...: the action __folders__ to execute the test.
///  - test_config.json: the test case configuration file.
/// Test case is expected to be run in order: setups first, then actions.
pub struct TestCase {
    // Test case name.
    pub name: String,
    // Ordered steps from setup to action.
    pub steps: Vec<Step>,
    // Test case configuration.
    pub test_config: Option<TestCaseConfig>,
}

impl TestCase {
    pub fn load(test_case_path: PathBuf) -> Result<Self, anyhow::Error> {
        // Load the test case configuration.
        let test_config_path = test_case_path.join(TEST_CONFIG_FILE);
        let test_config = match test_config_path.exists() {
            true => {
                let test_config_raw_string = std::fs::read_to_string(test_config_path)
                    .context("Failed to read test config file.")?;
                let test_config: TestCaseConfig = serde_json::from_str(&test_config_raw_string)
                    .context("Failed to parse test config file.")?;
                Some(test_config)
            },
            false => None,
        };

        // Get steps.
        let mut steps = vec![];
        let test_case = std::fs::read_dir(&test_case_path).context(format!(
            "Failed to read test case folder at {:?}",
            &test_case_path
        ))?;
        for step in test_case {
            let step = step.context(format!(
                "Failed to read step folder at {:?}",
                &test_case_path
            ))?;
            let step_path = step.path();
            let step = Step::try_from(step_path)?;
            steps.push(step);
        }

        // Filter out the unknown steps.
        let mut steps: Vec<Step> = steps
            .into_iter()
            .filter(|step| !matches!(step, Step::UNKNOWN))
            .collect();

        // Sort the steps by priority.
        steps.sort_by_key(|step| step.priority());

        let name = test_case_path
            .file_name()
            .context(format!(
                "Failed to get test case name at {:?}",
                &test_case_path
            ))?
            .to_str()
            .context(format!(
                "Failed to get test case name at {:?}",
                &test_case_path
            ))?
            .to_string();
        Ok(TestCase {
            name,
            steps,
            test_config,
        })
    }
}

/// Step is a wrapper around the path of the setup and action folders.
/// It's to facilitate the validation of the test case.
/// Error vs UNKNOWN:
///  - If a PathBuf can be regex matched to a setup or action step, but cannot be parsed as a folder, it's an error.
///  - If a PathBuf cannot be regex matched to a setup or action folder, it's UNKNOWN.
#[derive(PartialEq, Eq, Debug, Hash)]
pub enum Step {
    Setup((PathBuf, u64, Option<TestCaseConfig>)),
    Action((PathBuf, u64)),
    /// This is to handle the case when the step is not a setup or action folder.
    UNKNOWN,
}

impl Step {
    fn priority(&self) -> u64 {
        match self {
            Step::Setup((_, index, _)) => *index,
            Step::Action((_, index)) => *index + u64::MAX / 2,
            Step::UNKNOWN => u64::MAX,
        }
    }
}

impl Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Step::Setup((_, index, _)) => write!(f, "Setup {}", index),
            Step::Action((_, index)) => write!(f, "Action {}", index),
            Step::UNKNOWN => write!(f, "UNKNOWN"),
        }
    }
}

impl TryFrom<PathBuf> for Step {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file_name = path
            .file_name()
            .context(format!("Path file name cannot be parsed at {:?}", &path))?;
        let file_name = file_name
            .to_str()
            .context(format!("Path file name cannot be parsed at {:?}", &path))?;
        // Use regex to determine the step type and index.
        let setup_folder = regex::Regex::new(SETUP_FOLDER_PATTERN)
            .expect("Regex pattern for setup folder is invalid.");
        let action_folder = regex::Regex::new(ACTION_FOLDER_PATTERN)
            .expect("Regex pattern for action folder is invalid.");
        if let Some(index) = setup_folder.captures(file_name) {
            let index = index
                .get(1)
                .context("Setup folder in the wrong format")?
                .as_str()
                .parse::<u64>()
                .context("Folder index is invalid.")?;
            // If it's not a folder, return an error.
            if !path.is_dir() {
                return Err(anyhow::anyhow!("Setup step is not a folder."));
            }
            // Load the test case configuration.
            let test_config_path = path.join(TEST_CONFIG_FILE);
            let test_config = match test_config_path.exists() {
                true => {
                    let test_config_raw_string = std::fs::read_to_string(test_config_path)
                        .context("Failed to read test config file.")?;
                    let test_config: TestCaseConfig = serde_json::from_str(&test_config_raw_string)
                        .context("Failed to parse test config file.")?;
                    Some(test_config)
                },
                false => None,
            };

            return Ok(Step::Setup((path, index, test_config)));
        }

        if let Some(index) = action_folder.captures(file_name) {
            let index = index
                .get(1)
                .context("Action folder in the wrong format")?
                .as_str()
                .parse::<u64>()
                .context("Folder index is invalid.")?;
            // If it's not a folder, return an error.
            if !path.is_dir() {
                return Err(anyhow::anyhow!("Action step is not a folder."));
            }
            return Ok(Step::Action((path, index)));
        }
        Ok(Step::UNKNOWN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use tempfile::TempDir;
    #[test]
    fn test_try_from_pathbuf() {
        let temp_dir = TempDir::new().unwrap();
        // Create a setup folder.
        let setup_folder = temp_dir.path().join("setup_1");
        std::fs::create_dir(&setup_folder).unwrap();
        // Create an action folder.
        let action_folder = temp_dir.path().join("action_1");
        std::fs::create_dir(&action_folder).unwrap();
        // Create an unknown folder.
        let unknown_folder = temp_dir.path().join("unknown");
        std::fs::create_dir(&unknown_folder).unwrap();
        // Supports the setup and action folders with suffix.
        let setup_folder_with_suffix = temp_dir.path().join("setup_1_suffix");
        std::fs::create_dir(&setup_folder_with_suffix).unwrap();
        // Supports the setup and action folders with suffix.
        let action_folder_with_suffix = temp_dir.path().join("action_1_suffix");
        std::fs::create_dir(&action_folder_with_suffix).unwrap();
        let setup_step = Step::try_from(setup_folder).unwrap();
        let action_step = Step::try_from(action_folder).unwrap();
        let unknown_step = Step::try_from(unknown_folder).unwrap();
        let setup_step_with_suffix = Step::try_from(setup_folder_with_suffix).unwrap();
        let action_step_with_suffix = Step::try_from(action_folder_with_suffix).unwrap();
        match setup_step {
            Step::Setup((_, index, _)) => assert_eq!(index, 1),
            _ => panic!("Setup step is not parsed correctly."),
        }
        match action_step {
            Step::Action((_, index)) => assert_eq!(index, 1),
            _ => panic!("Action step is not parsed correctly."),
        }
        match unknown_step {
            Step::UNKNOWN => {},
            _ => panic!("Unknown step is not parsed correctly."),
        }
        match setup_step_with_suffix {
            Step::Setup((_, index, _)) => assert_eq!(index, 1),
            _ => panic!("Setup step with suffix is not parsed correctly."),
        }
        match action_step_with_suffix {
            Step::Action((_, index)) => assert_eq!(index, 1),
            _ => panic!("Action step with suffix is not parsed correctly."),
        }
    }

    #[test]
    fn test_step_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let setup_folder_1 = temp_dir.path().join("setup_1");
        std::fs::create_dir(&setup_folder_1).unwrap();
        let setup_folder_2 = temp_dir.path().join("setup_2");
        std::fs::create_dir(&setup_folder_2).unwrap();
        let action_folder_1 = temp_dir.path().join("action_1");
        std::fs::create_dir(&action_folder_1).unwrap();
        let action_folder_2 = temp_dir.path().join("action_2");
        std::fs::create_dir(&action_folder_2).unwrap();
        let steps: Vec<Step> = vec![
            Step::try_from(setup_folder_1).unwrap(),
            Step::try_from(setup_folder_2).unwrap(),
            Step::try_from(action_folder_1).unwrap(),
            Step::try_from(action_folder_2).unwrap(),
        ];

        // Get all permutations of the steps.
        for perm in steps.iter().permutations(steps.len()).unique() {
            let mut perm = perm.clone();
            perm.sort_by_key(|step| step.priority());
            assert_eq!(perm[0].to_string(), "Setup 1");
            assert_eq!(perm[1].to_string(), "Setup 2");
            assert_eq!(perm[2].to_string(), "Action 1");
            assert_eq!(perm[3].to_string(), "Action 2");
        }
    }

    #[test]
    fn test_simple_case() {
        // Read from src/tests/simple_test.
        let test_case_path = PathBuf::from("src/tests/simple_test");
        let test_case = TestCase::load(test_case_path).unwrap();
        assert_eq!(test_case.name, "simple_test");
        assert_eq!(test_case.steps.len(), 4);
        assert_eq!(test_case.test_config.unwrap().fund_amount, Some(100));
    }

    #[test]
    fn test_malformed_case() {
        // Read from src/tests/simple_test.
        let test_case_path = PathBuf::from("src/tests/malformed_test");
        let test_case = TestCase::load(test_case_path);
        assert!(test_case.is_err());
    }
}
