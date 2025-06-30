// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{explicit_sync_wrapper::ExplicitSyncWrapper, scheduler_status::ExecutionStatuses};
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use crossbeam::utils::CachePadded;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    u32,
};

/**
 * In BlockSTMv2, validations are not scheduled in waves as separate tasks like
 * in BlockSTMv1. Instead normal validations occur granularly and on-demand, at
 * the time of particular updates. However, global code cache does not support
 * push validation by design. This because most blocks do not contain module
 * publishing, so the trade-off taken is to reduce the overhead on the common
 * read path. Instead, published modules become visible to other workers (executing
 * higher indexed txns) during a txn commit, and it is required that all txns
 * that are executed or executing to validate their module read set. This file
 * provides the primitives for BlockSTMv2 scheduler to manage such requirements.
 *
 * A high-level idea is that at any time, at most one worker is responsible for
 * fulfilling the module validation requirements for an interval of txns. The
 * interval starts at the index of a committed txn that published modules, and
 * ends at the first txn that has never been scheduled for execution. (Note: for
 * contended workloads, the scheduler currently may execute later txns early,
 * losing the benefits of this optimization for higher-indexed txns). The interval
 * induces a traversal of the interval to identify the set of txn versions
 * (txn index & incarnation pair) requiring module read set validation. In order
 * to reduce the time in critical (sequential) section of the code, the traversal
 * is performed after the txn is committed by the same worker if no requirements
 * were already active, or by the designated worker that may have already been
 * performing module validations. When this happens, the start of interval is
 * reset to the newly committed txn (which must be higher than recorded start
 * since txns can not be committed with unfulfilled requirements). The traversal
 * can be done locally, only needing access to the array of statuses. After the
 * traversal is finished and the requirements are properly recorded, the designated
 * worker may get module validation tasks to perform from scheduler's next_task
 * call - depending on a distance threshold from the committed prefix of the block.
 * The rationale for a distance threshold is to (a) prioritize more important
 * work and (b) avoid wasted work as txns that get re-executed after module
 * publishing (with higher incarnation) would no longer require module validation.
 *
 * When the interval is reset, the module requirements are combined together.
 * This might cause some txns to be validated against a module when strictly
 * speaking they would not require it. However, it allows a simpler implementation
 * that is easier to reason about, and is not expected to be a bottleneck.
 *
 * The implementation of ColdValidationRequirements is templated over the type of
 * the requirement. This allows easier testing, as well as future extensions to
 * other types of validation requirements that may be better offloaded to an uncommon
 * dedicated path for optimal performance. TODO(BlockSTMv2): a promising direction
 * is to enable caching use-cases in the VM, whereby cache invalidations might be
 * rare and infeasible to record every access for push validation.
 *
 * Finally, ColdValidationRequirements allows to cheaply check if a txn has
 * unfulfilled requirements, needed by the scheduler to avoid committing such txns.
 **/

// The requirements are active for the txns with indices keyed in the versions map,
// for corresponding specific incarnations.
#[derive(Debug)]
struct ActiveRequirements<R: Clone + Ord> {
    requirements: BTreeSet<R>,
    // txn_idx -> (incarnation, is_executing) implies that the specified incarnation
    // of the txn requires additional uncommon / cold validation to be performed before
    // it can be committed. At the time when the active requirement was recorded,
    // the status of the given incarnation must have been Executing or Executed (as
    // otherwise new incarnation will read updated information and not require additional
    // validation). The boolean is_executing distinguishes between the two cases.
    versions: BTreeMap<TxnIndex, (Incarnation, bool)>,
    // Used as a cache to avoid cloning the requirements for each txn that is executing.
    // For those txns the arced validation requirements are deferred in the status to
    // be performed after the incarnation finishes.
    maybe_arced_requirements: Option<Arc<BTreeSet<R>>>,
}

