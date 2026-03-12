# Incident Report — INC-14309

**Title:** Decibel Mainnet Loadtest Performance Problems
**Severity:** SEV2
**Status:** Investigating
**Date Opened:** 2026-02-03
**Slack Channel:** [#inc-14309-decibel-mainnet-loadtest-performance-problems](https://aptos-org.slack.com/archives/C0ACWG0BGRX)

---

## Summary

Mainnet load testing for Decibel launch revealed severe performance degradation compared to Forge benchmarks. Initial tests achieved only **~300 TPS with 2.4s P90 latency**, far below Forge's **1,000 TPS with 900ms latency**. Multiple contributing factors were identified including cache flushing due to speculative forks, consensus thread oversubscription, randomness verification CPU overhead, storage latency differences, and hardware inconsistencies across validators. After implementing hotfixes and governance proposals, throughput improved to **500–700 TPS** with latency around **1,000ms**.

---

## Impact

- **Throughput limitation:** Mainnet initially capped at ~300 TPS, improved to 500–700 TPS after fixes
- **Latency degradation:** P90 latency started at 2.4s, improved to ~1.3s at 500 TPS
- **Business impact:** Decibel mainnet launch timeline at risk; pre-deposit decision required by Thursday night with partners already preparing funds
- **Node availability:** Several validator nodes remained on outdated releases (1.38/1.39), contributing to network-wide performance issues

---

## Timeline

| Date/Time | Event |
|-----------|-------|
| **Feb 3, 12:06 PM** | Incident declared — mainnet loadtest achieved only ~300 TPS with 2.4s P90 latency |
| **Feb 3, 12:18 PM** | Joshua Lind identifies execution bottleneck on validators |
| **Feb 3, 1:05 PM** | zekun runs replay-benchmark comparing mainnet vs testnet blocks — similar execution performance |
| **Feb 3, 1:13 PM** | Storage latency identified: ~100ms on mainnet vs ~40ms on testnet |
| **Feb 3, 2:00 PM** | zekun discovers speculative forks causing cache resets on mainnet (0 on testnet) |
| **Feb 3, 3:00 PM CT** | First war room meeting |
| **Feb 3, 3:45 PM** | Guoteng notes AMD CPUs much faster than Intel; Forge runs on AMD |
| **Feb 3, 10:47 PM** | Analysis reveals validators with low concurrency (8–16 threads) execute 2.7x faster than high concurrency (38–48 threads) |
| **Feb 4, 11:09 AM** | Igor requests Move perf features be enabled on mainnet via governance proposals |
| **Feb 4, 1:48 PM** | zekun identifies layout cache flushing as potential cause — cache size fluctuating during loadtest |
| **Feb 4, 3:42 PM** | Concurrency benchmarks completed: optimal throughput at level 8–16 |
| **Feb 4, 3:57 PM** | Action items assigned: concurrency/blocktime changes, perf build, async paranoid mode |
| **Feb 4, 7:47 PM** | Hot-fix release v1.40.4 created with config changes |
| **Feb 4, 8:20 PM** | Governance proposals 175/176/177 submitted for Move perf improvements |
| **Feb 4, 10:25 PM** | Release v1.40.4 deployed to Aptos nodes |
| **Feb 5, 2:08 AM** | Async paranoid type checks enabled on testnet nodes |
| **Feb 5, 2:59 PM** | PR for CPU pinning for execution threads submitted |
| **Feb 5, 4:31 PM** | CONCURRENT_FUNGIBLE_BALANCE found to be off on mainnet |
| **Feb 6, 10:07 AM** | Hot-fix rolled out to all node groups |
| **Feb 6, 11:00 AM** | Friday check-in meeting |
| **Feb 6, 12:48 PM** | CPU profile reveals most consensus CPU spent on **randomness verify share** |
| **Feb 6, 1:18 PM** | Trusted code not enabled in mainnet DB benchmark — explains 30% difference |
| **Feb 6, 3:53 PM** | 500 TPS test: latency 1441ms (p50: 1300ms, p90: 1900ms) |
| **Feb 6, 4:13 PM** | zekun confirms disabling randomness verification brings validator execution time on par with fullnodes |
| **Feb 6, 5:19 PM** | PRs submitted: optimistic randomness verification (#18646), disable randomness fast path (#18644, #18645) |
| **Feb 6** | Governance proposals 175/176/177 executed |
| **Feb 9, 12:59 PM** | 500 TPS test improved: latency 1076ms (p50: 900ms, p90: 1300ms) |
| **Feb 9, 2:38 PM** | Remaining items documented; optimistic randomness verification to be landed |
| **Feb 13** | Randomness fast path disabled on mainnet via governance |
| **Ongoing** | Async paranoid mode pending OtterSec audit completion |

---

## Root Cause Analysis

### Primary Contributing Factors

#### 1. Speculative Fork Cache Resets
Mainnet experiences frequent speculative forks that trigger cache resets, while testnet shows zero such resets. This causes the layout cache to flush repeatedly during load tests.
- **Evidence:** [Grafana metrics](https://grafana.aptoslabs.com/explore?schemaVersion=1&panes=%7B%22yeo%22...) showing cache resets on mainnet vs 0 on testnet
- **Fix:** PR #18592 deployed to euwe2-2 for evaluation

#### 2. Randomness Verification CPU Overhead
CPU profiling revealed that **most consensus CPU time is spent on randomness share verification**. This creates significant overhead on validators compared to fullnodes.
- **Evidence:** [CPU profile (profilez.svg)](https://aptos-org.slack.com/files/...) showing randomness verify share dominance
- **Fixes:**
  - **Optimistic randomness shares verification** — reduces CPU cost by ~3x ([PR #18646](https://github.com/aptos-labs/aptos-core/pull/18646))
  - **Disable randomness fast path** — reduces cost by ~2x, no longer useful after stake quorum moved to Europe ([PR #18644](https://github.com/aptos-labs/aptos-core/pull/18644), [governance PR #18645](https://github.com/aptos-labs/aptos-core/pull/18645), [mainnet proposal](https://github.com/aptos-foundation/mainnet-proposals/pull/203))

#### 3. Consensus Thread Oversubscription
144 consensus threads running on 48-CPU validator machines (3x oversubscription). 41 threads named `consensus-37` — suspicious duplication.
- **Evidence:** `ps -T -p 7 -o pid,tid,comm | grep consensus | wc -l` returns 144
- **Finding:** Validators with low concurrency (8–16 threads) execute blocks **2.7x faster** than high concurrency (38–48 threads)
- **Fix:** Thread pinning for execution threads ([PR #382](https://github.com/aptos-labs/aptos-core-private/pull/382))

#### 4. Storage Latency Differences
`get_state_value` latency ~100ms on mainnet vs ~40ms on testnet. Correlation observed between this latency and block execution time.
- **Contributing factors:**
  - RocksDB compaction not keeping up on some nodes
  - Hardware differences (disk speed, CPU architecture)

#### 5. Hardware Inconsistencies
- **AMD vs Intel:** AMD CPUs significantly faster than Intel. Forge runs on AMD, mainnet has mixed hardware.
- **Geographic differences:** Hardware variations between `mainnet-validator-euwe4-2` (EU) and `mainnet-validator-apne1-0` (AP)
- **Disk performance:** Some nodes struggling with compaction; commit time high despite reasonable raw disk write time

#### 6. Missing Performance Features
- **Trusted code governance proposal** not enabled — accounts for ~30% performance difference in benchmarks
- **CONCURRENT_FUNGIBLE_BALANCE** off on mainnet
- **Async paranoid mode** not yet enabled (pending OtterSec security audit)
- **Per block gas limit** not enabled

#### 7. Outdated Node Versions
Nodes running old releases (1.38/1.39) are noticeably slower with lower hot state cache hit rates.

---

## Investigation & Benchmarks

### Concurrency Level Testing (Igor Kabiljo)

Block size = 80, measuring TPS across concurrency levels:

| Concurrency | Run 1 (TPS) | Run 2 (TPS) |
|-------------|-------------|-------------|
| 1           | 423         | 428         |
| 4           | 981         | 988         |
| 8           | 1,259       | 1,286       |
| 16          | 1,320       | 1,303       |
| 32          | 1,083       | 1,087       |

**Recommendation:** Reduce concurrency to 16 for optimal performance with headroom for other workloads.
[Full test run](https://github.com/aptos-labs/etna/actions/runs/21651327485)

### Replay Benchmark Comparison (zekun)

| Network | Block | Txns | Median (μs) | Mean (μs) | Min (μs) | Max (μs) |
|---------|-------|------|-------------|-----------|----------|----------|
| Mainnet | [595872260](https://explorer.aptoslabs.com/block/595872260/transactions?network=mainnet) | 34 | 73,464 | 77,032 | 71,833 | 96,201 |
| Testnet | [637861159](https://explorer.aptoslabs.com/block/637861159/transactions?network=testnet) | 65 | 74,256 | 76,111 | 69,992 | 96,205 |

Execution performance similar — difference comes from storage and consensus overhead.

### Sustained Load Tests — Testnet (Igor Kabiljo)

10-minute sustained tests (on top of existing ~100 TPS baseline):

| Target TPS | Result |
|------------|--------|
| 100        | Sustained and stable |
| 500        | Sustained and stable |
| 1,000      | **Not sustained or stable** |

[Grafana dashboard](https://grafana.aptoslabs.com/goto/ffc8z2yas6tq8b?orgId=stacks-340750)

### Mainnet Load Test Progression

| Date | TPS | Latency (avg) | P50 | P90 | P99 | Notes |
|------|-----|---------------|-----|-----|-----|-------|
| Feb 3 | ~300 | — | — | 2,400ms | — | Initial test |
| Feb 6 | 500 | 1,441ms | 1,300ms | 1,900ms | 4,000ms | After config changes |
| Feb 9 | 500 | 1,076ms | 900ms | 1,300ms | 2,100ms | After governance proposals executed |

### Forge Comparison with 100 Nodes (Balaji Arun)

Reproduced TPS issues in Forge with 100 nodes — peak ~500 TPS without hotfix optimizations.
- [100 nodes](https://grafana.aptoslabs.com/goto/efcn8ub1i3e2oe?orgId=stacks-340750)
- [7 nodes](https://grafana.aptoslabs.com/goto/afcn8vaa7isjkd?orgId=stacks-340750)

---

## Fixes & Mitigations

### Governance Proposals (Executed Friday, Feb 6)

| Proposal | Description | Expected Impact |
|----------|-------------|-----------------|
| 175 | Move perf improvements | ~30% combined |
| 176 | Move perf improvements | |
| 177 | Move perf improvements | |
| [Per block gas limit](https://github.com/aptos-foundation/mainnet-proposals/pull/202) | Enable per block gas limit | Pending |
| [Disable randomness fast path](https://github.com/aptos-foundation/mainnet-proposals/pull/203) | No longer useful after stake quorum in Europe | ~2x reduction in randomness CPU |

*(Proposal 174 submitted with wrong CLI version — ignored)*

### Hot-Fix Release v1.40.4 (Feb 4)

[Release PR #381](https://github.com/aptos-labs/aptos-core-private/pull/381)
- Concurrency level reduction
- Block time adjustments
- Config changes for consensus threads

**Expected impact:** ~30% improvement

### Performance Build

Private fix released to node operators with performance optimizations.
**Expected impact:** ~15%

### Async Paranoid Mode

Enabled on testnet nodes for evaluation ([Grafana VM metrics](https://grafana.aptoslabs.com/goto/efcbbpxw7ygowb?orgId=stacks-340750), [block times](https://grafana.aptoslabs.com/goto/dfcbchjsyacjke?orgId=stacks-340750)).
- Observable decrease in execution time on testnet
- **Mainnet enablement:** Pending OtterSec security audit results (expected with 1.41 release)
- **Expected impact:** ~15–20%

### Randomness Verification Optimizations

| Change | PR | Status | Expected Impact |
|--------|-----|--------|-----------------|
| Optimistic randomness shares verification | [#18646](https://github.com/aptos-labs/aptos-core/pull/18646) | In progress | ~3x CPU reduction |
| Disable randomness fast path | [#18644](https://github.com/aptos-labs/aptos-core/pull/18644), [#18707](https://github.com/aptos-labs/aptos-core/pull/18707) | Executed on mainnet | ~2x CPU reduction |
| Additional randomness CPU optimization | [#18699](https://github.com/aptos-labs/aptos-core/pull/18699) | In progress | Further reduction |

### Thread Pinning & CPU Isolation

[PR #382](https://github.com/aptos-labs/aptos-core-private/pull/382) — CPU pinning for execution threads:
- Default to `PinExeThreadsToCores` strategy on Linux
- `min(num_cpus/2, 16)` execution threads
- Move WVUF derivation (consensus randomness) to non-exe pool
- Guoteng building binary with pinned consensus threads for testing

### Other Fixes

| Fix | PR | Status |
|-----|-----|--------|
| Cache flush reduction | [#18592](https://github.com/aptos-labs/aptos-core/pull/18592) | Deployed to euwe2-2 |
| Jemalloc tuning | [#18642](https://github.com/aptos-labs/aptos-core/pull/18642) | Cherry-pick to 1.41 pending |
| RocksDB compaction fix | [#18688](https://github.com/aptos-labs/aptos-core/pull/18688) | Tested on euwe4-1 |
| Node operator env var cleanup | [internal-ops #6923](https://github.com/aptos-labs/internal-ops/pull/6923) | Pending review |

---

## Expected Improvement Stack

| Change | Expected Improvement |
|--------|---------------------|
| Governance proposals (175/176/177) | ~30% |
| Concurrency/blocktime config changes | ~30% |
| Performance build | ~15% |
| Async paranoid mode | ~15–20% |

**Theoretical combined throughput:** ~600 TPS (still short of 1,000 TPS target)

---

## Remaining Work Items

### Immediate
- [ ] Land optimistic randomness verification in main and 1.41 (@Daniel Xiang, @zekun)
- [ ] Verify in Forge with 100 nodes (@Balaji Arun)
- [ ] Cherry-pick jemalloc tuning to 1.41 (@qinfan)
- [ ] Enable async paranoid mode after OtterSec review
- [ ] Better isolate computation load via thread pinning (@zekun, @Guoteng Rao, @qinfan)
- [ ] Reduce layout cache flush (@George Mitenkov)
- [ ] Review hardware recommendations — faster single-core machines with potentially lower core counts (@zekun)
- [ ] Understand DB compaction issue (@Guoteng Rao, @qinfan)
- [ ] Long-running load test ~500 TPS on testnet (@Igor Kabiljo)

### Operator Coordination
- [ ] Work with slow operators on hardware upgrades:
  - "Very slow" group: Most improved; Amnis and Cryptomind getting new disks
  - "Moderate slow" group: Ongoing background work
- [ ] Ensure all nodes upgrade from 1.38/1.39 to latest release
- [ ] Announce Decibel load to node operators

### Testing
- [ ] Run load test on mainnet with latest binary (@Igor Kabiljo)
- [ ] Test latency when submitting transactions to European validator nodes (@Kent)
- [ ] Evaluate moving quorum to Asia (Japan) for latency improvements (@Balaji Arun, @Stelian)

---

## Resolution Criteria

The incident will be considered resolved when:
1. **Throughput:** Mainnet sustains **1,000 TPS**
2. **Latency:** Transaction latency maintained around **500ms** (target: 300–400ms)

### Long-Term Goals
- Scale to **1M orders/second**
- Reduce e2e latency to **200ms**
- Reserved capacity for Decibel (blockspace reservation or native operations)

---

## Key Meetings

| Date | Time | Meeting |
|------|------|---------|
| Feb 3 | 3:00 PM CT | War room (Tuesday) |
| Feb 4 | 5:00 PM CT | War room (Wednesday) |
| Feb 6 | 11:00 AM PT | Friday check-in |
| Feb 9 | 2:30 PM CT | Monday check-in |

[Google Meet link](https://meet.google.com/vti-yqno-hdo)

---

## Responders

| Name | Role / Contribution |
|------|---------------------|
| Igor Kabiljo | Load testing, benchmarking, concurrency analysis |
| zekun | Cache flush investigation, CPU profiling, release management, thread pinning |
| Guoteng Rao | Storage analysis, hardware investigation, operator coordination, compaction fixes |
| qinfan | Consensus thread analysis, RocksDB investigation, jemalloc tuning |
| Joshua Lind | Fullnode analysis, initial bottleneck identification |
| Balaji Arun | Forge reproduction, concurrency/blocktime changes, randomness fixes |
| Daniel Xiang | Randomness verification optimizations |
| George Mitenkov | Async paranoid mode, layout cache, Move perf features |
| Zhoujun Ma | Randomness fast path governance proposal |
| sherry | Governance proposals, coordination, release management |
| Kent | Business decisions, pre-deposit timeline |
| Stelian | Hardware investigation, benchmarking infrastructure |
| Avery Ching | Executive oversight |

---

## Reference Links

### Grafana Dashboards
- [Consensus](https://grafana.aptoslabs.com/d/consensus/consensus)
- [Execution](https://grafana.aptoslabs.com/d/execution/execution)
- [Testnet sustained load test](https://grafana.aptoslabs.com/goto/ffc8z2yas6tq8b?orgId=stacks-340750)
- [500 TPS mainnet test (Feb 6)](https://grafana.aptoslabs.com/goto/bfcgyo3p4vh1ce?orgId=stacks-340750)
- [500 TPS mainnet test (Feb 9)](https://grafana.aptoslabs.com/goto/efcr8l1scjif4e?orgId=stacks-340750)

### PRs
- [Cache flush fix #18592](https://github.com/aptos-labs/aptos-core/pull/18592)
- [Optimistic randomness verification #18646](https://github.com/aptos-labs/aptos-core/pull/18646)
- [Disable randomness fast path #18644](https://github.com/aptos-labs/aptos-core/pull/18644)
- [Randomness fast path governance #18645](https://github.com/aptos-labs/aptos-core/pull/18645)
- [Randomness fast path release yaml #18707](https://github.com/aptos-labs/aptos-core/pull/18707)
- [Jemalloc tuning #18642](https://github.com/aptos-labs/aptos-core/pull/18642)
- [RocksDB compaction fix #18688](https://github.com/aptos-labs/aptos-core/pull/18688)
- [Additional randomness optimization #18699](https://github.com/aptos-labs/aptos-core/pull/18699)

### Governance Proposals
- [Per block gas limit](https://github.com/aptos-foundation/mainnet-proposals/pull/202)
- [Disable randomness fast path](https://github.com/aptos-foundation/mainnet-proposals/pull/203)

### Other
- [Mainnet block replay analysis](https://gist.github.com/zekun000/6dd15bd947f8e82a9d7f910884def3cb)
- [Expensive transaction example](https://explorer.aptoslabs.com/txn/4217518682/userTxnOverview?network=mainnet)
- [Decibel perf work items (Notion)](https://www.notion.so/aptoslabs/Decibel-perf-work-items-H1-26-31a8b846eb7280269ae8d4292c1dbc62)

---

## Lessons Learned

*To be completed post-incident*

---

**Last Updated:** 2026-03-12
