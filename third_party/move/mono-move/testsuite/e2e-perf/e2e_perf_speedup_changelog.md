# MonoMove end-to-end performance calibration log

Recalibration history, newest first. Each entry lists the workloads whose calibrated speedup drifted out of band, as `old -> new`; new rows show `new`. A speedup is MonoMove throughput over V1 MoveVM throughput on the same recorded blocks, so a number above 1.00x means MonoMove is faster.

## 2026-09-18

| workload | metric | runs | speedup |
| --- | --- | --- | --- |
| account-generation | execution | 5 | 3.79x -> 5.70x (+50.2%) |
| account-generation | inner_block_executor | 5 | 3.89x -> 6.00x (+54.0%) |
| account-generation | total | 5 | 3.67x -> 5.40x (+47.2%) |
| airdrop-fanout | execution | 4 | new |
| airdrop-fanout | inner_block_executor | 4 | new |
| airdrop-fanout | output_bytes_per_txn | 4 | new |
| airdrop-fanout | total | 4 | new |
| apt-fa-transfer | execution | 5 | 3.11x -> 4.64x (+49.3%) |
| apt-fa-transfer | inner_block_executor | 5 | 3.17x -> 4.85x (+53.2%) |
| apt-fa-transfer | total | 5 | 3.06x -> 4.08x (+33.0%) |
| bridge-relay | execution | 4 | new |
| bridge-relay | inner_block_executor | 4 | new |
| bridge-relay | output_bytes_per_txn | 4 | new |
| bridge-relay | total | 4 | new |
| cdp-liquidation | execution | 4 | new |
| cdp-liquidation | inner_block_executor | 4 | new |
| cdp-liquidation | output_bytes_per_txn | 4 | new |
| cdp-liquidation | total | 4 | new |
| clmm-swap | execution | 4 | new |
| clmm-swap | inner_block_executor | 4 | new |
| clmm-swap | output_bytes_per_txn | 4 | new |
| clmm-swap | total | 4 | new |
| clob-avl | execution | 4 | new |
| clob-avl | inner_block_executor | 4 | new |
| clob-avl | output_bytes_per_txn | 4 | new |
| clob-avl | total | 4 | new |
| dex-aggregator | execution | 4 | new |
| dex-aggregator | inner_block_executor | 4 | new |
| dex-aggregator | output_bytes_per_txn | 4 | new |
| dex-aggregator | total | 4 | new |
| lending-market | execution | 4 | new |
| lending-market | inner_block_executor | 4 | new |
| lending-market | output_bytes_per_txn | 4 | new |
| lending-market | total | 4 | new |
| nft-mint-market | execution | 4 | new |
| nft-mint-market | inner_block_executor | 4 | new |
| nft-mint-market | output_bytes_per_txn | 4 | new |
| nft-mint-market | total | 4 | new |
| no-op | execution | 5 | 3.73x -> 7.48x (+100.6%) |
| no-op | inner_block_executor | 5 | 3.83x -> 8.14x (+112.4%) |
| no-op | total | 5 | 3.67x -> 6.70x (+82.6%) |
| oracle-batch | execution | 4 | new |
| oracle-batch | inner_block_executor | 4 | new |
| oracle-batch | output_bytes_per_txn | 4 | new |
| oracle-batch | total | 4 | new |
| stableswap | execution | 4 | new |
| stableswap | inner_block_executor | 4 | new |
| stableswap | output_bytes_per_txn | 4 | new |
| stableswap | total | 4 | new |

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

