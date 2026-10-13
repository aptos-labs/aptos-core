# Verifying Move and Rust sources

Status: implemented for Move files, Move packages whose modules link the
modules they use (the Move standard library is verified this way), and
single Rust files. Open work is listed in [`roadmap.md`](roadmap.md),
section 3.

A Move or Rust file is verified by compiling it to VerexSL and verifying
that rendering; every message is reported in the coordinates of the file it
came from.

Move VIR import/validation failures return error reports before rendering,
so callers can record the rejection and continue other files. These reports
currently use the input path at line 1; any embedded byte ranges still need
mapping to their precise source locations. Source numeric bitwise operators,
`bv`/`bv_ret` pragmas and `int2bv`/`bv2int` wrappers are rejected before VIR
encoding until their representation propagation is modeled. Conversion and
subsequent arithmetic can wrap even when the exported expression has type
`num`. Classification can also propagate from executable code into a contract,
or from a signed wrapper into a later unsigned cast.

Opaque `bv_internal` functions with scalar bodies are an exception: their
ordinary contracts and callers stay in integer representation, while their
executable operations retain Move semantics. Calls from the body, aggregate
or generic values, inline specifications, concrete clauses and proof blocks
still require representation analysis and are rejected. The scalar regression
includes mutable-reference updates and callers. Specification casts implicitly
read reference operands, including their distinct current and `old` values.

The producer and consumer use XAST version 10, which preserves whether the
compiler defaulted a specification numeric literal's type. Representation
analysis needs that fact to adapt unsuffixed literals without silently changing
explicit widths; see [`numeric-representation.md`](numeric-representation.md).

```text
foo.move ─ compiler-v2 XAST ─┐
                             ├─ validated VIR ─ VerexSL rendering ─ elaboration ─ messages
foo.rs ─── rustc MIR ────────┘                  (+ foo.spec.lean items)                  │
                                                                                          ▼
                                                           foo.move / foo.rs / foo.spec.lean positions
```

## Commands

```bash
cd verex-move && lake build verex-move
lake env .lake/build/bin/verex-move verify foo.move [--output foo.move.lean]

cd verex-rust && lake build verex-rust
lake env .lake/build/bin/verex-rust verify foo.rs [--spec foo.spec.lean] \
  [--output foo.rs.lean] [-- <rustc arguments>]
```

