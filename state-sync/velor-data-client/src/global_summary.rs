// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_storage_service_types::{responses::CompleteDataRange, Epoch};
use velor_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use itertools::Itertools;
use std::{fmt, fmt::Display};

/// A snapshot of the global state of data available in the Velor network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalDataSummary {
    pub advertised_data: AdvertisedData,
    pub optimal_chunk_sizes: OptimalChunkSizes,
}

impl GlobalDataSummary {
    /// Returns an empty global data summary. This can be used on startup
    /// before the global state is known, or for testing.
    pub fn empty() -> Self {
        GlobalDataSummary {
            advertised_data: AdvertisedData::empty(),
            optimal_chunk_sizes: OptimalChunkSizes::empty(),
        }
    }

    /// Returns true iff the global data summary is empty
    pub fn is_empty(&self) -> bool {
        self == &Self::empty()
    }
}

impl Display for GlobalDataSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, {:?}",
            self.advertised_data, self.optimal_chunk_sizes
        )
    }
}

/// Holds the optimal chunk sizes that clients should use when
/// requesting data. This makes the request *more likely* to succeed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimalChunkSizes {
    pub epoch_chunk_size: u64,
    pub state_chunk_size: u64,
    pub transaction_chunk_size: u64,
    pub transaction_output_chunk_size: u64,
}

impl OptimalChunkSizes {
    pub fn empty() -> Self {
        OptimalChunkSizes {
            epoch_chunk_size: 0,
            state_chunk_size: 0,
            transaction_chunk_size: 0,
            transaction_output_chunk_size: 0,
        }
    }
}

/// A summary of all data that is currently advertised in the network.
#[derive(Clone, Eq, PartialEq)]
pub struct AdvertisedData {
    /// The ranges of epoch ending ledger infos advertised, e.g., if a range
    /// is (X,Y), it means all epoch ending ledger infos for epochs X->Y
    /// (inclusive) are available.
    pub epoch_ending_ledger_infos: Vec<CompleteDataRange<Epoch>>,

    /// The ranges of states advertised, e.g., if a range is
    /// (X,Y), it means all states are held for every version X->Y
    /// (inclusive).
    pub states: Vec<CompleteDataRange<Version>>,

    /// The ledger infos corresponding to the highest synced versions
    /// currently advertised.
    pub synced_ledger_infos: Vec<LedgerInfoWithSignatures>,

    /// The ranges of transactions advertised, e.g., if a range is
    /// (X,Y), it means all transactions for versions X->Y (inclusive)
    /// are available.
    pub transactions: Vec<CompleteDataRange<Version>>,

    /// The ranges of transaction outputs advertised, e.g., if a range
    /// is (X,Y), it means all transaction outputs for versions X->Y
    /// (inclusive) are available.
    pub transaction_outputs: Vec<CompleteDataRange<Version>>,
}

impl fmt::Debug for AdvertisedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let synced_ledger_infos = self
            .synced_ledger_infos
            .iter()
            .map(|LedgerInfoWithSignatures::V0(ledger)| {
                let version = ledger.commit_info().version();
                let epoch = ledger.commit_info().epoch();
                let ends_epoch = ledger.commit_info().next_epoch_state().is_some();
                format!(
                    "(Version: {:?}, Epoch: {:?}, Ends epoch: {:?})",
                    version, epoch, ends_epoch
                )
            })
            .join(", ");
        write!(
            f,
            "epoch_ending_ledger_infos: {:?}, states: {:?}, synced_ledger_infos: [{}], transactions: {:?}, transaction_outputs: {:?}",
            &self.epoch_ending_ledger_infos, &self.states, synced_ledger_infos, &self.transactions, &self.transaction_outputs
        )
    }
}

/// Provides an aggregated version of all advertised data (i.e, highest and lowest)
impl Display for AdvertisedData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Calculate the highest advertised data
        let highest_epoch_ending_ledger_info = self.highest_epoch_ending_ledger_info();
        let highest_synced_ledger_info = self.highest_synced_ledger_info();
        let highest_synced_version = highest_synced_ledger_info
            .as_ref()
            .map(|li| li.ledger_info().version());
        let highest_synced_epoch = highest_synced_ledger_info.map(|li| li.ledger_info().epoch());

        // Calculate the lowest advertised data
        let lowest_transaction_version = self.lowest_transaction_version();
        let lowest_output_version = self.lowest_transaction_output_version();
        let lowest_states_version = self.lowest_state_version();

        write!(
            f,
            "AdvertisedData {{ Highest epoch ending ledger info, epoch: {:?}. Highest synced ledger info, epoch: {:?}, version: {:?}. \
            Lowest transaction version: {:?}, Lowest transaction output version: {:?}, Lowest states version: {:?} }}",
            highest_epoch_ending_ledger_info, highest_synced_epoch, highest_synced_version,
            lowest_transaction_version, lowest_output_version, lowest_states_version
        )
    }
}

impl AdvertisedData {
    pub fn empty() -> Self {
        AdvertisedData {
            epoch_ending_ledger_infos: vec![],
            states: vec![],
            synced_ledger_infos: vec![],
            transactions: vec![],
            transaction_outputs: vec![],
        }
    }

    /// Returns true iff all data items (`lowest` to `highest`, inclusive) can
    /// be found in the given `advertised_ranges`.
    pub fn contains_range(
        lowest: u64,
        highest: u64,
        advertised_ranges: &[CompleteDataRange<u64>],
    ) -> bool {
        for item in lowest..=highest {
            let mut item_exists = false;

            for advertised_range in advertised_ranges {
                if advertised_range.contains(item) {
                    item_exists = true;
                    break;
                }
            }

            if !item_exists {
                return false;
            }
        }
        true
    }

    /// Returns the highest epoch ending ledger info advertised in the network
    pub fn highest_epoch_ending_ledger_info(&self) -> Option<Epoch> {
        self.epoch_ending_ledger_infos
            .iter()
            .map(|epoch_range| epoch_range.highest())
            .max()
    }

    /// Returns the highest synced ledger info advertised in the network
    pub fn highest_synced_ledger_info(&self) -> Option<LedgerInfoWithSignatures> {
        let highest_synced_position = self
            .synced_ledger_infos
            .iter()
            .map(|ledger_info_with_sigs| ledger_info_with_sigs.ledger_info().version())
            .position_max();

        if let Some(highest_synced_position) = highest_synced_position {
            self.synced_ledger_infos
                .get(highest_synced_position)
                .cloned()
        } else {
            None
        }
    }

    /// Returns the lowest advertised version containing all states
    pub fn lowest_state_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.states)
    }

    /// Returns the lowest advertised transaction output version
    pub fn lowest_transaction_output_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.transaction_outputs)
    }

    /// Returns the lowest advertised transaction version
    pub fn lowest_transaction_version(&self) -> Option<Version> {
        get_lowest_version_from_range_set(&self.transactions)
    }
}

/// Returns the lowest version from the given set of data ranges
fn get_lowest_version_from_range_set(
    data_ranges: &[CompleteDataRange<Version>],
) -> Option<Version> {
    data_ranges
        .iter()
        .map(|data_range| data_range.lowest())
        .min()
}
