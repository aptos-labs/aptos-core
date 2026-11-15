# `verex-rust-export`

This is the project-owned Rustc Public driver for Verex. Its M0 spike uses
Rustc Public's `run_with_tcx!` callback, inventories the public queries needed by the
mapper, and returns `ControlFlow::Break`, which stops rustc after analysis and
before code generation/linking. The inventory is a diagnostic, not another
exchange format; the only detached artifact will be `RawUnit` JSON.

Successful artifacts carry an explicit trusted import-evidence statement that
rustc analysis admitted the crate and exposed optimized generic MIR. Shared
validation retains this as an upstream assumption; it is not a proof and does
not yet bind source or configuration hashes.

It has no normal Cargo dependency and is not intended for `cargo install`.
The Rustc Public build requires Verex's pinned nightly and matching
`rustc-dev` sysroot:

```bash
cd verex-rust/rust-exporter
cargo build --features rustc-public
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/verex-rust-export -- tests/raw-unit/basic.rs --crate-type=lib --edition=2024
```

Pass `--output <path>` before the `--` delimiter to emit the currently
supported RawUnit slice:

```bash
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/verex-rust-export --output basic.raw.json -- \
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

A crate is one namespace and its modules are scopes of it:
`tests/raw-unit/modules.rs` names each function and type in the module path
it is declared in, and the namespace declares those modules at their
declarations. An associated function is named in the scope of its type
(inherent impl), trait, or trait impl (`<Token as std::ops::Drop>` in
`drop.rs`). Rustc Public does not say which definitions are modules, so a
module is an enclosing definition that is neither a body nor the impl or trait
of an associated item.

`tests/raw-unit/dependency_types.rs` reaches `Option`, `Result`, and `Vec`:
each dependency crate is a namespace after the crate's, stating
`verify = false`, with the types the crate reaches. A type whose shape a body
observes (constructs, matches, or projects) carries it; any other is opaque,
marked `struct.opaque`. Dependency declarations have generated locations
naming their definitions, since the unit does not record the toolchain's
sources. `tests/raw-unit/dependency_calls.rs` calls dependency functions:
each call is resolved as codegen resolves it (a trait method on a concrete
type reaches its impl's method), and each called dependency function is
declared by its generic signature without its body.
`tests/raw-unit/dependency_projection.rs` calls a function whose signature names
an associated type. Rustc Public has no normalizing signature query, but the
types in MIR are normalized, so each distinct associated type is a generic
parameter after the function's generics, named after the associated item, and a
call instantiates it by unifying the declared signature with the call's operand
and destination types. A call this does not bind is an export error at the call.
Parameter-dependent dispatch uses explicit dictionaries for local single-method
`&mut self` traits returning unit and for declared `Fn`, `FnMut`, and `FnOnce` bounds. Other trait
interfaces still require checked evidence (`tests/fixtures/generic_trait_call.rs`).

Export evaluates each constant a body uses and reads the result through the
type's layout, so no constant is matched by name. Integers, `bool`, `char` and
`&str` are literals (`tests/raw-unit/const_scalar.rs`); tuples, arrays, structs and
enum variants are constructions of their parts (`const_aggregate.rs`, padding is
never read); a constant at `&T` is a shared borrow of its referent's value, the
core operation of a computed value, since a shared reference is the value it
observes (`const_reference.rs`). An immutable `static` reads the same way, through
its initializer (`const_static.rs`). A zero-sized constant of an ADT is the
constructor of its layout's single variant, recursively, and optimized MIR assigns
no zero-sized place, so a body returning a zero-sized type returns that value
(`const_zero_sized.rs`). The confined `driver/polyfill.rs` bridge evaluates
promoted constants (`const_promoted.rs`, `polyfill.rs`). Constants whose values
depend on generic parameters and statics with interior mutability remain refused;
`static mut` stays rejected.

The bridge maps external macro expansion spans to their outer source call sites,
so `assert!`, `assert_eq!`, and `panic!` carry primary locations in the user's
source. The dominator fallback remains for generated spans without a local call site.

A failure in a body is not an export error: an rvalue, place, call or constant the
unit cannot state (a generic constant or a trait call that needs
evidence) is an opaque node where it stands, stating why, and the rest of the body
maps. A diverging region holding one collapses; any other survivor refuses its
function at import. A type only a body states, of a local or of a call's instantiation,
refuses the function the same way, before anything reads the body
(`tests/opaque-unit/local_types.rs`). A function whose signature cannot be mapped, by a
type, a struct field it reaches, or a binder, is declared `function.opaque` with no
generics or parameters: import reports it as refused and refuses every caller, whatever
its contract (`tests/opaque-unit/signature_types.rs`). What the whole unit depends on
stays an export error: names and scopes, `Copy` and destructor properties. The exporter pins `-Copt-level=0` beside
`-Cpanic=abort`, so a source has one MIR whatever Cargo's profile says.

`tests/opaque-unit` holds artifacts that validation rejects. A constant with no
statement in the unit (a function pointer, a float) is `ConstValue.opaque` with its
type's text, and a callee that is unsafe or whose signature has no mapping is
declared with the Rust property `function.opaque`, with no generics or parameters
and a `!` result. An import pass collapses the diverging regions that hold them,
and the import prunes an opaque declaration nothing then calls. Validation rejects a
node a function reaches and any opaque declaration left (`VIR-OPAQUE-SURVIVOR`).
A constant states the shape only of an ADT whose variants and fields are all
visible. Rustc Public exposes no field's visibility, so that is a local ADT, an enum,
a struct without fields, or a struct a body observes, whose fields rustc admitted;
a constant of any other dependency struct is opaque.

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
default edge; the shared structurizer lowers them to a typed VIR match.
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
`tests/raw-unit/slice.rs` preserves Rust's unsized `[T]` as the distinct core
`Slice<T>` type behind a shared reference. Owned `Vec<T>` remains
`Vector<T>`; fixed arrays retain their length in their type. Slice pointer metadata lowers to core
reference dereference plus `length`, and dynamic indexing retains the usual
explicit bounds assertion; target width is no longer inferred or a preparation
blocker. Canonical Verex source prints receiver auto-deref as `values.length`
and `values[index]`. The baseline executes both a normal first-element read and its empty-
slice bounds panic.
`tests/raw-unit/string_slice.rs` preserves Rust's unsized UTF-8 `str` behind
shared and mutable references. It maps to core VIR `string`, prints as
VerexSL `&string` / `&mut string` and canonical Rust `&str` / `&mut str`,
and compares returned references plus final string heap state across re-import.
`tests/raw-unit/string_length.rs` maps the typed `core::str::len` library call
to neutral VIR `length`; its interpreter semantics count UTF-8 bytes, including
non-ASCII text, and canonical Rust re-import reconstructs the same call.
`tests/raw-unit/subslice.rs` preserves the from-end MIR subslice generated by a
rest pattern as a first-class core place. Shared borrows read the selected
range as a slice snapshot; arbitrary mutable subslices remain unsupported
until their fixed-extent write-back is carried by the native semantics.
`tests/raw-unit/unsize.rs` folds the coercion of `&[T; N]` to `&[T]` into a borrow
of the core place `unsize p`, preserving the distinction between fixed storage
and the exposed slice type. The mutable form in
`tests/raw-unit/unsize_mutable.rs` is supported: its loan retains
the storage extent, and its native current and final readings share that extent
by construction. Callers do not supply a length-preservation clause. This does
not permit moving or replacing an unsized slice as a whole, resizing through a
slice reference, or borrowing a computed slice value mutably. Core and Rust
source-verification regressions cover these distinctions; the differential gate
requires every function in `unsize_mutable` and `slice_extent` to execute
conclusively. The compiler-agreement assumption remains explicit in
`doc/status/assumptions.md`.
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
continuation; validation, preparation, and the VerexSL `drop(place)` baseline
all check that destruction is not silently erased. Cleanup edges and effectful
drop glue remain explicit M4 boundaries. The Rust property `struct.destructor`
states of an ADT that it or an ADT its fields reach has a `Drop` impl, read from
the trait impls of each crate the unit reaches; a field's type parameter counts
for an ADT with visible fields only where it is instantiated, and for an opaque
one always (`tests/raw-unit/destructors.rs`).
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
The `tests/raw-unit` corpus contains positive fixtures admitted by the default
validation and roundtrip suites. `tests/fixtures` also contains focused positive
probes alongside rejection/API probes: `partial_move_tag` exercises post-move tag
reads outside the standard Rust renderer, `associated_aliases` checks dependency
signature shapes, and `evidence_composition` checks dictionary assembly. Native
nested slice redirection uses the shared `raw-unit/slice_extent` fixture.

`tests/fixtures/corpus.rs` covers scalar arithmetic, an enum switch, a loop,
direct calls, an ordinary borrow, explicit drop and cleanup edges, and a raw
pointer operation. `tests/fixtures/unsupported.rs` checks that inline assembly
is visible as an explicit unsupported MIR terminator. Run the pinned API and
corpus checks with:

```bash
cargo test --features rustc-public
```

The source-to-VerexSL E2E suite owns concrete error baselines for generic
trait code, raw pointers, inline assembly, and a borrow-check-invalid overlapping
mutable borrow. Those baselines retain the full normalized exporter diagnostic
and verify that failure leaves no partial JSON artifact. The exporter-local
probe tests still check that generic bodies, raw pointers in the capability
corpus, and inline assembly are observed before mapping. The compact
repeated-array fixture remains a positive canonical export case.

The design now explicitly selects the generic `optimized_mir` returned by the
pinned Rustc Public API. Plain lifetime/type-generic declarations and local
direct generic calls retain one parametric body; `generic_identity.rs` checks
validation, execution, VerexSL, and canonical-Rust re-import. The remaining
public API gaps are covered by `driver/polyfill.rs`: item predicates (including
parent identities and higher-ranked binders), caller-aware instance selection,
const-parameter types, promoted evaluation, and macro call sites. The bridge also
preserves early-bound signature lifetimes and queries declared Copy bounds,
including supertraits. Only this module uses compiler queries and conversion;
`mapper.rs` and `probe.rs` use public types. Import evidence names the polyfill.
The mapper exports Copy abilities, region outlives predicates, and the static
method-evidence slice described below. General trait predicates still need M2's
checked evidence.
Retire each wrapper when the pinned public API supplies its replacement.

The M0 probe deliberately does **not** serialize a substitute JSON format. The
detached writer implements the versioned `RawUnit` JSON spelling owned by
`verex-ir` directly. The mapper is CFG-driven rather than fixture-shaped: it
interns compilation-unit scalar and nominal types and names, remaps
declaration-local MIR locals, shares expression/place arenas across functions,
preserves explicit copy/move operations and core nominal constructors, and records source ranges for
declarations, locals, statements, and terminators. Ordinary scalar references
use core VIR reference types, lifetimes, place borrows, and dereference
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
cd verex-rust
lake exe verex-rust -- import-file rust-exporter/tests/raw-unit/basic.rs \
  --output .lake/verex-rust/basic.raw.json
```

