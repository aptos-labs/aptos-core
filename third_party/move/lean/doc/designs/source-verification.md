# Verifying Move and Rust sources

A Move or Rust file is verified by compiling it to VerexSL and verifying
that rendering; every message is reported in the coordinates of the file it
came from. Move files, Move packages whose modules link the modules they
use, and single Rust files are supported.

```text
foo.move ─ compiler-v2 XAST ─┐
                             ├─ validated VIR ─ VerexSL rendering ─ elaboration ─ messages
foo.rs ─── rustc MIR ────────┘                  (+ foo.spec.lean items)    (+ foo.proof.lean)
                                                                                          ▼
                                            foo.move / foo.rs / foo.spec.lean / foo.proof.lean positions
```

Move VIR import and validation failures are reported before rendering, so
a caller can record the rejection and continue with other files. Source
numeric bitwise operators, `bv`/`bv_ret` pragmas and `int2bv`/`bv2int`
wrappers are rejected before VIR encoding, since their representation
propagation is not modeled; opaque `bv_internal` functions with scalar
bodies are the exception, their contracts and callers staying in integer
representation. The exchange records whether the compiler defaulted a
specification literal's type, which the representation analysis needs
([`numeric-representation.md`](numeric-representation.md)).

## Commands

```bash
cd verex-move && lake build verex-move
lake env .lake/build/bin/verex-move verify foo.move [--output <rendering.lean>]

cd verex-rust && lake build verex-rust
lake env .lake/build/bin/verex-rust verify foo.rs [--spec foo.spec.lean] \
  [--output foo.rs.lean] [-- <rustc arguments>]
```