#[derive(Debug)]
struct PendingRequirement<R: Clone + Ord> {
    requirements: BTreeSet<R>,
    from_idx: TxnIndex,
    to_idx: TxnIndex,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ValidationRequirement<'a, R: Clone + Ord> {
    Active(&'a BTreeSet<R>),
    Deferred(&'a Arc<BTreeSet<R>>),
}

impl<'a, R: Clone + Ord> ValidationRequirement<'a, R> {
    fn new(active_reqs: &'a mut ActiveRequirements<R>, is_executing: bool) -> Self {
        if is_executing {
            if active_reqs.maybe_arced_requirements.is_none() {
                active_reqs.maybe_arced_requirements =
                    Some(Arc::new(active_reqs.requirements.clone()));
            }
            Self::Deferred(
                active_reqs
                    .maybe_arced_requirements
                    .as_ref()
                    .expect("Arced requirements must be set"),
            )
        } else {
            Self::Active(&active_reqs.requirements)
        }
    }
}

/// Exposes 4 main APIs:
/// (1) [ColdValidationRequirements::record_requirements] to record module
/// validation requirements, called from a sequential commit hook of a transaction.
/// (2) [ColdValidationRequirements::get_validation_requirement] if a requirement exists
/// and has high enough priority,
/// (3) [ColdValidationRequirements::validation_requirement_processed] to report the
/// results of performing validation task. It must immediately follow a get task call.
/// (4) [ColdValidationRequirements::is_commit_blocked] is used by the scheduler
/// to avoid committing a txn for which unsatisfied validationrequirements exist.

#[derive(Debug)]
pub(crate) struct ColdValidationRequirements<R: Clone + Ord> {
    num_txns: u32,
    /// Set to u32::MAX if no requirements are recorded, but the requirements may be
    /// pending and not yet processed (by the dedicated worker) to become active.
    /// Cache padded & optimized for reads (updated from txn commit, which is synchronized
    /// under the commit hooks lock, allowing relaxed memory ordering).
    /// dedicated_worker_id is accessed by different workers from scheduler's next_task,
    /// while min_idx_with_unscheduled_requirement is for determining commit eligibility.
    dedicated_worker_id: CachePadded<AtomicU32>,
    min_idx_with_unscheduled_requirement: CachePadded<AtomicU32>,
    /// The above minimum unscheduled index tracks requirements from being recorded to
    /// being returned from [ColdValidationRequirements::get_validation_requirement].
    /// However, this alone is not enough to assure that it is safe to commit a txn, as
    /// [ValidationRequirement::Deferred] when a txn is executing gets deferred to until
    /// the execution finishes. Below array tracks the status of deferred requirements.
    /// The bits except 2 least significant contain an affected incarnation, while the
    /// last two bits encode the following:
    /// 00: default: incarnation is not affected.
    /// 01: requirement is deferred until the txn finishes execution.
    /// 10: requirement is completed.
    /// 11: unreachable.
    deferred_requirements_status: Vec<CachePadded<AtomicU32>>,

    /// When a txn committed with published modules, they are stored here with from_idx =
    /// txn's index, and to_idx being the upper bound on which txns may need to be validated.
    /// If dedicated worker is not yet assigned, the caller takes on the responsibility.
    /// Pending requirements are processsed by the dedicated worker and transformed into
    /// active requirements (but this is done later and off the commit sequential path).
    pending_requirements: Mutex<Vec<PendingRequirement<R>>>,

    /// No cache padding since these are accessed less frequently and by the designated
    /// worker. Note: It is important to make sure there are no dangling references.
    active_requirements: ExplicitSyncWrapper<ActiveRequirements<R>>,
}

impl<R: Clone + Ord> ColdValidationRequirements<R> {
    pub(crate) fn new(num_txns: u32) -> Self {
        Self {
            num_txns,
            dedicated_worker_id: CachePadded::new(AtomicU32::new(u32::MAX)),
            min_idx_with_unscheduled_requirement: CachePadded::new(AtomicU32::new(u32::MAX)),
            deferred_requirements_status: (0..num_txns)
                .map(|_| CachePadded::new(AtomicU32::new(0)))
                .collect(),
            pending_requirements: Mutex::new(Vec::new()),
            active_requirements: ExplicitSyncWrapper::new(ActiveRequirements {
                requirements: BTreeSet::new(),
                versions: BTreeMap::new(),
                maybe_arced_requirements: None,
            }),
        }
    }

    /// Record is called during the sequential portion of txn commit (at calling_txn_idx),
    /// and schedules validation for specificed requirements starting at calling_txn_idx + 1
    /// until min_not_scheduled_idx, i.e. for all txns that might be affected. For instance,
    /// record is called after a txn publishes any modules during commit.
    ///
    /// Requirements should not be empty (o.w. there is no reason to record them).
    pub(crate) fn record_requirements(
        &self,
        worker_id: u32,
        calling_txn_idx: TxnIndex,
        min_not_scheduled_idx: TxnIndex,
        requirements: impl Iterator<Item = R>,
    ) -> Result<(), PanicError> {
        if min_not_scheduled_idx > self.num_txns || min_not_scheduled_idx <= calling_txn_idx {
            return Err(code_invariant_error(format!(
                "Invalid min_not_scheduled_idx = {} for calling_txn_idx = {} and num_txns = {}",
                min_not_scheduled_idx, calling_txn_idx, self.num_txns
            )));
        }

        if calling_txn_idx == self.num_txns - 1 || calling_txn_idx + 1 == min_not_scheduled_idx {
            // Requirements are void, since it applies to txns before min_not_scheduled_idx.
            return Ok(());
        }

        let requirements = requirements.collect::<BTreeSet<_>>();
        if requirements.is_empty() {
            return Err(code_invariant_error(format!(
                "Empty requirements to record for calling_txn_idx = {}",
                calling_txn_idx
            )));
        }

        let mut pending_reqs = self.pending_requirements.lock();
        pending_reqs.push(PendingRequirement {
            requirements,
            from_idx: calling_txn_idx + 1,
            to_idx: min_not_scheduled_idx,
        });

        // Updates to atomic variables while recording pending requirements occur under the
        // pending_requirements lock to ensure atomicity versus draining to activate.
        // However, for simplicity and simpler invariants, all updates (including in
        // finish_validation_requirement) are under the same lock.
        let _ = self.dedicated_worker_id.compare_exchange(
            u32::MAX,
            worker_id,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        let prev_min_idx = self
            .min_idx_with_unscheduled_requirement
            .swap(calling_txn_idx + 1, Ordering::Relaxed);
        if prev_min_idx <= calling_txn_idx {
            // Record may not be called with with a calling_txn_idx that is higher
            // or equal to the min_from_idx, as committing calling_txn_idx is impossible
            // before the pending requirements with lower min index have not been processed
            // and then any (lower or equal) required validations have not been performed.
            return Err(code_invariant_error(format!(
                "Recording validation requirements, min idx = {} <= calling_txn_idx = {}",
                prev_min_idx, calling_txn_idx
            )));
        }

        Ok(())
    }

    pub(crate) fn is_dedicated_worker(&self, worker_id: u32) -> bool {
        self.dedicated_worker_id.load(Ordering::Relaxed) == worker_id
    }

    /// If the caller is the dedicated worker, this method:
    /// (1) Clears the pending requirements: certain statuses (of executing txns) are marked
    /// for validation after finishing the execution, and the others that may be affected
    /// (ones that are executed) are transformed into active requirements.
    /// (2) Returns the highest priority (lowest idx) active requirement as long as it is
    /// below the idx_threshold. The reason requirements are drained one by one is (a) for
    /// simplicity, and (b) to allow unblocking txns for commit as soon as possible.
    /// Note: The caller may prefer to check is_dedicated_worker before calling this method
    /// to avoid computing the idx_threshold.
    pub(crate) fn get_validation_requirement<'a>(
        &self,
        worker_id: u32,
        idx_threshold: TxnIndex,
        statuses: &ExecutionStatuses,
    ) -> Result<Option<(TxnIndex, Incarnation, ValidationRequirement<'a, R>)>, PanicError> {
        if !self.is_dedicated_worker(worker_id) {
            return Ok(None);
        }

        self.activate_pending_requirements(statuses)?;
        // Double check as if pending requirements were empty and no new requirements were
        // activated, the dedicated worker id would be reset.
        if !self.is_dedicated_worker(worker_id) {
            return Ok(None);
        }

        // After the drain, another worker may have concurrently added pending requirements,
        // reducing the min_idx_with_unscheduled_requirement (to make sure it's blocked from
        // getting committed). Hence, when obtaining an active validation requirement, the
        // index should be based on the versions map in active_requirements.
        let active_reqs = self.active_requirements.dereference();
        let (min_active_requirement_idx, (incarnation, is_executing)) =
            active_reqs.versions.first_key_value().ok_or_else(|| {
                // Should not be empty as dedicated worker was set in the beginning of the method
                // and can only be reset by the worker itself.
                code_invariant_error("Empty active requirements in get_validation_requirement")
            })?;

        if *min_active_requirement_idx <= idx_threshold {
            return Ok(Some((
                *min_active_requirement_idx,
                *incarnation,
                ValidationRequirement::new(
                    self.active_requirements.dereference_mut(),
                    *is_executing,
                ),
            )));
        }

        Ok(None)
    }

    /// Caller must be the dedicated worker, calling after processing a requirement that
    /// it just obtained from get_validation_requirement (the calls must be alternating).
    ///
    /// Note that processing validation requirement may mean (a) completing the actual
    /// required validation (always it requirement was [ValidationRequirement::Active]),
    /// or (b) scheduling it in [ValidationRequirement::Deferred] case to be performed
    /// if the txn was observed to still be executing. validation_completed parameter
    /// is true in case (a) and false in case (b).
    ///
    /// The return value indicates if this was the last requirement (i.e. there are no more
    /// cold validation requirements and the worker is no longer assigned to process them).
    pub(crate) fn validation_requirement_processed(
        &self,
        worker_id: u32,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        validation_completed: bool,
    ) -> Result<bool, PanicError> {
        if !self.is_dedicated_worker(worker_id) {
            return Err(code_invariant_error(format!(
                "Worker {} is not the dedicated worker in finish_validation_requirement",
                worker_id
            )));
        }

        let active_reqs = self.active_requirements.dereference_mut();
        let min_idx = active_reqs.versions.keys().min().ok_or_else(|| {
            code_invariant_error(format!(
                "Active requirements are empty in finish_validation_requirement for idx = {}",
                txn_idx
            ))
        })?;
        if *min_idx != txn_idx {
            return Err(code_invariant_error(format!(
                "min idx in recorded versions = {} != validated idx = {}",
                *min_idx, txn_idx
            )));
        }
        let required_incarnation = active_reqs.versions.remove(&txn_idx);
        if !required_incarnation.is_some_and(|(req_incarnation, _)| req_incarnation == incarnation)
        {
            return Err(code_invariant_error(format!(
                "Required incarnation {:?} != validated incarnation {} in finish_validation_requirement",
                required_incarnation, incarnation
            )));
        }
        if !validation_completed {
            // min_idx_with_unscheduled_requirements may be increased below, after deferred
            // status is already updated. When checking if txn can be committed, the access
            // order is opposite, ensuring that if minimum index is higher, we will also
            // observe the incremented count below (even w. Relaxed ordering).
            //
            // The reason for using fetch_max is because the deferred requirement can be
            // fulfilled by a different worker (the one executing the txn), which may report
            // the requirement as completed before the current worker sets the status here.
            self.deferred_requirements_status[txn_idx as usize]
                .fetch_max(blocked_incarnation_status(incarnation), Ordering::Relaxed);
        }

        let active_reqs_is_empty = active_reqs.versions.is_empty();
        let pending_reqs = self.pending_requirements.lock();
        if pending_reqs.is_empty() {
            // Expected to be empty most of the time as publishes are rare and the requirements
            // are drained by the caller when getting the requirement. The check ensures that
            // the min_idx_with_unscheduled_requirement is not incorrectly increased if pending
            // requirements exist for validated_idx. It also allows us to hold the lock while
            // updating the atomic variables.
            if active_reqs_is_empty {
                active_reqs.requirements.clear();
                active_reqs.maybe_arced_requirements = None;
                self.min_idx_with_unscheduled_requirement
                    .store(u32::MAX, Ordering::Relaxed);
                // Since we are holding the lock and pending requirements is empty, it
                // is safe to reset the dedicated worker id.
                self.dedicated_worker_id.store(u32::MAX, Ordering::Relaxed);
            } else {
                self.min_idx_with_unscheduled_requirement
                    .store(txn_idx + 1, Ordering::Relaxed);
            }
        }

        Ok(active_reqs_is_empty)
    }

    pub(crate) fn deferred_requirements_completed(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<(), PanicError> {
        let new_status = unblocked_incarnation_status(incarnation);
        self.deferred_requirements_status[txn_idx as usize]
            .fetch_max(new_status, Ordering::Relaxed);
        Ok(())
    }

    /// Correctness of this method relies on the assumption that calls are for monotonically
    /// increasing txn_idx, which holds for BlockSTMv2 as the method is used to check if the
    /// next idx can be committed.
    pub(crate) fn is_commit_blocked(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        // The order of checks is important to avoid a concurrency bugs (since recording
        // happens in the opposite order). We first check that there are no unscheduled
        // requirements below (incl.) the given index, and then that there are no scheduled
        // but yet unfulfilled (validated) requirements for the index.
        self.min_idx_with_unscheduled_requirement
            .load(Ordering::Relaxed)
            <= txn_idx
            || self.deferred_requirements_status[txn_idx as usize].load(Ordering::Relaxed)
                == blocked_incarnation_status(incarnation)
    }
}

fn blocked_incarnation_status(incarnation: Incarnation) -> u32 {
    (incarnation << 2) | 1
}

fn unblocked_incarnation_status(incarnation: Incarnation) -> u32 {
    (incarnation << 2) | 2
}

// Private utilities / interfaces.
impl<R: Clone + Ord> ColdValidationRequirements<R> {
    // Drain and activate any pending requirements.
    fn activate_pending_requirements(
        &self,
        statuses: &ExecutionStatuses,
    ) -> Result<(), PanicError> {
        let pending_reqs = {
            let mut guard = self.pending_requirements.lock();
            if guard.is_empty() {
                // No requirements to drain.
                return Ok(());
            }
            std::mem::take(&mut *guard)
        };

        let starting_idx = pending_reqs
            .iter()
            .map(|req| req.from_idx)
            .min()
            .expect("Expected at least one requirement");
        let ending_idx = pending_reqs
            .iter()
            .map(|req| req.to_idx)
            .max()
            .expect("Expected at least one requirement");
        if starting_idx >= ending_idx || ending_idx > self.num_txns {
            return Err(code_invariant_error(format!(
                "Invariant broken, starting idx {} >= ending idx {} or ending idx > num_txns {}",
                starting_idx, ending_idx, self.num_txns
            )));
        }

        let new_versions: BTreeMap<TxnIndex, (Incarnation, bool)> = (starting_idx..ending_idx)
            .filter_map(|txn_idx| {
                statuses
                    .requires_module_validation(txn_idx)
                    .map(|(incarnation, is_executing)| (txn_idx, (incarnation, is_executing)))
            })
            .collect();
        let new_requirements = pending_reqs
            .into_iter()
            .fold(BTreeSet::new(), |mut acc, req| {
                acc.extend(req.requirements.into_iter());
                acc
            });

        let active_reqs = self.active_requirements.dereference_mut();
        active_reqs
            .requirements
            .extend(new_requirements.into_iter());
        active_reqs.versions.extend(new_versions.into_iter());
        // Clear the cached requirements as they may have changed.
        active_reqs.maybe_arced_requirements = None;

        if active_reqs.versions.is_empty() {
            // It is possible that the active versions map was empty, and no pending
            // requirements needed to be activated (i.e. not executing or executed).
            // In this case, we may need to update min_idx_with_unscheduled_requirement
            // as validation_requirement_processed does so only when the pending
            // requirements are empty.
            let pending_reqs_guard = self.pending_requirements.lock();
            if pending_reqs_guard.is_empty() {
                self.min_idx_with_unscheduled_requirement
                    .store(u32::MAX, Ordering::Relaxed);
                self.dedicated_worker_id.store(u32::MAX, Ordering::Relaxed);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler_status::{ExecutionStatus, SchedulingStatus, StatusWithIncarnation};
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some, assert_some_eq};

    // Test requirements type for easier testing
    type TestRequirement = u32;

    // Helper function to create mock ExecutionStatuses
    fn create_mock_execution_statuses(num_txns: u32) -> ExecutionStatuses {
        let mut statuses = Vec::new();
        for _ in 0..num_txns {
            statuses.push(ExecutionStatus::new());
        }
        ExecutionStatuses::new_for_test(
            crate::scheduler_v2::ExecutionQueueManager::new_for_test(num_txns),
            statuses,
        )
    }

    // Helper function to create ExecutionStatuses with specific transaction statuses
    fn create_execution_statuses_with_txns(
        num_txns: u32,
        mut txn_configs: BTreeMap<TxnIndex, (SchedulingStatus, Incarnation)>,
    ) -> ExecutionStatuses {
        let mut statuses = Vec::new();
        for i in 0..num_txns {
            // Check if this transaction has a specific configuration
            if let Some((status, incarnation)) = txn_configs.remove(&i) {
                statuses.push(ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(status, incarnation),
                    0, // num_stalls
                ));
            } else {
                statuses.push(ExecutionStatus::new());
            }
        }
        ExecutionStatuses::new_for_test(
            crate::scheduler_v2::ExecutionQueueManager::new_for_test(num_txns),
            statuses,
        )
    }

    fn test_active_requirements_empty(requirements: &ColdValidationRequirements<TestRequirement>) {
        assert_eq!(
            requirements
                .active_requirements
                .dereference()
                .requirements
                .len(),
            0
        );
        assert_eq!(
            requirements
                .active_requirements
                .dereference()
                .versions
                .len(),
            0
        );
        assert_none!(
            &requirements
                .active_requirements
                .dereference()
                .maybe_arced_requirements
        );
    }

    #[test]
    fn test_new_cold_validation_requirements() {
        let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
        let statuses = create_mock_execution_statuses(10);

        // Initial state should have no dedicated worker
        for id in 0..10 {
            assert!(!requirements.is_dedicated_worker(id));
            assert_none!(requirements
                .get_validation_requirement(id, 10, &statuses)
                .unwrap());
        }

        // No transactions should be blocked initially
        for i in 0..10 {
            assert!(!requirements.is_commit_blocked(i, 0));
            assert!(!requirements.is_commit_blocked(i, 1));
        }
    }

    #[test]
    fn test_incarnation_status_encoding() {
        for incarnation in 0..100 {
            assert_eq!(blocked_incarnation_status(incarnation), 4 * incarnation + 1);
            assert_eq!(
                unblocked_incarnation_status(incarnation),
                4 * incarnation + 2
            );
        }
    }

    #[test]
    fn test_no_qualifying_transactions() {
        let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
        let statuses = create_execution_statuses_with_txns(
            10,
            [
                (4, (SchedulingStatus::PendingScheduling, 1)),
                (5, (SchedulingStatus::Aborted, 1)),
                (6, (SchedulingStatus::Aborted, 1)),
                (7, (SchedulingStatus::PendingScheduling, 1)),
            ]
            .into_iter()
            .collect(),
        );

        // Record requirements
        requirements
            .record_requirements(1, 3, 9, vec![100].into_iter())
            .unwrap();
        assert!(requirements.is_dedicated_worker(1));

        // Should not get any validation requirements
        assert_none!(requirements
            .get_validation_requirement(1, 20, &statuses)
            .unwrap());

        // Worker should be reset
        assert!(!requirements.is_dedicated_worker(1));
    }

    mod record_requirements_tests {
        use super::*;

        #[test]
        fn test_record_requirements_basic() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
            let worker_id = 1;
            let calling_txn_idx = 3;
            let min_not_scheduled_idx = 7;
            let test_requirements = vec![100, 200, 300];

            let result = requirements.record_requirements(
                worker_id,
                calling_txn_idx,
                min_not_scheduled_idx,
                test_requirements.into_iter(),
            );

            assert!(result.is_ok());
            assert!(requirements.is_dedicated_worker(worker_id));
            // Must be recorded as pending.
            assert_eq!(requirements.pending_requirements.lock().len(), 1);
            test_active_requirements_empty(&requirements);

            // Must not be dedicated.
            assert!(!requirements.is_dedicated_worker(0));
            assert!(!requirements.is_dedicated_worker(2));

            // Transactions above calling_txn_idx+1 should be blocked
            for i in calling_txn_idx + 1..10 {
                assert!(requirements.is_commit_blocked(i, 0));
                // This kind of block applies to all incarnations.
                assert!(requirements.is_commit_blocked(i, 1));
            }

            // Transactions outside range should not be blocked
            assert!(!requirements.is_commit_blocked(0, 0));
            assert!(!requirements.is_commit_blocked(calling_txn_idx, 0));
        }

        #[test]
        fn test_record_requirements_edge_cases() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);

            // Test void requirements (adjacent indices)
            let result = requirements.record_requirements(5, 5, 6, vec![100].into_iter());
            assert!(result.is_ok());

            // Test last transaction
            let result = requirements.record_requirements(0, 9, 10, vec![100].into_iter());
            assert!(result.is_ok());

            assert!(requirements.pending_requirements.lock().is_empty());
            test_active_requirements_empty(&requirements);

            // Dedicated worker should not be assigned.
            assert!(!requirements.is_dedicated_worker(0));
            assert!(!requirements.is_dedicated_worker(5));
        }

        #[test]
        fn test_record_requirements_error_conditions() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);

            // Test invalid min_not_scheduled_idx > num_txns
            assert_err!(requirements.record_requirements(0, 5, 15, vec![100].into_iter()));

            // Test min_not_scheduled_idx <= calling_txn_idx
            assert_err!(requirements.record_requirements(0, 5, 5, vec![100].into_iter()));
            assert_err!(requirements.record_requirements(0, 5, 4, vec![100].into_iter()));

            assert_ok!(requirements.record_requirements(0, 1, 5, vec![100].into_iter()));
            assert_ok!(requirements.record_requirements(0, 1, 5, vec![100].into_iter()));
            // test that calling_txn_idx > min_not_scheduled_idx is checked.
            assert_err!(requirements.record_requirements(0, 2, 5, vec![100].into_iter()));

            // Empty requirements should be rejected.
            assert_err!(requirements.record_requirements(0, 1, 5, vec![].into_iter()));
        }
    }

    mod dedicated_worker_tests {
        use super::*;

        #[test]
        fn test_dedicated_worker_assignment() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);

            // Initially no dedicated worker
            assert!(!requirements.is_dedicated_worker(0));
            assert!(!requirements.is_dedicated_worker(1));

            // First worker to record requirements becomes dedicated
            assert_ok!(requirements.record_requirements(5, 2, 8, vec![100].into_iter()));
            assert!(requirements.is_dedicated_worker(5));
            assert!(!requirements.is_dedicated_worker(3));

            // Second worker cannot become dedicated
            assert_ok!(requirements.record_requirements(3, 1, 9, vec![200].into_iter()));
            assert!(requirements.is_dedicated_worker(5)); // Still worker 5
            assert!(!requirements.is_dedicated_worker(3));
        }

        #[test]
        fn test_dedicated_worker_reset() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
            let statuses = create_execution_statuses_with_txns(
                10,
                [
                    (4, (SchedulingStatus::Executed, 1)),
                    (5, (SchedulingStatus::Executed, 2)),
                ]
                .into_iter()
                .collect(),
            );

            // Record requirements
            requirements
                .record_requirements(1, 3, 7, vec![100].into_iter())
                .unwrap();
            assert!(requirements.is_dedicated_worker(1));
            test_active_requirements_empty(&requirements);

            // Get and process requirements.
            assert_some_eq!(
                requirements
                    .get_validation_requirement(1, 4, &statuses)
                    .unwrap(),
                (4, 1, ValidationRequirement::Active(&BTreeSet::from([100])))
            );
            assert!(!requirements
                .validation_requirement_processed(1, 4, 1, true)
                .unwrap());

            assert!(requirements.is_dedicated_worker(1));
            assert_some_eq!(
                requirements
                    .get_validation_requirement(1, 10, &statuses)
                    .unwrap(),
                (5, 2, ValidationRequirement::Active(&BTreeSet::from([100])))
            );
            assert!(requirements
                .validation_requirement_processed(1, 5, 2, true)
                .unwrap());

            // Worker should be reset when no more requirements.
            assert!(!requirements.is_dedicated_worker(1));
        }
    }

    mod validation_requirement_processing_tests {
        use super::*;

        #[test]
        fn test_get_validation_requirement() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);

            let txn_configs: BTreeMap<TxnIndex, (SchedulingStatus, Incarnation)> = [
                (2, (SchedulingStatus::Executing, 3)),
                (3, (SchedulingStatus::Executed, 1)),
                (5, (SchedulingStatus::Executed, 2)),
                (6, (SchedulingStatus::Executing, 1)),
                (7, (SchedulingStatus::Executed, 2)),
            ]
            .into_iter()
            .collect();
            let statuses = create_execution_statuses_with_txns(10, txn_configs.clone());

            // Record requirements
            assert_ok!(requirements.record_requirements(1, 2, 7, vec![100, 200].into_iter()));

            let btree_reqs = BTreeSet::from([100, 200]);
            let arc_reqs = Arc::new(btree_reqs.clone());

            // Since calling_txn_idx is 2 and min_not_scheduled_idx is 7, only
            // txns 3, 5, and 6 are going to have requirements.
            for (txn_idx, (status, incarnation)) in txn_configs.into_iter() {
                if txn_idx == 2 || txn_idx == 7 {
                    continue;
                }

                // Repeated calls should return the same requirement.
                for _ in 0..2 {
                    // Get validation requirement
                    assert_some_eq!(
                        requirements
                            .get_validation_requirement(1, 10, &statuses)
                            .unwrap(),
                        (
                            txn_idx,
                            incarnation,
                            if status == SchedulingStatus::Executed {
                                ValidationRequirement::Active(&btree_reqs)
                            } else {
                                ValidationRequirement::Deferred(&arc_reqs)
                            }
                        )
                    );
                }

                assert!(requirements.is_commit_blocked(txn_idx, incarnation));

                assert_ok_eq!(
                    requirements.validation_requirement_processed(1, txn_idx, incarnation, true),
                    txn_idx == 6
                );

                assert!(!requirements.is_commit_blocked(txn_idx, incarnation));
            }

            // No more requirements.
            assert!(!requirements.is_dedicated_worker(1));
            test_active_requirements_empty(&requirements);
        }

        #[test]
        fn test_get_validation_requirement_threshold() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
            let statuses = create_execution_statuses_with_txns(
                10,
                [(7, (SchedulingStatus::Executed, 1))].into_iter().collect(),
            );

            // Record requirements
            assert_ok!(requirements.record_requirements(1, 3, 9, vec![100].into_iter()));

            // Get validation requirement with low threshold
            assert_none!(requirements
                .get_validation_requirement(1, 6, &statuses)
                .unwrap()); // Should be None due to threshold

            // Get validation requirement with high threshold
            assert_some!(requirements
                .get_validation_requirement(1, 7, &statuses)
                .unwrap());
        }

        #[test]
        fn test_validation_requirement_processed_deferred() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);
            let statuses = create_execution_statuses_with_txns(
                10,
                [(4, (SchedulingStatus::Executing, 1))]
                    .into_iter()
                    .collect(),
            );

            // Record and activate requirements.
            assert_ok!(requirements.record_requirements(1, 3, 7, vec![100].into_iter()));
            assert_ok!(requirements.activate_pending_requirements(&statuses));

            // Process as deferred (not completed) w.o. calling get (not needed for test).
            assert_ok!(requirements.validation_requirement_processed(1, 4, 1, false));

            // Should still be blocked for commit
            assert!(requirements.is_commit_blocked(4, 1));
            // Higher incarnation should not be blocked (incarnation 1 might be aborted).
            assert!(!requirements.is_commit_blocked(4, 2));

            // Complete the deferred requirement and check unblocked.
            assert_ok!(requirements.deferred_requirements_completed(4, 1));

            assert!(!requirements.is_commit_blocked(4, 1));
        }

        #[test]
        fn test_validation_requirement_processed_error_conditions() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(10);

            assert_err!(requirements.validation_requirement_processed(2, 4, 1, true));
            assert_err!(requirements.validation_requirement_processed(1, 6, 2, false));
            assert_err!(requirements.validation_requirement_processed(1, 5, 1, true));

            let statuses = create_execution_statuses_with_txns(
                10,
                [(7, (SchedulingStatus::Executed, 1))].into_iter().collect(),
            );
            requirements
                .record_requirements(1, 3, 8, vec![100].into_iter())
                .unwrap();
            assert_some_eq!(
                requirements
                    .get_validation_requirement(1, 10, &statuses)
                    .unwrap(),
                (7, 1, ValidationRequirement::Active(&BTreeSet::from([100])))
            );
            // Wrong worker ID, wrong txn indices, and wrong incarnations should fail.
            assert_err!(requirements.validation_requirement_processed(2, 7, 1, true));
            assert_err!(requirements.validation_requirement_processed(1, 6, 1, true));
            assert_err!(requirements.validation_requirement_processed(1, 8, 1, true));
            assert_err!(requirements.validation_requirement_processed(1, 7, 2, false));
        }
    }

    mod multiple_requirements_tests {
        use super::*;

        #[test]
        fn test_multiple_pending_requirements() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(20);

            // Record multiple requirements from different transactions
            assert_eq!(
                requirements
                    .min_idx_with_unscheduled_requirement
                    .load(Ordering::Relaxed),
                u32::MAX
            );
            assert_eq!(requirements.pending_requirements.lock().len(), 0);

            assert_ok!(requirements.record_requirements(1, 10, 15, vec![500].into_iter()));
            assert_eq!(
                requirements
                    .min_idx_with_unscheduled_requirement
                    .load(Ordering::Relaxed),
                11
            );
            assert_eq!(requirements.pending_requirements.lock().len(), 1);

            assert_ok!(requirements.record_requirements(2, 5, 12, vec![300, 400].into_iter()));
            assert_eq!(
                requirements
                    .min_idx_with_unscheduled_requirement
                    .load(Ordering::Relaxed),
                6
            );
            assert_eq!(requirements.pending_requirements.lock().len(), 2);

            assert_ok!(requirements.record_requirements(3, 2, 8, vec![100, 200].into_iter()));
            assert_eq!(
                requirements
                    .min_idx_with_unscheduled_requirement
                    .load(Ordering::Relaxed),
                3
            );
            assert_eq!(requirements.pending_requirements.lock().len(), 3);
            test_active_requirements_empty(&requirements);

            // First worker should remain dedicated
            assert!(requirements.is_dedicated_worker(1));
            assert!(!requirements.is_dedicated_worker(2));
            assert!(!requirements.is_dedicated_worker(3));

            // All affected ranges should be blocked
            assert!(!requirements.is_commit_blocked(2, 0));
            for i in 3..20 {
                assert!(requirements.is_commit_blocked(i, 0));
            }
        }

        #[test]
        fn test_merged_requirements() {
            let requirements = ColdValidationRequirements::<TestRequirement>::new(15);
            let statuses = create_execution_statuses_with_txns(
                15,
                [
                    (6, (SchedulingStatus::Executed, 1)),
                    (9, (SchedulingStatus::Executing, 2)),
                ]
                .into_iter()
                .collect(),
            );

            // Record overlapping requirements
            assert_ok!(requirements.record_requirements(1, 6, 10, vec![100, 200].into_iter()));
            assert_ok!(requirements.record_requirements(2, 5, 8, vec![300, 400].into_iter()));

            let btree_reqs = BTreeSet::from([100, 200, 300, 400]);
            let arc_reqs = Arc::new(btree_reqs.clone());

            // Get validation requirement - should contain merged requirements
            assert_some_eq!(
                requirements
                    .get_validation_requirement(1, 20, &statuses)
                    .unwrap(),
                (6, 1, ValidationRequirement::Active(&btree_reqs))
            );
            assert_ok!(requirements.validation_requirement_processed(1, 6, 1, true));

            assert_some_eq!(
                requirements
                    .get_validation_requirement(1, 20, &statuses)
                    .unwrap(),
                (9, 2, ValidationRequirement::Deferred(&arc_reqs))
            );
            assert_ok!(requirements.validation_requirement_processed(1, 9, 2, false));
            test_active_requirements_empty(&requirements);
        }
    }
}
