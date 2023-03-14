// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::endpoints::{AptosTapError, RejectionReason};
use anyhow::Result;
use aptos_sdk::types::account_address::AccountAddress;
use async_trait::async_trait;
use poem::http::HeaderMap;
use std::{net::IpAddr, sync::Arc};
use tokio::task::JoinSet;

// Note: At one point I had an undo function that worked like this:
//
//   If a Checker returned no rejection reasons, but a later Checker
//   did, the first checker might want to undo anything it did. It can
//   do so in this function.
//
// I decided this was needlessly complex, and I saw a few ways it could
// be possible to abuse this mechanism to overload the tap if the `undo`
// implementation wasn't very careful. So I removed it. This means that
// this scenario is possible:
//
//   1. User makes request.
//   2. It passes some ratelimiting check and we increment the ratelimit.
//   3. It fails some other check.
//   4. We reject the request, without decrementing the ratelimit.
//
// Indeed, this is probably desirable behavior anyway.

/// Implementers of this trait are responsible for checking something about the
/// request, and if it doesn't look valid, returning a list of rejection reasons
/// explaining why. It may also do something extra after the funding happened
/// if there is something to clean up afterwards.
#[async_trait]
pub trait Checker: Sync + Send + 'static {
    /// Returns a list of rejection reasons for the request, if any. If dry_run
    /// is set, if this Checker would store anything based on the request, it
    /// instead will not. This is useful for the is_eligible endpoint.
    async fn check(
        &self,
        data: CheckerData,
        dry_run: bool,
    ) -> Result<Vec<RejectionReason>, AptosTapError>;

    /// If the Checker wants to do anything after the funding has completed, it
    /// may do so in this function. For example, for the storage Checkers, this
    /// function is responsible for marking a request in storage as complete,
    /// in both success and failure cases. It can also store additional metadata
    /// included in CompleteData that we might have from the call to the Funder.
    /// No dry_run flag for this, because we should never need to run this in
    /// dry_run mode.
    async fn complete(&self, _data: CompleteData) -> Result<(), AptosTapError> {
        Ok(())
    }

    /// Aribtrary cost, where lower is less cost. We use these to determine the
    /// order we run checkers.
    fn cost(&self) -> u8;

    /// This function will be called once at startup. In it, the trait implementation
    /// should spawn any periodic tasks that it wants and return handles to them.
    /// If tasks want to signal that there is an issue, all they have to do is return.
    /// If the task wants to tolerate some errors, e.g. only cause the process to die
    /// if the task has failed n times, it must handle that itself and only return
    /// when it wants this to happen.
    // Sadly we can't use ! here yet: https://github.com/rust-lang/rust/issues/35121.
    fn spawn_periodic_tasks(&self, _join_set: &mut JoinSet<anyhow::Result<()>>) {}
}

#[derive(Clone, Debug)]
pub struct CheckerData {
    pub time_request_received_secs: u64,
    pub amount: u64,
    pub receiver: AccountAddress,
    pub source_ip: IpAddr,
    pub headers: Arc<HeaderMap>,
}

#[derive(Clone, Debug)]
pub struct CompleteData {
    pub checker_data: CheckerData,
    pub txn_hashes: Vec<String>,
    pub response_is_500: bool,
}
