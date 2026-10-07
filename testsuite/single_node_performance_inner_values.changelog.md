# Single-node inner block executor calibration log

Recalibration history, newest first. Each entry lists the tests whose calibrated value drifted out of band, as a signed `inner tps % change` (negative means slower); new tests show `new`.

## 2026-10-07

| transaction_type | module_working_set | executor | runs | inner tps % change |
| --- | --- | --- | --- | --- |
| no-op | 1 | VM | 174 | new |
| no-op | 1000 | VM | 173 | new |
| apt-fa-transfer | 1 | VM | 173 | new |
| account-generation | 1 | VM | 172 | new |
| account-resource32-b | 1 | VM | 172 | new |
| modify-global-resource | 1 | VM | 172 | new |
| modify-global-resource | 100 | VM | 172 | new |
| publish-package | 1 | VM | 172 | new |
| mix_publish_transfer | 1 | VM | 172 | new |
| batch100-transfer | 1 | VM | 172 | new |
| vector-picture30k | 1 | VM | 172 | new |
| vector-picture30k | 100 | VM | 172 | new |
| smart-table-picture30-k-with200-change | 1 | VM | 172 | new |
| smart-table-picture30-k-with200-change | 100 | VM | 172 | new |
| modify-global-resource-agg-v2 | 1 | VM | 172 | new |
| modify-global-flag-agg-v2 | 1 | VM | 172 | new |
| modify-global-bounded-agg-v2 | 1 | VM | 172 | new |
| modify-global-milestone-agg-v2 | 1 | VM | 172 | new |
| resource-groups-global-write-tag1-kb | 1 | VM | 172 | new |
| resource-groups-global-write-and-read-tag1-kb | 1 | VM | 172 | new |
| resource-groups-sender-write-tag1-kb | 1 | VM | 172 | new |
| resource-groups-sender-multi-change1-kb | 1 | VM | 172 | new |
| token-v1ft-mint-and-transfer | 1 | VM | 172 | new |
| token-v1ft-mint-and-transfer | 100 | VM | 172 | new |
| token-v1nft-mint-and-transfer-sequential | 1 | VM | 172 | new |
| token-v1nft-mint-and-transfer-sequential | 100 | VM | 172 | new |
| coin-init-and-mint | 1 | VM | 172 | new |
| coin-init-and-mint | 100 | VM | 172 | new |
| fungible-asset-mint | 1 | VM | 172 | new |
| fungible-asset-mint | 100 | VM | 172 | new |
| no-op5-signers | 1 | VM | 172 | new |
| token-v2-ambassador-mint | 1 | VM | 172 | new |
| token-v2-ambassador-mint | 100 | VM | 172 | new |
| liquidity-pool-swap | 1 | VM | 170 | new |
| liquidity-pool-swap | 100 | VM | 170 | new |
| liquidity-pool-swap-stable | 1 | VM | 170 | new |
| liquidity-pool-swap-stable | 100 | VM | 170 | new |
| deserialize-u256 | 1 | VM | 170 | new |
| no-op-fee-payer | 1 | VM | 170 | new |
| no-op-fee-payer | 100 | VM | 170 | new |
| simple-script | 1 | VM | 170 | new |
| vector-trim-append-len3000-size1 | 1 | VM | 170 | new |
| vector-remove-insert-len3000-size1 | 1 | VM | 170 | new |
| order-book-no-matches50-markets | 1 | VM | 170 | new |
| order-book-balanced-matches25-pct50-markets | 1 | VM | 170 | new |
| order-book-balanced-matches80-pct50-markets | 1 | VM | 170 | new |
| order-book-balanced-size-skewed80-pct50-markets | 1 | VM | 170 | new |
| monotonic-counter-single | 1 | VM | 170 | new |
| fibonacci-recursive20 | 1 | VM | 169 | new |
| fibonacci-tail-recursive20 | 1 | VM | 169 | new |
| fibonacci-iterative20 | 1 | VM | 169 | new |
| no_commit_apt-fa-transfer | 1 | VM | 169 | new |

