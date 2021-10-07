// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    stream_progress_tracker::{EpochEndingStreamTracker, StreamProgressTracker},
};
use diem_data_client::AdvertisedData;

/// Verifies that the data required by the given data stream tracker can be
/// satisfied using the currently advertised data available in the network.
/// If not, an error is returned.
pub fn ensure_data_is_available(
    stream_progress_tracker: &StreamProgressTracker,
    advertised_data: &AdvertisedData,
) -> Result<(), Error> {
    match stream_progress_tracker {
        StreamProgressTracker::EpochEndingStreamTracker(stream_tracker) => {
            if epoch_ending_ledger_infos_available(stream_tracker, advertised_data) {
                return Ok(());
            }
        }
    }

    Err(Error::DataIsUnavailable(format!(
        "Unable to satisfy requested data stream: {:?}, with advertised data: {:?}",
        stream_progress_tracker, advertised_data
    )))
}

/// Returns true iff all epoch ending ledger infos are available in the
/// advertised epoch ending ledger infos.
fn epoch_ending_ledger_infos_available(
    epoch_ending_stream_tracker: &EpochEndingStreamTracker,
    advertised_data: &AdvertisedData,
) -> bool {
    let start_epoch = epoch_ending_stream_tracker.request.start_epoch;
    let end_epoch = epoch_ending_stream_tracker.end_epoch;

    // Verify all epoch ending ledger infos can be found in the advertised data
    for epoch in start_epoch..=end_epoch {
        let mut epoch_exists = false;
        for epoch_range in &advertised_data.epoch_ending_ledger_infos {
            if !epoch_exists && epoch_range.contains(epoch) {
                epoch_exists = true;
            }
        }

        if !epoch_exists {
            return false;
        }
    }
    true
}