`lake exe` works as well but replays the build log first. The Move command
needs the exchange frontend (`APTOS_MOVE_CLI`, see the tree's `CLAUDE.md`).
A package directory is exported with its dependency modules, which are read
whole and not verified, as from the Move CLI (below), so calls into them
inline. For a package directory, `--filter <part>` verifies the modules
whose source file name contains the part, and `--modules <a::m>,…` verifies
the named modules with only what verifying them reads exported
([`verification-benchmarks.md`](../status/verification-benchmarks.md),
"Module selection"). A filter or module name that selects nothing is an
error. `--dev` compiles a package directory in dev mode, with its
`[dev-addresses]` and `[dev-dependencies]`. `VEREX_OPTIONS=name=value,…`
sets Lean options for the verification a command or the benchmark runs: a
value `true` or `false` is a Bool, a numeral a Nat, anything else a String
(for example `VEREX_OPTIONS=verex.solverDebug=true` prints the solver's
steps).

Each command writes the rendering, for inspection and for the source's
proof file, which loads it in the editor ("Proofs"): a package's goes to
`build/verex-verify.lean` under the package, a Move file's to the build
directory of the package around it (`build/verex/<path>.lean`) or, for a
file of no package, to `.verex/foo.move.lean` beside it, a Rust file's
beside it (`foo.rs.lean`). The rendering records the sources it was made
from and the hashes of their content (`verex sources at`). The command
prints one `file:line:column: severity: message` line per message, exits
non-zero when any is an error, and ends with the wall time per phase, as
the Move Prover reports its own:

```text
verex-move: 0.82s load, 0.11s frontend, 0.30s render, 0.30s lowering, 22.63s certification, 80.32s verification, total 107.49s
```

`load` imports the Lean environment, `frontend` takes the sources to their
validated unit, `render` writes the VerexSL, `lowering` elaborates the
rendered modules back to validated, linked units, `certification` builds
the definitions and kernel-checked certificates the proofs use, and
`verification` runs the proofs. The rest of the total is parsing and
mapping the messages.

A Move declaration the export leaves out, such as a function constructing
a function value, is an error at its module: the rendering keeps it only
as an `-- unsupported Move declaration` comment, so nothing verifies it.

### From the Move CLI

A Move package is verified in place by the Move Prover command with
`--lean`:

```bash
move prove --lean --package-dir aptos-move/framework/move-stdlib   # standalone CLI
aptos move prove --lean --package-dir <package>                     # full CLI
```

The flag lives on the framework's `ProverOptions`
(`aptos-move/framework/src/prover.rs`), the entry point the CLI and the
framework's prover tests share; the backend it calls is the Move Prover's
(`third_party/move/move-prover/src/verex.rs`). With `--lean`, the package
model is built as for the Prover but without bytecode, the package's
modules and what verifying them reads are exported in the typed-AST
exchange format to a temporary directory (the producer, `dump_ast_module`,
lives in the `move-model-exchange` crate), and `verex-move verify <package>
--export <that directory>` runs; the rendering goes to
`build/verex-verify.lean` under the package. Dependency modules with source
are read whole, rendered before the package's own modules in dependency
order, and marked `verify = false` (in their namespace pragmas and,
resolved, in each function's), so their calls inline and their specs apply
while only the package's own functions are verified, as the Move Prover
verifies its targets. A spec `global x: T` of any module arrives as the
ghost resource `Ghost$x` backing it, with `update x = e` a write of that
resource and its existence a `spec module where axiom` of the module. The
verifier's messages are the command's diagnostics, and a verification error
fails the command as a Prover error does. The Prover's timing line becomes
`build, export, verex-move, total`, beside the verifier's own. `--filter`
narrows the targets as usual; the Boogie backend's options have no effect.

The adapter omits non-opaque inline declarations already expanded by
compiler-v2. Opaque inline declarations remain: their callers and
behavioral predicates reference them, and their bodies must satisfy their
contracts. So does a non-opaque inline function with a specification of its
own (a condition the source states, not one a schema application injects):
the Move Prover verifies its body against it (`is_inline_verified`), while
its calls stay expanded and a caller proves what the body does. An explicit
`verify = false` keeps the trusted-contract treatment.

The verifier is found through `VEREX_MOVE_EXE` (its executable, run as is,
so the caller supplies `LEAN_PATH`), `VEREX_MOVE_HOME` (its Lean package),
or the enclosing Aptos Core checkout, where
`verex-move/.lake/build/bin/verex-move` runs through `lake --dir <package>
env` under the toolchain the package pins. The CLI and framework tests of
the flag run only where the verifier is available.

### From the Move Prover

The Move Prover's own command verifies one Move file the same way: `mvp
--lean <file.move>` (with its usual `--dependency` and `--named-addresses`)
builds the model of the file and its dependencies as one program through
the checker and rewriters, exports the file's modules and what verifying
them reads, and runs `verex-move verify <file.move> --export <that
directory>`, which verifies the modules the file declares with the others
linked and writes the rendering where the file's proof file finds it.
`--heartbeats` sets the default budget; `--jobs` how many targets of a
module are proved at once ("Parallel targets").

The Prover's unit tests (`move-prover/tests/sources`) run this way as the
test feature `lean`:

```bash
MVP_TEST_FEATURE=lean cargo test -p move-prover --test testsuite [<path part>]
```

Its baselines are `foo.lean_exp` beside `foo.move` (`UPBL=1` updates
them). Each function gets the default budget; a function that verifies but
needs more raises its own with `pragma heartbeats`, and a function the
automation does not prove is proved in `foo.proof.lean`.

The run builds the verifier and takes a private copy of its executable
and import closure under `target/tmp`, so later `lake` builds cannot
change what a running test loads; a concurrent modification discards the
copy and retries. A `VEREX_MOVE_EXE` the caller sets is run as is, and the
tests are skipped where the verifier's package is not found. A failure of
the harness fails its test and is never written as a baseline: exit code 1
is a verification result only with the CLI's explicit completion marker.
[`prover-test-problems.md`](../status/prover-test-problems.md) registers the
problems the run shows.

## Specifications

A Move file's specifications are its own `spec` blocks. A Rust file has
none; they are VerexSL items (`spec f where …` blocks, specification
functions, module invariants) in `foo.spec.lean` beside `foo.rs`, or in
the file `--spec` names:

```lean
spec max where
  ensures result >= left && result >= right
  ensures result == left || result == right
```

The items are spliced into the rendered namespace, indented one level, so
they may name every declaration of the crate.

A native without a specification is read as the Move Prover reads it: by
the model its prelude or translator gives the native, when it gives one,
and otherwise not at all; a call to an unknown native is rejected ("has
neither a specification nor a prelude model"). The Move profile lists the
models case by case (`VerexSL.moveNativeModels`), each an uninterpreted
function of the type arguments and arguments: `hash::sha2_256` and
`hash::sha3_256` with a 32-byte result, `bcs::to_bytes` as the
specification function `bcs::serialize`, `type_info::type_name` and
`type_info::type_of` as functions of the type, the latter aborting unless
`type_info::spec_is_struct` holds. The prelude's injectivity and length
axioms are not mirrored.

A Move specification function is partial: it is defined where its
parameters declared with fixed-width integer types hold values of those
types, and its value elsewhere is unspecified (G15 in
[`prover-test-problems.md`](../status/prover-test-problems.md)). The
rendering keeps the declared types (`spec fun f(x : u64) : Int`); the body
reads `x` as an `Int`, and the verifier derives the domain from the type.

A recursive specification function unfolds where its measure descends, and
a leaf holds it unfolded once at each application whose guard the context
decides. The measure is the `decreases` clause or an integer parameter
every recursive call provably decreases on its path; failing that, one the
calls decrease on for non-negative parameters, and each call the path
conditions do not prove descending is guarded by its descent, taking an
arbitrary value where the measure does not descend: the definable reading
of the Prover's unguarded axiom. Parameters of bounded integer type also
guard the unfolding by their bounds.

## Proofs

A specification is established in one of three ways, tried in this order:

1. **Automatically.** The solver proves the function's obligations within
   a budget (`verex.verifyHeartbeats`, 1000000 thousand heartbeats by
   default, about thirty seconds of elaboration).
2. **By a proof file.** Move has no syntax for a proof, so the proofs of a
   Move file live in the Lean file beside it, `foo.proof.lean`:

   ```lean
   import VerexMove.Proofs

   verex proofs for "foo.move"
   verex proofs where

   verify f by lean =>
     …
   ```

   The items are `verify f by …` items, whose tactics prove `f`'s goal in
   place of the automatic verification, the `theorem`s the tactics use,
   which may name every declaration of the module, and `open N …` items;
   any other item is refused, and so is a `verify` before `verex proofs
   where`. `verex proofs where` verifies the module as its own command
   does: theorems, then lemmas, then the targets, callees first, the
   functions no item names automatically. A proof file sees its module and
   the modules rendered before it, not another proof file's theorems.
   Theorems and tactics see `VerexIR` opened. A proof file accompanies a
   source file that declares one module. A Rust file's `.spec.lean`,
   spliced into its namespace, carries its `verify` items too.
3. **By raising the budget.** `pragma heartbeats = N` on the spec block,
   for a function the solver proves given more time.

`pragma verify = manual` on the spec block states that the function is
established by a proof file: it is not attempted automatically, and a
missing `verify f by …` is an error. The Move Prover reads it as
`verify = true`.

`pragma verex_verify = false` on the spec block or the module resolves
`verify` to false for Verex alone: a function the Move Prover verifies is
left unverified here. The Move Prover ignores it.

When the automatic verification of `f` fails, the message says which to
do and where:

```text
foo.move:12:5: error: verex verification failed: the automatic verification
of `f` exceeded its budget of 1000000 maxHeartbeats; provide a proof:
`verify f by …` in `sources/foo.proof.lean` (`verify f by lean => verex_obligations`
lists its obligations), or raise the budget with `pragma heartbeats = N`,
in thousands of maxHeartbeats
```

An obligation the solver cannot decide reports its clause and the same
hint without the budget alternative. The file the hint names is the
module's `proof_file` pragma, which the driver sets on every target module.
`verify f by lean => verex_obligations` shows the obligations: the leaves, one
`case` each, reported as unsolved at the item; `all_goals verex_prepare`
after it shows them in source form. A proof that fails reports at its
tactic in the proof file, and the function's summary says the proof does
not establish its specification.

### Tactics

The automatic verification is `verex_obligations` followed by
`verex_solve` on every leaf ([`denotation.md`](denotation.md)).

- `verex_obligations` splits the goal into the leaves `f` owes, each named
  by its clause (`ensures_2`, `aborts_if_1`, `invariant_1_entry`,
  `invariant_1_step`, `requires_1_of_g_at_call_2` for the first
  precondition of `g` at the second call of `g` in the body, by source
  order). A
  clause owed on several paths is one goal, the conjunction of its paths;
  `verex_paths` takes it apart into `ensures_2_1`, `ensures_2_2`, …, as
  `verex_prepare`, `verex_decide` and `verex_solve` do first. A leaf is
  one path and one clause, whose own conditionals stay in it
  (`verex_split` splits them).
- `verex_prepare` states a leaf in source form, its context named from the
  source: a parameter or local by its Move name (`m`, `count`, `it`), a
  parameter's entry value `old_m` where a later value shadows it, a map a
  callee writes back `m'`, a variant's payload `it_Position`, a callee's
  result `iter_next_result`; an assumed clause by the clause
  (`invariant_3`, `requires_1`, a callee's `iter_next_ensures_1`), a bound
  against a literal `count_upper`, a map's validity `m_valid`, an equation
  defining a value `m'_eq`. The map lemmas are named from the module's own
  specification functions (`lemma_get_set`, `lemma_len_remove`,
  `lemma_contains_key_key_at`, …), one theorem per intrinsic map owner and
  lemma. `verex_prepare only` names and normalizes without speculative map
  facts or variant enumeration; `verex_prepare typed` also names the maps
  the leaf reads (`verex_name_maps`) as `SpecMap K V`, whose lemmas live in
  `SpecMap`; `verex_prepare values splitting [x, y]` prepares source
  values and splits the named outer variants as they become available (a
  name absent from the path is skipped); `verex_prepare cases [x, y]`
  performs only the selected case analysis on a prepared goal;
  `verex_cases it, iter_next_result` splits selected outer variants and
  their product payloads while retaining variant-valued fields.
  `verex_map_simp` evaluates closed map observations while symbolic
  lookups and sizes stay in `SpecMap` form.
- `verex_proof` is the structured interface:

  ```lean
  verify f by lean =>
    verex_proof preparing
      verex_prepare typed splitting [iter_next_result]
    | ensures_1 => exact result_property
    | ensures_2 => omega
    | _ => verex_decide
  ```

  Preparation runs on shared program contexts before their clauses split,
  each step on every surviving case; clause alternatives receive
  individual paths with obligation markers removed and are ordinary Lean
  tactic blocks. Omitted clauses use only the cheap decider.
- `verex_obligations preparing` with an indented block runs it on each
  completed path before the clause conjunctions split, once per path and
  its logical children, with no automatic attempt; `verex_on` then routes
  the clauses. `verex_group observations := assert_1, assert_2` joins
  clauses with identical contexts into one conjunction; each original
  obligation receives its projection. `verex_group_contexts` groups the
  remaining obligations whose contexts agree up to local names.
- `verex_on | ensures_1, aborts_if_2 => tac | _ => tac'` proves each
  clause by the first alternative naming it, by its tag or its key
  (`ensures 1`, `invariant 2 step`, `requires 1 of g at call 2`; a key
  without a part, such as `requires of g`, takes every leaf it matches). An
  alternative runs without
  recovery, so a failure is reported at its failing step and a bullet that
  leaves its goal open is reported as such; split cases with `rcases … with
  a | b` and `all_goals` rather than bullets. `verex_decide` tries the
  bounded deciders on every path and fails where one stays open;
  `verex_solve` decides a leaf as the automatic verification does, or
  reports its clause. A failed automatic verification prints the proof it
  asks for: `verex_obligations`, then `verex_on` with one alternative per
  undecided leaf and `_ => verex_decide`.
- As Lean's tactics do, each acts on the main goal and keeps the others;
  `verex_bounded`, `verex_inheriting`, `verex_settle`,
  `verex_saturate_round` and `verex_canonical_families` act on every goal.

A leaf of the automatic verification has its own budget, `pragma
leaf_heartbeats = N` on the spec block or `verex.leafHeartbeats`, in
thousands of heartbeats, twenty million by default; one cap covers the
complete attempt (call observations, deciders, preparation, fallback),
past which the leaf is reported for the proof file
([`proof-strategy.md`](proof-strategy.md), P3). An authored proof hands
every obligation to its script without the automatic leaf solver; the leaf
cap applies there only under an explicit `verex_solve`, while the function
budget bounds the whole proof.

The integer bounds a specification names, `MAX_U8` … `MAX_U256`, `MAX_I8`
… `MAX_I256`, `MIN_I8` … `MIN_I256`, are Lean constants of type `Int`
(`VerexSL/Bounds.lean`), so `‹x.val ≤ MAX_U64›` finds `x.val ≤
18446744073709551615`; `omega` reads a constant as an atom, `unfold
MAX_U64 at *` gives it the value.

### The editor

In the editor, an open file's `verify` items and automatic targets are
skipped with a message, except `verify! f` items (`verex.skip_verify`:
unset, it holds in the language server only, which the command line tells
apart by `internal.cmdlineSnapshots`; the driver sets it off). The code
action "Verify `f` here" marks an item so, "Skip `f` in the editor"
unmarks it; `lake test` runs every item.

A proof file opens in the Lake workspace at the root of its tree
(`aptos-move/framework`, `move-prover/tests/sources`, `verex-e2e-tests`),
which requires `verex-move`. Its header loads the module's context from
the latest rendering of its Move file or of its package. A source changed
since the rendering warns at the header; after verifying the source again,
restart the file to load the new rendering. An edit re-elaborates `verex
proofs where` alone. A message about the rendering is reported at the
proof file's item that raised it, or at `verex proofs where`
(`verex.relocateMessages`, which the driver turns off to report it in the
Move file).

The package tooling never compiles a proof file as a Lean-authored Move
module: `.proof.lean` files are not package sources and do not enter the
package digest (`move-command-line-common::files::is_lean_source`).

### Move frames

A VerexSL frame is closed: without `modifies` a function changes no global
memory. A function with a body that declares no contract (and plays no map
or table role) claims no frame: it is verified for the invariants it
writes, and its callers inline it. A Move specification frames only the
resource families it names targets for, which the Move frontend states
explicitly (`VerexMove/Frontend/Frames.lean`): no targets and a transitive
global write become `modifies *`; targets on a non-`opaque` function become
`modifies <targets>, *`.

### Move signers, hashes, serialization

The Move Prover assumes of every signer value that it signs the
transaction. Where the unit declares `std::signer`'s predicates, a Move
contract states it of the signers a function takes, as preconditions, and
of those it returns: `is_txn_signer(s)` and `is_txn_signer_addr` of its
address. A caller establishes it from its own precondition or the native's
contract; an address no signer in scope holds stays unconstrained.

The Move Prover assumes `hash::sha2_256` and `hash::sha3_256` injective,
which contradicts their 32-byte results. A Move contract states instead,
as preconditions, that no two of the function's byte-vector parameters
collide under a hash the function applies, in code or in a specification,
at any depth. A hash of bytes a function computes is not covered.

The Move Prover's prelude states of `bcs::serialize`, for every type, that
it is injective and never empty, and that all addresses serialize to one
length. A Move contract states these as preconditions of the function's
parameters where it applies `bcs::serialize` or `bcs::to_bytes` at any
depth (`Contract.serializeFacts`): each parameter's serialization is not
empty, two parameters of one type with equal serializations are equal, and
two address parameters serialize to one length.

### Move vector intrinsics

The Move Prover's prelude defines the `std::vector` functions the library
marks `pragma intrinsic`, and verifies none of their bodies. The exchange
reads them likewise, in code and in specifications, as the VIR operations
their bodies compute, with the library's abort codes: `is_empty`,
`contains`, `index_of`, `remove`, `reverse`, `reverse_slice`, `append`,
`reverse_append`, `trim`, `trim_reverse`, `insert`, and `remove_value`.
Their bodies are loops without invariants, or move ranges VIR has no
operation for. `swap_remove`, `rotate`, and `rotate_slice` keep their
bodies, which are free of loops once these are operations.

## Mapping messages

The shared driver is `VerexSL.SourceVerify` in `verex-ir`. The rendering
is elaborated in the command's own process, so the elaborators run
natively, and a message is mapped by the first rule that applies:

1. A message in a proof file is at its own position. A position in the
   items spliced from a specification file maps line by line to that file.
2. Otherwise the source's validated namespace and the one elaborated from
   the rendering are aligned: declarations by name, then node by node while
   the two shapes agree (expressions, patterns, places, and contract
   clauses by kind and occurrence). The position takes the source range of
   the innermost aligned node around it. A contract's location is its whole
   `spec` item, so a failed function's summary lands on the source's `spec`
   block.
3. A position outside every aligned node stays in the generated file.

## Budgets

Every target runs within a heartbeat budget (`verex.verifyHeartbeats`, in
`maxHeartbeats` units, 1000000 by default: about thirty seconds at the 32M
heartbeats a second the elaborator sustains). `prove --lean --heartbeats N`
and `verex-move verify --heartbeats N` set the default for a run; a target
that needs more raises it for itself with `pragma heartbeats = N;` in its
Move spec block (a pragma the Move model accepts on functions and modules;
the Move Prover ignores it) or `pragma heartbeats = N` in VerexSL, or gets
a proof. `N` counts thousands of `maxHeartbeats` units: the default is
`1000`.

`verify --ignore-proofs` (`prove --lean --ignore-proofs`,
`verex.ignoreProofs`) verifies every function automatically, its authored
proof and `pragma verify = manual` ignored: it tells whether a proof is
still needed. The stage log (`verex.stageLog`) records each target's
heartbeats, its outcome (`verified`, `failed`, `budget`) and whether its
proof was authored.

## Parallel targets

The targets of a module are proved several at a time, `verex.jobs` of
them (`verex-move verify --jobs N`; half the hardware threads by default).
The command first prepares every target in order: its certificates,
contracts and the definitions its proof names, elaborated and
kernel-checked while no proof runs, since every kernel check the command
requests queues behind the proofs already running. Then it elaborates each
target's typed theorem, whose proof elaborates in a task of its own
(`Elab.async`); a target begins once its specified callees among the
module's targets concluded, callees before callers. Once a proof ended, its
target concludes on the command's thread: the task's messages are
reported, a failure raised, the theorems carrying the proof to the runtime
elaborated, and the artifacts audited. Each target's messages are kept
until the module's targets concluded and then reported in the targets'
order, so the order in a baseline does not depend on the schedule.

A proof's task commits its theorem whether or not the proof succeeded, as
a `sorry` where it failed: each proof is an attempt of its own
(`f.typedVerified.attempt_1`, …), `f.typedVerified` is the attempt that
proved it, and a caller uses a callee's theorem only once the callee's
verification was audited, assuming its contract until then. The task
reports the proof's heartbeats on its thread, which the stage log and the
cost benchmark record.
