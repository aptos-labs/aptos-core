# Single-node execution-performance calibration log

Recalibration history, newest first. Each entry lists the tests whose calibrated value drifted out of band, as a signed `tps % change` (negative means slower); new tests show `new`.

## 2026-06-24

| transaction_type | module_working_set | executor | runs | tps % change |
| --- | --- | --- | --- | --- |
| modify-global-resource | 1 | VM | 9 | -18.6% |
| smart-table-picture30-k-with200-change | 1 | VM | 9 | +18.8% |
| modify-global-flag-agg-v2 | 1 | VM | 9 | +8.7% |
| modify-global-milestone-agg-v2 | 1 | VM | 9 | +10.0% |
| resource-groups-global-write-and-read-tag1-kb | 1 | VM | 9 | +14.1% |
| token-v1ft-mint-and-transfer | 1 | VM | 9 | -10.8% |
| liquidity-pool-swap | 1 | VM | 9 | -7.0% |
| no-op-fee-payer | 1 | VM | 9 | -11.7% |

