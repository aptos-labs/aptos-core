# Editing proof files in the editor

Option B is implemented for Move proof files (P1–P3; see
"Implementation"); Rust specification files (P4) and importable renderings
(P5, option C) are open. The design addresses parse errors, unresolved
names and missing InfoView goals when a companion file lacks its module context.

## Problem

A Move file's proofs live in `foo.proof.lean` beside `foo.move`, a Rust
file's specifications in `foo.spec.lean` beside `foo.rs`
(`source-verification.md`, "Proofs"). Such a companion file is a sequence of
VerexSL *items* (`verify f by …`, `theorem …`, `open …`), not a Lean
module: it has no `import`, and its items only mean something inside the
`verex module` the driver renders from the source. The driver gives them
that context by splicing the file's text, indented, into the rendering of
the source's module, and maps messages back line by line
(`VerexSL/SourceVerify.lean`, `compose`).

The Lean language server knows none of this. Opened in VS Code, the file
is elaborated as a bare Lean file:

1. **No Lake workspace.** The framework packages
   (`aptos-move/framework/*/sources/`), the Prover's test sources, and the
   paper examples have no `lakefile` or `lean-toolchain` above them, so the
   extension starts a server without the `VerexSL` search path, or none.
2. **No prelude.** Without `import VerexSL` the `verify` syntax and the
   `verex_*` tactics do not exist; the file does not even parse.
3. **No module context.** The items name `f`, its parameters, specification
   functions, and the module's other declarations. These exist only in the
   environment after the rendering of `foo.move` *and of every module it
   uses* has been elaborated (registration, certification), which needs the
   exchange export of the package (`move prove --lean`, a Cargo-built CLI).
4. **Cost.** The context is expensive (lowering plus certification: seconds
   for a small module, tens of seconds for `pool_u64`), and the server
   re-elaborates whatever a change invalidates.

A rendered module is one Lean command: `verex module P where` followed by
its indented items (`VerexSL/Syntax.lean`). Its elaborator
(`elaborateNamespaceWithVerification` in `VerexSL/Verify.lean`) does in one
pass what the items need: it registers and certifies the module, elaborates
its theorems, verifies its lemmas, and verifies its targets. The targets
are the `verify` items, callees before callers and the members of a cycle
of calls together, each with its script, then the specified functions no
item names, automatically. The spliced items are inside that command, and
the command has no incremental reuse: Lean re-elaborates it as a whole.

A lakefile alone fixes 1 and 2. The design question is 3 and 4: how an
editor gets the module context of a companion file, and how the file keeps
one meaning between the editor and the driver.

## What the editor needs

- Goals of a `verify f by …` step in the InfoView, at the cursor, in the
  companion file's own coordinates.
- Messages (unsolved goals, failing tactics) at their positions in the
  file, as the driver already reports them.
- The usual editor behavior of VerexSL modules: `verify` skipped unless
  `verify!` (`verex.skip_verify`), the "Verify `f` here" code action.
- A fresh context: edits to `foo.move` (or its dependencies) must reach the
  editor without hand steps beyond a re-run that is already part of the
  workflow.
- One meaning: the driver and the editor must elaborate the same items in
  the same context. A file the editor accepts must verify from the command
  line and vice versa.

## Options

### A. Session file: edit the generated rendering

The driver already writes the rendering with the spliced items
(`foo.move.lean`, `build/verex-verify.lean`). Give that output a Lake
workspace (a generated `lakefile.toml` beside it requiring `verex-move` by
path) so it opens with a working InfoView, and add the reverse step:
`verex-move verify --extract` copies the spliced region back into
`foo.proof.lean`.

- Mechanics: nothing changes in the language or the driver's model; a
  generated lakefile and a text extraction keyed on the splice lines the
  driver records.
- Cost in the editor: the whole rendering elaborates on open (all modules
  in dependency order). The splice is inside its module's command, so an
  edit to a proof re-elaborates that whole module (lowering, certification,
  theorems: tens of seconds for `pool_u64`). Lean reuses only the modules
  before it.
- Weaknesses: two copies of the proof, the authored one and the session
  one, with the user responsible for extracting; edits above the splice are
  lost on the next render; the InfoView shows generated coordinates. This
  is the hand workflow made repeatable, not a solution.

### B. Self-describing companion: a header that loads the context in process

Make the companion a Lean file with a header that establishes its own
context:

```lean
import VerexMove.Proofs                 -- VerexSL syntax, tactics, the Move profile
verex proofs for "pool_u64.move"        -- loads the module's context
verex proofs where                      -- the items, as companions have them today

verify buy_in by
  …
```

The approach rests on one change to the module elaborator: registration and
verification become separate steps.

