# Editing proof files in the editor

Status: options, no decision (2026-10-12). Raised in `handoff.md` ("Open
findings not yet in a design"): opening `foo.proof.lean` in VS Code shows
parse errors, unresolved names, and no goals in the InfoView.

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
  in dependency order); edits inside the splice region re-elaborate only
  what follows them, by Lean's snapshot reuse.
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

verify buy_in by
  …
```

`verex proofs for` is a VerexSL command. It locates the rendering of the
named source (the `.move.lean` the driver writes beside it, or the package
rendering under `build/`), elaborates it into the current environment
(`processCommands` over the rendering's commands, as the driver does), and
opens the module: the items that follow are elaborated exactly as the
spliced items are today, with the module as their namespace. The driver
elaborates the companion the same way instead of splicing: render, then run
the companion file over that environment. The header lines are what the
driver skips or checks (the name must be the companion's source).

- Workspace: a committed `lakefile.toml` and `lean-toolchain` at the root
  of each tree of Move packages that carries proofs
  (`aptos-move/framework/`, `third_party/move/move-prover/tests/sources/`,
  the paper examples), each requiring `verex-move` by relative path. The
  extension serves a file from the nearest lakefile above it, and `lake
  setup-file` resolves the imports of a file that belongs to no library.
- Freshness: the header elaborates whatever rendering is on disk. The
  rendering records a hash of the exported sources; a mismatch is a warning
  at the header ("rendering older than `pool_u64.move`; run `move prove
  --lean`"). A variant has the header run the export itself through the CLI
  (`APTOS_MOVE_CLI`, as the e2e driver does); it is slower on open, spawns
  Cargo-built processes from the server, and needs the CLI configured per
  machine. Keep it as an opt-in (`verex.exportOnOpen`).
- Cost: the context elaborates once per open, since the header command's
  snapshot is reused while the header is unchanged; every edit below it
  costs what the splice costs today. Nothing is cached across opens.
- One meaning: by construction, the driver runs the same command.
- Changes: a `verex proofs for` command (in `verex-ir`, with a frontend
  hook that finds the rendering of a source: Move and Rust each know their
  layout), the driver rewritten from splicing to running the companion over
  the rendered environment, message mapping rule 1 (splice lines) retired,
  the lakefiles, and a header on every existing companion (a one-time
  migration the driver can check: a companion without a header is an
  error naming the lines to add).

### C. Generated Lake package: the rendering as importable modules

As B, but the rendering is persisted as Lake modules whose `.olean`s carry
the registered units (`VerexSL/Registry.lean`'s environment extension is
importable), so a companion imports its module's rendering instead of
re-elaborating it:

```lean
import Verex.Gen.AptosStdlib.pool_u64   -- the rendering, built by Lake
verex proofs for 0x1::pool_u64

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
  order as module imports, and the `verify` items of the rendering skipped
  when building the `.olean` (they are the driver's to run; the generated
  modules carry registration only).
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
  and has the same editor problem. Under B/C the header is `verex
  specifications for "foo.rs"`; its items (`spec f …`) attach contracts to
  the rendered module rather than prove its functions, which the command
  must allow before verification. Whichever option is chosen must cover
  both companions, or the Rust one is explicitly deferred.
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
  smallest set. Their `.lake/` directories are ignored. Alternatively one
  lakefile per Move package, generated by `move prove --lean`: more files,
  no central knowledge of where proofs live.

## Recommendation

B, with C as its second step once B works: B is the smallest change that
gives a self-describing companion, one semantics for driver and editor, and
a working InfoView, and C only replaces how the header finds its context
(re-elaborate a rendering versus import its `.olean`), which is the cost
lever. A is a stop-gap that can be offered immediately (a generated
lakefile beside the rendering) while B is built; D and E are rejected.

## Milestones

- **P1.** Lakefiles at the three trees; `verex-move verify` writes a
  rendering hash. A `.proof.lean` opens with `VerexSL` resolved (parses,
  tactics known) but still without context.
- **P2.** The `verex proofs for` command; the driver runs companions over
  the rendered environment instead of splicing; all companions gain the
  header; message rule 1 retired. Gate: `SourceVerify/proofs.move` and the
  framework's proof files verify unchanged; opening
  `pool_u64.proof.lean` shows goals under `verify! buy_in by …`.
- **P3.** Rust `.spec.lean` through the same command.
- **P4 (C).** The rendering as Lake modules under `build/verex/`; companion
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