`lake exe` works as well but replays the build log first. The Move command
needs the exchange frontend (`APTOS_MOVE_CLI`, see the tree's `CLAUDE.md`).
A package directory is exported with its dependency modules, which are read
whole and not verified, as from the Move CLI (below), so calls into them
inline. For a package directory, `--filter <part>` verifies the modules whose
source file name contains the part, and `--modules <a::m>,…` verifies the
named modules with only what verifying them reads exported
(`verification-benchmarks.md`, "Module selection"). A filter or a module
name that selects nothing is an error rather than a run that verifies
nothing. `--dev` compiles a package directory in dev mode, with its
`[dev-addresses]` and `[dev-dependencies]`. `VEREX_OPTIONS=name=value,…`
sets Lean options for the verification a command or the benchmark runs, for
debugging: a value `true` or `false` is a Bool, a numeral a Nat, anything
else a String (for example `VEREX_OPTIONS=verex.solverDebug=true` prints
the solver's steps).
Each command writes the rendering beside the source (`foo.move.lean`,
`foo.rs.lean`) for inspection, prints one `file:line:column: severity:
message` line per message, and exits non-zero when any is an error. Its
last line is the wall time per phase, as the Move Prover reports its own:

```text
verex-move: 0.82s load, 0.11s frontend, 0.30s render, 0.30s lowering, 22.63s certification, 80.32s verification, total 107.49s
```

`load` imports the Lean environment, `frontend` takes the sources to their
validated unit, `render` writes the VerexSL, `lowering` elaborates the
rendered modules back to validated, linked units, `certification` builds the
definitions and kernel-checked certificates the proofs use (the unit, its
semantics, compiled bodies, contracts), and `verification` runs the proofs.
Each moment counts toward the innermost phase running; the rest of the total
is parsing and mapping the messages.

A Move declaration the export leaves out, such as a function constructing a
function value, is an error at its module: the rendering keeps it only as an
`-- unsupported Move declaration` comment, so nothing verifies it.

### From the Move CLI

A Move package is verified in place, with the usual package tooling, by the
Move Prover command with `--lean`:

```bash
move prove --lean --package-dir aptos-move/framework/move-stdlib   # standalone CLI
aptos move prove --lean --package-dir <package>                     # full CLI
```

The flag lives on the framework's `ProverOptions` (`aptos-move/framework/src/prover.rs`),
the entry point the CLI and the framework's prover tests share, so the
framework packages are verified the same way from both; the backend it calls,
which exports a model and runs the verifier, is the Move Prover's
(`third_party/move/move-prover/src/verex.rs`). With `--lean`, the
package model is built as for the Prover but without bytecode, the
package's modules and what verifying them reads are exported in the typed-AST
exchange format to a temporary
directory (the producer, `dump_ast_module`, lives in the `move-model-exchange`
crate), and `verex-move verify <package> --export <that directory>` runs;
the rendering goes to `build/verex-verify.lean` under the package. The
dependency modules with source (for `aptos-stdlib`, the Move standard
library) are read whole, rendered before the package's own modules in
dependency order, and marked `verify = false` (in their namespace pragmas
and, resolved, in each function's), so their calls inline and their specs
apply while only the package's own functions are verified, as the Move
Prover verifies its targets. A spec `global x: T` of any module arrives as
the ghost resource `Ghost$x` backing it, with `update x = e` a write of that
resource and its existence a `spec module where axiom` of the module. The
verifier's messages are the command's diagnostics, and a verification error
fails the command as a Prover error does. The Prover's timing line becomes
`build, export, verex-move, total`, beside the verifier's own. `--filter` narrows the targets as
usual; the Boogie backend's options have no effect.

The adapter omits non-opaque inline declarations already expanded by compiler-v2.
Opaque inline declarations remain: their callers and behavioral predicates still
reference them, and their bodies must satisfy their contracts. So does a
non-opaque inline function with a specification of its own (a condition the
source states, not one a schema application injects): the Move Prover verifies
its body against it (`is_inline_verified`), while its calls stay expanded and a
caller proves what the body does. An explicit
`verify = false` retains the existing trusted-contract treatment. Their comments
follow the same retention decision.

The verifier is found through `VEREX_MOVE_EXE` (its executable, run as is, so the caller supplies
`LEAN_PATH`), `VEREX_MOVE_HOME` (its Lean package), or the enclosing Aptos
Core checkout, where `verex-move/.lake/build/bin/verex-move` runs through
`lake --dir <package> env` under the toolchain the package pins (`lake` from
`LAKE`, `PATH`, or elan). Build it with `cd third_party/move/lean/verex-move
&& lake build verex-move`. The tests (`aptos-move/cli/src/tests/prove/lean_*`
and `move_stdlib_lean_prover_tests` in `aptos-move/framework/tests`) run only
where the verifier is available.

### From the Move Prover

The Move Prover's own command verifies one Move file the same way: `mvp
--lean <file.move>` (with its usual `--dependency` and `--named-addresses`)
builds the model of the file and its dependencies as one program through
the checker and rewriters, exports the file's modules and what verifying
them reads, and runs `verex-move verify <file.move> --export <that
directory>`, which verifies the modules the file declares with the others
linked; the rendering goes beside the output path, with extension `lean`.
`--heartbeats` sets the default budget.

The Prover's unit tests (`move-prover/tests/sources`) run this way as the
test feature `lean`, only on request and not in CI:

```bash
MVP_TEST_FEATURE=lean cargo test -p move-prover --test testsuite [<path part>]
```

Its baselines are `foo.lean_exp` beside `foo.move` (`UPBL=1` updates them).
Each function's verification gets the default budget, as on the command
line; a function that verifies but needs more raises its own
with `pragma heartbeats`, and a function the automation does not prove, where
the Prover reads a Move `proof` block or proves nonlinear arithmetic, is
proved in `foo.proof.lean`. The tests are skipped where the verifier is not
built.
[`prover-test-problems.md`](prover-test-problems.md) registers the problems
the run shows.

## Specifications

A Move file's specifications are its own `spec` blocks. A Rust file has none;
they are VerexSL items — `spec f where …` blocks, specification functions,
module invariants — in `foo.spec.lean` beside `foo.rs`, or in the file
`--spec` names:

```lean
spec max where
  ensures result >= left && result >= right
  ensures result == left || result == right
```

The items are spliced into the rendered namespace, indented one level, so
they may name every declaration of the crate.

A native without a specification is read as the Move Prover reads it: by
the model its prelude or translator gives the native, when it gives one,
and otherwise not at all — the Prover rejects a call to an unknown native,
and so does the verifier ("has neither a specification nor a prelude
model"). The Move profile lists the models case by case
(`VerexSL.moveNativeModels`): each is an uninterpreted function of the
type arguments and arguments, so two calls at equal arguments agree and a
clause applying the function denotes the same value. `hash::sha2_256` and
`hash::sha3_256` have a 32-byte result; `bcs::to_bytes` is the
specification function `bcs::serialize` of its value; `type_info::type_name`
and `type_info::type_of` are functions of the type, the latter aborting
unless `type_info::spec_is_struct` holds. None of them aborts otherwise.
The prelude's injectivity and length axioms and the concrete names the
Prover computes for concrete types are not mirrored. A caller never fails
on a modelled callee; it fails only on what its own clauses claim.

A Move specification function is partial: it is defined where its
parameters declared with fixed-width integer types hold values of those
types, and its value elsewhere is unspecified (G15 in
[`prover-test-problems.md`](prover-test-problems.md)). The rendering keeps the
declared types (`spec fun f(x : u64) : Int`); the body reads `x` as an `Int`,
and the verifier derives the domain from the type.

A recursive specification function unfolds where its measure descends,
and a leaf holds it unfolded once at each of its applications whose guard
the context decides. The
measure is the `decreases` clause or an integer parameter every recursive
call provably decreases on its path; failing that, one the calls decrease
on for non-negative parameters (the Move types the specification's `num`
projection widened), and each call the path conditions do not prove
descending is guarded by its descent, taking an arbitrary value where the
measure does not descend — the definable reading of the Prover's unguarded
axiom for the function. Parameters of bounded integer type also guard the
unfolding by their bounds.

## Proofs

A specification is established in one of three ways, tried in this order:

1. **Automatically.** The solver proves the function's obligations within a
   budget (`verex.verifyHeartbeats`, 1500000 thousand heartbeats by
   default: about a minute of elaboration). Most functions end here.
2. **By a proof file.** Move has no syntax for a proof, so the proofs of a
   Move file live in the VerexSL file beside it: `foo.proof.lean` beside
   `foo.move`. Its items are spliced into the rendering of the file's
   module, indented one level — `verify f by …` items, whose tactics prove
   `f`'s goal in place of the automatic verification, and any `theorem` the
   tactics use, which may name every declaration of the module. Theorems and
   tactics see `VerexIR` opened, and an `open N …` item opens further
   namespaces for all of them, wherever it stands. The tactics
   of a proof (`designs/denotation.md`): `verex_obligations` splits the goal
   into the leaves `f` owes, each named by its clause (`ensures_2`,
   `aborts_if_1`, `invariant_1_entry`, `invariant_1_step`, `requires_of_g`
   for a callee `g`'s precondition). A clause owed on several paths is one
   goal: the conjunction of its paths, each quantified over what its path
   introduced; `verex_paths` takes it apart into the path goals
   `ensures_2_1`, `ensures_2_2`, …, as `verex_prepare`, `verex_decide` and
   `verex_solve` do first. A leaf is one path and one clause, whose own
   conditionals, such as a specification function's cases, stay in it
   (`verex_split` splits them). `verex_prepare` states a leaf in source
   form; `verex_decide` tries the bounded deciders on every path and fails
   where one stays open; `verex_solve` decides a leaf as the automatic
   verification does, or reports its clause. `verex_on | ensures_1,
   aborts_if_2 => tac | _ => tac'` proves each clause by the first
   alternative naming it, `_` taking the others; an alternative's failure is
   reported at its goal. The automatic verification is `verex_obligations`
   followed by `verex_solve` on every leaf. As Lean's tactics do, each of Verex's acts on the main goal and
   keeps the others; `verex_bounded`, `verex_inheriting`, `verex_settle`,
   `verex_saturate_round` and `verex_canonical_families` act on every
   goal. A Rust file's `.spec.lean` carries
   its `verify` items the same way. A proof file accompanies a source file
   that declares one module; a file declaring several is refused.
   `pragma verify = manual` on the spec block states that the function is
   established this way: it is not attempted automatically, and a missing
   `verify f by …` is an error. The Move Prover reads it as `verify = true`.
3. **By raising the budget.** `pragma heartbeats = N` on the spec block,
   for a function the solver proves given more time.

When the automatic verification of `f` fails, the message says which of
the two to do and where:

```text
foo.move:12:5: error: verex verification failed: the automatic verification
of `f` exceeded its budget of 1000000 maxHeartbeats; provide a proof:
`verify f by …` in `sources/foo.proof.lean` (`verify f by verex_obligations`
lists its obligations), or raise the budget with `pragma heartbeats = N`,
in thousands of maxHeartbeats
```

An obligation the solver cannot decide reports its clause as before and the
same hint without the budget alternative. The file the hint names is the
module's `proof_file` pragma, which the driver sets on every target module
whether or not the file exists. `verify f by verex_obligations` is the way
to see the obligations: the leaves, one `case` each, are reported as unsolved
at the item, in the proof file's coordinates; `all_goals verex_prepare`
after it shows them in source form, the intermediate values the solver names
replaced by the expressions they stand for. A proof that fails
reports at its tactic in the proof file, and the function's summary says
the proof does not establish its specification.

In the editor, an open file's `verify` items are skipped with a message,
since every change would run them again, except `verify! f` items, which
run (`verex.skip_verify`: unset, it holds in the language server only, which
the command line tells apart by `internal.cmdlineSnapshots`; the verification
driver sets it off). The packages' `moreServerOptions` would set it for the
server once Lake passes them to library modules, leanprover/lean4#15626. The code action "Verify `f` here" on an item marks it so, "Skip
`f` in the editor" unmarks it; `lake test` runs every item, marked or not.

The integer bounds a specification names, `MAX_U8` … `MAX_U256`, `MAX_I8` …
`MAX_I256`, and `MIN_I8` … `MIN_I256`, are Lean constants of type `Int` as
well (`VerexSL/Bounds.lean`), so a proof names the bound an obligation
states as a literal: `‹x.val ≤ MAX_U64›` finds `x.val ≤
18446744073709551615`. `omega` reads a constant as an atom; `unfold MAX_U64
at *` gives it the value.

The package tooling never compiles a proof file as a Lean-authored Move
module: `.proof.lean` files are not package sources and do not enter the
package digest (`move-command-line-common::files::is_lean_source`).

### Move frames

A VerexSL frame is closed: without `modifies` a function changes no global
memory. A function with a body that declares no contract (and plays no map
or table role) claims no frame: it is verified for the invariants it writes,
and its callers inline it. A Move specification frames only the resource
families it names targets for. The Move frontend states Move's reading
explicitly (`VerexMove/Frontend/Frames.lean`): no targets and a transitive
global write become `modifies *`; targets on a non-`opaque` function become
`modifies <targets>, *`.

### Move signers

The Move Prover assumes of every signer value that it signs the
transaction. Where the unit declares `std::signer`'s predicates, a Move
contract states it of the signers a function takes, as preconditions, and
of those it returns: `is_txn_signer(s)` and `is_txn_signer_addr` of its
address (decided 2026-10-02). A signer comes from a parameter or a native,
so a caller establishes it from its own precondition or the native's
contract; an address no signer in scope holds stays unconstrained.

### Move hashes

The Move Prover assumes `hash::sha2_256` and `hash::sha3_256` injective,
which contradicts their 32-byte results. A Move contract states instead,
as preconditions, that no two of the function's byte-vector parameters
collide under a hash the function applies, in code or in a specification,
at any depth (decided 2026-10-02). A hash of bytes a function computes is
not covered, and a caller passing computed bytes to a function that
assumes it cannot establish it.

### Move serialization

The Move Prover's prelude states of `bcs::serialize`, for every type, that
it is injective and never empty, and that all addresses serialize to one
length. A Move contract states these, as preconditions, of the function's
parameters where it applies `bcs::serialize` or `bcs::to_bytes` at any
depth, as it states the hashes' collision freedom (2026-10-09): each
parameter's serialization is not empty, two parameters of one type with
equal serializations are equal, and two address parameters serialize to one
length (`Contract.serializeFacts`). Unlike the hashes' injectivity, these
hold of BCS itself.

### Move vector intrinsics

The Move Prover's prelude defines the `std::vector` functions the library
marks `pragma intrinsic`, and it verifies none of their bodies. The
exchange reads them likewise, in code and in specifications, as the VIR
operations their bodies compute, with the library's abort codes:
`is_empty`, `contains`, `index_of`, `remove`, `reverse`, `reverse_slice`,
`append`, `reverse_append`, `trim`, `trim_reverse`, `insert`, and
`remove_value` (decided 2026-10-02). Their bodies are loops without
invariants, or move ranges VIR has no operation for. `swap_remove`,
`rotate`, and `rotate_slice` keep their bodies, which are free of loops
once these are operations.

## Mapping messages

The shared driver is `VerexSL.SourceVerify` in `verex-ir`. The rendering
is elaborated in the command's own process, so the elaborators run natively,
and a message is mapped by the first rule that applies:

1. A position in the items spliced from a specification or proof file maps
   line by line to that file.
2. Otherwise the source's validated namespace and the one elaborated from the
   rendering are aligned: declarations by name, then node by node while the
   two shapes agree — expressions, patterns, places, and contract clauses by
   kind and occurrence. The position takes the source range of the innermost
   aligned node around it. A contract's location is its whole `spec` item, so
   the summary a failed function reports at its `spec` lands on the source's
   `spec` block.
3. A position outside every aligned node stays in the generated file.

## Tests

`verex-e2e-tests/VerexE2ETests/SourceVerify/` holds the examples: every
`.move` and `.rs` file there is verified as the commands do it, and its
messages are the baseline `<name>.exp` (none when it verifies). The Move
standard library of the Move corpus is verified as a package, its modules
rendered in dependency order and each linking the modules it uses; its
baseline `MoveStdlib.exp` lists what does not verify yet.

## Known gaps

- A residual obligation no clause marker locates (for example an
  unprovable intermediate fact) lands on the enclosing module, in authored
  VerexSL as well.
- A clause's message quotes its VerexSL rendering, not the source text.

## Budgets

Every target runs within a heartbeat budget (`verex.verifyHeartbeats`, in
`maxHeartbeats` units, 1000000 by default: about thirty seconds at the 32M
heartbeats a second the elaborator sustains on a current machine). `prove --lean --heartbeats N`
and `verex-move verify --heartbeats N` set the default for a run; a target
that needs more raises it for itself with `pragma heartbeats = N;` in its
Move spec block (a pragma the Move model accepts on functions and modules;
the Move Prover ignores it) or `pragma heartbeats = N` in VerexSL, or gets
a proof (above). `N` counts thousands of `maxHeartbeats` units: the default
is `1000`. The standard library needs neither.

`verify --ignore-proofs` (`prove --lean --ignore-proofs`, `verex.ignoreProofs`)
verifies every function automatically, its authored proof and `pragma verify
= manual` ignored: it tells whether a proof is still needed. The stage log
(`verex.stageLog`) records each target's heartbeats, its outcome (`verified`,
`failed`, `budget`) and whether its proof was authored.
