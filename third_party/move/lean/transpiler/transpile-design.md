# Move-to-Leaner transpilation via XAST

Status: living design document; M1–M2 implemented, M3 underway (see
*Implementation status*)

> **Future architecture.** This document records the current XAST-to-Leaner
> implementation and its milestone status. The independent
> [unified LIR design](../designs/lir-design.md) defines the target shared semantic
> boundary for Move and the upcoming Rust MIR importer: XAST becomes a
> temporary Move-frontend transport, and validation, capability reporting, and
> source emission consume profile-aware LIR.
> Existing milestone statuses below describe the current implementation, not
> the final ownership boundary.

This document designs the reverse direction of the Leaner pipeline: importing
existing Move source — up to the whole Aptos framework — into Lean.  The
compiler-v2 typed AST, including specifications, is exported through a new
**XAST** exchange format and compiled by a Lean-side transpiler into the
*textual* Leaner Move surface language defined in
[`leaner-move.md`](../move/Move/leaner-move.md).  The output is ordinary `.lean`
source: `module` blocks with `struct`/`enum` declarations, `fun`s, and
`spec`s that elaborate, lower, and verify like hand-authored Leaner code.

Related documents: [`leaner-move.md`](../move/Move/leaner-move.md) (the
language), [`project-plan.md`](../move/Move/project-plan.md) (what the surface
and the verifier do not yet handle), [`design-plan.md`](../move/Move/design-plan.md)
(Leaner-to-bytecode, XIR), [`verification-design.md`](../move/Move/verification-design.md)
(source contracts), [`invariant-design.md`](../move/Move/invariant-design.md)
(data and global invariants), [`loop-design.md`](../move/Move/loop-design.md)
(structured loops), and [`lir-design.md`](../designs/lir-design.md) (the future
profile-aware intermediate representation shared with Rust MIR).

## Implementation status

M1–M2 are implemented and tested; M3 is underway:

- **Producer.** `move-model-exchange::ast` (schema, version 4, with interned
  tables), the producer `aptos-move/cli/src/exchange/ast.rs`, the single-file
  entry `move_file_to_ast` (`source.rs`, checker and rewriters only), and
  `aptos move exchange --format ast [--include-deps]`; baselines under
  `move-model/exchange/tests/ast_sources` (`ast_testsuite`, `UB=1`).
- **Consumer.** The `transpiler` package: `Transpiler.{Xast, Decode, Names,
  Effects, Order, Comments, Print, Cli, Driver}`, `lake exe transpile` (Move
  package, Move files, or an existing export as input — the CLI export is an
  internal step), and its tests — decoder, byte-exact printer baselines, and
  the elaboration gate: each `Transpiler/Tests/Programs/<name>.move` is
  exported by the CLI at test time and compared with the generated
  `<Name>.lean` beside it, which is built against `Move`.
- **Leaner.** E2/E3: the `pragma aborts_if_is_partial | aborts_if_is_strict`
  clause and the two-directional `aborts_if` semantics (`Contract.mustAbort`,
  `Move/Verify/{Contract,Syntax,Tactics}.lean`, tested by
  `Tests/Verification/AbortDirections.lean`); E7: `Bool`, `Address`, and
  vector-literal named constants fold in the compiler
  (`Move/Compiler/Normalize.lean`); E20: the loose frame `modifies …, *`
  (`Tests/Verification/LooseFrame.lean`); `assert!` and `x ← e` accepted by
  the verification translator; `&mut r.f.g` through a reference variable.
  E1 is confirmed: a module of an aliased package transpiles under its root
  namespace (`AptosFramework/Counter.lean`); receiver functions print in
  prefix form (the nested-namespace receiver tier is not needed for
  elaboration).
- **Verification.** `lake exe transpile --verify` appends `verify f` to
  every `spec f` of a function with a body (a native's contract is assumed);
  the `Tests/Programs` baselines are generated with it, so every transpiled
  spec there (`account`, `basic_coin`, `constants`, `enums`, `generics`,
  `loops`, the OrderedMap core, `aptos_framework::counter`) is proved
  automatically as part of the elaboration gate.  The stdlib baselines are
  generated without it (loop invariants and the nonlinear fixed-point
  arithmetic are beyond the automatic prover for now).  The acceptance
  programs transpile to Leaner source that elaborates and whose specs are
  accepted.
- **Specification functions.** A Move function applied in a specification
  denotes its specification version, which Leaner derives from the
  function's retained body at its declaration (`f.specFun`, the pure
  reading: `Move.Verify.Source.pureReadingTerm`; `#derive_move_spec_function`),
  so the transpiler emits nothing for the compiler's companions `$f`
  beyond the call `f args`; `spec fun f … := body` declares a version by hand
  where none is derived (natives: the intrinsic models) or a standalone
  specification function, stateful when it reads global memory
  (`Tests/Verification/SpecFunctions.lean`, `Tests/Negative/SpecFunctions.lean`).
  The producer runs the spec rewriter, so calls arrive resolved and checked.

## Goals

- Transpile **full Move without constructed or stored function values** — all
  types, expressions, statements, and declarations of compiler-v2 Move;
  retained inline helpers may have function-typed parameters and invoke them,
  while lambdas, closures/captures, storage, and general dynamic dispatch
  remain excluded — into readable Leaner source. Function-value behavior
  summaries are transported by XAST v4 and lower to the target language's
  closed behavior-operation family.
  Signed integers (`i8`–`i256`, Move 2.3) are part of the language on both
  sides and are transported like the unsigned widths.
- Transpile **specifications**: `requires`, `ensures`, `aborts_if` (with
  codes), `modifies`, loop invariants, spec functions, and spec `let`,
  preserving the production prover's spec semantics.
- Represent the **semantics-bearing core pragmas** (`aborts_if_is_partial`,
  `aborts_if_is_strict`) in Leaner Move; drop all others.  `pragma opaque`
  is dropped by decision: Leaner verification is modular by construction.
- Preserve **readability**: receiver/dot notation for `self` functions and
  standard-library operations, original names, doc comments and ordinary
  comments, source structure (loops stay loops, matches stay matches), and a
  layout that reads like hand-written Leaner.
- Scale to the target corpus: **every module of the Aptos framework**
  (move-stdlib, aptos-stdlib, aptos-framework, aptos-token,
  aptos-token-objects; 167 module declarations, ~78 k implementation lines,
  ~19 k spec lines).

## Non-goals

- Constructed and stored function values, closures/captures, and
  dispatchable-fungible-asset `FunctionInfo` machinery. XAST and Leaner do
  support the narrow compile-time-only slice
  needed by retained `inline fun`s: function-typed parameters and their
  invocations. Inlining removes almost all framework lambda usage (339 lambda
  sites feed `inline fun`s); construction that remains is rejected explicitly.
- Round-tripping transpiled modules back to bytecode through the Leaner
  executable pipeline.  Transpiled output targets *source verification*;
  re-export through XIR is at most a differential-testing aid for the
  executable subset.
- Proving the transpiler correct.  Generated Leaner source is re-elaborated,
  re-type-checked, and re-verified by Lean; the transpiler itself is trusted
  only for *fidelity of translation*, which testing addresses.
- A general Move parser in Lean.  XAST is produced by compiler v2; Lean never
  parses `.move` text.

## Architecture

```text
Move package sources (.move + .spec.move)
        |
  compiler v2: parse, expand, type-check, build GlobalEnv
        |
  env pipeline: checks, match transforms, INLINING,
  acquires check, lambda lifting, spec checker, spec REWRITER
        |                                   (stop before optimizations)
  XAST export (Rust)  --- schema: move-model/exchange (ast module)
        |
  one <module>.xast.json per module  (transient: the transpiler runs the
        |                             CLI itself; the .move sources are the
        |                             inputs of record)
  Lean transpiler (lake exe transpile, new package `transpiler`)
        |-- decode JSON into Transpiler.Xast.*  (mirror data structures)
        |-- feature check, name legalization, policy (pragmas, dot notation)
        |-- print textual Leaner Move
        |
  generated .lean files (one Move module per file)
        |
  ordinary lake build: Lean elaboration re-checks everything;
  `spec`/`verify` provide source verification
```

The split of responsibilities is a settled principle:

> **XAST is a faithful, policy-free snapshot of the move-model AST.  All
> transpilation policy — dropping pragmas, name mangling, dot notation,
> feature gating — lives in the Lean consumer.**

Tightening or loosening policy therefore never changes the wire format, and
the Rust side stays a dumb, complete exporter.

## Export stage selection

The exporter runs on the `GlobalEnv` **after** compiler v2's
`env_check_and_transform_pipeline` (through the spec checker and spec
rewriter — the producer asks for the `SPEC_REWRITE` experiment, which the
checker-and-rewriters entry does not run by default) and **before**
`env_optimization_pipeline` (whose AST simplifier folds constants and
reshapes code, hurting source fidelity).  Consequences:

- **Inlining has happened.**  Lambdas passed to `inline fun`s are gone; the
  callers contain the expanded bodies.  Inline function declarations are
  deleted from the model, except verify-mode retentions
  (`FunctionEnv::is_inline_verified`, `is_inline_opaque_retained` in
  `move-model/src/model.rs` — inline functions with explicit specs and no
  function-typed parameters).  Retained ones are first-order and transpile as
  ordinary functions; their calls remain calls.
