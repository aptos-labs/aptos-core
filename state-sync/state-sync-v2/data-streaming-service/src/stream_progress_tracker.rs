// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    streaming_client::{Epoch, GetAllEpochEndingLedgerInfosRequest, StreamRequest},
};
use diem_data_client::AdvertisedData;
use itertools::Itertools;

/// An enum holding different types of data streams and progress tracking
/// indicators for tracking and serving that data along the stream.
#[derive(Debug)]
pub enum StreamProgressTracker {
    EpochEndingStreamTracker(EpochEndingStreamTracker),
}

impl StreamProgressTracker {
    pub fn new(
        stream_request: &StreamRequest,
        advertised_data: &AdvertisedData,
    ) -> Result<StreamProgressTracker, Error> {
        match stream_request {
            StreamRequest::GetAllEpochEndingLedgerInfos(request) => {
                StreamProgressTracker::new_epoch_ending_stream_tracker(request, advertised_data)
            }
            _ => Err(Error::UnsupportedRequestEncountered(format!(
                "Stream request not currently supported: {:?}",
                stream_request
            ))),
        }
    }

    fn new_epoch_ending_stream_tracker(
        request: &GetAllEpochEndingLedgerInfosRequest,
        advertised_data: &AdvertisedData,
    ) -> Result<StreamProgressTracker, Error> {
        let end_epoch = match most_common_highest_epoch(advertised_data) {
            Some(max_advertised_epoch) => {
                if max_advertised_epoch == 0 {
                    return Err(Error::NoDataToFetch(
                        "The maximum advertised epoch is 0. No epoch changes have occurred!".into(),
                    ));
                } else {
                    max_advertised_epoch.checked_sub(1).ok_or_else(|| {
                        Error::IntegerOverflow("Maximum advertised epoch has underflow!".into())
                    })?
                }
            }
            None => {
                return Err(Error::DataIsUnavailable(format!(
                    "Unable to find any maximum advertised epoch in the network: {:?}",
                    advertised_data
                )));
            }
        };

        if end_epoch < request.start_epoch {
            return Err(Error::DataIsUnavailable(format!(
                "The epoch to start syncing from is higher than any advertised highest epoch! Highest: {:?}, start: {:?}",
                end_epoch, request.start_epoch
            )));
        }

        Ok(StreamProgressTracker::EpochEndingStreamTracker(
            EpochEndingStreamTracker {
                request: request.clone(),
                end_epoch,
                current_epoch: request.start_epoch,
            },
        ))
    }
}

/// Returns the most common highest epoch advertised in the network.
/// Note: we use this to reduce the likelihood of malicious nodes
/// interfering with syncing progress by advertising non-existent epochs.
fn most_common_highest_epoch(advertised_data: &AdvertisedData) -> Option<Epoch> {
    // Count the frequencies of the highest epochs
    let highest_epoch_frequencies = advertised_data
        .epoch_ending_ledger_infos
        .iter()
        .map(|epoch_range| epoch_range.highest)
        .clone()
        .counts();

    // Return the most common epoch
    highest_epoch_frequencies
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(epoch, _)| epoch)
}

#[derive(Debug)]
pub struct EpochEndingStreamTracker {
    // The original epoch ending ledger infos request made by the client
    pub request: GetAllEpochEndingLedgerInfosRequest,

    // The last epoch ending ledger info that this stream will send to the client
    pub end_epoch: Epoch,

    // The current epoch that we're waiting to send to the client along the
    // stream. All epochs before this have already been sent.
    pub current_epoch: Epoch,
}
