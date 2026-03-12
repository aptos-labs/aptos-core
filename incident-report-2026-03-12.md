# Incident Report — INC-14309

**Severity:** SEV2
**Status:** Investigating
**Date:** 2026-03-12
**Slack Channel:** [#INC-14309](https://aptos-org.slack.com/archives/C0ACWG0BGRX/p1773282697648049?thread_ts=1773282645.450869&cid=C0ACWG0BGRX)

---

## Summary

Mainnet throughput is limited to 750–800 TPS under load testing. Sustained rates above this level result in elevated and unstable latency. The target for incident closure is 1,000 TPS at ~500 ms latency (ideally 300–400 ms). Multiple contributing factors have been identified including cache flushing behavior, consensus thread oversubscription, concurrency configuration, and validator hardware inconsistencies.

---

## Impact

- **Throughput limitation:** Mainnet unable to sustain more than 750–800 TPS
- **Latency degradation:** Higher transaction rates cause unsustainable latency
- **Node availability:** Several validator nodes remain down due to missed hotfix upgrades
- **Business impact:** Pre-deposit timeline at risk — decision needed by Thursday night, as partners are already preparing funds

---

## Investigation & Findings

### Concurrency Level Testing (Igor Kabiljo)

Load testing against different concurrency levels (block size = 80) showed that reducing concurrency yields 20–30% gains. Optimal throughput observed at concurrency level 8–16:

| Concurrency Level | Run 1 (TPS) | Run 2 (TPS) |
|--------------------|-------------|-------------|
| 1                  | 423         | 428         |
| 4                  | 981         | 988         |
| 8                  | 1,259       | 1,286       |
| 16                 | 1,320       | 1,303       |
| 32                 | 1,083       | 1,087       |

**Recommendation:** Reduce concurrency to 16 to leave headroom for other non-conflicting workloads.

[Full test run](https://github.com/aptos-labs/etna/actions/runs/21651327485)

### Sustained Load Tests — Testnet (Igor Kabiljo)

10-minute sustained tests on testnet (on top of existing ~100 TPS baseline):

| Target TPS | Result |
|------------|--------|
| 100        | Sustained and stable |
| 500        | Sustained and stable |
| 1,000      | **Not sustained or stable** |

Latency from the transaction emitter was not captured in this run (display issue being investigated).

[Grafana dashboard](https://grafana.aptoslabs.com/goto/ffc8z2yas6tq8b?orgId=stacks-340750)

### Sustained Load Tests — Mainnet (Igor Kabiljo)

Mainnet load tests were run subsequently (100 TPS initial run), with team monitoring in real time for anomalies.

### Cache Flush Issue (zekun)

A cache flushing issue was identified as a contributing factor. Validator `euwe2-2` was updated with a fix from [PR #18592](https://github.com/aptos-labs/aptos-core/pull/18592) to evaluate the impact of the cache flushing change.

### Block Replay Analysis (zekun)

[Replayed mainnet blocks](https://gist.github.com/zekun000/6dd15bd947f8e82a9d7f910884def3cb) and spot-checked expensive ones. No anomalies found — all expensive blocks contained transactions that legitimately cost >700 gas units.

### Consensus Thread Oversubscription (qinfan)

Discovered 144 consensus threads running on 48-CPU validator machines, with 41 threads named `consensus-37`. This represents potential thread oversubscription (~3x CPU count) which may contribute to contention and degraded throughput.

### Hardware Inconsistencies (Guoteng Rao)

Investigating hardware differences between `mainnet-validator-euwe4-2` (EU) and `mainnet-validator-apne1-0` (AP). Geographic and hardware variations may account for performance discrepancies across validator nodes.

### Operator Issues (Guoteng Rao)

Identified operator-level issues contributing to node instability. Several nodes remain down due to missed hotfix upgrades.

---

## Actions Taken

### Completed
- [x] Concurrency level benchmarking across multiple configurations
- [x] Sustained load tests on testnet at 100, 500, and 1,000 TPS
- [x] Mainnet block replay and analysis of expensive transactions
- [x] Submitted governance proposals 175, 176, 177 for performance improvements (sherry) — executable Friday
  - *(Proposal 174 submitted with wrong CLI version — should be ignored)*
- [x] Identified consensus thread oversubscription (144 threads on 48 CPUs)
- [x] Identified cache flush issue as contributing factor

### In Progress
- [ ] Running sustained mainnet load tests with team monitoring
- [ ] Deploying cache flush fix to `euwe2-2` via [PR #18592](https://github.com/aptos-labs/aptos-core/pull/18592) and measuring improvement
- [ ] Investigating hardware differences between EU and AP validator nodes
- [ ] Testing transaction submission to European validator nodes for latency improvements
- [ ] Coordinating hotfix adoption with operators running outdated versions
- [ ] Reviewing [performance work items and timelines](https://www.notion.so/aptoslabs/Decibel-perf-work-items-H1-26-31a8b846eb7280269ae8d4292c1dbc62)

### Planned
- [ ] Execute governance proposals 175/176/177 on Friday
- [ ] Address consensus thread oversubscription on validator nodes
- [ ] Fix transaction emitter latency reporting for future test visibility
- [ ] Develop longer-term scaling strategy for higher throughput targets

---

## Resolution Criteria

The incident will be considered resolved when:
1. **Throughput:** Mainnet sustains **1,000 TPS**
2. **Latency:** Transaction latency maintained around **500 ms** (target: 300–400 ms)

---

## Key Meetings & Coordination

- **War room meeting** held Wednesday, Feb 4 at 5:00 PM CT ([Google Meet](https://meet.google.com/vti-yqno-hdo))
- Daily regrouping to track action items and root cause analysis (per Avery Ching)

---

## Responders

| Name | Role / Area |
|------|-------------|
| Igor Kabiljo | Load testing, benchmarking |
| zekun | Cache flush investigation, block replay analysis |
| Guoteng Rao | Hardware investigation, operator issues |
| qinfan | Consensus thread analysis |
| sherry | Governance proposals, coordination |
| Avery Ching | Executive oversight |
| Kent | Pre-deposit decision coordination |
| Stelian | Hardware investigation support |

---

## Lessons Learned

*To be completed post-incident*

---

**Last Updated:** 2026-03-12
