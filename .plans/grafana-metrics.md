# Prefix Consensus Grafana Metrics

## Metric: `pc_slot_duration_s`

Prometheus histogram with a `stage` label. Measures latency (in seconds) for each stage of the prefix consensus slot pipeline.

Defined in: `consensus/src/prefix_consensus/counters.rs`
Instrumented in: `consensus/src/prefix_consensus/slot_manager.rs`

## PromQL

Average latency per stage (across all validators):
```promql
avg(rate(pc_slot_duration_s_sum{stage=~".+"}[1m]) / rate(pc_slot_duration_s_count{stage=~".+"}[1m])) by (stage)
```

## Slot Pipeline Stages

```
|-- payload_pull --|-- proposal_wait --|-- spc_to_vlow --|-- vlow_* --|-- vlow_to_vhigh --|-- vhigh_* --|-- finalization --|
                                                                                                         (next slot starts)
```

### Main Stages

| Stage | Description | Measured span |
|-------|-------------|---------------|
| `payload_pull` | Pull transactions from quorum store/mempool | `pull_payload()` entry to return |
| `proposal_wait` | Wait for other validators' proposals (timer or all-received) | After own proposal broadcast to `run_spc()` call |
| `spc_to_vlow` | SPC View 1: inner PC 3-round voting producing v_low | SPC spawn to `on_spc_v_low()` entry |
| `vlow_entry_resolution` | Resolve missing entry data for v_low positions | `resolve_missing_entries()` in v_low path |
| `vlow_commit_wave` | Build per-entry blocks for v_low, send to execution | `commit_wave()` for wave 1 (block building only) |
| `vlow_to_vhigh` | Wait for SPC View 2 to produce v_high (after v_low processing) | After v_low commit_wave to `on_spc_v_high_complete()` entry |
| `vhigh_entry_resolution` | Resolve missing entry data for v_high delta positions | `resolve_missing_entries()` in v_high path |
| `vhigh_commit_wave` | Build per-entry blocks for v_high delta, send to execution | `commit_wave()` for wave 2 (block building only) |
| `finalization` | Ranking update, cleanup, state teardown | `finalize_slot()` (excludes `start_new_slot`) |
| `total` | End-to-end slot duration | `start_new_slot()` to next `start_new_slot()` |

### Finalization Sub-stages

| Stage | Description |
|-------|-------------|
| `fin_extract_proof` | Look up canonical commit proof from v_high entries |
| `fin_ranking_update` | `ranking_manager.update_with_proof()` — SPC-aware demotion |
| `fin_cleanup` | Drop slot state, SPC channels, message buffers, payloads |

## Verification Property

All main stages (excluding `total`) should sum to `total`:
```
total ≈ payload_pull + proposal_wait + spc_to_vlow + vlow_entry_resolution
       + vlow_commit_wave + vlow_to_vhigh + vhigh_entry_resolution
       + vhigh_commit_wave + finalization
```

Finalization sub-stages should sum to `finalization`:
```
finalization ≈ fin_extract_proof + fin_ranking_update + fin_cleanup
```

## Metric: `pc_spc_round_duration_s`

Prometheus histogram with a `round` label. Measures latency (in seconds) for each round of the inner PC protocol within SPC.

Defined in: `consensus/prefix-consensus/src/counters.rs`
Instrumented in: `consensus/prefix-consensus/src/inner_pc_impl.rs`

### PromQL

Average latency per round (across all validators):
```promql
avg(rate(pc_spc_round_duration_s_sum[1m]) / rate(pc_spc_round_duration_s_count[1m])) by (round)
```

### Rounds

| Round | Description | Measured span |
|-------|-------------|---------------|
| `round1` | Vote1 broadcast → QC1 formation | `start()` / `start_round1()` to quorum in `process_vote1()` |
| `round2` | Vote2 broadcast → QC2 formation | `cascade_from_round2()` / `start_round2()` to quorum in `process_vote2()` |
| `round3` | Vote3 broadcast → QC3 formation | `cascade_from_round3()` / `start_round3()` to quorum in `process_vote3()` |

### Verification Property

All rounds should sum to approximately `spc_to_vlow` (from `pc_slot_duration_s`):
```
spc_to_vlow ≈ round1 + round2 + round3 + inter-round overhead
```

Each round is dominated by network RTT (waiting for >2/3 stake of votes to arrive).

## Notes

- `proposal_wait` excludes `payload_pull` time (measured from after broadcast)
- `vlow_to_vhigh` measures actual critical-path wait (starts after v_low processing, not when v_low arrives). If SPC View 2 finishes during v_low processing, this is near zero.
- `vlow_commit_wave` / `vhigh_commit_wave` measure only block building + execution send, not finalization
- `finalization` excludes `start_new_slot()` of the next slot
- Bucket range: 1ms to 5s (15 buckets)

## Typical Values (forge run export 15, 7 validators)

- `payload_pull`: ~5ms
- `proposal_wait`: ~300ms (timer-dominated)
- `spc_to_vlow`: ~350ms (View 1: 3 rounds of inner PC)
- `vlow_to_vhigh`: near 0 (View 2 finishes during v_low processing)
- `finalization`: ~117ms (dominated by cleanup/drops)
- `total`: ~950ms