- **Schemas and `apply` are already expanded.**  The model reduces the 148
  framework schema declarations and 510 `include` sites (with argument
  substitution and conditional includes) into flat per-function `Condition`
  lists during model building.  XAST contains only expanded conditions;
  schemas need no representation.
- **Match transforms have normalized patterns**, the acquires check has run,
  and specs are checked and rewritten — the same state the production prover
  consumes.
- The model is built in **verify mode** (like the existing exchange package
  mode), so spec-relevant retention rules apply.

The existing `aptos move exchange` package mode builds through
`build_model(…, verify_mode = true, with_bytecode = true)`, which reaches
`run_move_compiler_to_model` (`move-compiler-v2/src/lib.rs`).  That entry
runs `run_checker_and_rewriters` and then **also** the optimization pipeline
and stackless generation.  The XAST producer therefore needs a model-only
entry that stops after `run_checker_and_rewriters`; besides fidelity this
avoids paying for bytecode generation across the framework.

The model's proof-facing material — `Spec.proof` blocks and lemma functions
(`FunctionEnv::is_lemma`) — are Boogie proof hints and are not exported;
Lean proofs are written in Lean.

## The XAST exchange format

### Principles

- **Name-based, not positional.**  XIR uses dense per-module indices because
  its consumer is a semantic IR.  A source-level format regenerating source
  names gains nothing from indices and loses debuggability.  All references
  are by name; cross-module references are fully qualified
  `{address, address_alias, module, name}`.  Model ids (`ModuleId`,
  `NodeId`, …) are unstable across builds and never appear on the wire.
- **Fully typed, over interned tables.**  move-model keeps node types,
  locations, and instantiations in side tables keyed by unstable `NodeId`s;
  XAST records them per node — as indices into top-level tables of the export
  unit (`types`, `locs`, `modules`, `names`), so a per-node type and location
  cost a few bytes instead of a tree, and the document size stays linear in
  the source.  Type table entries reference their components by index
  (interned bottom-up, so components always precede).  The Lean decoder
  resolves the tables, so the consumer works on the logical tree.  The
  `SurfaceSyntax` side table (`ReceiverCall`, `IndexNotation`) is emitted as
  per-node metadata.
- **Source facts the model drops.**  A `value` node carries the module
  constant the source named at it (`constant`), since compiler v2 folds
  constants; a comment carries whether it stands on a line of its own
  (`own_line`), which decides leading versus trailing attachment.
- **Un-overloaded.**  `Condition::additional_exps` is kind-dependent in the
  model; XAST gives each condition kind explicit named fields
  (e.g. `aborts_if: {cond, code?}`).
- **Normalized values.**  The model's redundant constant encodings
  (`ByteArray` vs `Vector(Number)` vs `AddressArray`) are normalized to one
  vector-of-values form; numbers travel as decimal strings (they are
  `BigInt` in the model), addresses as `0x`-hex.
- **Out-only.**  Unlike the bidirectional XIR wrapper, XAST has no loader
  back into compiler v2.
- **Versioned.**  `XAST_SCHEMA = "move-xast-module"`, `XAST_VERSION = 4`,
  checked by the consumer; same serde wire conventions as the existing
  exchange format (externally tagged snake_case enums, documented version
  history, `XIR_SCHEMA`/`XIR_VERSION` naming pattern).

The schema lives in the `move-model-exchange` crate
(`third_party/move/move-model/exchange`) as a new `ast` module, keeping that
crate free of a move-model dependency (it depends on serde only; mirror types
with serde derives; the producer converts).  The producer lives beside the
existing one in `aptos-move/cli/src/exchange/` (new `ast.rs` + `ast_spec.rs`;
`model_spec.rs`, which already converts model `Spec`s for the XIR path, is
the closest template).  The same change corrects the exchange crate's
version-history note that claims Move source has no signed integer types —
it has had them since language version 2.3.

### Top-level schema (sketch)

```rust
pub struct XastModule {
    pub schema: String,            // "move-xast-module"
    pub version: u64,              // 2
    pub address: String,           // "0x1"
    pub address_alias: Option<String>, // "aptos_framework"
    pub name: String,              // "coin"
    pub doc: String,
    pub loc: LocId,
    pub named_addresses: Vec<NamedAddress>, // the package's alias table
    pub friends: Vec<ModuleId>,
    pub pragmas: Vec<Pragma>,      // module-level spec properties, raw
    pub constants: Vec<Constant>,  // name, type, value, doc
    pub structs: Vec<Struct>,      // incl. enums; see below
    pub functions: Vec<Function>,
    pub spec_funs: Vec<SpecFun>,
    pub spec_vars: Vec<SpecVar>,   // ghost variables
    pub invariants: Vec<Invariant>,// module-level global invariants + axioms
    pub comments: Vec<Comment>,    // non-doc comments: loc, text, own_line
    // interned tables
    pub sources: Vec<String>,      // file table for locations
    pub locs: Vec<Loc>,            // LocId -> [file, start, end]
    pub types: Vec<Type>,          // TypeId; components by TypeId
    pub modules: Vec<ModuleRef>,   // ModuleId
    pub names: Vec<QualifiedName>, // NameId: (ModuleId, name)
}
```

- `Comment`: the ordinary `//` and `/* … */` comments of the module's source
  files with their spans.  compiler v2's lexer keeps only doc comments (it
  matches `///` and `/** */` to the following item, which is what the model's
  `get_doc` accessors return) and discards the rest, so the producer scans
  the source text for them — raw source data, not policy, like the file
  table itself.  Doc comments travel on the declarations they document.

- `Struct`: name, doc, abilities, type parameters (name, ability constraints,
  phantom), attributes (`event`, `resource_group`, …, as raw strings),
  `is_native`, fields (name, type, doc), optional variants (enums), and the
  struct spec (data invariants, pragmas).  An intrinsic struct additionally
  carries the model builder's validated intrinsic kind and its resolved,
  qualified Move-function and specification-function role bindings; the
  builder consumes those role properties from the ordinary pragma bag.
