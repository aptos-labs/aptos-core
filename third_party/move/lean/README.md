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

## Packages

| Package | Purpose |
|---|---|
| [`leaner-ir`](leaner-ir/) | The shared typed LIR: RawUnit JSON import, validation, semantics, interpreter, proofs, the denotation-based verifier, and LeanerLang. |
| [`leaner-move`](leaner-move/) | The Move semantic profile and intrinsic registry, plus the Move exchange frontend (`LeanerMove/Frontend`). |
| [`leaner-rust`](leaner-rust/) | The Rust semantic profile, source backend, and the Lean-owned import CLI over [`rust-exporter`](leaner-rust/rust-exporter/), a standalone Rustc Public exporter of borrow-checked MIR. |
| [`leaner-e2e-tests`](leaner-e2e-tests/) | Move-to-LeanerLang and Rust-to-LeanerLang baselines and the verification check ledger ([`designs/test-organization.md`](designs/test-organization.md)). |

`move`, `move-model`, and `transpiler` are deprecated reference packages;
nothing current links them. Designs live in [`designs/`](designs/), with
executed or superseded ones under [`designs/historical/`](designs/historical/);
[`designs/roadmap.md`](designs/roadmap.md) orders the work.

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
