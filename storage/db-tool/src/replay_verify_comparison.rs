// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_logger::info;
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        diff::TransactionDiff,
        diff_filter::{ChunkFilter, DiffFilter},
        TransactionAuxiliaryData, TransactionInfo, TransactionOutput, TransactionStatus, Version,
    },
    write_set::WriteSet,
};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Configuration for flexible replay verification that can also be loaded from JSON.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReplayVerifyComparisonConfig {
    /// Configuration with exact comparisons, i.e., no differences in any part of the
    /// transaction outputs are allowed.
    Exact,
    /// Configuration with relaxed comparison: filters are applied (in-order) to the diff to ignore
    /// certain differences, e.g., gas used, for different ranges of transactions.
    Relaxed(Vec<ChunkFilter>),
}

impl ReplayVerifyComparisonConfig {
    /// Loads configuration from a JSON file. Supports single-line comments starting with `//`.
    #[allow(dead_code)]
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|err| anyhow!("Failed to read config file {}: {}", path.display(), err))?;

        let cleaned_content = content
            .lines()
            .map(|line| {
                if let Some(comment_pos) = line.find("//") {
                    &line[..comment_pos]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let config = serde_json::from_str::<Self>(&cleaned_content)
            .map_err(|err| anyhow!("Failed to parse JSON config: {}", err))?;
        config.validate()?;
        Ok(config)
    }

    /// Analyze differences between executed and expected transaction outputs
    pub fn ensure_match_transaction_info(
        &self,
        version: Version,
        output: &TransactionOutput,
        expected_txn_info: &TransactionInfo,
        expected_write_set: &WriteSet,
        expected_events: &[ContractEvent],
        fee_payer: Option<AccountAddress>,
    ) -> Result<()> {
        // Always run exact comparison first - this will not compare individual events or writes,
        // instead comparing hashes. Fallback to relaxed comparison only if there is a mismatch.
        let exact_comparison_result = output.ensure_match_transaction_info(
            version,
            expected_txn_info,
            Some(expected_write_set),
            Some(expected_events),
        );
        let filters = match self {
            ReplayVerifyComparisonConfig::Exact => {
                // Exact comparison by equality.
                return exact_comparison_result;
            },
            ReplayVerifyComparisonConfig::Relaxed(filters) if exact_comparison_result.is_err() => {
                filters
            },
            ReplayVerifyComparisonConfig::Relaxed(_) => {
                // Exact comparison succeeded, no need to check anything else.
                return Ok(());
            },
        };

        let expected_output = self.build_expected_output(
            expected_txn_info,
            expected_write_set.clone(),
            expected_events.to_vec(),
        );
        let mut diff =
            TransactionDiff::build_from_outputs(expected_output, output.clone(), fee_payer);
        for filter in filters
            .iter()
            .filter(|filter| filter.applies_to_version(version))
        {
            let original_num_differences = diff.num_differences();
            diff = diff.evaluate(&filter.filter);

            if diff.num_differences() < original_num_differences {
                info!(
                    "Filter applied to transaction at version {}: {:?} reduced diff from {} to {} items",
                    version,
                    filter.filter,
                    original_num_differences,
                    diff.num_differences()
                );
            }

            if diff.is_empty() {
                return Ok(());
            }
        }

        Err(anyhow!(
            "TransactionOutput does not match TransactionInfo: {:?}",
            diff
        ))
    }
}

// Private interfaces.
impl ReplayVerifyComparisonConfig {
    /// Validates replay configuration.
    fn validate(&self) -> Result<()> {
        let filters = match self {
            ReplayVerifyComparisonConfig::Exact => {
                return Ok(());
            },
            ReplayVerifyComparisonConfig::Relaxed(filter) => filter,
        };

        for filter in filters {
            if let Some(range) = filter.range.as_ref() {
                if range.start > range.end {
                    return Err(anyhow!(
                        "Invalid version range: start {} > end {}",
                        range.start,
                        range.end
                    ));
                }

                match &filter.filter {
                    DiffFilter::GasChange {
                        min_delta,
                        max_delta,
                    } => {
                        if let (Some(min_delta), Some(max_delta)) = (min_delta, max_delta) {
                            if min_delta > max_delta {
                                return Err(anyhow!(
                                    "Invalid allowed gas delta: min {} > max {}",
                                    min_delta,
                                    max_delta
                                ));
                            }
                        }
                    },
                    DiffFilter::SoftStatusChange { from, to }
                    | DiffFilter::HardStatusChange { from, to } => {
                        if from == to {
                            return Err(anyhow!(
                                "Invalid status change: cannot have same statuses ({:?})",
                                from
                            ));
                        }
                    },
                }
            }
        }

        Ok(())
    }

    /// Builds expected [TransactionOutput] from individual components for comparison.
    fn build_expected_output(
        &self,
        txn_info: &TransactionInfo,
        write_set: WriteSet,
        events: Vec<ContractEvent>,
    ) -> TransactionOutput {
        TransactionOutput::new(
            write_set,
            events,
            txn_info.gas_used(),
            TransactionStatus::Keep(txn_info.status().clone()),
            // Auxiliary data is irrelevant for comparison.
            TransactionAuxiliaryData::None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::transaction::{diff_filter::DiffFilter, ExecutionStatus};
    use move_core_types::vm_status::StatusCode;

    fn parse_relaxed_config(json: &str, expected_filter_count: usize) -> Vec<ChunkFilter> {
        let config = serde_json::from_str::<ReplayVerifyComparisonConfig>(json).unwrap();
        if let ReplayVerifyComparisonConfig::Relaxed(filters) = config {
            assert_eq!(filters.len(), expected_filter_count);
            filters
        } else {
            panic!("Expected relaxed config");
        }
    }

    fn parse_single_filter_config(json: &str) -> ChunkFilter {
        let mut filters = parse_relaxed_config(json, 1);
        filters.pop().unwrap()
    }

    fn assert_range(filter: &ChunkFilter, expected_start: Option<u64>, expected_end: Option<u64>) {
        match (filter.range.as_ref(), expected_start, expected_end) {
            (Some(range), Some(start), Some(end)) => {
                assert_eq!(range.start, start);
                assert_eq!(range.end, end);
            },
            (None, None, None) => {},
            _ => panic!("Range mismatch"),
        }
    }

    #[test]
    fn test_exact_config_json() {
        let json = r#""exact""#;
        let config = serde_json::from_str::<ReplayVerifyComparisonConfig>(json).unwrap();
        matches!(config, ReplayVerifyComparisonConfig::Exact);
    }

    #[test]
    fn test_move_abort_status_change_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "soft_status_change": {
                            "from": "Success",
                            "to": {
                                "MoveAbort": {
                                    "location": {
                                        "Module": {
                                            "address": "0x1",
                                            "name": "coin"
                                        }
                                    },
                                    "code": 65542,
                                    "info": {
                                        "reason_name": "EINSUFFICIENT_BALANCE",
                                        "description": "Insufficient balance"
                                    }
                                }
                            }
                        }
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert!(matches!(filter.filter,
            DiffFilter::SoftStatusChange {
                from: ExecutionStatus::Success,
                to: ExecutionStatus::MoveAbort { code, .. }
            } if code == 65542
        ));
        assert_range(&filter, None, None);
    }

    #[test]
    fn test_multiple_filters_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "gas_change": {
                            "max_delta": 50
                        }
                    },
                    "range": {
                        "start": 0,
                        "end": 1000
                    }
                },
                {
                    "filter": {
                        "hard_status_change": {
                            "from": "Success",
                            "to": "OutOfGas"
                        }
                    }
                }
            ]
        }"#;

        let filters = parse_relaxed_config(json, 2);

        assert_eq!(filters[0].filter, DiffFilter::GasChange {
            min_delta: None,
            max_delta: Some(50)
        });
        assert_range(&filters[0], Some(0), Some(1000));

        assert_eq!(filters[1].filter, DiffFilter::HardStatusChange {
            from: ExecutionStatus::Success,
            to: ExecutionStatus::OutOfGas
        });
        assert_range(&filters[1], None, None);
    }

    #[test]
    fn test_miscellaneous_error_status_change_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "soft_status_change": {
                            "from": {
                                "MiscellaneousError": 1124
                            },
                            "to": {
                                "MiscellaneousError": 1130
                            }
                        }
                    },
                    "range": {
                        "start": 0,
                        "end": 9223372036854775807
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::SoftStatusChange {
            from: ExecutionStatus::MiscellaneousError(Some(StatusCode::DEPENDENCY_LIMIT_REACHED)),
            to: ExecutionStatus::MiscellaneousError(Some(StatusCode::ZERO_VARIANTS_ERROR))
        });
        assert_range(&filter, Some(0), Some(9223372036854775807));
    }

    #[test]
    fn test_execution_failure_status_change_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "hard_status_change": {
                            "from": "Success",
                            "to": {
                                "ExecutionFailure": {
                                    "location": {
                                        "Module": {
                                            "address": "0x1",
                                            "name": "test_module"
                                        }
                                    },
                                    "function": 5,
                                    "code_offset": 42
                                }
                            }
                        }
                    },
                    "range": {
                        "start": 100,
                        "end": 200
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert!(matches!(filter.filter,
            DiffFilter::HardStatusChange {
                from: ExecutionStatus::Success,
                to: ExecutionStatus::ExecutionFailure { function, code_offset, .. }
            } if function == 5 && code_offset == 42
        ));
        assert_range(&filter, Some(100), Some(200));
    }

    #[test]
    fn test_out_of_gas_status_change_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "soft_status_change": {
                            "from": "Success",
                            "to": "OutOfGas"
                        }
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::SoftStatusChange {
            from: ExecutionStatus::Success,
            to: ExecutionStatus::OutOfGas
        });
        assert_range(&filter, None, None);
    }

    #[test]
    fn test_relaxed_gas_change_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "gas_change": {
                            "min_delta": -100,
                            "max_delta": 100
                        }
                    },
                    "range": {
                        "start": 1000,
                        "end": 2000
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::GasChange {
            min_delta: Some(-100),
            max_delta: Some(100)
        });
        assert_range(&filter, Some(1000), Some(2000));
    }

    #[test]
    fn test_gas_change_with_both_deltas_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "gas_change": {
                            "min_delta": -500,
                            "max_delta": 1000
                        }
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::GasChange {
            min_delta: Some(-500),
            max_delta: Some(1000)
        });
        assert_range(&filter, None, None);
    }

    #[test]
    fn test_gas_change_min_delta_only_json() {
        let json = r#"{
            "relaxed": [
                {
                    "filter": {
                        "gas_change": {
                            "min_delta": -100
                        }
                    }
                }
            ]
        }"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::GasChange {
            min_delta: Some(-100),
            max_delta: None
        });
        assert_range(&filter, None, None);
    }

    #[test]
    fn test_json_with_comments() {
        let json = r#"{
            "relaxed": [ // List of relaxed filters
                {
                    "filter": {
                        "gas_change": { // Allow gas usage changes
                            "min_delta": -100, // Minimum allowed gas delta
                            "max_delta": 100   // Maximum allowed gas delta
                        }
                    }, // End filter
                    "range": {
                        "start": 1000, // Start version
                        "end": 2000    // End version
                    }
                } // End first filter
            ] // End relaxed array
        } // End config"#;

        let filter = parse_single_filter_config(json);
        assert_eq!(filter.filter, DiffFilter::GasChange {
            min_delta: Some(-100),
            max_delta: Some(100)
        });
        assert_range(&filter, Some(1000), Some(2000));
    }
}
