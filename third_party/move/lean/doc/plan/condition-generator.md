# A condition generator in place of the walk

> **Proposal, not approved.** Written to be reviewed while
> [`plain-values.md`](plain-values.md) is in progress. It starts after that
> plan's gate G2 and changes nothing before it. The figures marked *estimate*
> are not measured.

## The problem

The Move Prover translates a function to Boogie, and Boogie to SMT-LIB, in
passes that are linear in the code and do no reasoning. Verex produces its
verification condition by a *walk*: the solver steps through the denotation of
the body inside Lean, and normalizes the goal after each step. The walk is
often the larger part of a target's cost, and it grows faster than the code.

The causes, from [`performance.md`](../designs/performance.md) ("What was
learned", "What cannot change") and from the trace of a one-line function
([`lowering.md`](../designs/lowering.md)):

1. **Every step is a checked rewrite, and a rewrite costs per node of the
   goal.** Applying the rule of one bind costs a few thousand heartbeats;
   normalizing the goal after it costs from tens of thousands to over a
   million.
2. **The goal carries the interpreter.** Between two steps the goal is
   `wp (Flow.bind (Term.denoteIn …) fun …) …`. Each of `Spec`, `Flow`,
   `Flow.bind` and `Term.denoteIn` takes the state, the result shape, the row
   of locals and a value type as arguments, and the value types reduce. About
   half of every normalization is that reduction.
3. **Values and continuations are copied.** A computed value substituted into
   the continuation is copied at every read; a branch copies the continuation
   into its arms. Shared continuations and named results repair this case by
   case.
4. **A callee without a contract is walked again at every call**, on every
   path that reaches it. In the attribution of 2026-11-04 over four framework
   modules, callees were 52% of the heartbeats (loop-free 30%, with loops
   14%, generic 8%) and the target's own body 8.5%.

Points 1 to 3 are properties of *how* the condition is produced, not of the
condition. Point 4 is a property of having no reusable form of a callee's
condition.

## The idea

Produce the condition in one pass over the typed term, with no simplifier,
together with a proof that the condition implies the statement. Normalize
only the leaves.

The statement proved does not change: `Satisfies (denote body …) contract`,
unfolded to `wp (body.denoteIn ops env) ensures aborts initial`. The trusted
base does not change. The generator is a metaprogram and is not trusted: what
it emits is a Lean proposition `V` and a term of type
`V → wp (body.denoteIn ops env) ensures aborts initial`, which the kernel
checks. A wrong generator produces a term that does not typecheck.

### Why this is possible here

The metaprogram already holds the typed term as a value: `compileFunction`
returns it, and `quoteTerm` walks it to write the definition `f.body`. The
walk does not use that. It rediscovers the term's structure from the goal,
one constructor at a time, by unfolding `Term.denoteIn` with the simplifier.
A generator that recurses over the term it already holds knows at each node
which rule applies and with which arguments, so it can build the application
directly.

### One rule per construct, in continuation form

For each constructor of `Term` there is one lemma that states the weakest
precondition of its denotation in terms of those of its parts, with the
continuation as an argument. For the conditional:

```
theorem Term.wp_ite
    (condition : wp (c.denoteIn ops env)
      (fun flow s => flow.onValue post s fun b env =>
        (b = true  → wp (t.denoteIn ops env) post aborts s) ∧
        (b = false → wp (e.denoteIn ops env) post aborts s))
      aborts s) :
    wp ((Term.ite site c t e).denoteIn ops env) post aborts s
```

The lemmas are compositions of what exists: the 27 `wp_` theorems of
`Term.lean` (`wp_flowBind`, `wp_opAt`, `wp_branchAt`, `wp_loopAt`, `wp_callAt`,
`wp_certifyingAt_intro`, `wp_prophecyOf`, …) and the 31 of `Types.lean` and
`Contract.lean`. `Term` has 107 constructors; they fall into a few shapes
(below), so the lemmas are fewer than that in kind though not in number.

The generator's certificate is the tree of these applications, one per node
of the term: linear in the code, built with explicit arguments, with no
unification search and no rewriting. The kernel's work per node is to check
one application and to reduce a continuation applied to a constructor
(`flow.onValue post s k` at `Flow.value v env`), which is a few steps.

### What the condition looks like

A formula of `∀`, `→`, `∧` over the program's values, with marked obligations
at its leaves. For `bump` of [`lowering.md`](../designs/lowering.md):

```
Holds Counter (c_n, ()) →                                  -- entry assumption
∀ sum, sum = ModularOp.add.run c_n 1 →                     -- a named value
  CheckAt 27 exit (Holds Counter (sum, ())) ∧              -- the exit check
  ((sum, ()) = c_final →                                   -- the prophecy equation
    Obligation … (c_final.fst.val = c_n.val + 1) ∧ final = initial)
```

No `Spec`, `Flow`, `Term` or environment remains in it. Its size is that of
the code.

## The design

### D1. Symbolic state at generation time

The generator carries, as metaprogram data: one expression per local (its
current value, or absent), the expression of the memory, and the continuation
it was called with. A read of a local is a lookup in that table. A write
replaces an entry. Nothing of `HEnv`, `Var.get` or `Var.set` appears in the
condition.

The rule lemmas are stated over `HEnv`. Where one is applied, the generator
writes the environment as the tuple of the table's entries, and the kernel
reduces `Var.get` of a literal variable on a literal tuple. The tuple is
shared between applications as one expression.

### D2. Every computed value is named once

A value that is not a variable or a literal is bound:
`∀ v, v = e → …`, and the table holds `v`. This is the passive form of
Boogie. It replaces the present case-by-case naming (checked arithmetic,
shifts) by one rule, and it bounds the condition's size by the code's size
whatever a value's number of reads.

A pure expression of several nodes is named once as a whole, not per node.

### D3. Checks, assumptions, aborts

- A check (`checkingAt site kind τ v`, after the switch of
  [`plain-values.md`](plain-values.md)) becomes the conjunct
  `CheckAt site kind (Holds τ v)` and the hypothesis `Holds τ v` of what
  follows. The marker carries what names the obligation today.
- An assumption (`Spec.assume P`, a prophecy's equation, an entry condition)
  becomes a hypothesis.
- An aborting operation becomes two conjuncts: its failure condition implies
  the contract's abort continuation at that failure, and its success
  condition implies the rest.

### D4. Control

- `ite`, `caseOf`: one conjunct per arm under its guard. Where the scrutinee's
  expression in the table is a constructor application, only that arm is
  generated. That decision is syntactic and needs no simplifier.
- `return_`, `break_`, `continue_`: the generator is called with one
  continuation per kind of outcome, so a transfer of control is the
  application of the matching one. The `match` on `Flow` never reaches the
  condition.

### D5. Joins

A branch whose continuation holds obligations is a join. The continuation is
generated once, as a hypothesis over the locals it reads:

```
(∀ locals s, Entry locals s → Continuation locals s) ∧
  ((b = true  → … Entry localsThen sThen) ∧
   (b = false → … Entry localsElse sElse))
```

`Entry` is the disjunction of the arrivals, each stated as the equations of
the locals that differ and the facts common to all arms outside the
disjunction. This is the relation that shared continuations state today
([`performance.md`](../designs/performance.md), "Shared continuations"); the
generator emits it at every join by construction, where the walk discovers
arrivals and partitions them. A continuation that only exits is not shared,
as today.

The condition is then linear in the code for any number of sequential
branches. The proof of a shared continuation is still made once under a
disjunction; that part of the cost is the leaf engine's and is not changed by
this design.

### D6. Loops

A loop takes its invariant from the clause translator, as today. The rule is
`wp_loopAt` in continuation form: the invariant at entry; for arbitrary
values of the modified locals under the invariant, the body's condition with
the invariant as the continuation of the next iteration and the loop's
continuation at a `break`; the continuation after the loop under the
invariant. The set of modified locals is the one `loopInvariants` computes.

### D7. Calls

- **A callee with a contract.** Its `requires` as an obligation, then for an
  arbitrary result and final state under its `ensures` and `frame`, the
  continuation. The premise is the callee's `Satisfies`, as today.
- **A callee without a contract.** Its condition is generated once, with the
  continuation and the abort continuation as parameters, and proved once:

  ```
  theorem g.condition_sound :
    g.condition post aborts args s → wp (g.denote args) post aborts s
  ```

  A call applies it at the caller's continuation. The callee's body is not
  generated again; its obligations appear at the call as an instance of
  `g.condition`, to be discharged under the caller's facts.

The second case is what the derived summaries of today approximate with a
separate budgeted walk, and refuse for generic callees, loops and returned
references. A condition that takes its continuation as a parameter has none
of those limits: it is the callee's summary in the general form.

### D8. Borrows

A borrow chooses a prophecy: `∀ p, Holds σ p → …`, with the owner's entry
replaced by the owner rebuilt with `p`. A loan's death is the checks of
[`plain-values-semantics.md`](plain-values-semantics.md), section 3, as
conjuncts in their order, then the equations as hypotheses. The lender is in
the term already (`Lender`, `SliceLender`), so the generator reads which
checks to emit and decides nothing.

### D9. Leaves

A leaf is one marked obligation with the hypotheses on its path. Extraction
is by introduction and splitting of the condition's connectives, with no
rewriting. Each leaf is then prepared as today (`verex_prepare`): bounds,
invariants opened to clauses, the normal form. Authored proofs address leaves
by the same names, since the markers carry the same sites and kinds.

Normalization therefore runs once per leaf on a formula without the
interpreter, and not after every step on a goal that contains it.

### D10. Points of the proof language

`at`, `after op j` and loop points place authored facts inside the walk
today, where the goal is open at that point
([`proof-language.md`](../designs/proof-language.md)). The generator emits a
marker at each point a script names, and the continuation after a point is a
hypothesis in the form of D5, so that a fact proved at the point is stated
once and is available to everything after it. **This is the interface with
the most risk in the design** and is specified in detail before any stage
that touches authored proofs.

## What stays and what goes

| Stays | Goes, once every construct is generated |
|---|---|
| The denotation, `wp`, `Satisfies`, the contract generator | `bindStep`, `splitStep`, `exitStep`, `callStep`'s inlining, the unfolding of `Term.denoteIn` in the solver |
| The existing `wp_` theorems, as the content of the rule lemmas | `normalizeAround` and the fixed-point cache between steps |
| Leaf preparation and the leaf deciders | The discovery of arrivals and their partition |
| The proof language and its obligation names | `summaryWalk` and the derived summaries |
| The markers of sites and kinds | The pins that keep locals readable across a branch |

`Solve.lean` and its parts are 30,000 lines. The table's right column is the
walk's share of them; the leaf engine is the rest. I have not measured the
split.

## What is not known

- **How much of the walk's cost is the interpreter, and how much is work the
  leaves would do anyway.** Opening an invariant where it is assumed, or
  reading memory after a write, is done inside the walk's normalization
  today and moves to the leaf. For `bump` the walk and setup are about 2.4M
  of 8.6M heartbeats and the two leaves 6.4M, so on a small function the
  generator can save at most a quarter. On framework functions the walk and
  the inlined callees dominate. The first step below measures the split
  before the stages are specified.
- **The kernel's cost of checking the certificate** on a large body. It is
  linear in applications, but each carries the environment tuple. *Estimate:*
  small against today's proof terms; to be measured at stage 1.
- **Whether leaves stay provable.** The walk simplifies with path facts as it
  goes, so a leaf sometimes arrives with a branch already decided. The
  generator decides only what is syntactic (D4). Contradictory leaves must
  then be closed by the cheap closers. `performance.md` records that few
  leaves are contradictory; that was measured on the walk's leaves, not on
  these.

## Order of work

Each stage adds constructs to the generator. A function takes the generator
only if every construct of its body, and of the callees it inlines, is
covered; otherwise it takes the walk, unchanged. The choice is per function,
by an option that is on for the covered fragment once its gate holds. So the
tree verifies everything it verifies today at every commit.

**G0. The split, and the rule forms on paper.** With the solver's profile on
three targets (one arithmetic, one storage, one with inlined callees), each
normalization is attributed to the heads it reduces: the interpreter's
(`Spec`, `Flow`, `Term.denoteIn`, `HEnv`, `Var`, `Proj`), value types in
implicit arguments, and the rest. Beside it, the rule lemma of every
constructor is written as a statement, grouped by shape, with the existing
theorem it composes. Output: the expected gain per class of target, and the
list of lemmas. If the interpreter's share is under a third on all three, the
proposal is reduced to D7 alone.

**G1. Straight-line code.** Literals, variables, pure operations, `let_`,
`drop`, `assign`, place reads and writes without a borrow, aborting
arithmetic, checks, the entry assumption and the exit of `&mut` parameters.
Gate: for every function of `DenotePerformance` in the fragment, the prepared
leaves equal the walk's (compared by content, modulo the names of bound
variables), and the cost is recorded beside the walk's.

**G2. Branches without joins**, then **G3. calls with a contract**, then
**G4. borrows and loan deaths**, then **G5. loops**. Same gate each, on the
fixtures the stage reaches.

**G6. Callee conditions** (D7, second case). Gate: the four framework modules
of the 2026-11-04 attribution, callee share before and after.

**G7. Joins** (D5). Gate: the targets with the most join nodes; leaves equal
per arrival.

**G8. Points** (D10), then the authored proofs of `OrderedMap` on the
generator.

**G9. Removal** of the walk's parts that no function takes.

G6 is placed after the constructs it needs and before joins because it is
the larger expected gain and does not depend on them.

### Stop rules

- A stage whose gate fails on "leaves equal" for a fixture is not worked
  around by changing the leaf: the difference is explained, and either the
  generator is corrected or the difference is accepted in writing with its
  reason.
- A stage that does not reach its gate in three working days of effort stops
  and is reported, with the fixture and the leaf. The walk still verifies it.
- No stage changes a budget, a baseline or an expectation to pass.

## Dependencies

- **After the plain-values switch.** With certified carriers a check returns
  a value of another type, so every continuation would carry a transport;
  the rule lemmas over plain values have none. Writing them before the
  switch would mean writing them twice.
- **Independent of nominal types.** The generator reads types as codes and
  does not open their layout.
- **G0 can be done now**; it reads the tree and changes nothing.

## Alternatives considered

- **A verification-condition function defined in Lean and evaluated by the
  kernel** (`Term.pre`, with one soundness theorem by induction). The
  per-function proof would be constant in size. It depends on the kernel
  evaluating a recursive definition over a mutual inductive of 107
  constructors with dependent types, which is the case `performance.md`
  records as expensive ("Unfolding a big recursive definition makes the
  kernel redo it"), and its condition would have to be reduced again to be
  shown as leaves. Kept as a later step for the pure fragment only, where the
  recursion is simple.
- **Keeping the walk and removing its normalization.** The walk finds its
  next step by reading the goal, which needs the goal in normal form; that
  is the dependency this design removes by reading the term instead.
- **Checkers compiled to native code.** Adds to the trusted base.
- **An SMT solver for the leaves.** Decided against
  ([`proof-strategy.md`](../designs/proof-strategy.md)); independent of this
  proposal, which changes how the condition is produced and not how a leaf is
  closed.
