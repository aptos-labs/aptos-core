// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    stream_progress_tracker::StreamProgressTracker,
    streaming_client::{GetAllEpochEndingLedgerInfosRequest, StreamRequest},
};
use claim::assert_matches;
use diem_data_client::GlobalDataSummary;
use storage_service_types::CompleteDataRange;

#[test]
fn test_epoch_ending_stream_tracker() {
    // Create an epoch ending stream request
    let stream_request =
        StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
            start_epoch: 0,
        });

    // Try to create a progress stream tracker where there is no advertised data
    // and verify an error is returned.
    let result =
        StreamProgressTracker::new(&stream_request, &GlobalDataSummary::empty().advertised_data);
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Create a data summary with various advertised epoch ranges (common highest is zero)
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![
        CompleteDataRange::new(0, 0),
        CompleteDataRange::new(0, 0),
        CompleteDataRange::new(0, 1),
    ];

    // Try to create a progress stream tracker where the highest epoch is zero
    // and verify an error is returned.
    let result = StreamProgressTracker::new(&stream_request, &global_data_summary.advertised_data);
    assert_matches!(result, Err(Error::NoDataToFetch(_)));

    // Create a global data summary with non-zero advertised epoch ranges
    let mut global_data_summary = GlobalDataSummary::empty();
    global_data_summary
        .advertised_data
        .epoch_ending_ledger_infos = vec![
        CompleteDataRange::new(0, 1),
        CompleteDataRange::new(0, 100),
        CompleteDataRange::new(0, 99999999),
        CompleteDataRange::new(0, 100),
    ];

    // Create a new data stream progress tracker and verify the most common highest
    // epoch is chosen.
    let stream_progress_tracker =
        StreamProgressTracker::new(&stream_request, &global_data_summary.advertised_data).unwrap();
    let StreamProgressTracker::EpochEndingStreamTracker(stream_tracker) = stream_progress_tracker;
    assert_eq!(stream_tracker.end_epoch, 99); // End epoch is highest - 1
}
