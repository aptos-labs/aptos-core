This file provides guidance to Claude Code when working on this paper.
For general Move Prover guidance see `../../CLAUDE.md`.

## Directory layout

LaTeX source for the FMCAD'26 submission *Formal Verification of
Imperative Higher-Order Functions* (Grieskamp, Zhang, …; Aptos Labs).
Outline in `plan.md`.

- `main.tex` — document root; `\input`s the section files.
- Sections: `intro.tex`, `move.tex`, `encoding.tex`, `validation.tex`,
  `conclusion.tex`.
- `prelude.tex` — packages; `\MVP` / `\MSL` / `\WP` / `\Section` /
  `\Paragraph` / `TODO` macros; the `Move` / `MoveBox` /
  `MoveBoxNumbered` / `MoveDiag` / `ivl` `lstlisting` environments;
  the `WideFigure` / `WideTable` two-column floats. `|~` in Move
  listings renders as `⊨` via the `literate` table — do not try to
  typeset it as text.
- `biblio.bib` — bibliography; style is `IEEEtran` via `fmcad.cls`.
- `fmcad.cls`, `esz.sty` — vendored conference template and Z-style
  notation. Do not edit unless the upstream needs fixing.
- `examples/` — self-contained Move package used as source of truth
  for the paper listings (`amm_example.move`, `find.move`,
  `vault.move`). Edit the Move source first, then re-copy snippets
  into the `.tex` — don't edit listings in place. Move work in this
  subtree is routed through the `move-flow` skills/agents
  (`/move-inf`, `/move-prove`, `/move-test`, `/move-check`, `/move`);
  use those instead of calling `move_package_wp` /
  `move_package_verify` MCP tools directly.

## Formatting

One sentence per line in `*.tex` files; no hard wraps inside a
sentence. (Not enforced for `*.md`.)

## Building

```
latexmk main.tex
```

Configuration is in `latexmkrc`; intermediate files go to `build/`.

## Running the AMM example through the prover

The AMM pushes the solver hard; use `--split-vcs-by-assert`:

```
cargo run -p move-prover -- \
    --language-version 2.4 \
    -d third_party/move/move-stdlib/sources \
    -a std=0x1 \
    --split-vcs-by-assert \
    third_party/move/move-prover/doc/higher-order-paper-26/examples/sources/amm_example.move
```

`--language-version 2.4` is required for `@`-state labels and the
`|~` state-quantification syntax. Without `--split-vcs-by-assert` the
module runs ~85s single-core and barely fits the default 40s budget.

## `examples/sources/amm_example.move` overview

`Pool` stores its pricing curve as a `has copy + store` closure; the
spec asserts the pool-level invariants (no-abort, output ≤ output
reserve, monotonicity, constant-product preservation), so pack time
is the proof-obligation point. Three pricing implementations:

- `constant_product` — fee-free, satisfies every invariant; used by
  `create_constant_product_pool`.
- `constant_product_with_fee_non_compliant` — reads `Fee[client].bps`
  directly, aborts when `Fee` is missing; used by
  `create_noncompliant_fee_pool` to demonstrate the *expected*
  `requires` violations (correct diagnostics, not timeouts).
- `constant_product_with_fee` — compliant fee variant; defaults to
  `DEFAULT_FEE_BPS` (500) when `Fee` is missing or out of range,
  returns 0 when the fee consumes the whole input. Verified via a
  `split amount_in == 0` proof hint in `create_compliant_fee_pool`.

`swap` is generic over the stored closure and verifies from the Pool
invariants alone.

## Editorial conventions in listings

- `|~` binds **weaker** than `==>` and every logical/relational
  operator: `S.. |~ a ==> b` parses as `S.. |~ (a ==> b)`. Keep the
  paren-free style; do not "clarify" by adding parens.
- In the Pool spec, the no-abort invariant is stated **first**.
  Order matters for Z3 heuristics — this ordering verifies `swap`
  reliably without requiring the pricing function's nonlinear CP
  ensures to be emitted to callers.
- `constant_product_with_fee`'s spec includes the CP-preservation
  inequality directly as `ensures`, so `create_compliant_fee_pool`
  discharges the corresponding `requires` without re-deriving it
  through the fee-adjusted input formula. The function's own
  verification uses `split amount_in == 0` to split the nonlinear
  body's VC into two linear obligations.

## `esz.sty` cheat sheet

The conference template loads `esz.sty` (Z-style notation). The
paper uses only a small subset:

- **Keyword builders** (used in `prelude.tex` to define the IVL
  vocabulary): `\Zkeyword{x}` (bold), `\Zpreop{x}` (sans-serif
  prefix op), `\Zinrel{x}` (sans-serif infix relation). The IVL
  keywords (`\IF`, `\ELSE`, `\LET`, `\HAVOC`, `\ASSERT`, `\ASSUME`,
  `\PROC`, `\FUN`, `\AXIOM`, `\DATATYPE`, …) are built on these.
- **Tab indents** inside `zed`/`ivl`: `\t{n}` inserts `n × \zedtab`
  of horizontal space — use `\t1`, `\t2`, `\t3` at the start of a
  line after `\\` to step indentation.
- **Active characters in math mode** (re-activated by `esz.sty`):
  `;` appends a thick space, `@` becomes `\bullet` (used in
  `\forall x @ …`), `|` becomes `\mid`, `~` becomes a thin space.
- **Symbols used in the paper**: `\fun` (→), `\nat`, `\power`,
  `\cross`, plus standard `\forall`, `\exists`, `\iff`, `\land`,
  `\lnot`.

The full set of esz macros is documented in `esz.sty` itself —
reach for them only when extending notation.
