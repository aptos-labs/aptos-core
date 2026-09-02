# AF-aggregator-v2-017

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x1::aggregator_v2::string_concat`
- Granularity: `function`
- Original source: `aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move`
- Source inside the shared package: `sources/AptosFramework/aggregator_v2/aggregator_v2.move`
- Source root: `aptos-move/framework/aptos-framework`
- Aptos Core commit: `6d836beedc56fc70c54f3b3046d1d248d850c64b`
- Shared package SHA-256: `a9681689d48bd9b0fd092b67670030c79da1d2f56df796af4a14f75d74a3b70e`
- Prepared tree SHA-256: `d3e0acd0977966b4df9150570f4f57e9c77e98fb3afbaa56f30e9e2b95f942f9`
- Required contract categories: `normal-result`, `abort`

Target functions:

- `string_concat`

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

- `0x1::error::canonical`
- `0x1::error::invalid_state`

Transitive specification functions referenced by those boundary contracts:

- `0x1::error::$canonical`

Transitive source modules required to compile the sample:

- `0x1::bcs`
- `0x1::error`
- `0x1::features`
- `0x1::mem`
- `0x1::option`
- `0x1::signer`
- `0x1::storage_slots_allocator`
- `0x1::string`
- `0x1::table`
- `0x1::table_with_length`
- `0x1::type_info`
- `0x1::vector`

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosFramework/aggregator_v2/aggregator_v2.spec.move`: `string_concat` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosFramework/aggregator_v2/aggregator_v2.move`
- `sources/AptosFramework/aggregator_v2/aggregator_v2.spec.move`