- **Registration** lowers, registers, and certifies the module.
- **The module's verification step** is everything after that: theorems,
  lemmas, and targets, over the module's own items plus any items given to
  it.

An authored module runs both steps in its command, as now. A rendering is
elaborated with verification deferred (`verex.deferVerification`), so its
modules are only registered, and each module's verification step then runs
from a separate command. Without the split, a proof placed after its module
would come too late. The module command would already have verified `f`
automatically, at the full budget and usually failing, because no item in
the module names `f`.

`verex proofs for` is a VerexSL command:

1. It locates the rendering of the named source: the `.move.lean` the driver
   writes beside a single source, or the package rendering
   `build/verex-verify.lean`.
2. It elaborates the named module, and the modules before it in dependency
   order, into the current environment, with verification deferred.
   It does nothing when the module is already registered.
3. It reports any message raised by the rendering's commands at the header
   line, since the rendering is not the companion's text.

`verex proofs where` then runs the module's verification step with the
items that follow as additional items. It parses them as a module's items
are parsed (`manyIndent`, which holds every item to the first item's
column), so they stay at column 0 as companions have them now; no item is
indented. The step treats them exactly as it
treats spliced items today: with the module as their namespace, its `open`
items, and the order and cycles above.

The driver runs the same commands:

1. Render with verification deferred.
2. Run each companion file; its header finds the module already registered.
3. Run the verification step, with no additional items, for every module
   without a companion.

The header must name the companion's own source, which the driver checks.

Items as separate top-level commands would re-run only the edited item. The
existing `verify path by …` command (`verexVerifyCommand`) already verifies
one function of a registered unit. But the verification step needs all of a
module's items at once:

- callees before callers;
- a cycle's members together, each with its own script;
- the unnamed functions verified after the named ones.

One items command keeps that.

- Workspace: a committed `lakefile.toml` and `lean-toolchain` at the root
  of each tree of Move packages that carries proofs
  (`aptos-move/framework/`, `third_party/move/move-prover/tests/sources/`,
  the paper examples), each requiring `verex-move` by relative path. The
  extension serves a file from the nearest lakefile above it, and `lake
  setup-file` resolves the imports of a file that belongs to no library.
- Rendering: the header loads a rendering written without the
  companions; the driver writes one so from P3 on (today's rendering
  carries the spliced items of the last run).
