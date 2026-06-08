This file provides guidance to Claude Code when working on this paper.

## What this is

LaTeX source for *Runtime Safety in Move* (Gao, Grieskamp,
Kashyap, Mitenkov, Zhang, Zhou, Cappa, Illadi; Aptos Labs). The
paper describes the runtime safety guards that Aptos enforces on
top of Move's statically-verified bytecode --- a second line of
defense against verifier bugs, compromised bytecode, and any other
path by which ill-typed or unsafe code could reach execution.

Submission target: Springer LNCS proceedings (e.g. RV 2026, which
uses `llncs.cls`). The bibliography style is `splncs04`.

## Directory layout

- `main.tex` --- document root; `\input`s the section files.
- Section files, in the order `main.tex` inputs them:
  - `intro.tex` --- Introduction (`sec:intro`). Motivates the
    second runtime layer against verifier bugs; lists the three
    check groups (type, ability, reference) and points forward
    to trusted code and asynchronous checking.
  - `move.tex` --- Move on Aptos overview (`sec:move`). Bulleted
    summary of typing/generics, abilities, ADTs, references,
    global storage, and the static bytecode verifier.
  - `checks.tex` --- umbrella section "Runtime Safety Checks"
    (`sec:bugs`); `\input`s the four subsections below:
    - `type.tex` --- Type Safety (`sec:type`). Capability /
      faked-capability example, type-stack mechanism.
    - `ability.tex` --- Ability Safety (`sec:ability`).
      `copy`/`drop`/`key`/`store` checks; `drop` tracking at
      function exit.
    - `ref.tex` --- Reference Safety (`sec:ref`). Enum
      variant-swap example, the temporary-aliasing challenge,
      and the shadow-state / access-path-tree / poisoning
      description of the runtime checker.
    - `oom.tex` --- Liveness (`sec:bugs:oom`). Type-instantiation
      depth blow-up and the runtime depth/size bound.
  - `trusted.tex` --- Trusted Code (`sec:trusted`). Governance
    basis, what trusted code waives, and how the type stack is
    synthesised across trusted/untrusted boundaries.
  - `async.tex` --- Asynchronous Checking (`sec:async`).
    Block-STM context (cites `BlockSTM`), the trailed execution
    trace (ticks + fingerprint + branch bits + dynamic calls),
    and replay on transaction finalisation. Points at the
    implementation in `move-vm-runtime` (`execution_tracing`,
    `runtime_type_checks_async.rs`).
  - `perf.tex` --- Performance (`sec:perf`). Headline numbers:
    ~20% slowdown for type+ability, ~40% for reference safety;
    `\TODO` for benchmark setup and breakdown.
  - `concl.tex` --- Conclusion (`sec:concl`). Currently a
    related-work paragraph (JVM, CLR, sibling Move VMs;
    dynamically typed languages; EVM and SVM) and a
    forward-looking note on a new-VM redesign.
- `concepts.tex` --- bytecode syntax (`fig:syntax`) and small-step
  semantics (`fig:semantics`) figures, plus a "Concepts" prose
  subsection that walks through the per-instruction transitions
  and the call-frame / reference / path model used by the
  semantics. Currently **not** included from `checks.tex` (the
  `\input{concepts}` line is commented out); kept here for reuse.
  Edit with care --- the figure labels and notation feed any
  future section that re-enables it.
- `prelude.tex` --- shared packages and macros. Adapted from the
  inference-paper-26 prelude with two adjustments needed under
  `llncs`: an explicit `\usepackage{hyperref}` (fmcad loaded it
  implicitly; llncs does not) and a `\let\subparagraph\paragraph`
  bracket around the `titlesec` load to dodge the well-known
  titlesec / llncs `\subparagraph` clash. Defines the MASM
  bytecode mnemonics (`\MOVELOC`, `\COPYLOC`, `\BORROWLOC`,
  `\BORROWFIELD`, `\READREF`, `\WRITEREF`, `\PACK`, `\UNPACK`,
  `\BRTRUE`, `\EVAL`, `\CALL`, `\RET`), the `masm` / `masm_figure`
  / `MoveBox` environments, the `\sat` (`|~`) composite glyph
  used for state-labelled expressions in the Move listings, and
  the `\TODO{tag}{body}` macro.
- `biblio.bib` --- merged bibliography (originally from
  rv26-dynref and higher-order-paper-26, de-duplicated). The
  current `concl.tex` related-work paragraph cites `jvm`, `clr`,
  `OLD_MOVE_LANG`, `ethereum`, `kevm`, `rustbelt`,
  `stackedborrows`, and `treeborrows`.
- `llncs.cls`, `splncs04.bst`, `esz.sty` --- vendored class /
  bibstyle / Z-style notation. Do not edit unless upstream needs
  fixing.
- `latexmkrc` --- intermediate files go to `build/`; final PDF
  lands beside `main.tex`.

## Conventions you'll trip over

- `|...|` (and the alternative `!...!`) are short inline
  delimiters for Move source code, set up via
  `\lstMakeShortInline` in `prelude.tex`. That means a literal `|`
  inside math will be eaten by listings and produce a
  `\ttfamily invalid in math mode` error. Use `\lvert ... \rvert`
  (or a named macro) instead of `|x|` in math.
- The `masm` and `masm_figure` environments are thin wrappers
  around `esz.sty`'s `zed` environment. The latter sets up a
  three-column halign, so nesting a single `\begin{array}` works,
  but mixing two arrays or general math inside one `masm` does
  not align independently (see `concepts.tex`, which uses two
  separate arrays separated by `\Also` to get this).

## Prose formatting

One sentence per line in `*.tex` files. Let the editor soft-wrap
long sentences; do **not** insert hard line breaks inside a
sentence. This convention gives clean line-by-line diffs when
prose is revised --- each diff hunk corresponds to a sentence
edit, not a reflow.

(Not enforced for `*.md`.)

## Building

```
latexmk main.tex
```

Intermediate `.aux` / `.bbl` / `.log` files go to `build/`; the
final `main.pdf` ends up beside `main.tex`. To force a clean
rebuild, delete `build/` and re-run `latexmk`.

## Disclosure of Interests

`main.tex` carries the Springer-required `\discintname` block in
a `\begin{credits} ... \end{credits}` environment. The current
text states that all authors are Aptos Labs employees and may
hold equity, and that the work is implemented in the Aptos
blockchain. Keep that wording until camera-ready unless author
circumstances change.