The command builds this exporter with its pinned local toolchain, supplies the
matching sysroot library path, validates the detached RawUnit through the Rust
profile, and only then writes the artifact. Repeating the command revalidates
and reuses an artifact whose source, exporter, toolchain, profile, target, and
rustc-argument key still matches.

`VEREX_RUST_EXPORTER` selects an explicit development binary and
`VEREX_RUST_SYSROOT` selects its paired sysroot. This file mode is not yet the
normal-user prebuilt toolchain distribution promised by M1.5.

The command also imports a selected Cargo library package:

```bash
lake exe verex-rust -- import-crate Tests/CargoFixture/Cargo.toml \
  --package verex-rust-cargo-fixture \
  --features extra --no-default-features \
  --output .lake/verex-rust/cargo-fixture.raw.json
```

`import-crate` runs locked Cargo metadata and a locked Cargo check under the
managed toolchain. The exporter acts as `RUSTC_WRAPPER`: dependency crates are
passed to Cargo's matching rustc and produce ordinary metadata, while the
explicit primary library target alone is extracted and stopped before
codegen. The cache binds the canonical manifest, Cargo metadata and lockfile,
all local path-package files, package/features/target selection, exporter,
toolchain, and semantic profile. Each uncached input gets a content-addressed
Cargo target directory, avoiding both destructive `cargo clean` calls and a
stale Cargo-fresh root that would skip extraction. `VEREX_RUST_CACHE` may
override the managed cache root.