- `Function`: name, doc, visibility (`private | public | friend | package`),
  `is_entry`, `kind` (`regular | inline_retained | native`), attributes,
  type parameters, parameters (name, type — the first parameter's name `self`
  marks Move receiver style; `FunctionEnv::is_receiver_function` is the
  model's predicate), result type (tuple for multiple returns), resolved
  pragmas, spec (see below), and `body: Option<Exp>`.
- `SpecFun`: name, type parameters, parameters, result type,
  `body: Option<Exp>` (`None` for uninterpreted/native spec funs), the
  model's `uninterpreted`/`is_native`/`is_move_fun`/`uses_old` flags, and
  the conditions attached to uninterpreted ones.
- `Invariant`: kind (`global | global_update | axiom`), type parameters,
  condition expression, properties.
- Locations: every declaration and statement-level node carries
  `[file_index, start, end]`; expression-level locations are optional
  (exporter flag) to bound file size.

### Expressions

`Exp` mirrors `ExpData` minus the rejected variants, with the node type
inlined:

| XAST node | From model | Notes |
|---|---|---|
| `value` | `Value` | normalized constants |
| `local` | `LocalVar` | by name |
| `param` | `Temporary` | by parameter index; printer maps to the name |
| `call` | `Call` | operation + args + instantiation; `surface: receiver_call \| index_notation?` |
| `invoke` | `Invoke` | function-valued expression + arguments; retained inline helpers |
| `block` | `Block` | pattern, optional binding, body |
| `if` | `IfElse` | |
| `match` | `Match` | arms: pattern, optional guard, body |
| `sequence` | `Sequence` | |
| `loop` | `Loop` | with `loop_cont {nest, is_continue}` for break/continue |
| `return` | `Return` | |
| `assign` | `Assign` | pattern ← value |
| `mutate` | `Mutate` | `*lhs = rhs` |
| `spec_block` | `SpecBlock` | in-body spec: loop invariants, assert/assume |
| `quant` | `Quant` | kind, typed binders with ranges, triggers, where, body |

Rejected by the current Rust producer/schema (hard error naming the function):
`Invalid`, `Lambda`, `Operation::Closure`, `Type::Error`, and `Type::Var`.
`Operation::Behavior` carries its closed kind and pre/post memory range through
the Rust producer, Lean mirror, decoder, and XAST-to-LIR adapter. `Type::Fun`
and `Invoke` are carried for retained inline helpers. Every
`PrimitiveType` — `Bool`, `U8`–`U256`, `I8`–`I256`, `Address`, `Signer`,
and the spec-only `Num`, `Range`, `EventStore` — is transported.

The `Operation` enum is mirrored one-to-one (structured payloads with
qualified names instead of ids): `move_function`, `pack {struct, variant?}`,
`select {struct, field}`, `select_variants`, `test_variants`, `tuple`,
`borrow {kind}`, `borrow_global {kind}`, `deref`, `move_to`, `move_from`,
`exists {label?}`, `freeze`, `abort`, `vector`, `cast`, arithmetic /
bitwise / shift / comparison / boolean operators, and the spec-only
operations (`spec_function`, `global {label?}`, `old`, `result {i}`,
`len`, `index`, `slice`, `range`, `update_field`, `update_vec`,
`concat_vec`, `contains_vec`, `in_range_*`, `type_domain`, `implies`,
`iff`, `max_u*`, `well_formed`, the state-snapshot anchors, …).  Memory
labels are function-scoped naturals exactly as in the model, with label
names carried alongside.

`Exp` values are interned (DAG) in the model; XAST encodes the tree and
accepts duplication — framework functions are small after inlining, and the
per-module JSON stays manageable.

### Specifications

```rust
pub struct Spec {
    pub pragmas: Vec<Pragma>,       // resolved: module→function inheritance applied
    pub conditions: Vec<Condition>,
    pub frame: Option<Frame>,       // Spec::frame_spec: modifies/reads targets, wildcards
}
pub struct Condition {
    pub kind: ConditionKind,        // full mirror of the model's 19 kinds
    pub properties: Vec<Property>,  // "abstract", "concrete", "injected", ...
    pub exp: Exp,
    // kind-specific named payloads instead of additional_exps:
    pub abort_code: Option<Exp>,    // AbortsIf `with`
    // ...
}
```

All 19 `ConditionKind`s (`LetPost`, `LetPre`, `Assert`, `Assume`,
`Decreases`, `AbortsIf`, `AbortsWith`, `SucceedsIf`, `Emits`, `Ensures`,
`Requires`, `StructInvariant`, `FunctionInvariant`, `LoopInvariant`,
`GlobalInvariant`, `GlobalInvariantUpdate`, `SchemaInvariant`, `Axiom`,
`Update`) are transportable so the schema never lags the model; the Lean
consumer accepts the subset it implements and reports the rest.  Framework
measurements say the initial consumer subset can be small: `AbortsWith`,
`Update`, and `TRACE` have zero framework occurrences, and `Emits`,
`Decreases`, and choose-quantifiers only a handful each.

`modifies` is not a condition in the model: it lives in `Spec::frame_spec`
(`modifies_targets`, `reads_targets`, `modifies_all`/`reads_all`) and is
exported as the `frame` field.  The same structure exists on spec functions.

Pragmas are exported twice: raw property bags per declaration, and the
**resolved** view per function (the model's accessors apply
module-to-function inheritance; a consumer reading raw bags alone would
lose inherited values).

## Producer CLI

Extend the existing command (`ExchangePackage` in `aptos-move/cli/src/commands.rs`,
today with `--export-dir`, `--masm-file`, `--move-file`, `--out-file`):

```bash
aptos move exchange --package-dir <pkg> --format ast [--include-deps]
```

- `--format` selects `xir` (default, today's stackless export) or `ast`.
- Package mode writes one `<address>_<module>.xast.json` per target module
  (same atomic-write, stale-file-cleanup, and naming conventions as the
  existing exporter, default directory `<pkg>/exchange-json`);
  `--include-deps` also exports dependency modules so a framework package
  transpiles closed under imports.
- `--move-file`/`--out-file` single-file mode is kept for tests and for a
  possible future `moveAst%` Lean elaborator.

The exporter walks `ModuleEnv`/`FunctionEnv`/`StructEnv` exactly as the
model exposes them (`get_def()`, `get_spec()`, `get_named_constants()`,
`get_spec_funs()`, `get_global_invariants_by_module`, …), skipping
test-only declarations, compiler-generated ghost memory
(`is_ghost_memory()`), and lemma functions.

## Lean consumer

A new Lake package beside `move` and `move-model`, following the same
naming convention (package and executable lowercase, library PascalCase):

```text
third_party/move/lean/transpiler/     package "transpiler" -> library Transpiler
  transpile-design.md  this document
  Transpiler/
    Xast.lean        mirror data structures (the logical tree; the wire
                     format's interned tables are resolved by the decoder)
    Decode.lean      JSON decoder: tables, version check, structured errors
    Names.lean       name legalization, namespaces, the stdlib mapping table
    Effects.lean     `Action`/global-write inference over the package call graph
    Order.lean       declaration ordering (Lean elaborates in order; Move does not care)
    Comments.lean    span-based re-attachment of source comments
    Print.lean       `Std.Format`-based Leaner Move pretty printer; the
                     feature gate lives here too (an unsupported construct
                     raises, the declaration is emitted commented out with
                     the reason and recorded in the report)
    Cli.lean         runs the exchange frontend with `--format ast` on Move
                     files or a package and decodes the result (located as the
                     `MoveModel` frontend does: `APTOS_MOVE_CLI`, then
                     `APTOS_CLI`, checkout binary, `PATH`)
    Driver.lean      batch driver: transpile a decoded package, write, report
    Tests.lean       decoder test, printer baselines, elaboration gate
    Tests/Programs/  `<name>.move` beside its generated `<Name>.lean` (the
                     baseline, built against `Move`)
  Main.lean          `lake exe transpile <package-dir> | --move-file … |
                     --xast <dir> -o <out-dir>`
```

The library itself depends only on Lean core (`Lean.Data.Json`,
`Std.Format`): it manipulates text and never elaborates Leaner.  The package
requires `move` for its test suite — generated showcase modules are built
against `Move` as the elaboration gate — and so that a checkout builds the
tool and its targets together; `Transpiler.Names` mirrors Leaner's primitive
names as data, not as imports.

The driver is a batch tool: it takes Move input (a package directory, or
self-contained module files; an existing XAST export is also accepted), runs
the CLI export internally, orders modules by dependency, and writes one
`.lean` file per module plus a transpilation report.  XAST documents are
never stored; the `.move` sources are the inputs of record.  The driver
performs no elaboration itself; `lake build` on the output is the check.

### Module and name mapping

- **Module identity is Leaner's `module M at address where`.**  The Move
  module name is the Lean identifier verbatim and the address is a literal or
  a registered alias, so `aptos_framework::coin` becomes

  ```lean
  module coin at aptos_framework where
  ```

  Module names are therefore *not* re-cased: the Move identity must be exact
  for linkage with on-chain modules, `module` equates the Lean namespace
  component with the Move name, and `coin.transfer` then reads exactly like
  `coin::transfer`.  Struct, function, field, and parameter names are kept as
  written (Move's casing rules coincide with Leaner's).
- **Addresses.**  `Move.ConventionalAddresses` already registers `std`,
  `aptos_std`, `aptos_framework`, `aptos_token`, `aptos_token_objects`, and
  the other conventional aliases, so framework modules need no declarations.
  For any other package the transpiler emits `address_alias name = 0x…`
  declarations from the exported `named_addresses` table.
- **Root namespaces from the named address.**  Module identity in Move is
  (address, name) and the framework has genuine name collisions
  (`0x3::token` and `0x4::token`), so each generated module is wrapped in a
  Lean namespace derived from the alias used in its declaration:
  `Std.vector`, `AptosStd.table`, `AptosFramework.coin`,
  `AptosTokenObjects.token`.  `module` registers its identity under the
  *current* Lean namespace (`Move.registerModuleNamespace`) and expands to
  `namespace M … end M`, so an enclosing `namespace AptosFramework` is
  expected to compose; M1 confirms this (E1).
- **Collisions and keywords** are escaped with Lean's guillemets
  (`«end»`, `«Sort»`, `«open»`), which preserve the exact name and work
  with dot notation (`x.«end»`).  The legalizer keeps a fixed reserved-word
  list and disambiguates duplicates deterministically.
- **Friends** print as Leaner friend items (`friend aptos_framework::coin;`),
  which already exist; Leaner checks that a friend shares the module's
  address, as Move does.
- One Move module per generated file (matching Leaner's one-export-per-file
  rule); the file path follows the namespace with Lake's PascalCase file
  convention (`AptosFramework/Coin.lean` declares `AptosFramework.coin`), and
  imports of transpiled dependencies are ordinary Lean `import`s.
- Each generated file starts with a `-- generated by move-to-lean from
  <source>` header.

### Declaration order and layout

Move accepts declarations in any order; a Leaner `module` block is
elaborated by Lean **in order**, so a `fun` must follow the `fun`s it calls,
a `spec` the `def`s (spec functions) and types it mentions, and a data
invariant the declarations its predicate uses.  `Order.lean` therefore
emits a dependency-respecting order and keeps Move source order among
independent declarations:

1. constants, then structs and enums in dependency order, each followed by
   its data invariant (`spec T where invariant …`);
2. spec functions as Lean-only `def`s, in dependency order, under a
   `/-! ## Model -/` section per Leaner's file layout convention;
3. functions in call-graph order, each `spec` directly under its `fun`;
   recursive functions are `partial`, and a strongly connected component
   prints as one `mutual … end` block;
4. global invariants after the families they name;
5. no `/-! ## Proofs -/` or `/-! ## Tests -/` section — `verify` commands and
   tests are added by proof campaigns, never generated.

Within that order the printer is a `Std.Format` pretty printer (groups,
nesting, and soft breaks at the project's line width), not string
concatenation: long argument lists, `spec` clauses, and `match` arms wrap
the way the checked-in Leaner sources do, indentation is two spaces, and the
output is deterministic for identical input so framework transpiles diff
cleanly.  Blank lines separate items; a `spec` hugs its `fun`.

### Comments

Two kinds of comment survive, through two channels:

- **Doc comments** (`///`, `/** … */`) on modules, structs, fields,
  constants, functions, and spec functions travel on the XAST declarations
  (the model's `get_doc`) and print as Lean doc comments (`/-- … -/`,
  `/-! … -/` for the module).
- **Ordinary comments** (`//`, `/* … */`) come from the `comments` table and
  are re-attached by span in `Comments.lean`: a comment is a *leading*
  comment of the first statement-level node (or declaration) whose span
  starts after it, or a *trailing* comment of the node whose span ends on
  the same line before it — the policy source formatters use.  Leading
  comments print on their own lines before the node, trailing ones after it
  on the same line, in Lean's `--` / `/- … -/` spelling.

A comment whose anchor does not survive the export — one inside an
`inline fun` body that was expanded into callers, inside a schema that was
expanded into conditions, or attached to a test-only declaration — is
dropped and counted in the transpilation report, so comment loss is visible
rather than silent.  Statement-level locations (the default) anchor every
comment that sits between statements; a comment inside a single expression
needs the full-span exporter flag.

### Dot notation

Three tiers, the first two on by default:

1. **Move receiver functions.**  Any function whose first parameter is
   `self: T`, `self: &T`, or `self: &mut T` (Move 2 receiver style,
   necessarily declared in `T`'s defining module; 225 framework
   declarations) is emitted inside a nested `namespace T` within the module
   block, so `x.f args` works exactly where `x.f(args)` worked in Move.
   The model records per call whether the source used receiver syntax
   (`SurfaceSyntax::ReceiverCall`); XAST carries that bit and the printer
   reproduces the author's call style.  Requirement on Leaner (part of E1):
   a `fun` declared inside a nested namespace of a `module` is exported
   under its last name component (`T.f` is Move function `f`).
2. **Curated stdlib table.**  Standard functions that predate receiver
   syntax get dot notation via the mapping onto Leaner's primitives:
   `vector::length(&v)` → `v.length`, `vector::push_back(&mut v, x)` →
   `v.push x`, `vector::is_empty(&v)` → `v.isEmpty`, `vector::swap(&mut v,
   i, j)` → `v.swap i j`, `signer::address_of(s)` → `s.address`
   (`Ref.address`), etc.  The table decides which stdlib modules map onto
   Lean-native primitives (`std::vector` → `Move.Vector`, `std::signer` →
   `Ref.address`) instead of being transpiled; everything else in the
   standard library (`option`, `string`, `error`, …) is transpiled as an
   ordinary module.  Where Leaner's `spec` does not yet accept a receiver
   spelling (`v.get i`, `r.insert i e`, `r.remove i`), the printer emits the
   qualified `Move.Vector.get`/`insert`/`remove` form until E11 closes that
   gap — the choice is per operation, not per module.
3. **Heuristic fallback (printer option, off by default).**  Emit a
   non-`self` function into its first parameter's namespace when that type
   is declared in the same module.  Off by default so generated code stays
   predictable; Move's explicit `self` convention is the principled signal.

### Pragma policy

Per direction: **only the core abort-semantics pragmas are supported**;
everything else is dropped, each with a documented disposition and an entry
in the transpilation report.

| Pragma (framework count) | Disposition |
|---|---|
| `aborts_if_is_partial` (127) | **Leaner pragma clause** (implemented): the clauses interpret sufficiency only; completeness stays uninterpreted (see semantics below).  The printer also emits it for a pure function whose spec declares no abort clause and is not strict: that states the production default and selects Leaner's relational contract shape |
| `aborts_if_is_strict` (30) | **Leaner pragma clause** (implemented): an empty clause list means "never aborts" (module-level occurrences resolved per function at transpile time) |
| `opaque` (529) | **Leaner pragma clause** (implemented, E9): the function is summarized for its callers by its contract (`Contract.summary`); its body is still verified against the contract when the verifier's translator can follow it, and assumed with a warning otherwise.  The frame reading of `modifies` is closed on opaque functions, loose otherwise (see *Printer normalizations*) |
| `verify` (391) | no pragma needed: Leaner separates spec declaration from the `verify` command; the transpiler emits specs always and `verify` commands never (proof campaigns add them deliberately) |
| `intrinsic`, `intrinsic = map …` (131) | schema v4 transports the model builder's resolved role mapping, but the transpiler currently rejects the owning module with a fatal E16-suspended diagnostic; no attributes or partial intrinsic model are emitted |
| `timeout`, `unroll`, `seed`, `verify_duration_estimate`, `bv`, `bv_ret` | Boogie/SMT tuning; meaningless in Lean; dropped |
| `disable_invariants_in_body` (5), `delegate_invariants_to_caller` (2), `emits_is_*` (0), others | dropped with a report entry; Leaner's global invariants are re-established at every write, so a suspended-invariant region has no counterpart yet (E18) |

XAST transports all pragmas regardless, so changing this policy later needs
no schema change.

### Printer normalizations

- **Effects.**  Leaner marks effectful functions with an `Action` result;
  the model has no such distinction.  `Effects.lean` computes it as a fixed
  point over the package call graph (imports included): a function is printed
  as `Action` iff its body uses an `Action`-requiring primitive — global
  storage (`existsAt`, `moveTo`, `moveFrom`, `&R[a]`, `&mut R[a]`), a borrow,
  dereference, reference write, or `freeze`, vector mutation through a
  reference, or an explicit `abort`/`assert!` — or calls an `Action`
  function; entry functions are printed as `Action`.  Effectful bodies print as
  `do` blocks; pure bodies as terms (Leaner wraps pure `do` in `Id.run`
  automatically).
- Reads through references in value position (`*balance >= amount`) are
  hoisted into `let x ← *r` bindings in evaluation order, the form Leaner's
  verifier translates; a write of the shape `*r = *r ± e` prints as Leaner's
  sequenced `r := *r ± e`.
- `assert!(c, e)` arrives from the AST as a conditional `abort` and prints as
  Leaner's `assert!(c, e)`; a negated condition prints with the comparison
  flipped (`assert!(current >= amount, E)`).  The verification translator
  accepts the macro (desugared like `Move.assert`, to `if c then pure () else
  abort e`).
- `Assign`/`Mutate` print as `let mut` reassignment resp. reference `:=`
  writes, using the type-directed forms Leaner already elaborates.
- Loops print as `while c do` / `loop` with `break`/`continue`
  (`loop_cont.nest > 0` becomes labeled `loop@l` with `break@l`/`continue@l`);
  Move's `for` loops are already desugared in the model.
- Tuple-typed lets, multi-return calls, and tuple results print as Lean
  products (`U64 × Bool`, `let (a, b) := f x`); a tuple assignment to
  existing locals prints as a product `let` followed by the reassignments.
- Guarded match arms (`MatchArm.condition`) print as Leaner's guarded arms
  (`| pat if c => e`); enum matches survive the export as matches.  Matches
  over primitive values (literal and range arms) do not: compiler v2's match
  transform lowers them to `if` chains before the export stage, which print
  as `if … then … else if …` — faithful, but a readability loss to weigh
  against exporting before the match transform.
- Constant references print by name (`E_INSUFFICIENT_BALANCE`), preserving
  the error-code idiom; a constant declaration prints as `def NAME : T := e`
  (Leaner folds integer constant expressions with checked semantics, and
  `Bool`, `Address`, and vector-literal constants as literals — E7).
- **Loans and places.**  Leaner's place grammar accepts an element step only
  directly after the root (`x[i].f`, not `x.f[i]`), borrows fields and
  elements through references only, and its verifier requires later borrows
  of a loaned place to chain through the live loan.  The printer therefore
  keeps, per block, the live loans by textual place: a borrow reuses the
  place's loan (a mutable loan serves immutable borrows; a loan is kept only
  for places whose index terms are immutable), rebases a sub-place onto its
  longest loaned prefix, loans an owned local before borrowing inside it
  (`let ref ← &mut c; bump (&mut ref.value)` shape), splits a path at later
  element steps, and reads an owned local back through its loan after every
  mutation (`c ← *ref`) — the loan is the local's access path for the rest of
  the block, so direct assignments and `push_back` go through it too.
  Nested places through a reference variable print as `&mut r.f.g`.
- A boolean literal condition prints as the proposition (`aborts_if False`).
- Spec clauses print in Leaner's fixed order `requires; modifies; ensures;
  aborts_if …`, conjoining repeated `requires`/`ensures`; a spec without
  `ensures` prints `ensures True`.
- Inside `aborts_if` clauses global places print as `old(R[a])`, making the
  pre-state reading that Move gives them explicit; `requires` reads the
  pre-state by construction, and in `ensures` bare `R[a]` is the post-state
  and `old(…)` the pre-state, exactly as in Move.
- **`modifies` rendering.**  Leaner's frame is closed — an omitted
  `modifies` means no global memory changes, `modifies R[a]` means nothing
  else changes — while the prover's `modifies` is per resource type: a
  `CanModify<R>(a)` assertion guards each write to `R` only if the spec
  names targets for `R` (`generate_modifies_check` in
  `spec_instrumentation.rs`), unlisted families are unconstrained, and an
  opaque callee's listed targets are the havoc frame at its call sites
  (unlisted memory it writes is havoced coarsely, with a warning).  The
  printer maps this without inventing frame information:
  - no `modifies` clause and the function (transitively, per the effect
    analysis) performs no global write → no clause, as in Leaner;
  - no `modifies` clause but global writes → `modifies *;`, the loose frame
    (E20);
  - clauses on an **opaque** function → `modifies <targets>;` — the closed
    frame, the same reading as Leaner's;
  - clauses on a non-opaque function → `modifies <targets>, *;` — the listed
    families closed at their addresses, everything else loose, exactly the
    per-family check the prover performs.

  This is the one place the dropped `opaque` pragma leaves a semantic
  residue.  Leaner callers normally unfold a callee's `sourceSpec`, so a
  loose frame costs a caller proof nothing; where a proof campaign uses a
  contract directly (`wp_call`, verified recursive callees) it may tighten
  the transpiled clause by hand.
- Spec binders follow Leaner's asymmetry: `&T` parameters lose the `&` in
  `spec` binders, `&mut T` parameters keep it.

### Transpilation report

Per package, the driver writes a machine-readable report plus a summary:
modules transpiled cleanly; per-module dropped pragmas; declarations
skipped and why (feature, callee, type); and a feature-frequency table.
The report drives the framework coverage scoreboard (below).  Nothing is
dropped silently.

## Spec semantics mapping

### `aborts_if`, partiality, and strictness

The production prover gives a non-partial `aborts_if P1; …; aborts_if Pn`
two directions:

- **sufficiency** (always): if `Pi` holds at entry, the function aborts,
  and with a `with Ci` clause the code is pinned;
- **completeness** (unless `aborts_if_is_partial`): the function aborts
  *only if* some `Pi` holds.

`aborts_if_is_strict` makes an *empty* clause list mean `aborts_if false`
(never aborts) instead of "no claim".

Leaner's abort clauses (`leaner-move.md`, *Abort behaviour*) already have
the production *shape*: `aborts_if P with C` clauses are repeatable and
disjoined, `aborts_if P` without a code permits any code, `aborts_if False`
states that the function never aborts, and an *omitted* clause leaves abort
behaviour uninterpreted — which is deliberately not the same as providing
`True` or `False`.  What they check is the completeness direction only:
"the function *may* abort where `P` holds, with code `C`", and `ensures` is
owed where every declared condition is ruled out.  As the reference states,
*a contract cannot express that a function must abort*.  To preserve
production semantics the Leaner contract's abort condition becomes a pair of
**optional** (interpreted or uninterpreted) Prop-valued components:

```lean
mustAbort  : Option (Args → State → Prop)         -- sufficiency
abortsOnly : Option (Args → State → U64 → Prop)   -- completeness
```

- **Omitted `aborts_if` means uninterpreted.**  With no clauses (and no
  pragma) both components are `none`: nothing is checked at the
  definition, and callers learn nothing about abort behavior.  This is the
  production default, and Leaner's today.
- **Provided clauses interpret the condition.**  Each clause
  `aborts_if Pi with Ci` contributes the sufficiency conjunct
  `Pi ⟹ mustAbort` and the completeness disjunct `Pi ∧ code = Ci`; a
  non-partial spec interprets both components, so the clause list is an
  iff-characterization of aborting.  In particular `aborts_if False`
  interprets the condition as "never aborts" and `aborts_if True` as
  "always aborts" — providing `True`/`False` is meaningful precisely
  because omission is uninterpreted.
- `pragma aborts_if_is_partial` keeps the clause-given sufficiency
  component but leaves the completeness component uninterpreted (`none`,
  not `some True`).
- `pragma aborts_if_is_strict` on a clause-free function reduces to the
  explicit clause `aborts_if False`; the transpiler emits exactly that.

Pragmas still have no representation in the semantic layer: `Contract`,
`Satisfies`, and the WP rules see only optionally-present Props, where an
absent component contributes no proof obligation and no call-site
knowledge.  The `pragma` spellings are surface markers deciding which
components are interpreted, kept so transpiled specs read 1:1 against the
Move original.

This is a semantic strengthening of today's Leaner `aborts_if` (existing
tests gain sufficiency obligations, which hold for the current examples)
and is prerequisite work for faithful spec transport.  Example:

```lean
spec withdraw (addr : Address) (amount : U64) where
  pragma aborts_if_is_partial;
  modifies Coin[addr];
  ensures Coin[addr].value = old(Coin[addr].value) - amount;
  aborts_if ¬existsAt<Coin>(addr)
```

### Condition properties

- `[abstract]` / `[concrete]` (165 + 6 framework sites, paired with
  `opaque`): with `opaque` dropped there is one contract per function.
  Default policy: plain and `[concrete]` conditions form the contract;
  `[abstract]`-only conditions are dropped with a report entry (they were
  caller-side substitutes for the dropped opaque view).  Revisit if proof
  campaigns need the abstract view as a separate weaker theorem.
- `[injected]` (from `apply`): already expanded into the conditions;
  transported as a property, no special handling.
- Other properties (`isolated`, `deactivated`, …) are transported and
  reported when dropped.

### Spec expression mapping

| Move spec construct | Leaner rendering |
|---|---|
| `old(e)` | `old(e)` (existing) |
| `global<R>(a)` / `exists<R>(a)` | `R[a]` places / `existsAt<R>(a)` (existing); generic families `(R T)[a]`, `existsAt<R T>(a)` |
| `result`, `result_i` (124 uses) | `result`, product projections of `result` |
| arithmetic (`num` or bounded) | **unbounded**, as in MSL (every integer type is evaluated as `num` in specifications): an arithmetic tree prints over the mathematical values — `.toNat` on unsigned leaves, or `.toInt` on every leaf when the tree subtracts, negates, or has a signed or `num`-valued leaf (so `Nat` subtraction never truncates a claim) — and a comparison with an arithmetic side widens its other side; literals are neutral.  The model's implicit widening of a bounded variable read at `num` arrives as an explicit `cast` (producer) |
| `len(v)` | `v.toList.length` |
| `v[i]` (spec index) | `v.toList[i]!` (out-of-range is unspecified in production too) |
| `update(v, i, x)`, `concat`, `contains`, `in_range` | `List` operations on `.toList` |
| `update_field(s, f, x)` | `{ s with f := x }` (0 framework uses) |
| `x.f` on an enum value (`SelectVariants`) | the variant dispatch `match x with \| .V … f … => f \| _ => Move.Spec.arbitrary _ site`, the prover's reading |
| `abort` in a spec expression | `Move.Spec.arbitrary _ site`: the unspecified value the prover gives an aborting spec expression (`$Arbitrary_value_of`) |
| `forall x: T : P` | `∀ x : T, P` |
| `forall i in a..b : P`, `forall x in v : P` (155 range uses) | `∀ i, a ≤ i → i < b → P`, `∀ x ∈ v.toList, P` |
| `exists …` quantifier | `∃ …` |
| spec `let x = e;` / `let post x = e;` (pre / post) | Lean `let` in the clause; a `let post` is a `let` inside the post-state clause, which already sees post-state places |
| `P ==> Q`, `P <==> Q` | `→`, `↔` |
| `modifies global<R>(a)` (119 uses) | `modifies R[a]` — the existing Leaner clause — closed on opaque functions, followed by the loose wildcard `*` otherwise; `modifies *` alone for a writing function without clauses (rendering rule above, E20) |
| spec fun with body (212) | Leaner's `spec fun` declaration under the Move name (a Lean definition for specifications, never lowered); one reading global memory is *stateful* and reads the state of the clause that applies it |
| Move function called in a specification (pure in MSL's sense; the rewriter's companion `$f`) | the call `f args`: Leaner derives the specification version of `f` from `f`'s own body at its declaration (the same reading the compiler derives for `$f`), so nothing is printed for the companion; `std::vector` operations render as the list view (curated module); the natives' versions come from the intrinsic models (`spec fun borrow_address …`, `sha2_256`, `to_bytes`) |
| uninterpreted / `spec native fun` (62) | Lean `opaque` declarations; their attached conditions become hypotheses |
| `axiom` | Lean `axiom`, flagged prominently in the report |
| `invariant` in `spec StructName` | `spec T where invariant …` — the existing data invariant (`this`/`.field`) |
| module-level `invariant` / `invariant update` | `spec module where invariant ∀ a, …; invariant update ∀ a, …` — existing; `[suspendable]` (20) and `disable_invariants_in_body` regions are E18 |
| ghost `global` spec vars (all in `stake.spec.move`) | later milestone (E18) |
| `Emits`, `AbortsWith`, `choose`, `decreases`, `TRACE` | zero or near-zero framework uses; rejected by the consumer with a report entry |

In-body `spec { … }` blocks arrive as `SpecBlock` nodes: loop invariants and
their preceding logical `let` bindings attach to the enclosing loop and are
emitted in its trailing `where` region (E12); `assert`/`assume` (350 + 28) map
to independent spec statement extensions (E14).

## Required Leaner extensions

The transpiler targets the Leaner language as specified in `leaner-move.md`,
whose surface now covers the bulk of the framework's base language: all
integer widths including signed, casts, bit operations and shifts, the full
operator set, tuples and multiple returns, byte strings, `assert!`, range and
guarded matches, `is`, the complete vector operation set, `public`/`friend`/
`package`/`entry` visibility with `friend` declarations, `native` and
`inline` declarations, literal and named addresses, `signer::address_of`
(`Ref.address`), `modifies` framing, data invariants, and regular and update
global invariants.  The table tracks the required framework extensions,
including the ones completed since the plan was written; milestones refer to
the plan below.

| # | Extension | Status | Why (framework data) | Milestone |
|---|---|---|---|---|
| E1 | Module identity: address, alias, friends; root namespaces and nested-namespace receiver functions | **confirmed**: `module M at alias` inside an enclosing `namespace` (the aliased-package fixture `AptosFramework/Counter.lean`); receiver functions print in prefix form, the nested-namespace receiver tier is a readability item | 167 modules at real addresses; `0x3::token` vs `0x4::token`; 225 receiver functions | M1 |
| E2 | Spec grammar: `pragma` clauses; explicit relational contract shape for a pure function with only `ensures` (Leaner's pure value contract reads aborting executions with wrapping values, unlike production) | **implemented**: `pragma aborts_if_is_partial \| aborts_if_is_strict;` clauses (`Move/Verify/Syntax.lean`); a pragma-led `ensures` on a pure function selects the relational contract; the printer conjoins repeated `requires`/`ensures` | 530 specs, 1889 `aborts_if`, 1513 `ensures`, 214 `requires` | M1 |
| E3 | Two-directional `aborts_if` contract semantics with optional (uninterpreted) abort components; `aborts_if_is_partial` pragma; strictness | **implemented**: `Contract.mustAbort` (sufficiency) beside `aborts`/`mayAbort` (completeness); without the partial pragma every abort matches a clause; strict with no clauses means "never aborts" (`Tests/Verification/AbortDirections.lean`) | 127 partial, 30 strict pragma sites | M1 |
| E7 | Named constants of type `Bool`, `Address`, and `vector<u8>` | **implemented**: the compiler folds `def NAME : Bool \| Address \| Vector U8 := literal` (`literalConstant?` in `Move/Compiler/Normalize.lean`), beside the checked integer folding | error-code and config constants everywhere | M1 |
| E20 | Loose frame: the `modifies` wildcard target `*` — "every family not listed is unconstrained" — generalizing the existing `modifies R` (one family unconstrained) to the rest; `modifies *` alone is the fully open frame | **implemented** (`Move/Verify/Syntax.lean`, `Tests/Verification/LooseFrame.lean`) | the prover's `modifies` is per family and absent for most of the framework (119 clauses for 167 modules); mirrors the model's own `modifies_of<f> *` wildcard | M1 |
| E9 | Native-function modeling: a `spec` on a `native fun` yields an assumed contract, and curated Lean-backed models for stdlib natives (hash, bcs, string internals, event natives) | **implemented**: natives and `pragma opaque` functions are summarized by their contracts (`Contract.summary`, `f.summarySpec`; `Tests/Verification/Summaries.lean`); the intrinsic models of `Transpiler/Intrinsics.lean` supply the specs the source lacks (signer, hash, bcs) | 225 `native fun` | M2 |
| E11 | Receiver-style vector operations accepted by `spec`/`verify`; spec-side `toList` lemma library | operation set complete; `v.get i`/`r.insert`/`r.remove` rejected at `spec` | heavy vector use | M2 |
| E12 | Loop invariants on `while`/`loop` (verification-side; compilation unaffected) | **done** (2026-08-23): XAST loop invariants print as `invariant P` at the loop head; Leaner translates them to fixed-point induction obligations over loop locals, live mutations, and anchored entry values; verification and transpiled-program baselines cover the path | 169 loop-invariant sites | M4 |
| E13 | More than two simultaneous `&mut` parameters | **done** (2026-08-24), without a fixed arity limit: calls, prophecy opening/reconciliation, dynamic reference returns, conservative all-input poisoning, forwarding, and contract verification use heterogeneous mutation tuples | rare in the framework | M4 |
| E14 | Spec statements `assert`/`assume` in bodies | **done** (2026-08-24): XAST spec blocks print as source-only `assert P` / `assume P` statements; Leaner turns assertions into state-certification obligations and assumptions into path restrictions at the exact program state, including state-anchored `old` observations | 350 + 28 sites | M4 |
| E16 | Intrinsic types: `Table`, `TableWithLength`, `SimpleMap`, `SmartTable`, `OrderedMap`, `BigOrderedMap`, aggregators → Lean-native models honoring the `intrinsic = map` function mapping | **Transport implemented; semantics suspended** (2026-08-24): schema v4 carries resolved mappings, and the six-type corpus checks all 158 binding occurrences. Modules that own an intrinsic are rejected explicitly. The generic model, complete role semantics, validation, diagnostics, and source round trip wait for the unified LIR; see [`intrinsic-design.md`](intrinsic-design.md) | 131 intrinsic pragmas, ~700 qualified `spec_*` calls | M5 |
| E17 | Events: emit natives as ghost/no-op effects (`#[event]` attributes already pass through as metadata) | attributes carried | 155 event structs, 6 `emits` specs | M5 |
| E18 | `[suspendable]` invariants and `disable_invariants_in_body` regions, ghost spec variables | regular and update global invariants exist | 20 + 5 + 8 sites | M6 |
| E19 | Enum variant-field borrows | **done** (2026-08-23): `&r.f` / `&mut r.f` on enum referents and `match` through `&`/`&mut` binding payload references, end to end (IR `borrowVariantField`/`testVariantRef`, XIR + Rust importer, Leaner surface/compiler/verifier, printer) | 77 enums, 32 matches | M5 |

Dropped without replacement: `acquires` clauses (Leaner infers them),
`#[test]`/`#[test_only]` declarations (excluded at export), `inline fun`
declarations (expanded; verify-mode retentions transpile as ordinary
functions — Leaner's own `inline fun` is not used because the model's calls
to retained functions are ordinary calls).

## Worked example

Move source:

```move
module 0x42::basic_coin {
    struct Coin has key { value: u64 }

    const E_INSUFFICIENT: u64 = 1;

    public fun withdraw(addr: address, amount: u64) acquires Coin {
        let balance = &mut Coin[addr].value;
        assert!(*balance >= amount, E_INSUFFICIENT);
        *balance = *balance - amount;
    }
    spec withdraw {
        pragma aborts_if_is_partial;
        aborts_if !exists<Coin>(addr);
        aborts_if Coin[addr].value < amount with E_INSUFFICIENT;
        ensures Coin[addr].value == old(Coin[addr].value) - amount;
    }
}
```

Generated Leaner (target shape):

```lean
-- generated by move-to-lean from basic_coin.move
import Move

open Move
open scoped Move Move.Spec

module basic_coin at 0x42 where

  struct Coin has Key where
    value : U64

  def E_INSUFFICIENT : U64 := 1

  public fun withdraw (addr : Address) (amount : U64) : Action Unit := do
    let balance ← &mut Coin[addr].value
    let current ← *balance
    assert!(current >= amount, E_INSUFFICIENT)
    balance := *balance - amount

  spec withdraw (addr : Address) (amount : U64) where
    pragma aborts_if_is_partial;
    modifies *;
    ensures Coin[addr].value = old(Coin[addr].value) - amount;
    aborts_if ¬existsAt<Coin>(addr);
    aborts_if old(Coin[addr].value) < amount with E_INSUFFICIENT
```

Everything in the function body and the `struct`, `def`, `ensures`, and
`aborts_if … with` clauses is the current surface; the `pragma` clause and
the two-directional reading of the abort clauses are E2/E3, and the loose
frame `modifies *` is E20 — emitted because the Move spec has no `modifies`
clause while the body writes `Coin` (had the spec said `pragma opaque;
modifies global<Coin>(addr);`, the output would be the closed
`modifies Coin[addr]`).  `*balance` is hoisted into `current`.

## Validation and testing

- **Schema tests** in `move-model-exchange`: pin exact XAST wire fragments,
  round-trip serde, version checks — mirroring the existing exchange tests.
- **Producer baselines**: datatest suite over `.move` inputs with
  `.xast.exp` baselines (`UB=1` update flow), covering every AST and spec
  construct.
- **Decoder/printer baselines** in Lean: `Transpiler/Tests/Programs/<name>.move`
  is exported by the Aptos CLI at test time (no stored XAST), transpiled, and
  compared byte for byte with the generated `<Name>.lean` beside it
  (regenerated with `lake exe transpile --move-file Transpiler/Tests/Programs/*.move
  -o Transpiler/Tests/Programs`); printer determinism so framework
  transpiles diff cleanly in CI.
- **Elaboration gate**: the generated files are imported by the test root and
  built against `Move` by `lake test` — Lean elaboration re-checks
  representability, typing, and spec acceptance end to end.
- **Verification test** (`Transpiler/Tests/Verification.lean`): `verify`
  over the transpiled specs that Leaner proves automatically; it documents
  the ones it does not (a dropped loop invariant, an enum `match`, a
  `Vector.length`/`toList.length` lemma), none of them transpiler gaps.  The
  `--verify` flag produces the same commands inside generated files.
- **Differential execution (stretch)**: for modules inside the Leaner
  executable subset, compile the generated Leaner through the existing
  XIR/compiler-v2 path and compare execution against the directly compiled
  original on transactional tests.
- **Framework scoreboard**: CI job runs `exchange --format ast` +
  `transpile` over the five framework packages and publishes the report
  (modules clean / partial / rejected, features blocking each).  The
  scoreboard, not committed generated code, tracks progress; full generated
  framework output stays out of the repository until it is stable.
- **Verification campaigns** are explicitly separate from transpilation
  correctness: `verify` emission is opt-in per module once proofs are
  feasible.

## Milestones

### M1 — XAST core and the base language
Schema (`ast` module, version 4), producer for the full AST (rejections only
for constructed/stored function values), the model-only build entry, CLI
`--format ast`;
Lean mirror types, decoder, effect inference, declaration ordering, and the
`Std.Format` printer (with doc comments) for the base language Leaner
already accepts, plus E1–E3, E7, and E20 (module identity confirmations,
`pragma` clauses and the relational contract shape, two-directional abort
semantics, non-integer constants, the loose `modifies` frame).
Acceptance: the
Leaner test programs (`Tests/Verification/Account`, `Tests/Language/Loops`,
`Tests/Language/EnumPayloads`, `Tests/Language/Generics`,
`Tests/Verification/OrderedMap` core) written in Move transpile to
elaborating Leaner source; baselines in place.  **Done** (see *Implementation
status*).

### M2 — move-stdlib
E9 and E11 plus the curated stdlib mapping.  Acceptance: **move-stdlib
transpiles completely** (natives via the curated table and assumed
contracts, inline functions expanded), `lake build` green on the output.

Work list (the historical first dry run exported 12 of 15 modules; the current
e2e package exports and canonically re-imports all 15, while the remaining
work is semantic and presentation completeness):

1. **Layout.**  The canonical `move-stdlib` package (`Move.toml`, `sources/`,
   spec files included) lives under
   `leaner-e2e-tests/LeanerE2ETests/MoveToLeanerLang/MoveStdlib/`, where every
   exported module has a side-by-side LeanerLang-or-error expectation. The
   legacy generated Lean remains under `Transpiler/Tests/Programs/MoveStdlib/Std/`
   with its report. The transpiler baseline reads the E2E-owned source package,
   and its elaboration gate continues to compile the legacy generated modules.
2. **Lean module root.**  Generated `import`s and file paths take a Lean
   module prefix (`--lean-root Transpiler.Tests.Programs.MoveStdlib`); the
   root-level `Std` would otherwise collide with Lean's own `Std` library.
   The Lean *namespaces* stay alias-rooted (`Std.error`).
3. **Function values, per function.** XAST v2 carries function-typed
   parameters and `Invoke`, so the retained inline helpers in `option` and
   `result` export. Leaner renders callbacks conservatively as
   `… → Action …`; public and private `inline fun` declarations are forced
   into callers and excluded from standalone source-spec derivation and Move
   output. Lambdas, closures/captures, and storage still produce named
   `skipped` entries. XAST v4 carries structured behavior payloads, so the four
   behavior-summary functions in `std::vector` now export and round-trip as
   ordinary `spec fun` declarations. `std::vector` stays curated: it *is*
   Leaner's `Move.Vector`.
4. **Natives.**  `native fun f (…) : T` (Leaner's body-less item, opaque at
   the Lean level) with its Move spec as an assumed contract (E9 in
   `Move/Verify`); uninterpreted spec functions print as `opaque` with their
   attached conditions as axioms.
5. **Intrinsic models in Lean.**  A hand-written prelude beside the generated
   stdlib (the analogue of the prover's `prelude.bpl`/`native.bpl`): hash
   (`sha2_256`/`sha3_256` injective, 32 bytes), `bcs::to_bytes` (injective,
   non-empty), `signer::borrow_address` (the signer's address, Leaner's
   `Ref.address`), `mem::swap` (referents exchanged), `cmp::compare` (equal
   values compare `Equal`, otherwise an arbitrary fixed ordering), string
   internals (unconstrained, as in the prover), reflection natives.
6. **Printer fixes** surfaced by the run: bare field selection in struct
   invariants (`invariant … list[i] …` → `this.list`); the prover-only
   `int2bv`/`bv2int` representation switches erase onto Leaner's bounded
   integers while `pragma bv` is dropped and reported; loop invariants and
   spec statements were initially dropped and were completed in M4
   (E12/E14).

Status (2026-08-23).  Items 1–6 are implemented; the package copy is in
place with its baselines, report, and gate. All 14 transpiled stdlib modules
(`vector` is curated) elaborate and the gate imports them. `std::result` and
`std::option` are clean; all nine function-valued option helpers and
`spec borrow_mut` transpile and elaborate, and the generated `borrow_mut`
contract is proved against its payload-returning body in the transpiler gate.
Decisions taken on the way:

- **Contract summaries (E9).**  A native, or a function specified `pragma
  opaque`, is *summarized*: its callers reason through `Contract.summary`
  (the computation of every outcome the contract permits; `f.summarySpec`),
  not through its body; an opaque function's body is still what `verify`
  checks when its source semantics can be generated, and only assumed — with
  a warning — when the translator cannot follow it (`mem::replace`).  The
  printer keeps `pragma opaque`; `[abstract]` conditions are kept for natives
  (their only contract) and dropped for functions with bodies, whose
  `[concrete]` conditions are verified and summarized.
- **Intrinsic models** (`Transpiler/Intrinsics.lean`, the prelude's terms): a
  native without a `spec` of its own gets one from the table —
  `signer::borrow_address` returns the signer's address; `hash::sha2_256`/
  `sha3_256` return an uninterpreted injection onto 32-byte vectors
  (`opaque spec_sha2_256` with its axioms); `bcs::to_bytes` returns the
  source's uninterpreted `serialize`, axiomatized injective and non-empty.
  Axioms are about the uninterpreted spec functions only (never about the
  placeholder-bodied native Lean functions) and the report lists them.
- `std::signer` is transpiled like any module (its `address_of` is an
  ordinary call in code; in specifications `signer::address_of(s)` is
  `s.address`); `std::vector` stays curated.

- **No dummy specs.**  A function without a spec block gets no `spec`:
  Leaner derives a function's source semantics from its body at its
  declaration, for callers in any module, exactly when there is no `spec`
  block (`#derive_move_source_spec`, emitted by the module items for every
  `fun` with a body; best effort and silent — `set_option
  move.reportDerivation true` reports the failures).  A `spec` with only
  pragmas is still printed.  Every body thus meets the verifier's translator,
  whose gaps then show where a caller needs the semantics (several were
  fixed on the way: `else if` chains, `let x ← if …`, effectful statements
  before others, comparisons in value position, `let _ ← …`, dotted uses of
  loop locals, enum constructors in pure bodies, the `modifies …; ensures`
  clause form, loans inside loop bodies, hygienic loop locals).
- **Certified creation.**  A struct with a data invariant is created in code
  through `T.certify f₁ … fₙ`, an `Action` the compiler packs and the
  verifier translates to `Spec.certified` — the invariant is owed at that
  creation, by verification, not by the elaborator at a literal
  (`bit_vector::new` builds its value from loop results).  The printer
  emits `let v ← T.certify …` for every certified struct literal in code;
  specifications keep the literal.
- **Spec quantifier ranges** `forall x in lo..hi` range over `num`: the
  binder is `(x : Int)`, the bounds are the mathematical values of their
  leaves, and an `Int`-valued vector index narrows with `.toNat`.
- **Loop invariants (E12)** print as Leaner's `invariant P` statements at the
  loop head (the spec block the Move compiler places before the condition —
  of a `loop` body, or in the condition block of a `while`), over the body's
  locals; the verifier translates them to `Spec.withInvariant` and the
  automatic prover uses them (`bit_vector::new`, with its certified
  creation, verifies automatically; `bit_vector::length` too).
- **In-body spec statements (E14)** print as source-only `assert P` and
  `assume P` statements.  The verifier interprets an assertion as an
  obligation over the exact current state and an assumption as a restriction
  on the continuing path; anchored `old` observations use the capture at
  their source program point.
- **Spec arithmetic and `num`**: `num` is `Int`; spec `let`s of `num` type
  are `Int` lets over the mathematical values, bounded lets narrow their
  arithmetic value back (`MoveInt.ofInt`); struct fields, spec-function
  parameters, and spec-function results narrow or widen at the boundary;
  transitively needed spec `let`s are emitted.
- **Bit-vector representation switches**: `int2bv`/`bv2int` are no-ops in
  generated source because a Leaner `U8`/…/`U256` or signed integer already
  has fixed-width bit operations.  Arithmetic remains mathematical until a
  surrounding cast or declaration result reifies it with `MoveInt.ofInt`;
  bitwise operands stay bounded and only the completed result is widened.
  This transports `features::spec_contains` and its dependent contracts.
- **Move functions in specifications**: the producer runs the specification
  rewriter (`SPEC_REWRITE`), so a call of a Move function in a specification
  arrives resolved to its companion `$f` and checked for pureness; the
  transpiler prints the call as `f args` and nothing for the companion —
  Leaner derives the specification version of `f` from `f`'s own retained
  body at its declaration (the pure reading: references erased, `assert!`s
  dropped, global reads as places, calls as versions — the reading the
  compiler derives for `$f`), and a body without one makes the specification
  applying it report why.  A spec function reading global memory
  (`basic_coin::total`) is a stateful `spec fun`; a native's version is
  declared by its intrinsic model (`spec fun borrow_address …`).  `std::vector`
  operations render as the list view (the module is Leaner's).  `bit_vector`
  is now clean.  In specifications, `x.f` on an enum value and `abort` read
  as the prover reads them (a variant dispatch; an unspecified per-site value,
  `Move.Spec.arbitrary`), so `option::spec_borrow` transpiles — printed as
  `self.e`, Leaner's field selection on an enum value.  E19 — enum payload
  *references* in code (`borrow`, `borrow_mut`, `swap`, `contains`,
  `borrow_with_default`, `get_with_default`) — is done as a Leaner extension:
  IR `borrowVariantField(Inst)` / `testVariantRef(Inst)` (execution and
  typing semantics, interpreter, decoder/encoder; reference elimination
  rejects them for now), XIR `BorrowVariantField`/`TestVariantRef` with the
  Rust importer, Leaner `&r.f` / `&mut r.f` on enum references
  (`borrowVariantField(Mut)`) and `match` through a reference binding payload
  references (`Move.EnumAccess`: a `match` on a `Ref`/`MutRef` local
  dispatches with `testVariant(Mut)Ref` and borrows the payload), and the
  verifier's variant-payload focus (`Move.Semantics.variantFieldSpec` with
  the `variantMismatch` abort; type-directed `selectPath%`/`updatePath%`/
  `bindSelectPath%`/`guardPath%` elaborators in `Move.Verify.Paths`; a
  `match` through a live mutation loans its payload fields with
  `withMutation(s2)` and rebuilds the variant).  The printer borrows a
  variant field through the owner (`&self.e`) and prints a payload-binding
  `match` on the reference variable. Leaner's path-free mutation-level call
  relation now transports mutable-reference results, including
  `option::borrow_mut`, with conservative mutable-input poisoning and local
  payload reconciliation. See `Move/verification-design.md`, under
  **Returning a mutable reference**.
- **Parameters** that are reassigned or mutably borrowed are rebound under a
  fresh name (`x'`); `let mut` bindings that would shadow a mutable local of
  the block are renamed too (Lean forbids the shadowing).

Leaner extensions made for the stdlib, each with its witness:

- **Imported struct and enum types in the executable compiler** (`bcs`,
  `string`, `reflect` use `option::Option`): the module links to other
  modules' types by identity — IR/XIR `external_structs` (struct ids
  `numStructs + i`, like `external_functions`; XIR version 6), collected by
  the compiler from every type the module mentions, resolved by the Rust XIR
  importer against the loaded modules.
- **Verifier borrow discipline**: a mutable borrow of an owned local while a
  `&mut` parameter is live is an *independent* loan (`string::insert`),
  translated as a nested computation which returns its continuation as a
  closure. This retains writes to disjoint live mutations and reconciles the
  loan before normal continuation, `break`, `continue`, or early `return`.
  Loop-freshened locals compare by their hygiene. Structural loans selected
  for a mutable-reference result use the same exit stack: field, vector, and
  enum-payload prophecies are preserved on the returning edge and resolved
  before plain or labeled loop exits resume their owner.
- **Data invariants at construction**: `T.certify` (above).
- **Enum payload references** (E19): done (see the E-table and the M2
  status above); `option::borrow`, `borrow_mut`, `swap`, `contains`,
  `borrow_with_default`, `get_with_default` transpile.

### M3 — Stdlib ergonomics
The dot-notation tiers, ordinary-comment retention (`comments` table and
span re-attachment), the spec-side vector lemma library, and the
scoreboard.  Acceptance: **aptos-stdlib transpiles** except
intrinsic-backed modules; generated code uses receiver style and carries the
original comments; scoreboard published in CI.

Status (2026-08-23). **Underway.** XAST carries ordinary comments and receiver-call
metadata, and the consumer has comment-attachment helpers plus in-source
diagnostics for declarations and spec blocks it cannot emit.  The printer
does not yet invoke the attachment helpers, receiver calls still print in
prefix form, and the CI scoreboard has not been added.

### M4 — Specification depth
E12–E14: loop invariants, in-body assert/assume, additional `&mut`
parameters; spec `let`, quantifier ranges, `num` mapping hardened.
Acceptance: selected stdlib/framework modules carry complete transpiled
specs; first `verify` campaigns on leaf modules.

Status (2026-08-24). **Extension set complete.** E12, E13, and E14 are
implemented end to end.  The generated stdlib programs exercise loop
invariants and in-body assertions/assumptions; further module proof campaigns
are verification work rather than missing transpiler syntax.

### M5 — Intrinsics, events, enums at scale
E16, E17, E19.  Acceptance: **aptos-framework transpiles** minus
function-value modules (dispatchable assets, `FunctionInfo` users), which
are rejected with precise diagnostics; aptos-token/token-objects clean.

### M6 — Invariant regions and closure
E18 (suspendable invariants, disabled-invariant regions, ghost variables),
report review of every remaining dropped construct.  Acceptance: framework
scoreboard shows no silently-dropped semantics — every module is clean or
carries an explicit, categorized exclusion.

## Settled decisions

- XAST is name-based, fully typed per node, policy-free, versioned, and
  out-only; the schema lives in `move-model-exchange::ast`, the producer in
  `aptos-move/cli/src/exchange/`.
- Export happens after inlining and spec rewriting, before AST
  optimization, through a model-only build entry; schemas/`apply` arrive
  pre-expanded.
- Function-typed parameters, invocation, and typed behavior summaries are
  transported for retained inline helpers. Constructed/stored function values
  and closures/captures remain out of scope. Every other type, signed integers
  included, is transported.
- The Lean transpiler is its own Lake package `transpiler` (library
  `Transpiler`, executable `transpile`), a batch tool emitting one `.lean`
  file per Move module; it depends on Lean core only and requires `move`
  for its elaboration-gate tests; generated output is re-checked by
  ordinary elaboration.
- Move module names are kept verbatim as the Lean namespace component
  (`module coin at aptos_framework where`); root namespaces come from
  named-address aliases (`Std.*`, `AptosStd.*`, `AptosFramework.*`, …) by
  wrapping the module in a Lean namespace.
- `pragma opaque` is dropped; `aborts_if_is_partial` and
  `aborts_if_is_strict` become the only Leaner pragmas; all other pragmas
  are dropped with reported dispositions; `verify = false` is handled
  structurally by not emitting `verify`.
- Leaner `aborts_if` gains production two-directional semantics with
  explicitly optional abort components: an omitted `aborts_if` leaves the
  abort condition uninterpreted, which differs from providing `True` or
  `False`.  The contract and proof layers stay pragma-free;
  `aborts_if_is_partial` leaves the completeness component uninterpreted,
  and strictness reduces to an explicit `aborts_if False`.
- The printer owns effect inference (`Action` results), because Leaner's
  effect typing is explicit where Move's is implicit.
- `modifies` is rendered, never synthesized: closed on opaque functions
  (the prover's and Leaner's common reading), per-family with the loose
  wildcard `*` on non-opaque functions, `modifies *` for a writing function
  without clauses, omitted for a function that writes nothing.
- The printer emits a dependency-respecting declaration order (Lean
  elaborates in order), follows Leaner's file-layout convention, and is a
  `Std.Format` pretty printer with deterministic output.
- Doc comments travel on declarations; ordinary comments travel in a
  source-scanned span table and are re-attached by span, with every dropped
  comment counted in the report.
- Dot notation: `self`-receiver functions map into the type's namespace;
  curated table for pre-receiver stdlib; same-module heuristic exists but
  is off by default.
- Name collisions and keywords are escaped with guillemets, never renamed.
- Move functions in specifications: the producer runs the spec rewriter; the
  transpiler prints the call (`f args`) and nothing for the companion — Leaner
  derives the specification version of `f` from `f`'s body at its declaration
  and resolves `f` in clauses to it; `spec fun` is for what has no derivation.

## Open questions

- **Receiver functions in nested namespaces** (E1, readability tier): the
  printer emits prefix calls; whether generated modules should place
  receiver functions in the type's namespace for `x.f args` is open.
- **Location granularity**: resolved by interning — every node carries a
  `LocId`, and identical spans share one table entry, so full spans cost
  little; revisit only if the tables themselves grow too large.
- **`[abstract]` conditions**: is drop-with-report enough, or should the
  transpiler emit them as a named secondary contract for proof reuse?
- **Axioms**: Lean `axiom` is globally trusted; should the framework axioms
  instead become section hypotheses threaded into dependent proofs?
- **Generated-output hygiene**: when the framework transpile stabilizes, is
  the generated tree committed (reviewable, diffable) or always derived in
  CI (no drift risk)?
- **Retained-inline functions**: transpile under their Move name as
  ordinary functions — confirm no double-definition issues when the same
  package is also consumed pre-inlining elsewhere.
