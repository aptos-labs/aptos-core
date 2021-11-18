// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_json_rpc_types::response::{
    JsonRpcResponse, X_DIEM_CHAIN_ID, X_DIEM_TIMESTAMP_USEC_ID, X_DIEM_VERSION_ID,
};
use std::cmp::max;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct State {
    pub chain_id: u8,
    pub version: u64,
    pub timestamp_usecs: u64,
}

impl State {
    pub fn from_response(resp: &JsonRpcResponse) -> Self {
        Self {
            chain_id: resp.diem_chain_id,
            version: resp.diem_ledger_version,
            timestamp_usecs: resp.diem_ledger_timestampusec,
        }
    }

    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> anyhow::Result<Self> {
        let maybe_chain_id = headers
            .get(X_DIEM_CHAIN_ID)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_version = headers
            .get(X_DIEM_VERSION_ID)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());
        let maybe_timestamp = headers
            .get(X_DIEM_TIMESTAMP_USEC_ID)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok());

        let state = if let (Some(chain_id), Some(version), Some(timestamp_usecs)) =
            (maybe_chain_id, maybe_version, maybe_timestamp)
        {
            Self {
                chain_id,
                version,
                timestamp_usecs,
            }
        } else {
            todo!()
        };

        Ok(state)
    }
}

cfg_async_or_blocking! {
    use crate::{Error, Result};

    #[derive(Debug)]
    pub(crate) struct StateManager {
        last_known_state: std::sync::Mutex<Option<State>>,
    }

    impl Clone for StateManager {
        fn clone(&self) -> Self {
            Self::default()
        }
    }

    impl Default for StateManager {
        fn default() -> Self {
            Self {
                last_known_state: std::sync::Mutex::new(None),
            }
        }
    }

    impl StateManager {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn last_known_state(&self) -> Option<State> {
            self.last_known_state.lock().unwrap().clone()
        }

        pub(crate) fn update_state(&self, ignore_stale: bool, req_state: Option<&State>, resp_state: &State) -> Result<()> {
            // Ensure the response is fulfilled at a more recent ledger version than
            // when we made the request, though not necessarily the globally most
            // recent version.
            if let Some(req_state) = req_state {
                if !ignore_stale && resp_state < req_state {
                    return Err(Error::stale(format!(
                        "received response with stale metadata: {:?}, expected a response more recent than: {:?}",
                        resp_state,
                        req_state,
                    )));
                }
            }

            let mut state_writer = self.last_known_state.lock().unwrap();
            let curr_state = &*state_writer;

            assert!(
                req_state <= curr_state.as_ref(),
                "request state is not an ancestor state of the current latest state: \
                 request state: {:?}, current state: {:?}",
                req_state,
                curr_state,
            );

            // Compute the most recent state
            let new_state = if let Some(curr_state) = curr_state {
                // For now, trust-on-first-use for the chain id
                if curr_state.chain_id != resp_state.chain_id {
                    return Err(Error::chain_id(curr_state.chain_id, resp_state.chain_id));
                }
                max(curr_state, resp_state)
            } else {
                resp_state
            };

            // Store the new state
            *state_writer = Some(new_state.clone());
            Ok(())
        }
    }
}