- Freshness: the header elaborates whatever rendering is on disk. The
  rendering records a hash of the exported sources; a mismatch is a warning
  at the header ("rendering older than `pool_u64.move`; run `move prove
  --lean`"). A variant has the header run the export itself through the CLI
  (`APTOS_MOVE_CLI`, as the e2e driver does); it is slower on open, spawns
  Cargo-built processes from the server, and needs the CLI configured per
  machine. Keep it as an opt-in (`verex.exportOnOpen`).
- Cost:
  - The context elaborates once per open, since Lean reuses the header
    command's snapshot while the header is unchanged. Nothing is cached
    across opens.
  - An edit to the items re-elaborates the items command only. In the
    editor `verex.skip_verify` leaves that command the module's theorems,
    its lemmas, and the `verify!` items.
- One meaning: by construction, the driver runs the same commands.
- Changes:
  - the registration/verification split;
  - the `verex proofs for` and `verex proofs where` commands, in
    `verex-ir`, with a frontend hook that finds the rendering of a source
    (Move and Rust each know their layout);
  - the driver rewritten from splicing to the sequence above;
  - message mapping rule 1 (splice lines) retired;
  - the lakefiles;
  - a header on every existing companion. This is a one-time migration
    the driver can check: a companion without a header is an error naming
    the lines to add.

### C. Generated Lake package: the rendering as importable modules

As B, but the rendering is persisted as Lake modules whose `.olean`s carry
the registered units (`VerexSL/Registry.lean`'s environment extension is
importable), so a companion imports its module's rendering instead of
re-elaborating it:

```lean
import Verex.Gen.AptosStdlib.pool_u64   -- the rendering, built by Lake
verex proofs for 0x1::pool_u64
verex proofs where

verify buy_in by …
```

`move prove --lean` (and `verex-move verify`) write the rendering as one
module per Move module under `build/verex/`, and the tree's lakefile
declares that directory as a `lean_lib`. Opening the companion makes `lake
setup-file` build the imported modules; a second open, or a second
companion of the same package, pays nothing. The `verex proofs for`
command then only selects the registered namespace.

- Freshness: as B, through the hash; or, with a `lakefile.lean`, a custom
  target that runs the export when the Move sources' trace changes, so
  Lake itself regenerates on open. The latter makes the editor
  self-sufficient but couples Lake to the Cargo-built CLI, as the e2e
  driver's MonoVM target already does.
- Cost: registration and certification once per module change, shared by
  every open and by the driver (the driver can import the same `.olean`s
  rather than re-elaborate: the certification phase, 22 s of the 107 s in
  the design's timing example, would be paid once per source change).
- One meaning: the driver imports the same modules and runs the same
  command.
- Changes beyond B: the rendering split into modules with stable names
  (addresses are not Lean identifiers; a naming scheme such as
  `Verex.Gen.<package>.<module>`), the generated `lean_lib`, dependency
  order as module imports, and the generated modules built with
  verification deferred (B's split; the verification step is the driver's
  to run). The verification step must then work from what the `.olean`
  carries. The syntax of the rendered items is not persisted, so the
  step takes its targets from the registered unit, and messages about
  them take their positions from the unit's source ranges.
- Risk: generated Lake modules in `build/` of every Move package, and a
  build directory the extension builds into while the driver also writes
  it. Lake's locking covers concurrent builds; the driver must not write a
  module Lake is building.

### D. Custom server: preprocess the document

Keep companions as they are and teach the editor the splice: a language
server (or a VS Code extension with LSP middleware over the lean4 client)
that, for a `.proof.lean` document, sends the Lean server the composed
rendering with the file's text spliced in, and translates positions in
both directions by the splice lines, as the driver maps messages.

- Gives the InfoView without a header, exactly the driver's semantics.
- Needs a program of our own between VS Code and `lean --server`: either a
  wrapper server that owns the `textDocument/*` traffic for companion
  documents (every request type the InfoView uses needs translating:
  diagnostics, hover, goals, code actions, semantic tokens), or an
  extension that injects middleware into the lean4 extension's client,
  which that extension does not expose as an API. Both are fragile against
  lean4 extension and server changes and a separate deliverable to
  install.
- Not recommended unless the header of B/C is unacceptable.

### E. No InfoView: obligations as diagnostics

Treat the Move file as the editor's document: a Move language extension
runs `move prove --lean` and shows its messages, and `verify f by
verex_obligations; all_goals verex_prepare` prints the goals as unsolved
goals at the item, as the design already describes. This is the Move
Prover's workflow; it answers "what is left to prove" but not "what is my
goal at this tactic". It does not meet the need and is listed to mark the
boundary.

## Cross-cutting

- **Rust specification files.** `foo.spec.lean` is spliced the same way
  and has the same editor problem, but its items do not fit B's split. A
  `spec f …` item is part of what the module registers: contracts are
  attached to their functions before lowering and are validated with the
  unit (`attachContracts` in `VerexSL/Elab.lean`). So a header cannot load
  the module from a rendering without them, and the rendering cannot
  include them either, because the editor's copy of the file is the one
  being edited.
  - Under B/C a `verex specifications for "foo.rs"` header would have to
    register the module itself, with the items that follow. Every edit
    would then pay registration and certification of that module, which is
    A's cost.
  - Making contracts attachable after registration, as a separate
    validated step, is a design question of its own.

  Whichever option is chosen must cover both companions, or the Rust one
  is explicitly deferred.
- **Skipping in the editor.** `verex.skip_verify` applies unchanged: a
  companion's `verify` items are skipped in the server and `verify!` runs,
  as in authored modules. A companion opened in the editor should be cheap
  to edit; the context load is the one-time cost.
- **Message mapping.** With B/C, companion positions are native and rule 1
  of "Mapping messages" disappears; the alignment of the rendering to the
  Move file (rule 2) stays for messages about the source.
- **Header checking.** The driver must refuse a companion whose header
  names another source, and should refuse a companion without one once the
  migration is done, so the file cannot mean one thing to the editor and
  another to the driver.
- **Lakefile placement.** Lake requires the lakefile above the file; one
  per tree of Move packages (three in this repository today) is the
  smallest set. The verex packages require each other by path only, with
  no git dependencies. A tree's lakefile that requires `verex-move` by
  path therefore reuses that package's own build, and each tree commits
  its `lake-manifest.json`. Their `.lake/` directories are ignored.
  Alternatively one lakefile per Move package, generated by `move prove
  --lean`: more files, no central knowledge of where proofs live.

## Recommendation

B, with C as its second step once B works: B is the smallest change that
gives a self-describing companion, one semantics for driver and editor, and
a working InfoView, and C only replaces how the header finds its context
(re-elaborate a rendering versus import its `.olean`), which is the cost
lever. The split of registration from verification is what B and C share,
and it lands first. A is a stop-gap that can be offered immediately (a
generated lakefile beside the rendering) while B is built, at a module's
elaboration per edit; D and E are rejected. Rust specification files need
their own answer (see "Cross-cutting").

## Implementation

Option B, as P1–P3 describe it, with these decisions:

- **Renderings.** `verex-move` places them; the Prover leaves the place to
  it. A package renders to its `build/verex-verify.lean` (`move prove
  --lean`, `verex-move verify <package>`). A Move file verified on its own
  renders to `.verex/foo.move.lean` beside it (`verex-move verify`, the
  Prover's `--lean` and its tests, the e2e driver), a hidden directory so
  the rendering is not taken for a source (user decision, 2026-10-12), and
  `.verex/` is ignored. A file inside a package renders into the package's
  build directory instead, `build/verex/<path>.lean`, since the package
  elaborates the Lean files under its sources, hidden directories included.
  The header loads the latest of the file's and the package's rendering that
  renders the module. A rendering records
  its sources relative to the directory it was made in, with the hash of
  each one's content (`verex sources at`); the header warns where a source
  changed.
- **Deferral.** `VerexSL.Verify.Deferral` says which module commands only
  register (`none`, `all`, or those whose `proof_file` pragma names one of
  the files). The driver defers the modules whose proof file exists, the
  header every module of the rendering it loads.
- **One context per proof file.** The driver elaborates each proof file over
  the state its module's command left, not over the states other proof files
  leave: a proof file sees its module and the modules rendered before it, as
  in the editor, and no other proof file's theorems or `vir_denote_norm`
  lemmas.
- **Files of messages.** The verification step mixes the rendering's items
  and the proof file's, so every target, script, theorem, and `open` item
  carries the file it was parsed from (`ParsedFile`) and is verified with
  that file's positions. The driver maps a message by its file; a document
  shows a message about another file at the item that raised it, or at
  `verex proofs where`, opened by its position there
  (`verex.relocateMessages`, which the driver turns off).
- **Grammar.** A proof file is its header, `verex proofs where`, and items
  that are `verify` items, theorems, or `open` items. Another item, a
  `verify` command before `verex proofs where`, or a header naming another
  Move file is an error, in the editor and the driver alike; the driver also
  rejects a file of another shape.
- **Workspaces.** `aptos-move/framework`, `move-prover/tests/sources`, and
  the paper's examples have a `lakefile.toml` requiring `verex-move` by path
  and its `lean-toolchain`; `verex-e2e-tests` serves its own fixtures.
- **Cost.** With the `verify` items skipped, `pool_u64.proof.lean` opens in
  2.3 s and `ordered_map.proof.lean` in 4.0 s, the rendering's modules
  registered included. Elaborated on its own with every item verified, a
  proof file reports what the driver reports (the e2e `<name>.proof.exp`
  baselines).
- **Tools.** `scripts/isolate-target.py --proofs <proof file>` makes the
  target's proof, the theorems, and the `open` items items of the isolated
  module again.

## Milestones

- **P1 (done).** Lakefiles at the three trees; `verex-move verify` writes a
  rendering hash. A `.proof.lean` opens with `VerexSL` resolved (parses,
  tactics known) but still without context.
- **P2 (done).** Registration and verification split. The module command runs
  both steps; with `verex.deferVerification` it only registers, and the
  module's verification step is a function a later command calls with
  additional items. The driver is unchanged, still splicing. Gate: the
  four suites, the Prover's `.lean_exp` baselines, and
  `DenotePerformance.exp` unchanged.
- **P3 (done).** The `verex proofs for` and `verex proofs where` commands. The
  driver writes the rendering without companions, elaborates it with
  verification deferred, and runs the companions and the remaining
  verification steps instead of splicing. All Move companions gain the
  header, and message rule 1 is retired for them. Gate:
  `SourceVerify/proofs.move` and the framework's proof files verify
  unchanged; opening `pool_u64.proof.lean` shows goals under
  `verify! buy_in by …`, and an edit there does not re-certify the module.
- **P4.** Rust `.spec.lean`: a design for contracts attached after
  registration, or an explicit deferral (see "Cross-cutting").
- **P5 (C).** The rendering as Lake modules under `build/verex/`; companion
  imports; the driver imports the built modules. Gate: the certification
  phase of a second run with unchanged sources is zero.

## Open questions

- Should the header be the file's source of truth for *which* module it
  belongs to, replacing the file-name convention (`foo.proof.lean` beside
  `foo.move`), so a module may keep its proofs in a differently named or
  shared file? The convention currently refuses a source file with several
  modules; a header per module would lift that.
- Whether the header may run the export (the CLI) from the server, or only
  consume a rendering produced by a command the user runs.
- Under C, whether the driver and the editor should share `build/verex/`
  or the driver keep its own copy, given concurrent writes.
