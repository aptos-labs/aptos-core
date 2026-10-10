# AX-native-position-types-005

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x7::native_position_types`
- Granularity: `module`
- Original source: `aptos-move/framework/aptos-experimental/sources/trading/position/native_position_types.move`
- Source inside the shared package: `sources/AptosExperimental/trading/position/native_position_types.move`
- Source root: `aptos-move/framework/aptos-experimental`
- Aptos Core commit: `950e413e46090d2056740c36dd7a77b1764b6936`
- Shared package SHA-256: `1c41a4a754554758e1632217bb867a0dc8c622072f937edf1e1ef44adaf1f116`
- Prepared tree SHA-256: `0cf49fe0eb3cfd0874bc553fcd586c35fdcbbdca7604fda2aa4f6e189f245d23`
- Required contract categories: `normal-result`

Target functions:

- `new_accumulative_index`
- `new_perp_v1`
- `unpack_perp_v1`

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

- None.

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosExperimental/trading/position/native_position_types.spec.move`: `new_accumulative_index` (1 block(s))
- `sources/AptosExperimental/trading/position/native_position_types.spec.move`: `new_perp_v1` (1 block(s))
- `sources/AptosExperimental/trading/position/native_position_types.spec.move`: `unpack_perp_v1` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosExperimental/trading/position/native_position_types.move`
