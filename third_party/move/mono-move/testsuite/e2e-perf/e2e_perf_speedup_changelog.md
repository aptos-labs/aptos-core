# MonoMove end-to-end performance calibration log

Recalibration history, newest first. Each entry lists the workloads whose calibrated speedup drifted out of band, as `old -> new`; new rows show `new`. A speedup is MonoMove throughput over V1 MoveVM throughput on the same recorded blocks, so a number above 1.00x means MonoMove is faster.

## 2026-09-18

| workload | metric | runs | speedup |
| --- | --- | --- | --- |
| account-generation | execution | 5 | 3.79x -> 4.64x (+22.3%) |
| account-generation | inner_block_executor | 5 | 3.89x -> 4.83x (+23.9%) |
| account-generation | total | 5 | 3.67x -> 4.41x (+20.1%) |
| airdrop-fanout | execution | 5 | new |
| airdrop-fanout | inner_block_executor | 5 | new |
| airdrop-fanout | output_bytes_per_txn | 5 | new |
| airdrop-fanout | total | 5 | new |
| apt-fa-transfer | execution | 5 | 3.11x -> 3.84x (+23.6%) |
| apt-fa-transfer | inner_block_executor | 5 | 3.17x -> 3.97x (+25.3%) |
| apt-fa-transfer | total | 5 | 3.06x -> 3.58x (+16.8%) |
| bridge-relay | execution | 5 | new |
| bridge-relay | inner_block_executor | 5 | new |
| bridge-relay | output_bytes_per_txn | 5 | new |
| bridge-relay | total | 5 | new |
| cdp-liquidation | execution | 5 | new |
| cdp-liquidation | inner_block_executor | 5 | new |
| cdp-liquidation | output_bytes_per_txn | 5 | new |
| cdp-liquidation | total | 5 | new |
| clmm-swap | execution | 5 | new |
| clmm-swap | inner_block_executor | 5 | new |
| clmm-swap | output_bytes_per_txn | 5 | new |
| clmm-swap | total | 5 | new |
| clob-avl | execution | 5 | new |
| clob-avl | inner_block_executor | 5 | new |
| clob-avl | output_bytes_per_txn | 5 | new |
| clob-avl | total | 5 | new |
| dex-aggregator | execution | 5 | new |
| dex-aggregator | inner_block_executor | 5 | new |
| dex-aggregator | output_bytes_per_txn | 5 | new |
| dex-aggregator | total | 5 | new |
| lending-market | execution | 5 | new |
| lending-market | inner_block_executor | 5 | new |
| lending-market | output_bytes_per_txn | 5 | new |
| lending-market | total | 5 | new |
| liquidity-pool-swap | execution | 5 | 5.27x -> 6.03x (+14.3%) |
| liquidity-pool-swap | inner_block_executor | 5 | 5.50x -> 6.29x (+14.5%) |
| liquidity-pool-swap | total | 5 | 5.19x -> 5.48x (+5.6%) |
| nft-mint-market | execution | 5 | new |
| nft-mint-market | inner_block_executor | 5 | new |
| nft-mint-market | output_bytes_per_txn | 5 | new |
| nft-mint-market | total | 5 | new |
| no-op | execution | 5 | 3.73x -> 4.62x (+23.8%) |
| no-op | inner_block_executor | 5 | 3.83x -> 4.82x (+25.8%) |
| no-op | total | 5 | 3.67x -> 4.48x (+22.0%) |
| oracle-batch | execution | 5 | new |
| oracle-batch | inner_block_executor | 5 | new |
| oracle-batch | output_bytes_per_txn | 5 | new |
| oracle-batch | total | 5 | new |
| order-book-no-matches1-market | execution | 5 | 6.95x -> 7.41x (+6.6%) |
| order-book-no-matches1-market | inner_block_executor | 5 | 7.18x -> 7.68x (+6.9%) |
| order-book-no-matches1-market | output_bytes_per_txn | 5 | 1.00x -> 0.88x (-12.2%) |
| stableswap | execution | 5 | new |
| stableswap | inner_block_executor | 5 | new |
| stableswap | output_bytes_per_txn | 5 | new |
| stableswap | total | 5 | new |
| token-v2-ambassador-mint | execution | 5 | 4.64x -> 5.41x (+16.6%) |
| token-v2-ambassador-mint | inner_block_executor | 5 | 4.78x -> 5.58x (+16.9%) |
| token-v2-ambassador-mint | total | 5 | 4.50x -> 4.93x (+9.7%) |

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

