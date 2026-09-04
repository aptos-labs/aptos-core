# mono-move e2e performance calibration log

Recalibration history, newest first. Each entry lists the workloads whose calibrated speedup drifted out of band, as `old -> new`; new rows show `new`. A speedup is MonoMove throughput over legacy MoveVM throughput on the same recorded blocks, so a number above 1.00x means MonoMove is faster.

## 2026-09-04

| workload | metric | runs | speedup |
| --- | --- | --- | --- |
| account-generation | execution | 5 | new |
| account-generation | inner_block_executor | 5 | new |
| account-generation | output_bytes_per_txn | 5 | new |
| account-generation | total | 5 | new |
| apt-fa-transfer | execution | 5 | new |
| apt-fa-transfer | inner_block_executor | 5 | new |
| apt-fa-transfer | output_bytes_per_txn | 5 | new |
| apt-fa-transfer | total | 5 | new |
| batch100-transfer | execution | 5 | new |
| batch100-transfer | inner_block_executor | 5 | new |
| batch100-transfer | output_bytes_per_txn | 5 | new |
| batch100-transfer | total | 5 | new |
| liquidity-pool-swap | execution | 5 | new |
| liquidity-pool-swap | inner_block_executor | 5 | new |
| liquidity-pool-swap | output_bytes_per_txn | 5 | new |
| liquidity-pool-swap | total | 5 | new |
| modify-global-resource | execution | 5 | new |
| modify-global-resource | inner_block_executor | 5 | new |
| modify-global-resource | output_bytes_per_txn | 5 | new |
| modify-global-resource | total | 5 | new |
| no-op | execution | 5 | new |
| no-op | inner_block_executor | 5 | new |
| no-op | output_bytes_per_txn | 5 | new |
| no-op | total | 5 | new |
| order-book-no-matches1-market | execution | 5 | new |
| order-book-no-matches1-market | inner_block_executor | 5 | new |
| order-book-no-matches1-market | output_bytes_per_txn | 5 | new |
| order-book-no-matches1-market | total | 5 | new |
| token-v2-ambassador-mint | execution | 5 | new |
| token-v2-ambassador-mint | inner_block_executor | 5 | new |
| token-v2-ambassador-mint | output_bytes_per_txn | 5 | new |
| token-v2-ambassador-mint | total | 5 | new |

