# AX-trading-native-capability-010

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x7::trading_native_capability`
- Granularity: `module`
- Original source: `aptos-move/framework/aptos-experimental/sources/trading/position/trading_native_capability.move`
- Source inside the shared package: `sources/AptosExperimental/trading/position/trading_native_capability.move`
- Source root: `aptos-move/framework/aptos-experimental`
- Aptos Core commit: `1ffaa408ec8899d19e0933b1680dce339051cd9f`
- Shared package SHA-256: `5b885c344c622fa55054a3d51f8afcafdb5b6dc927e9c7a989ba86b229ee3204`
- Prepared tree SHA-256: `e05b0affffc39d704d881a1c4f6d0b2ca4bcdc0ae8b37df44f29c620f9abe761`
- Required contract categories: `normal-result`, `abort`, `state-transition`, `frame`

Target functions:

- `register`
- `init_module`
- `assert_active`
- `assert_valid`
- `deny`
- `get_capability`
- `is_denied`
- `reenable`

## Compilation context

The shared package contains the union of the target modules and their complete
source-level transitive module dependencies. Its module/file map and resolved
named addresses are recorded in
[`framework/corpus-modules.json`](framework/corpus-modules.json). Modules other
than this sample's target are compilation context, not additional inference
targets.

Opaque/bodyless boundaries whose contracts are visible while proving this
target. This closure traverses transparent executable callees and behavioral
predicates referenced from reached contracts:

- `0x1::big_ordered_map::add`
- `0x1::big_ordered_map::contains`
- `0x1::big_ordered_map::new`
- `0x1::big_ordered_map::remove`
- `0x1::error::canonical`
- `0x1::features::is_enabled`
- `0x1::signer::borrow_address`
- `0x1::system_addresses::assert_aptos_framework`
- `0x7::trading_native_capability::assert_active`

Transitive specification functions referenced by those boundary contracts:

- `0x1::big_ordered_map::spec_contains_key`
- `0x1::features::spec_is_enabled`
- `0x1::signer::$address_of`
- `0x1::signer::$borrow_address`
- `0x7::trading_native_capability::spec_active_aborts`
- `0x7::trading_native_capability::spec_registry`

Transitive source modules required to compile the sample:

- `0x1::bcs`
- `0x1::big_ordered_map`
- `0x1::cmp`
- `0x1::error`
- `0x1::features`
- `0x1::fixed_point32`
- `0x1::math64`
- `0x1::mem`
- `0x1::option`
- `0x1::ordered_map`
- `0x1::signer`
- `0x1::storage_slots_allocator`
- `0x1::system_addresses`
- `0x1::table`
- `0x1::table_with_length`
- `0x1::vector`

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `register` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `init_module` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `assert_active` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `assert_valid` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `deny` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `get_capability` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `is_denied` (1 block(s))
- `sources/AptosExperimental/trading/position/trading_native_capability.spec.move`: `reenable` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosExperimental/trading/position/trading_native_capability.move`