## Static method evidence

`tests/evidence-unit/` checks declared method dictionaries with strict
differential execution. Each dictionary is an ordinary function parameter,
marked by `method_evidence = <parameter index>`. Its name starts with
`__trait_<Trait>_<method>`; numeric suffixes disambiguate repeated bounds and
source locals. Callers select implementations using compiler identities, and
generic forwarding passes the dictionary without specializing the helper body.

Higher-order invocation and its behavioral predicates require mutable
referents to contain no references (including references passed as tuple
components). Nested mutable-reference arguments or results such as
`&mut &mut [T]` remain outside this higher-order subset; the shared loan
correspondence used by direct calls does not broaden that boundary.

`FnMut` evidence retains a mutable closure receiver and spreads its argument
tuple. Results may be generic or aggregate types admitted by the Rust profile.
Closures inside bounded generic functions capture their inherited dictionaries
as additional arguments. A non-mutating `Fn` closure gets a checked
`__fnmut_adapter_*` function that reborrows its receiver as shared.
Declared `Fn` bounds instead supply a dictionary taking `&F` and invoke the
shared closure body directly. `evidence-unit/static_fn` covers generic forwarding,
arguments, generic and tuple results, multiple bounds, and repeated unit calls.
Generic forwarding from `Fn` to `FnMut` uses a checked adapter that captures
that shared dictionary and reborrows its mutable receiver. The
`shared_method_adapter` attribute identifies the captured dictionary parameter;
Rust source verification generates and proves receiver-preservation and
behavioral specifications for these borrowing adapters. `closure_method_adapter`
identifies the concrete closure target, including consuming adapters described below. Users specify the source closure's
`call` method; they do not write specifications naming generated adapters.
Generated-contract diagnostics point into the rendered verification file. `evidence-unit/fn_forward` exercises this forwarding
with arguments, multiple bounds, repeated calls, generic/tuple results, and unit
results. Declared `FnOnce` bounds supply a dictionary taking `F` by value;
`evidence-unit/static_fnonce` covers consuming closure bodies, generic forwarding,
non-`Copy` captures, scalar/tuple results, and unit calls. Concrete `Fn` and
`FnMut` closures use checked consuming adapters that borrow the receiver for
the original call and explicitly drop it afterward. Their generated contracts
retain the invocation, existentially including the final mutable receiver.
`evidence-unit/fnonce_adapters` checks these conversions. Source verification
also accepts the generic result `R = ()` in opaque `mutable_call` and `once`
helpers: it independently checks the helper's contract at the Unit frame and
keeps the opaque call boundary. The source proof fixture no longer needs
unit-only helpers. This specialized proof path requires reference-free type
arguments and acyclic bodies without direct function calls, loops, body
specifications or assumptions, or model state; borrowed receiver parameters
remain supported. Native contracts that do not transfer to the Unit reading
cannot use this body-checking path. See the
[generic callable Unit results](../../doc/designs/rust-verification.md#generic-callable-unit-results).

Generic borrowed dictionaries can also supply consuming evidence when the
validator checks that the receiver borrow cannot escape through results or
mutable writes. `evidence-unit/fnonce_forward` covers these adapters and their
forwarding paths; the runtime and proof contract enforce the same checked
dictionary requirement. Generic Unit-result forwarding through direct-call
chains remains outside the bounded specialization path described above.
The `fixtures/evidence_composition` fixture covers these combinations through native
execution, interpreter expectations, and source round-trip.
`raw-unit/evidence_arguments` additionally exercises arguments, receiver adapters,
multiple bounds, and struct results in the differential suite. The evaluator also
checks generic closure construction and captured dictionary invocations, retaining runtime type frames and mutable arguments' prophecies.
Unsupported captures or incoherent frames remain inconclusive; the strict
evidence suite requires every input to be conclusive.

Export, execution, and verification are separate gates. A dictionary's
`MethodContract` requires a defined, non-panicking, memory-preserving invocation;
its target's proof must establish that obligation. Opaque generic contract
transfer still rejects types containing mutable loans. Tuple-returning callbacks
are covered by `raw-unit/tuple_callbacks`, including
generic results instantiated by tuples, one-element tuples, and nested tuples.
Tuples containing borrowed references retain the aggregate-reference restrictions.

`function.shapeOnly` marks dependency declarations whose contracts are supplied
only at modeled shapes. It does not grant unchecked trait evidence or make an
unmodeled implementation executable.
