# AF-big-ordered-map-038

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x1::big_ordered_map::internal_leaf_iter_borrow_entries_and_next_leaf_index`
- Granularity: `function`
- Original source: `aptos-move/framework/aptos-framework/sources/datastructures/big_ordered_map.move`
- Source inside the shared package: `sources/AptosFramework/datastructures/big_ordered_map.move`
- Source root: `aptos-move/framework/aptos-framework`
- Aptos Core commit: `6d836beedc56fc70c54f3b3046d1d248d850c64b`
- Shared package SHA-256: `a9681689d48bd9b0fd092b67670030c79da1d2f56df796af4a14f75d74a3b70e`
- Prepared tree SHA-256: `8c41c08d7a0d5cbb07751e0552ff8c4ceae2a55171be405ced3d058a569bf1df`
- Required contract categories: `normal-result`, `abort`, `state-transition`

Target functions:

- `internal_leaf_iter_borrow_entries_and_next_leaf_index`

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

- `0x1::storage_slots_allocator::borrow`

Transitive specification functions referenced by those boundary contracts:

- `0x1::option::$borrow`
- `0x1::option::$is_none`
- `0x1::table_with_length::spec_contains`
- `0x1::table_with_length::spec_get`

Transitive source modules required to compile the sample:

- `0x1::bcs`
- `0x1::cmp`
- `0x1::error`
- `0x1::fixed_point32`
- `0x1::math64`
- `0x1::mem`
- `0x1::option`
- `0x1::ordered_map`
- `0x1::storage_slots_allocator`
- `0x1::table`
- `0x1::table_with_length`
- `0x1::vector`

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosFramework/datastructures/big_ordered_map.spec.move`: `internal_leaf_iter_borrow_entries_and_next_leaf_index` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosFramework/datastructures/big_ordered_map.move`
- `sources/AptosFramework/datastructures/big_ordered_map.spec.move`
