# `leaner-rust-export`

This is the project-owned Rustc Public driver for Leaner. Its M0 spike uses
Rustc Public's `run!` callback, inventories the public queries needed by the
mapper, and returns `ControlFlow::Break`, which stops rustc after analysis and
before code generation/linking. The inventory is a diagnostic, not another
exchange format; the only detached artifact will be `RawUnit` JSON.

Successful artifacts carry an explicit trusted import-evidence statement that
rustc analysis admitted the crate and exposed optimized generic MIR. Shared
validation retains this as an upstream assumption; it is not a proof and does
not yet bind source or configuration hashes.

It has no normal Cargo dependency and is not intended for `cargo install`.
The Rustc Public build requires Leaner's pinned nightly and matching
`rustc-dev` sysroot:

```bash
cd leaner-rust/rust-exporter
cargo build --features rustc-public
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/leaner-rust-export -- tests/raw-unit/basic.rs --crate-type=lib --edition=2024
```

Pass `--output <path>` before the `--` delimiter to emit the currently
supported RawUnit slice:

```bash
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/leaner-rust-export --output basic.raw.json -- \
  tests/raw-unit/basic.rs --crate-type=lib --edition=2024
```

Artifacts are detached inside the callback and written only after rustc has
stopped. Every `.rs` under `tests/raw-unit` is discovered automatically and
checked against its side-by-side `.exp.json`; adding a positive exporter case
requires no driver registration. `UB=1 cargo test --features rustc-public`
updates these expectations, while a normal mismatch reports a focused line
diff. The exporter writes deterministic, two-space-indented JSON so artifacts
remain readable in reviews. The Lean test suite decodes them and verifies their
canonical JSON value independently of whitespace. It structurizes and validates
the executable subset, including direct-call destinations and their normal
continuation edges. Source line and nested block comments are scanned outside
Rust literals, retained with UTF-8 locations in `Namespace.comments`, and
round-trip through both canonical source backends as non-semantic provenance.
Artifact mode forces `-Cpanic=abort` after the supplied rustc arguments so its
declared initial Rust profile cannot disagree with the body rustc analyzed.

`tests/fixtures/generic.rs` exercises the same callback with one generic trait
body. The probe checks that it remains one unspecialized body and observes the
core trait declaration, trait predicate, and direct `Step::step` callee.

