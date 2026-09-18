# MonoMove end-to-end performance calibration log

Recalibration history, newest first. Each entry lists the workloads whose calibrated speedup drifted out of band, as `old -> new`; new rows show `new`. A speedup is MonoMove throughput over V1 MoveVM throughput on the same recorded blocks, so a number above 1.00x means MonoMove is faster.

## 2026-09-18

| workload | metric | runs | speedup |
| --- | --- | --- | --- |
| account-generation | execution | 5 | 4.64x -> 5.69x (+22.7%) |
| account-generation | inner_block_executor | 5 | 4.83x -> 6.00x (+24.3%) |
| apt-fa-transfer | execution | 5 | 3.84x -> 4.64x (+20.8%) |
| apt-fa-transfer | inner_block_executor | 5 | 3.97x -> 4.85x (+22.2%) |
| bridge-relay | execution | 5 | 3.99x -> 4.42x (+10.8%) |
| bridge-relay | inner_block_executor | 5 | 4.17x -> 4.62x (+10.9%) |
| bridge-relay | total | 5 | 3.90x -> 4.34x (+11.1%) |
| cdp-liquidation | execution | 5 | 6.11x -> 6.92x (+13.3%) |
| cdp-liquidation | inner_block_executor | 5 | 6.38x -> 7.28x (+14.2%) |
| cdp-liquidation | total | 5 | 6.00x -> 6.70x (+11.6%) |
| clmm-swap | execution | 5 | 7.76x -> 8.62x (+11.2%) |
| clmm-swap | inner_block_executor | 5 | 8.11x -> 9.12x (+12.5%) |
| clmm-swap | total | 5 | 7.52x -> 8.42x (+11.9%) |
| clob-avl | execution | 5 | 7.39x -> 8.10x (+9.5%) |
| clob-avl | inner_block_executor | 5 | 7.79x -> 8.54x (+9.6%) |
| clob-avl | total | 5 | 7.16x -> 7.89x (+10.2%) |
| dex-aggregator | execution | 5 | 6.65x -> 7.30x (+9.7%) |
| dex-aggregator | inner_block_executor | 5 | 6.92x -> 7.62x (+10.0%) |
| lending-market | execution | 5 | 6.75x -> 7.58x (+12.3%) |
| lending-market | inner_block_executor | 5 | 7.02x -> 7.94x (+13.1%) |
| lending-market | total | 5 | 6.56x -> 7.35x (+12.0%) |
| liquidity-pool-swap | execution | 5 | 6.03x -> 7.25x (+20.3%) |
| liquidity-pool-swap | inner_block_executor | 5 | 6.29x -> 7.68x (+22.1%) |
| nft-mint-market | execution | 5 | 5.05x -> 5.62x (+11.4%) |
| nft-mint-market | inner_block_executor | 5 | 5.23x -> 5.82x (+11.2%) |
| nft-mint-market | total | 5 | 4.89x -> 5.47x (+11.8%) |
| no-op | execution | 5 | 4.62x -> 7.48x (+61.9%) |
| no-op | inner_block_executor | 5 | 4.82x -> 8.13x (+68.7%) |
| no-op | total | 5 | 4.48x -> 6.70x (+49.7%) |
| order-book-no-matches1-market | execution | 5 | 7.41x -> 8.40x (+13.4%) |
| order-book-no-matches1-market | inner_block_executor | 5 | 7.68x -> 8.70x (+13.3%) |
| stableswap | execution | 5 | 7.27x -> 8.41x (+15.6%) |
| stableswap | inner_block_executor | 5 | 7.69x -> 8.97x (+16.6%) |
| token-v2-ambassador-mint | execution | 5 | 5.41x -> 6.14x (+13.4%) |
| token-v2-ambassador-mint | inner_block_executor | 5 | 5.58x -> 6.37x (+14.1%) |
| token-v2-ambassador-mint | total | 5 | 4.93x -> 5.92x (+19.9%) |

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

