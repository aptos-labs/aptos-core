# Post-Mortem: INC-17993 Testnet Down Due to Deadlock

**Incident Date:** May 2026  
**Severity:** SEV-1  
**Status:** Resolved  
**Author:** Aptos Incident Response Team  

## Executive Summary

Testnet experienced a complete outage due to a deadlock in the Block-STM parallel execution engine. The deadlock occurred when rayon work-stealing interacted with BlockSTM worker threads, causing transactions to stall indefinitely. The issue also existed on mainnet and required an emergency hotfix rollout.

## Timeline (All times UTC)

| Time | Event |
|------|-------|
| May 1, 2026 | Deadlock first identified in testnet validators |
| May 1, 18:15 | PR #19615 opened - Initial fix for `with_native_rayon` deadlock |
| May 1, 20:22 | PR #19615 merged to `aptos-release-v1.44` |
| May 1, 23:37 | PR #19619 opened - Structural fix using `std::thread::scope` |
| May 1, 23:42 | PR #19619 merged to `aptos-release-v1.44` |
| May 4, 22:07 | PR #19639 opened - Forward-port to main |
| May 5, 00:26 | PR #19643 opened - Complete BlockSTM thread pool replacement |
| May 5, 23:49 | PR #19643 merged to main |
| May 6, 2026 | v1.44.6 rollout to mainnet scheduled |

## Root Cause Analysis

### Technical Root Cause

The deadlock originated from an unsafe interaction between rayon's work-stealing scheduler and Block-STM's parallel transaction execution model.

**Failing Sequence:**

1. A BlockSTM worker starts executing transaction N, then enters a nested `par_iter()` (rayon parallel section) and waits for its rayon subtasks to finish.

2. While waiting, rayon lets that worker steal work from the same pool. It can steal an inner rayon task spawned by another BlockSTM worker.

3. The stolen task can hit a BlockSTM read dependency and park on the v1 dependency `Condvar`, waiting for transaction N to finish.

4. Transaction N cannot finish because its original worker is now parked inside the stolen task, but the BlockSTM scheduler still marks transaction N as `Executing`.

**Result:** The usual BlockSTM liveness argument no longer applies: the lowest blocked transaction is waiting on a rayon worker that has stopped running it, so execution stalls forever.

### Contributing Factors

- Block-STM worker loops ran under `rayon::ThreadPool::scope`, making them eligible for rayon's work-stealing
- Native functions using rayon internally (e.g., `ark_ff::batch_inversion` inside `algebra::hash_to_structure::hash_to_internal`) were not all wrapped by `with_native_rayon`
- The `with_native_rayon` wrapper used `ThreadPool::install`, which is cooperative - while blocked on the native-pool latch, rayon keeps the caller busy by stealing other jobs
- Writer-preferring per-txn `RwLock`s in the scheduler created lock ordering issues when combined with work-stealing

## Impact

| Metric | Value |
|--------|-------|
| Networks Affected | Testnet (confirmed), Mainnet (vulnerable) |
| Duration | Multiple hours |
| Transactions Affected | All transactions during outage |
| User Impact | Complete testnet unavailability |
| PFN Impact | Decibel PFNs on testnet also stuck |

## Resolution

### Immediate Mitigation (PR #19615)

- Replaced `pool.install(op)` with `pool.spawn(op) + rx.recv()` in `with_native_rayon`
- A channel `recv` is a real OS park, so the caller leaves rayon's steal set entirely while the native work runs
- Added regression test reproducing the deadlock scenario

### Structural Fix (PRs #19619, #19639)

- Switched Block-STM (v1 and v2) worker spawning from `rayon::ThreadPool::scope` to `std::thread::scope`
- Worker threads are no longer part of rayon's registry
- Prevents rayon work-stealing from pulling sibling `worker_loop` jobs onto threads

### Long-term Fix (PR #19643)

- Replaced rayon BlockSTM pool entirely with plain `std::thread` pool
- BlockSTM workers run on threads not registered with rayon
- Nested `par_iter()` work runs on the rayon global pool instead of making BlockSTM workers steal across nested parallel scopes
- Removed `execute_block_on_thread_pool` API
- Added `par_exec_pool_size` metric to track spawned worker threads

## Detection & Monitoring Gaps

- **Issue:** Decibel PFNs on testnet stuck without generating alerts
- **Action Required:** Review alerting for PFN health checks

## Action Items

| Priority | Action | Owner | Status |
|----------|--------|-------|--------|
| P0 | Rollout v1.44.6 to mainnet | @gregnazario | In Progress |
| P1 | Backport fix to v1.45 branch | @wqfish | Complete |
| P1 | Merge structural fix to main | @zekun000 | Complete |
| P2 | Add deadlock detection tests | TBD | Open |
| P2 | Review PFN alerting configuration | @sherryxiao | Open |
| P2 | Document BlockSTM threading model | TBD | Open |

## Lessons Learned

### What Went Well

- Root cause was identified quickly by the team
- Multiple fix approaches were developed and validated
- Clear communication through incident channel
- Regression tests added to prevent recurrence

### What Could Be Improved

- Need better monitoring/alerting for deadlock conditions
- PFN health monitoring gaps identified
- Rayon integration complexity was underestimated
- Need clearer documentation of BlockSTM threading assumptions

### Technical Insights

1. **Rayon work-stealing is unsafe when workers hold state:** Any code that runs on rayon workers should not hold locks or state that other rayon tasks might need.

2. **`ThreadPool::install` is cooperative, not blocking:** Using `install` doesn't prevent work-stealing - the caller remains available to steal work while waiting.

3. **Mixed threading models require careful analysis:** When combining custom thread pools with rayon, the interaction of work-stealing with lock ordering must be analyzed.

## Related Documents

- [Fix PR #19615](https://github.com/aptos-labs/aptos-core/pull/19615) - Initial `with_native_rayon` fix
- [Fix PR #19619](https://github.com/aptos-labs/aptos-core/pull/19619) - `std::thread::scope` fix for v1.44
- [Fix PR #19639](https://github.com/aptos-labs/aptos-core/pull/19639) - Forward-port to main
- [Fix PR #19643](https://github.com/aptos-labs/aptos-core/pull/19643) - Complete thread pool replacement
- [Incident Notes](https://docs.google.com/document/d/1jKwZxCiVkO3tuq7ccMwbE7OC8OJYjr4fpTiJfCeRtv8/edit?tab=t.ffgnn6bw6yvm)
- Slack Channel: #inc-17993-testnet-is-down-due-to-deadlock

## Appendix: Technical Details

### Deadlock Stack Trace Pattern

The deadlock was characterized by stack traces rooted at:
```
Scheduler::never_executed -> RawRwLock::lock_shared
```
under `rayon_core::WorkerThread::wait_until_cold`

### Code Locations Changed

- `aptos-move/block-executor/src/executor.rs` - Worker thread spawning
- `aptos-move/aptos-native-interface/src/lib.rs` - `with_native_rayon` implementation
- `aptos-move/framework/natives/src/cryptography/algebra/arithmetics/scalar_mul.rs` - MSM natives
- `aptos-move/framework/natives/src/cryptography/algebra/pairing.rs` - Pairing natives

### Verification

The fix was verified by:
1. Regression tests that reproduce the deadlock scenario
2. Cargo test suite passing (252 tests in aptos-block-executor)
3. Forge E2E performance tests
4. Replay-verify smoke tests