`tests/raw-unit/scalar.rs` is the first typed parameter/operation artifact;
`tests/raw-unit/signed_division.rs` preserves rustc's signed division and
remainder operations and their panic assertions. The Lean integration test
executes both operand-sign combinations and checks truncation toward zero;
`tests/raw-unit/control.rs` maps optimized MIR `SwitchInt` to a raw Boolean
branch and preserves both joining `goto` edges; `tests/raw-unit/loop.rs`
preserves a natural-loop backedge and continuation exit;
`tests/raw-unit/call.rs` preserves a recursive direct callee, arguments,
destination, normal successor, and unwind-unreachable action.
`tests/raw-unit/multi_call.rs` proves that compilation-unit names and bodies
are deterministic and that a non-recursive call resolves to the second local
function declaration.
`tests/raw-unit/function_pointer.rs` maps a safe, non-variadic Rust-ABI
function pointer to the core function type, reifies its local function item as
a closure, and invokes it through the core indirect-call operation. The Lean
integration test executes `apply(41)` and observes `42`; higher-ranked,
generic, unsafe, variadic, foreign-ABI, and external function pointers remain
precise mapper boundaries.
`tests/raw-unit/integer_switch.rs` preserves ordered `u32` cases and the
default edge; the shared structurizer lowers them to a typed LIR match.
`tests/raw-unit/integer_widths.rs` interns every fixed Rust integer width and
preserves a negative signed literal. `u128` types are supported, while literal
values use the exchange format's arbitrary-precision JSON integers rather than
a host-sized or signed mirror carrier. `tests/raw-unit/u128_max.rs` preserves
and executes `u128::MAX` without truncation.
`tests/raw-unit/integer_cast.rs` preserves fixed-width integer truncation,
signed-to-unsigned conversion, and sign extension as executable core `cast`
operations. Other rustc cast kinds remain exact mapper rejections.
`tests/raw-unit/aggregate.rs` preserves tuple and fixed-array types and
constructors plus a tuple field projection represented by a core literal index
place.
`tests/raw-unit/array_index.rs` preserves a dynamic fixed-array index as a core
index place whose operand retains Rust's pointer-width `usize` type. It also
preserves a constant fixed-array projection, rustc's fixed-array length
constant, and the dynamic access's bounds-check assertion. Lean checks
canonical decoding and the structurized panic branch; target-width index
execution uses the explicit rustc-target profile width and covers both an
in-bounds result and the preserved panic. `tests/raw-unit/array_repeat.rs` maps MIR
`Repeat` to the core compact `repeatVector` primitive, validates its fixed
length and `Copy` requirement, and executes the baseline without expanding the
RawUnit artifact in proportion to the array length.
`tests/raw-unit/slice.rs` preserves Rust's unsized slice as a dynamic core
vector behind a shared reference. Slice pointer metadata lowers to core
reference dereference plus `length`, and dynamic indexing retains the usual
explicit bounds assertion; target width is no longer inferred or a preparation
blocker. Canonical Leaner source prints receiver auto-deref as `values.length`
and `values[index]`. The baseline executes both a normal first-element read and its empty-
slice bounds panic.
`tests/raw-unit/string_slice.rs` preserves Rust's unsized UTF-8 `str` behind
shared and mutable references. It maps to core LIR `string`, prints as
LeanerLang `&string` / `&mut string` and canonical Rust `&str` / `&mut str`,
and compares returned references plus final string heap state across re-import.
`tests/raw-unit/string_length.rs` maps the typed `core::str::len` library call
to neutral LIR `length`; its interpreter semantics count UTF-8 bytes, including
non-ASCII text, and canonical Rust re-import reconstructs the same call.
`tests/raw-unit/subslice.rs` preserves the from-end MIR subslice generated by a
rest pattern as a first-class core place. Shared place semantics read and write
the selected range, and the baseline reaches structured validated LIR without
inventing pointer arithmetic.
`tests/raw-unit/slice_from_end.rs` lowers a from-end constant slice index to the
existing core expression `length(slice) - offset` and index place. The baseline
therefore preserves the runtime-dependent projection without adding a
Rust-specific place form, and executes both the empty fallback and a nonempty
last-element read.
`tests/raw-unit/never.rs` maps Rust's `!` directly to the core never type. Its
diverging natural loop validates without inventing a return value or a
Rust-profile-only type tag.
`tests/raw-unit/nested_reference.rs` interns reference types from the inside
out, preserving `&&u32` as two core reference layers and both dereference
projections. Reference nesting therefore no longer stops at a scalar referent.
If the selected optimized body retains MIR `CopyForDeref`, the mapper preserves
its documented non-consuming read directly as core `read`; the following
dereference remains an ordinary place projection.
MIR `Len` remains a constant for fixed arrays and lowers a dynamic slice place
to core `read` plus `length`; pointer metadata uses the same value-level length
semantics after dereferencing its reference operand.
`tests/raw-unit/reference_composite.rs` exercises the type dependency in the
opposite direction: a tuple depends on its reference element type. Composite
and reference nodes use one deterministic dependency loop, including
alternating nests of the two forms.
`tests/raw-unit/generic_adt.rs` preserves a local type-parameterized declaration
and a concrete `Wrapper<u32>` use with core generic binders, type-parameter
types, and nominal type arguments. `tests/raw-unit/nested_generic_adt.rs`
checks that substituting `Outer<u32>` fields also discovers and interns the
nested concrete `Wrapper<u32>` type. `tests/raw-unit/generic_lifetime_adt.rs`
adds a lifetime binder and a concrete `Borrowed<'_, u32>` use whose generic
reference field is recursively substituted and validated.
`tests/raw-unit/generic_const_adt.rs` preserves evaluated integer and Boolean
const binders/arguments on `Tagged<u32, 3, true>`. The mapper derives the
declaration's checked `usize` and `bool` binder types from the admitted concrete
instantiation; canonical Rust source re-imports the declaration and concrete
reader with equal semantics. Symbolic consts inside type shapes and generic
function predicates remain exact boundaries rather than partial data.
`tests/raw-unit/generic_enum.rs` preserves a concrete `Maybe<u32>` use,
including its nominal type argument, discriminant read, payload downcast, and
structured match.
`tests/raw-unit/unary.rs` preserves signed integer negation and its overflow
assertion as an explicit panic branch, retaining checked debug-mode behavior.
`tests/raw-unit/shift.rs` preserves fixed-width left/right shifts and both
rustc overflow assertions; the structured artifact prepares for execution.
`tests/raw-unit/boolean_bitwise.rs` preserves eager Boolean AND, OR, and XOR
without conflating them with logical operators.
`tests/raw-unit/boolean_ordering.rs` preserves all four MIR ordering comparisons
on Rust's `false < true` ordering and prepares the result for execution.
`tests/raw-unit/bitwise_not.rs` preserves unsigned and signed fixed-width
integer complement.
`tests/raw-unit/overflowing.rs` exercises Rustc Public `CheckedBinaryOp` for
addition, subtraction, and multiplication. These lower to typed core
operations returning the wrapped integer together with its overflow flag.
`tests/raw-unit/reference.rs` validates a typed shared reference and dereference
place end to end. `tests/raw-unit/borrow_call.rs` additionally preserves an
ordinary MIR borrow, its lifetime table entry, and a direct call using the
borrowed value.
`tests/raw-unit/mutable_reference.rs` validates a write through a mutable
reference followed by a read through the same dereference place.
`tests/raw-unit/struct.rs` preserves plain fields, aggregate construction, and
field selection, including an owned nominal return.
`tests/raw-unit/enum.rs` preserves the nominal declaration, variant and field
identities, aggregate construction, actual integer discriminants,
value-producing discriminant read, and downcast/field projections. Its raw
integer switch structurizes to a typed match; the rustc-generated impossible
default becomes a non-matching path. The standard-Rust backend recognizes a
complete discriminant/field-selection switch and emits one direct enum-pattern
match, which reimports to the same semantic projection for both variants.
`tests/raw-unit/drop.rs` detaches a MIR drop place, normal successor, and
unwind action. Under the fixed `panic=abort` profile, Lean structurizes the
exact unreachable-unwind form to an explicit core drop followed by the normal
continuation; validation, preparation, and the LeanerLang `drop(place)` baseline
all check that destruction is not silently erased. Cleanup edges and effectful
drop glue remain explicit M4 boundaries.
`tests/raw-unit/assert.rs` detaches an `expected=false` division-by-zero MIR
assertion, its normal target, and unwind action. The Rust mirror also has typed
forms for storage-live/dead, place-mention, and user-type-ascription statements.
Rustc's borrow-check-only `FakeRead` is normalized to the same inert
place-mention form after the artifact records successful rustc admission.
Lean erases the validated inert administration and structurizes the assertion
as an explicit panic branch under the initial `panic=abort` profile.
The mirror additionally spells RawUnit's destructive `deinit`,
`setDiscriminant`, and provenance `retag` nodes. Of those, the pinned Rustc
Public optimized-MIR API exposes `SetDiscriminant`, which the mapper resolves
to the enum's stable variant name and retains for the current explicit
structurization rejection. The public statement API has no `Deinit` or `Retag`
case, so the mapper does not fabricate them.
The public rvalue API does expose ubiquitous `Use(..., WithRetag::Yes)` nodes.
They are explicitly normalized only for this rustc-checked `unsafe=reject`
profile, where provenance has no safe observable runtime effect; a future
unsafe profile must preserve them.
`tests/raw-unit/match_guard.rs` preserves the optimized enum switch plus guard
branch and validates their checked CFG-to-tree correspondence.
`tests/fixtures/corpus.rs` covers scalar arithmetic, an enum switch, a loop,
direct calls, an ordinary borrow, explicit drop and cleanup edges, and a raw
pointer operation. `tests/fixtures/unsupported.rs` checks that inline assembly
is visible as an explicit unsupported MIR terminator. Run the pinned API and
corpus checks with:

