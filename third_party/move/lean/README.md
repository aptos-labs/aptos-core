# *WIP* Verex

This experimental project is a language-neutral intermediate representation
(VIR) in Lean 4 with semantic profiles for Move and Rust, a Lean-authored
source language over it (VerexSL), a verifier that proves contracts
against the source semantics, and frontends from Move and Rust.

A module is ordinary Lean syntax. `spec` attaches a contract and `verify`
turns it into a theorem checked by the Lean kernel:

```lean
verex module 0x42::account where
  struct Balance has Key where
    value : u64

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let balance := &mut Balance[addr]
    *balance := new Balance { value := (*balance).value + amount }

  spec deposit where
    requires exists<Balance>(addr)
    ensures global<Balance>(addr).value == old(global<Balance>(addr).value) + amount
    aborts_if global<Balance>(addr).value + amount > 18446744073709551615
    modifies global<Balance>(addr)

  verify deposit
```

Three claims stay separate:

1. **Source verification.** `verify f` proves a theorem about the big-step
   semantics of the validated VIR of `f`
   ([`doc/designs/denotation.md`](doc/designs/denotation.md)).
2. **Compilation.** Supported declarations lower to XIR and production Move
   bytecode.
3. A compiler-correctness theorem connecting the two is future work.

A Move file, or a Rust file with its specifications in a VerexSL file
beside it, is verified by rendering it as VerexSL, with every message
reported at its position in the original files
([`doc/designs/source-verification.md`](doc/designs/source-verification.md)):

```bash
verex-move verify vault.move
verex-rust verify bounds.rs      # specifications in bounds.spec.lean
```

## Packages

| Package | Purpose |
|---|---|
| [`verex-ir`](verex-ir/) | The shared typed VIR: RawUnit JSON import, validation, semantics, interpreter, proofs, the denotation-based verifier, and VerexSL. |
| [`verex-move`](verex-move/) | The Move semantic profile and intrinsic registry, the Move exchange frontend (`VerexMove/Frontend`), and the XIR backend (`VerexMove/Xir`) through which compiler-v2 compiles `.lean` sources to Move bytecode. |
| [`verex-rust`](verex-rust/) | The Rust semantic profile, source backend, and the Lean-owned import CLI over [`rust-exporter`](verex-rust/rust-exporter/), a standalone Rustc Public exporter of borrow-checked MIR. |
| [`verex-e2e-tests`](verex-e2e-tests/) | Move-to-VerexSL and Rust-to-VerexSL baselines, verification of Move and Rust sources, and the verification check fixtures under `VerexE2ETests/Check/`. |

[`v0/`](v0/) holds the deprecated reference packages `move`, `move-model`,
and `transpiler`; nothing links them. Documentation is under [`doc/`](doc/): designs, the plan
([`doc/plan/roadmap.md`](doc/plan/roadmap.md) is what is open), status, and
historical designs.

## Build and test

Install the pinned Lean toolchain from the repository root:

```bash
./scripts/dev_setup.sh -p -l
source "$HOME/.profile"
```

Every package is an independent Lake package; build and test it from its
own directory:

```bash
(cd verex-ir         && lake build && lake test)
(cd verex-move       && lake build && lake test)
(cd verex-rust       && lake build && lake test)
(cd verex-e2e-tests && lake build && lake test)
```

The core libraries need no Aptos CLI. The end-to-end driver builds the
standalone Move exchange CLI itself (`cargo build --locked --profile ci -p
aptos-move-cli --features binary --bin move`); see [`CLAUDE.md`](CLAUDE.md)
for the `APTOS_MOVE_CLI` and `APTOS_CLI` contracts and the rest of the
working rules.

## Verifying sources

The verifiers read Move through the standalone Move CLI, which
`APTOS_MOVE_CLI` names (see [Build and test](#build-and-test)). Each run
reports messages at their positions in the source files and ends with the
wall time per phase:

```bash
cd verex-move
lake exe verex-move verify <file.move | package>         # every specified function
lake exe verex-move verify <package> --filter vector     # modules whose file names contain it
lake exe verex-move verify <file.move> --heartbeats 400 # default budget, thousands
lake exe verex-move verify <package> --jobs 4            # targets proved at once per module
cd ../verex-rust
lake exe verex-rust verify <file.rs>                    # specifications in <file>.spec.lean
```

`verex-move verify` elaborates a package's targets one after another in one
process. `scripts/verify-modules.py <package> [--filter <part>] [--jobs N]`
runs one process per module with specifications, several at a time, and
prints the modules' reports in module order.

From the Move CLI, a package is verified in place; the Move Prover's unit
tests run with it on request:

```bash
move prove --lean --package-dir <package>                # or: aptos move prove --lean
MVP_TEST_FEATURE=lean cargo test -p move-prover --test testsuite
```

A function the automatic verification leaves open is proved in a proof
file beside its Move file; the failure message names the file. The file's
header establishes the context of its module, and `verex_obligations`
splits the function's goal into its obligations, each named by the clause it
owes, which `verex_decide` and `verex_prepare`, or any Lean tactics, then
prove:

```move
spec square_of_sum {
    pragma verify = manual;     // proved in proofs.proof.lean
    pragma heartbeats = 400;    // this function's budget, thousands
    ensures result == a * a + 2 * a * b + b * b;
}
```

```lean
-- proofs.proof.lean, beside proofs.move
import VerexMove.Proofs

verex proofs for "proofs.move"
verex proofs where

verify square_of_sum by lean =>
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  case ensures_1 => …
```

The Move Prover reads `verify = manual` as true and ignores `heartbeats`.
In the editor, `verify` items and automatic targets are skipped (`verex.skip_verify`, on by
default in the language server only); `verify! f`, marked by the code action "Verify `f` here",
runs. A proof file opens with its goals in the InfoView once its Move file
has been verified: the header loads the rendering that verification wrote
(`.verex/` beside the file, `build/` of a package).
Details: [`doc/designs/source-verification.md`](doc/designs/source-verification.md).
