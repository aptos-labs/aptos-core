# *WIP* Leaner

This experimental project is a language-neutral intermediate representation
(LIR) in Lean 4 with semantic profiles for Move and Rust, a Lean-authored
source language over it (LeanerLang), a verifier that proves contracts
against the source semantics, and frontends from Move and Rust.

A module is ordinary Lean syntax. `spec` attaches a contract and `verify`
turns it into a theorem checked by the Lean kernel:

```lean
leaner module 0x42::account where
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
   semantics of the validated LIR of `f`
   ([`designs/denotation.md`](designs/denotation.md)).
2. **Compilation.** Supported declarations lower to XIR and production Move
   bytecode.
3. A compiler-correctness theorem connecting the two is future work.

A Move file, or a Rust file with its specifications in a LeanerLang file
beside it, is verified by rendering it as LeanerLang, with every message
reported at its position in the original files
([`designs/source-verification.md`](designs/source-verification.md)):

```bash
leaner-move verify vault.move
leaner-rust verify bounds.rs      # specifications in bounds.spec.lean
```

## Packages

| Package | Purpose |
|---|---|
| [`leaner-ir`](leaner-ir/) | The shared typed LIR: RawUnit JSON import, validation, semantics, interpreter, proofs, the denotation-based verifier, and LeanerLang. |
| [`leaner-move`](leaner-move/) | The Move semantic profile and intrinsic registry, the Move exchange frontend (`LeanerMove/Frontend`), and the XIR backend (`LeanerMove/Xir`) through which compiler-v2 compiles `.lean` sources to Move bytecode. |
| [`leaner-rust`](leaner-rust/) | The Rust semantic profile, source backend, and the Lean-owned import CLI over [`rust-exporter`](leaner-rust/rust-exporter/), a standalone Rustc Public exporter of borrow-checked MIR. |
| [`leaner-e2e-tests`](leaner-e2e-tests/) | Move-to-LeanerLang and Rust-to-LeanerLang baselines, verification of Move and Rust sources, and the verification check fixtures (ledger in [`designs/roadmap.md`](designs/roadmap.md#tests)). |

[`v0/`](v0/) holds the deprecated reference packages `move`, `move-model`,
and `transpiler`; nothing links them. Designs live in [`designs/`](designs/), with
executed or superseded ones under [`designs/historical/`](designs/historical/);
[`designs/roadmap.md`](designs/roadmap.md) holds the status: what is open and the test ledger.

## Build and test

Install the pinned Lean toolchain from the repository root:

```bash
./scripts/dev_setup.sh -p -l
source "$HOME/.profile"
```

Every package is an independent Lake package; build and test it from its
own directory:

```bash
(cd leaner-ir        && lake build && lake test)
(cd leaner-move      && lake build && lake test)
(cd leaner-rust      && lake build && lake test)
(cd leaner-e2e-tests && lake build && lake test)
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
cd leaner-move
lake exe leaner-move verify <file.move | package>        # every specified function
lake exe leaner-move verify <package> --filter vector    # modules whose file names contain it
lake exe leaner-move verify <file.move> --heartbeats 400 # default budget, thousands
cd ../leaner-rust
lake exe leaner-rust verify <file.rs>                    # specifications in <file>.spec.lean
```

From the Move CLI, a package is verified in place:

```bash
move prove --lean --package-dir <package>                # or: aptos move prove --lean
```

A function the automatic verification leaves open is proved in a proof
file beside its Move file; the failure message names the file, and
`verify f by skip` shows the obligations left:

```move
spec square_of_sum {
    pragma verify = manual;     // proved in proofs.proof.lean
    pragma heartbeats = 400;    // this function's budget, thousands
    ensures result == a * a + 2 * a * b + b * b;
}
```

```lean
-- proofs.proof.lean, beside proofs.move
verify square_of_sum by
  case leaf_1 => …
  case leaf_2 => …
```

The Move Prover reads `verify = manual` as true and ignores `heartbeats`.
Details: [`designs/source-verification.md`](designs/source-verification.md).