```bash
cargo test --features rustc-public
```

The source-to-LeanerLang E2E suite owns concrete error baselines for generic
trait code, raw pointers, inline assembly, and a borrow-check-invalid overlapping
mutable borrow. Those baselines retain the full normalized exporter diagnostic
and verify that failure leaves no partial JSON artifact. The exporter-local
probe tests still check that generic bodies, raw pointers in the capability
corpus, and inline assembly are observed before mapping. The compact
repeated-array fixture remains a positive canonical export case.

The design now explicitly selects the generic `optimized_mir` returned by the
pinned Rustc Public API. Plain lifetime/type-generic declarations and local
direct generic calls retain one parametric body; `generic_identity.rs` checks
validation, execution, LeanerLang, and canonical-Rust re-import. The remaining
M0 API blockers are that `FnDef` exposes neither its predicates nor a const
function binder's declared type. The mapper does not work around those gaps
through `rustc_middle`.

The M0 probe deliberately does **not** serialize a substitute JSON format. The
detached writer implements the versioned `RawUnit` JSON spelling owned by
`leaner-ir` directly. The mapper is CFG-driven rather than fixture-shaped: it
interns compilation-unit scalar and nominal types and names, remaps
declaration-local MIR locals, shares expression/place arenas across functions,
preserves explicit copy/move operations and core nominal constructors, and records source ranges for
declarations, locals, statements, and terminators. Ordinary scalar references
use core LIR reference types, lifetimes, place borrows, and dereference
projections. Enums use the core nominal/data vocabulary rather than a Rust
profile tag. Unsupported declaration and operation arrays use an uninhabited
Rust element type, preventing accidental partial serialization.

