# Move specification-inference corpus

This is the human-inspectable source catalog for the corpus prepared from Aptos
Core commit `950e413e46090d2056740c36dd7a77b1764b6936`. Every experimental arm receives the same source
hash for a sample; treatment-specific skills and tools are stored separately.

## Metadata

- [`manifest.json`](manifest.json): the 20 selected sample records and hashes;
  its `corpus_status` is authoritative for round readiness.
- [`metadata/candidate-inventory.json`](metadata/candidate-inventory.json): the
  complete compiler-AST source frame.
- [`metadata/selection.json`](metadata/selection.json): inclusion, exclusion,
  reserve, and replacement decisions.
- [`screening/summary.json`](screening/summary.json) and
  [`screening/state-label-repair-005/`](screening/state-label-repair-005/): current
  compatibility evidence; historical results remain under `screening/results/`.

## Shared editable framework

[`framework/`](framework/) is the only Move package stored by the corpus. It
contains 154 modules and
257 Move source/specification files: the union
of all targets and their source-level transitive dependencies. Named addresses,
original paths, and the exact module-to-file mapping are in
[`framework/corpus-modules.json`](framework/corpus-modules.json).

Every sample is a small overlay recipe. At run time the controller copies the
shared package and applies the sample's preparation patch, which removes only
that target's reference specification and adds its task descriptor. There are
no per-sample framework snapshots.

## Samples

Each sample README records provenance, dependency closure, address aliases,
preparation edits, allowed edit paths, required contract categories, and hashes.

| Sample | Target | Granularity | Target source in shared package |
| --- | --- | --- | --- |
| [`AF-account-025`](samples/AF-account-025/) | `0x1::account::increment_sequence_number` | `function` | `sources/AptosFramework/account/account.move` |
| [`AF-account-036`](samples/AF-account-036/) | `0x1::account::revoke_any_signer_capability` | `function` | `sources/AptosFramework/account/account.move` |
| [`AF-aptos-coin-010`](samples/AF-aptos-coin-010/) | `0x1::aptos_coin` | `module` | `sources/AptosFramework/aptos_coin.move` |
| [`AF-aptos-governance-034`](samples/AF-aptos-governance-034/) | `0x1::aptos_governance::update_governance_config` | `function` | `sources/AptosFramework/aptos_governance.move` |
| [`AF-code-017`](samples/AF-code-017/) | `0x1::code` | `module` | `sources/AptosFramework/code.move` |
| [`AF-coin-003`](samples/AF-coin-003/) | `0x1::coin::balance` | `function` | `sources/AptosFramework/coin.move` |
| [`AF-epoch-timeout-config-007`](samples/AF-epoch-timeout-config-007/) | `0x1::epoch_timeout_config` | `module` | `sources/AptosFramework/configs/epoch_timeout_config.move` |
| [`AF-gas-schedule-002`](samples/AF-gas-schedule-002/) | `0x1::gas_schedule::on_new_epoch` | `function` | `sources/AptosFramework/configs/gas_schedule.move` |
| [`AF-multisig-account-015`](samples/AF-multisig-account-015/) | `0x1::multisig_account::can_execute` | `function` | `sources/AptosFramework/multisig_account.move` |
| [`AF-multisig-account-067`](samples/AF-multisig-account-067/) | `0x1::multisig_account::validate_owners` | `function` | `sources/AptosFramework/multisig_account.move` |
| [`AF-stake-004`](samples/AF-stake-004/) | `0x1::stake::append` | `function` | `sources/AptosFramework/stake.move` |
| [`AF-transaction-fee-010`](samples/AF-transaction-fee-010/) | `0x1::transaction_fee::store_aptos_coin_mint_cap` | `function` | `sources/AptosFramework/transaction_fee.move` |
| [`AF-vesting-042`](samples/AF-vesting-042/) | `0x1::vesting::vesting_schedule` | `function` | `sources/AptosFramework/vesting.move` |
| [`AX-bulk-order-book-009`](samples/AX-bulk-order-book-009/) | `0x7::bulk_order_book::get_remaining_size` | `function` | `sources/AptosExperimental/trading/order_book/bulk_order_book.move` |
| [`AX-dead-mans-switch-operations-001`](samples/AX-dead-mans-switch-operations-001/) | `0x7::dead_mans_switch_operations::cleanup_expired_bulk_order` | `function` | `sources/AptosExperimental/trading/market/dead_mans_switch_operations.move` |
| [`AX-native-position-types-005`](samples/AX-native-position-types-005/) | `0x7::native_position_types` | `module` | `sources/AptosExperimental/trading/position/native_position_types.move` |
| [`AX-order-book-006`](samples/AX-order-book-006/) | `0x7::order_book::client_order_id_exists` | `function` | `sources/AptosExperimental/trading/order_book/order_book.move` |
| [`AX-pending-order-book-index-006`](samples/AX-pending-order-book-index-006/) | `0x7::pending_order_book_index::take_ready_price_move_up_orders` | `function` | `sources/AptosExperimental/trading/order_book/pending_order_book_index.move` |
| [`AX-price-time-index-014`](samples/AX-price-time-index-014/) | `0x7::price_time_index::new_price_time_idx` | `function` | `sources/AptosExperimental/trading/order_book/price_time_index.move` |
| [`AX-trading-native-capability-010`](samples/AX-trading-native-capability-010/) | `0x7::trading_native_capability` | `module` | `sources/AptosExperimental/trading/position/trading_native_capability.move` |
