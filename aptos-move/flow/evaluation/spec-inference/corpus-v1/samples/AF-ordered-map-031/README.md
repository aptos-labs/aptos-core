# AF-ordered-map-031

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x1::ordered_map::iter_is_end`
- Granularity: `function`
- Original source: `aptos-move/framework/aptos-framework/sources/datastructures/ordered_map.move`
- Source inside the shared package: `sources/AptosFramework/datastructures/ordered_map.move`
- Source root: `aptos-move/framework/aptos-framework`
- Aptos Core commit: `6d836beedc56fc70c54f3b3046d1d248d850c64b`
- Shared package SHA-256: `a9681689d48bd9b0fd092b67670030c79da1d2f56df796af4a14f75d74a3b70e`
- Prepared tree SHA-256: `ea80613caeaa0e96a0b45abe9c15d0c389fe9e36f4db349033102398564d08a5`
- Required contract categories: `normal-result`

Target functions:

- `iter_is_end`

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

- None.

Transitive specification functions referenced by those boundary contracts:

- None.

Transitive source modules required to compile the sample:

- `0x1::bcs`
- `0x1::big_ordered_map`
- `0x1::cmp`
- `0x1::error`
- `0x1::fixed_point32`
- `0x1::math64`
- `0x1::mem`
- `0x1::option`
- `0x1::storage_slots_allocator`
- `0x1::table`
- `0x1::table_with_length`
- `0x1::vector`

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosFramework/datastructures/ordered_map.spec.move`: `iter_is_end` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosFramework/datastructures/ordered_map.move`
- `sources/AptosFramework/datastructures/ordered_map.spec.move`