Rust `isize` remains `IntWidth.pointer` so readable Rust reconstruction does
not confuse it with `i32` or `i64`. The exporter records rustc Public's selected
16-, 32-, or 64-bit target width in Rust profile v2; executable preparation
requires that state, and integer evaluation resolves it without consulting the
Lean host.

## Lean import driver

The first M1.5 integration slice imports one self-contained Rust library file
through the Lean-owned command:

```bash
cd leaner-rust
lake exe leaner-rust -- import-file rust-exporter/tests/raw-unit/basic.rs \
  --output .lake/leaner-rust/basic.raw.json
```

The command builds this exporter with its pinned local toolchain, supplies the
matching sysroot library path, validates the detached RawUnit through the Rust
profile, and only then writes the artifact. Repeating the command revalidates
and reuses an artifact whose source, exporter, toolchain, profile, target, and
rustc-argument key still matches.

`LEANER_RUST_EXPORTER` selects an explicit development binary and
`LEANER_RUST_SYSROOT` selects its paired sysroot. This file mode is not yet the
normal-user prebuilt toolchain distribution promised by M1.5.

The command also imports a selected Cargo library package:

```bash
lake exe leaner-rust -- import-crate Tests/CargoFixture/Cargo.toml \
  --package leaner-rust-cargo-fixture \
  --features extra --no-default-features \
  --output .lake/leaner-rust/cargo-fixture.raw.json
```

`import-crate` runs locked Cargo metadata and a locked Cargo check under the
managed toolchain. The exporter acts as `RUSTC_WRAPPER`: dependency crates are
passed to Cargo's matching rustc and produce ordinary metadata, while the
explicit primary library target alone is extracted and stopped before
codegen. The cache binds the canonical manifest, Cargo metadata and lockfile,
all local path-package files, package/features/target selection, exporter,
toolchain, and semantic profile. Each uncached input gets a content-addressed
Cargo target directory, avoiding both destructive `cargo clean` calls and a
stale Cargo-fresh root that would skip extraction. `LEANER_RUST_CACHE` may
override the managed cache root.
