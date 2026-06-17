# A Prophecy-Based Reference Model for the Move Prover

This note describes how the Move Prover represents mutable references using *prophecies* (in the
style of RustHorn and Creusot). This is the **default** reference model; the legacy `WriteBack`
(path-based) model remains available behind `--path-refs`. The two models coexist; by default every
prover unit test runs under both and is expected to produce the same result. The design covers both
verification and (separately) spec inference.

The motivation is twofold. The prophecy model is conceptually simpler — it eliminates reference
*paths* and the runtime machinery that maintains them. And it is strictly more expressive: it can
naturally express *free* mutations (a `&mut` obtained through a function value, and prospectively one
stored in data) that the path-based model cannot.

> File and line references below are anchors against the tree at the time of writing and should be
> re-checked against current code.

# 1. Background: the `WriteBack` model

The default model represents a mutable reference as a value that carries, besides the referent's
current value, a description of *where the reference points*, so a mutation can be propagated back to
its origin when the reference goes out of scope (`prelude.bpl`):

```boogie
datatype $Mutation<T> {
    $Mutation(l: $Location, p: Vec int, v: T)   // root location l, selection path p, current value v
}
```

Propagation is an explicit `WriteBack(node, edge)` instruction inserted by
`MemoryInstrumentationProcessor` after borrow analysis, reconstructing the parent value along the
path. Where a reference's parent is not statically unique, a runtime `IsParent` predicate selects the
right write-back target. This is *constructive*: it replays every write up a statically known borrow
path, and it collapses exactly where that path is not static. The clearest symptom is
`BorrowEdge::Invoke` (a borrow taken via a function value, of unknown structure), where the write-back
cannot compute an update and falls back to discarding the post-state (`bytecode_translator.rs`):

```rust
if matches!(edge, BorrowEdge::Invoke) {
    emitln!(writer, "call $t{} := $HavocMutation($t{});", idx, idx);
}
```

As a consequence `closures/closure_refs.move::update_a` (which mutates `&mut s.a` through a function
value) cannot be verified and is marked expected-fail (`TODO(#17904)`).

# 2. The prophecy model

A mutable reference `&mut T` is represented as a pair of two values of type `T`: the *current* value
`v`, and the *final* value `f` the referent will hold when the borrow ends. `f` is a *prophecy
variable* — unknown at creation, pinned at expiry. Under the flag, this is the entire datatype:

```boogie
datatype $Mutation<T> { $Mutation(v: T, f: T) }   // v = current, f = final (prophecy)
```

Three rules govern it:

- **Creation**, `let r = &mut x`: pick a fresh prophecy `f`, set `r = $Mutation(x, f)`, and *eagerly*
  set the lender `x := f`. This is sound because, while `r` is live, Move's uniqueness invariant makes
  `x` unobservable; the only obligation is that `x` ends up holding whatever the borrower leaves in
  `*r`.
- **Write**, `*r = w`: update only the current value, `r := $UpdateMutation(r, w)`; the prophecy is
  untouched.
- **Resolution**, when the borrow ends: `assume v == f`. Because the lender was already set to `f` at
  creation, pinning `f` here retroactively communicates the borrower's net effect back to the lender.

That single idea — make the final value a symbolic prophecy and install it into the lender at
creation — is what removes the need for a statically known borrow path. The rest of this section is a
consequence of it. Reborrows compose by *chaining* prophecies: a child gets a fresh prophecy, and the
parent's value is constrained in terms of the child's. Aliasing of two references is simply *sharing
the same `f` term*.

Worked example, `fun inc(x: &mut u64) { *x = *x + 1 }` then `let a = 5; inc(&mut a); a`:

```text
borrow:   a := f          and   x = $Mutation(5, f)
body:     inc's contract gives  f == old(v) + 1 == 6
result:   a == f == 6
```

The solver proves `a == 6` by first-order reasoning over the pair — no heap, no path, no aliasing
reasoning, no quantified frame conditions.

## 2.1 The encoding is path-free

`$Dereference` and `$UpdateMutation` keep their signatures, so `ReadRef`/`WriteRef` translate
unchanged:

```boogie
function {:inline} $Dereference<T>(ref: $Mutation T): T { ref->v }
function {:inline} $UpdateMutation<T>(m: $Mutation T, v: T): $Mutation T { $Mutation(v, m->f) }
```

All of the static model's location/path apparatus — `$Location`, `$ChildMutation`,
`$IsParentMutation`, `$GlobalLocationAddress`, and friends — is gated out under the flag and never
referenced. The structural information it used to *store* is instead consumed once, at creation, where
the lender, the field offset, and the dynamic index are all available as operands of the borrow
instruction:

```text
&mut x        (x a local)      r := $Mutation(x, f);                          x   := f
&mut s.field  (s a reference)  r := $Mutation(s->v->field, f);                s   := $UpdateMutation(s, s->v[field := f])
&mut v[i]                      r := $Mutation(ReadVec(v->v, i), f);           v   := $UpdateMutation(v, UpdateVec(v->v, i, f))
&mut T[addr]  (global)         r := $Mutation(ResourceValue(Mem, addr), f);   Mem := $ResourceUpdate(Mem, addr, f)
```

`f` is a havoc'd Boogie local introduced at translation time; no extra stackless temp is allocated.

## 2.2 `IsParent` becomes a logical constraint

In the static model `IsParent` is a *runtime* test that picks, at a write-back point with several
candidate parents, which one to update. Under prophecies there is nothing to pick: the parent/child
relation is the *defining equation* between prophecy terms, asserted at the borrow site (the child's
`f` literally occurs inside the parent's value). A conditional borrow needs no runtime branch — each
branch asserts its own equation under its own path condition, and Boogie's native path-sensitivity
carries it through the single resolve at the merge. The `IsParent` operation and its prelude support
have no analog, which as a side effect *removes* branches from the generated Boogie.

One qualification: *observations* of a lender while its borrow is live (in-code spec blocks, loop
invariants) do need one bit of runtime state per borrow site. The lender's slot holds the installed
prophecy, not the current value, so each observation is bracketed with syncs — the lender is
temporarily given the child's current value, the observation is evaluated, and the eager state
(the child's final value) is restored. Each sync is guarded by the site's *path flag*, a boolean
temp that is true exactly when that site is the child's reaching borrow on the executed path
(set at the site, cleared at sibling sites of the same child). This covers every read form,
including spec-function bodies, without rewriting the observed expressions. Loop invariants get
the same brackets from `LoopAnalysisProcessor` wherever it asserts or assumes them.

Sites exist for the syntactic borrow instructions, `&mut`-to-`&mut` assignments, and the native
vector/table `borrow_mut` calls — recognized by the `Index` edge their borrow summary installs,
with the element selector saved at the site like a global site's borrow address. Any other
`&mut` materialized as a call result has no statically known structure to sync through;
observing one of its lenders while it is live is rejected with an error (assert over the
returned reference instead).

## 2.2a Binding ends: resolve points and reborrows

A reference binding ends — and its prophecy is resolved (`assume v == f`) — at three kinds of
events, not just at graph-detected death:

1. **Live-range exit**, unconditionally. The borrow graph's in-use notion (a node borrowed by a
   live child) does not gate this: after a reference's last use, its value can only have been
   relinked to a child's fresh prophecy, so resolving at live-exit chains the pending IOUs
   correctly even while children are alive. (Gating on in-use would drop the obligation of a
   lender that exits before its child — e.g. the move-out/store-back pattern the compiler emits
   around a receiver call — because the graph walk never revisits skipped nodes.)
2. **Redefinition** of the temp (death by shadowing): resolved right before the redefining
   instruction. At a first definition the assume relates the components of an otherwise
   unconstrained local, which is unobservable.
3. **Exit merging**: `NormalizeExits` (which runs after this processor) moves returned refs into
   a unified return temp. That move is normally a *final reborrow* — a rename: the return temp
   inherits the binding, prophecy included, with no fresh prophecy and no resolve; the caller
   pins the inherited prophecy where the returned reference dies. Only when the temp is moved
   again later in the same result list (a returned `&mut` parameter also handed back as its own
   out-value) does the move stay a reborrow with the source's resolve emitted after it —
   inheriting the prophecy verbatim there would let two independently resolved handles pin it
   to two values.

Every in-body `&mut`-to-`&mut` assignment is a *reborrow*: the destination is minted with a fresh
prophecy over the source's current value, and the source is relinked to it. A verbatim copy would
share one prophecy between two independently written and resolved handles, forcing it to several
values (vacuity). Consequently `&mut` copies are stateful and are never alias-propagated by
`ReachingDefProcessor` in this pipeline; analyses reading Move-level alias semantics normalize
their own clone (`LambdaSpecInferenceProcessor`), the loop-target analysis sees the relink through
`Bytecode::modifies`, and the borrow graph may collapse two temporal lifetimes of one temp onto a
single node name — its traversals are cycle-robust for this reason (`BorrowInfo::is_in_use`).
The *final reborrow* rule of point 3 — a last-use assignment is a rename inheriting the binding —
is deliberately confined to the merged-exit moves, which are created after borrow analysis and so
exist outside the borrow graph. An in-body source stays on every ancestor chain of its
destination, so the dying-chain machinery (chain resolves, data-invariant packs, global commit
anchors) would read a renamed-away temp's stale value; making it rename-aware costs more than the
fresh prophecy saves (§8).

## 2.2b Rejected alternative: rewriting observations

Instead of syncing state, an observation could be *rewritten*: substitute each mention of a
borrowed lender with a reconstructed current value — `if (flag) update_field(s, f, *c) else s`
for a field borrow, `update(v, i, *c)` for an index, an address-equality-guarded read for a
global. This needs the same path flags and saved selector temps, but no sync operations, no
restore, and a rewritten loop invariant is correct wherever it is asserted, so the
`loop_invariant_prophecy_syncs` plumbing would disappear.

The blocking defect is spec functions: substitution corrects only what the observation
*mentions*, and a spec function reading global memory ambiently in its body is beyond its reach
— such observations would have to be rejected, an unacceptable restriction. Syncing the state
instead makes every read form correct by construction — field selects, whole-value mentions,
spec-function bodies, quantified global reads — with no expression analysis at all. (A first
attempt at this alternative also failed on enclosing reads: a mention of the whole lender,
e.g. as a spec-function argument, is not syntactically a read of the borrowed location, so
read-site redirection misses it. Reconstruction at the mention fixes that, but not the
ambient-memory case.)

## 2.3 Calls under a live global borrow: the re-pin

The eager update installs the prophecy into the resource memory at the borrow, so the memory slot
and the borrow are connected from creation on. An intervening call can sever that connection: a
callee that (transitively) invokes a function value declaring `modifies_of` over the borrowed
memory havocs the whole memory variable, prophecy included. The static model is immune — its
write-back lands *after* the call, clobbering whatever the havoc did at the borrowed address.

The prophecy counterpart is a *re-pin*: right after each call made while a global borrow is live,
the instrumentation restores the eager update, `mem := ResourceUpdate(mem, addr, child->f)`
(a `ProphecyRepin` marker carrying the child reference and the borrow address, saved
into a dedicated temp at the borrow site). This is semantically exact, not an approximation:
Move's borrow rules guarantee no non-aborting callee can change an exclusively borrowed slot —
the `acquires` discipline for static calls, the dispatch reentrancy check for function values. It
is emitted as an assignment rather than an assumption so that a callee spec that (impossibly)
constrains the borrowed slot is overridden, exactly as the static write-back would, instead of
becoming a false assumption. Re-pins are invisible to the global-invariant analysis: they restore
a value that was already installed, and invariant assertion stays deferred to the
`ProphecyCommitGlobal` marker.

Only single-origin children are re-pinned (every definition of the reference temp is a
`BorrowGlobal` of one memory), so the saved address always matches the reaching borrow on every
path. A mixed-origin reference (borrowing a global on one branch, a local on the other) keeps the
incomplete-but-sound unpinned slot across calls, as do havocs not modeled as calls (loop-head
memory havocs, which `loop_analysis` inserts after this processor runs).

## 2.4 Update invariants: the pre-state snapshot moves to the borrow

A global *update* invariant relates two states, `old(..)` against current, at each global-state
transition. Under the static model the transition is the write-back, and a snapshot taken right
before it sees the pre-mutation state. Under prophecies the mutation happens *eagerly at the
borrow*, while the transition is marked where the borrow dies — at `ProphecyCommitGlobal` for an
ordinary global borrow, and at the `WriteBack(GlobalRoot)` of the borrow-write-writeback sequence
for a ghost-variable update. A snapshot left at the transition would observe the installed
prophecy, not the pre-borrow state.

The assertion cannot move to the borrow site instead: there the prophecy is still unconstrained,
and in Boogie's operational encoding an `assert` placed *before* the resolving `assume` quantifies
over all possible futures — too strong. (RustHorn's CHC setting imposes no such order; this
asymmetry is a cost of the operational encoding, see §8.) So the assertion stays at the transition
and only the `old` snapshot relocates: it is taken at every `BorrowGlobal` defining the committed
reference, and a site re-executed in a loop re-snapshots, keeping `old` at the reaching borrow.
Since the transition point may also be reachable on paths that never executed a given borrow (a
reference borrowed from either of two resources, dying at the merge, with one commit per borrow),
each relocated transition's assertion is guarded by a path flag set exactly at its borrow site(s);
on other paths the assertion is vacuous, and the transition that did occur is asserted by its own
marker (`global_invariant_instrumentation.rs`).

# 3. Mapping onto the prover

A single boolean selects the model at three points. It defaults to the prophecy model; setting it
opts back into the legacy `WriteBack` model.

## 3.1 The switch

- `ProverOptions::path_refs` (`bytecode-pipeline/src/options.rs`) — `#[arg(long)]` auto-creates
  `--path-refs` and the TOML key `prover.path_refs`. When `false` (the default) the prophecy model is
  used; when `true` the legacy `WriteBack` model is used.
- `BoogieOptions::path_refs` (`boogie-backend/src/options.rs`) — a mirror set from `ProverOptions`
  in `cli.rs::post_process`, required because the prelude is rendered by Tera with `BoogieOptions` as
  the context (`{% if not options.path_refs %}` selects the prophecy `$Mutation`).

The Aptos `prove` CLI exposes the same flag (`aptos-move/framework/src/prover.rs`).

## 3.2 The processor chain

The pipeline is reused unchanged through `BorrowAnalysisProcessor`. The single change is to swap the
memory processor (`pipeline_factory.rs`):

```rust
if options.prophecy_refs && !options.inference {
    ProphecyInstrumentationProcessor::new()   // verification only; see §3.5
} else {
    MemoryInstrumentationProcessor::new()
}
```

`ProphecyInstrumentationProcessor` consumes the same `BorrowAnnotation` and instruments the code
with dedicated stackless-bytecode operations, each carrying its role in the type:

- `ProphecyBorrow(lender, edge)`, source the borrowing reference — at each borrow creation: the
  eager lender update (§2). It mirrors the shape of `WriteBack`, so variable remapping and
  instantiation are reused. Global borrows get no creation-time marker; their eager memory update
  is emitted directly by the translator, where the address operand is at hand.
- `Resolve`, source the dying reference — at each binding end (§2.2a): `assume v == f`.
- `ResolveReturn`, source a returned `&mut` — the resolve of a returned reference, gated on the
  function variant: it fires only when the function is verified standalone; at an inlined or
  opaque call site the resolution is the caller's, where the returned reference dies (§3.3).
- `ProphecyCommitGlobal(mem)`, source the resource-borrowing reference — where a global borrow
  dies: the global-state transition marker consumed by the global invariant analysis; the source
  anchors the relocation of update invariants' `old()` snapshots to its `BorrowGlobal` sites
  (§2.4).
- `ProphecyRepin(mem)`, sources the child reference and the saved borrow address — after each
  call made under a live global borrow: restores the eager update into the resource slot (§2.3).
- `ProphecySyncCurrent(lender, edge)` / `ProphecySyncFinal(lender, edge)`, sources the child, the
  site's path flag, and for a global lender the saved borrow address — the observation brackets
  (§2.2): guarded by the flag, the first temporarily installs the child's current value into the
  lender, the second restores the eager state (the child's final value). Emitted around in-code
  spec blocks by this processor, and around loop invariants by `LoopAnalysisProcessor`.

Placement reuses `dying_nodes`, whose child-first order is exactly the order in which references
must resolve. `PackRef`/`PackRefDeep` are still emitted at the dying nodes so
`DataInvariantInstrumentationProcessor` is unchanged. For a subtree committed to a global
resource, the data invariants are asserted on the *definite pack base* — the highest reference
every borrow chain of the dying leaf passes through, which is initialized on every path and
holds the committed value after the resolves. This covers unconditional borrows (the resource
reference itself), conditional borrows (the reference below the fork), and sequential borrows
whose reused temps make the chains re-converge. A data invariant declared on a struct *above* a
conditional fork has no path-independent reference to be asserted on and is rejected with an
error.

## 3.3 Inter-procedural boundary

The existing call shape is kept: each `&mut` parameter is an input plus a trailing implicit return,
and a call `f(x)` is rewritten `x := f(x)`. The threaded value is a `$Mutation(v, f)` whose prophecy
the caller chose at the borrow it passes in; the callee discharges the contract by its own in-body
`Resolve`s before `Ret`. For an **opaque** call, the parameter's effect is realized by resolving its
prophecy (`assume $t_i->v == $t_i->f`); the callee's assumed postconditions then constrain
`$Dereference($t_i)` exactly as before, so the `ensures_of`/`result_of` behavioral machinery is reused
unchanged.

## 3.4 Closures and free mutations

A closure that takes a `&mut` *parameter* needs no special handling: the function-value apply boundary
threads the `$Mutation` through `$UpdateMutation`, like any opaque call (§3.3). This includes the
closures produced by lambda-lifting inline higher-order functions (e.g. `for_each_mut` with `|&mut T|`).

A `&mut` *derived through* a function value — returned by the closure, `BorrowEdge::Invoke` — is **not
yet supported**. The prophecy treatment would constrain the dying reference's prophecy against the
closure's behavioral footprint (`ensures_of`/`result_of`), a value-level relation at the `(v, f)`
granularity. A naive over-approximation here is *unsound* — it lets the closure's effect be assumed
away, making any post-condition provable — so until it is implemented properly the translator emits a
clear error for this case rather than a result. This is the documented next step; it would unlock
`closure_refs.move::update_a`. It depends on the language ban (`closure_checker.rs`) that prevents a
closure from *capturing* a `&mut` into its environment: that ban keeps every `Invoke` edge
depth-increasing and so resolution acyclic (§5), and must not be relaxed.

## 3.5 Spec inference (independent tool)

Spec inference is a separate tool that happens to share bytecode infrastructure; it produces ordinary
Move specifications, which are *model-agnostic*. It therefore always runs against the static
instrumentation — the pipeline selects the prophecy processor only in verification mode
(`!path_refs && !inference`). This keeps the intricate weakest-precondition
machinery unchanged. The relevant property is that the *inferred specifications verify under the
prophecy model* for every supported borrow form. Running inference itself against the prophecy
instrumentation is a possible follow-up; it would not change the inferred specifications.

# 4. Coexistence and testing

The two models share one baseline. The `default` feature now runs the prophecy model, and the `path`
feature in `tests/testsuite.rs` (`flags: ["--path-refs"]`, `inclusion_mode: Implicit`,
`separate_baseline: false`, enabled in CI) runs *every* unit test under the legacy static model as
well, against the same `foo.exp` — that shared baseline is the cross-check that the two models agree.
A test whose static output legitimately differs (typically a different counterexample trace) escapes
the sharing with `// separate_baseline: path` (yielding `foo.path_exp`); a case the default prophecy
model cannot verify is excluded from the default with `// exclude_for: default`.

One constraint shapes this. **The Boogie prelude is rendered once per run**, so a single run cannot
mix prophecy and static lowerings: a function lowered to `WriteBack` would reference prelude functions
that the prophecy prelude gates out. A (default) prophecy run therefore requires *every* function it
translates to be expressible in the prophecy model. Currently exactly one test is excluded from the
default (`closures/closure_refs.move`, the `Invoke` case of §3.4); it runs under `--path-refs` only.

# 5. Soundness

Three conditions from the literature must hold; each is met structurally by the prover's existing
analyses.

- **Resolve exactly once.** A reference dies once in the live-variable lattice, so `dying_nodes`
  yields one resolution point per reference. The conditional case is audited so exactly one `Resolve`
  is reachable per dying reference along any path.
- **Acyclic resolution.** Reborrow chaining defines a parent's prophecy in terms of its child's.
  Borrow analysis produces a DAG (a child's level is its parent's plus one), so the dependency order is
  the child-first order `dying_nodes` produces. The only way a back-edge could arise is `Invoke`
  aliasing, which the `closure_checker` ban prevents.
- **No early resolution.** `assume v == f` before the last write would be unsound; `dying_nodes` is
  after the last use by the definition of liveness, so resolution is never placed ahead of a write.

# 6. Status

Covered by the default prophecy model: local-root, field-on-reference, vector-index, variant-field,
table, and global-root borrows; conditional reborrows and returned `&mut` parameters; the
inter-procedural boundary including opaque calls; and closures with `&mut` parameters, including those
from lambda-lifting inline higher-order functions. Under `--path-refs` (the legacy model) output is
byte-identical to the prover's previous default.

Not yet supported (each fails closed with a clear error):

- A `&mut` *derived through* a function value (`BorrowEdge::Invoke`, §3.4); implementing it
  would unlock `closure_refs::update_a`.
- Observing a lender while it is mutably borrowed through a function call other than the native
  vector/table `borrow_mut` (§2.2): the borrow's structure is not statically known, so no sync
  site exists. Assert over the returned reference instead. (Under `--path-refs` some of these
  observations crash the legacy instrumentation — see `prophecy_observe_index.move`.)
- A data invariant declared on a struct mutated through a *conditional* borrow of a global
  resource, when the struct sits above the borrow fork (§3.2): there is no path-independent
  reference to assert it on.

Sound but incomplete:

- A `&mut` held *across* a loop back-edge: the loop-head havoc severs the eager prophecy link,
  leaving the lender unconstrained unless the loop invariants characterize it. Observations of
  the lender inside the loop body (in-code specs, the loop invariants themselves) do see the
  borrow's current value via the sync brackets (§2.2), so invariants over the borrowed state can
  be stated; only the implicit lender/prophecy connection across iterations is lost.

The Move framework verifies under the default prophecy model (move-stdlib, aptos-stdlib, and
aptos-framework prover tests run it), with one function excluded —
`jwks::remove_oidc_provider_for_next_epoch` — for a borderline solver timeout at full-package
monomorphization (see §7). The model's soundness has not been systematically audited.

# 7. Performance

Verification time was compared between the two models on the whole `aptos-framework` package, function
by function, with `aptos prove --benchmark`. That mode verifies each function in isolation
(`verify_scope = Only`), single-threaded, so timings are deterministic and free of the cross-function
solver-context effects discussed below. All 1641 framework functions were measured under each model;
both completed with **no failures and no timeouts**. One function,
`jwks::remove_oidc_provider_for_next_epoch`, is marked `pragma verify = false` for this comparison: it
is the single function whose solver time is borderline against the 40s ceiling *at full-package
monomorphization* (a `&mut` local of a vector-of-structs passed through an opaque call), and it would
otherwise add noise unrelated to per-function cost.

| Metric | `WriteBack` (static) | Prophecy | Ratio |
| --- | --- | --- | --- |
| Total solver time, 1641 fns | 525.7 s | 493.7 s | 0.94× |
| Net of the ~276 ms boogie-startup floor | 72.8 s | 40.8 s | 0.56× |
| Functions slower under prophecy (>50 ms) | — | 41 | |
| Functions faster under prophecy (>50 ms) | — | 147 | |
| Roughly unchanged | — | 1453 | |

Per function, the prophecy model is at parity or slightly faster: total solver time is lower, and 3.6×
as many functions get faster as slower. Its path-free encoding (no `IsParent` checks, no location/path
component on `$Mutation`) is genuinely leaner per query. No function is made meaningfully slower — the
largest regression is `base16::base16_utf8_to_vec_u8` at +202 ms; the largest improvement is
`big_ordered_map::next_key` at −394 ms.

**This measures per-function cost, not whole-package cost.** Because each function is verified in
isolation, the benchmark does not exercise the one place the prophecy model is more expensive: a
single Boogie/Z3 session over a *large, shared* monomorphization. There, prophecy's per-`&mut`
`assume current == final` accumulates in the shared context, and whole-package verification is modestly
slower (≈1.2× on `aptos-stdlib`, ≈2.1× on the much smaller `move-stdlib`). This shared-context cost,
not any per-function penalty, is also why `remove_oidc_provider_for_next_epoch` sits on the timeout
boundary at full-package scale — a borderline-VC effect that, under parallel solving, also affects the
static model on other functions.

Artifacts (regenerate the plot with
`cargo run -p move-prover-lab -- plot --out=<f>.svg --sort <a>.fun_data <b>.fun_data`; series are
labelled by file name and sorted by the first file):

- [`static_vs_prophecy.svg`](static_vs_prophecy.svg) — per-function solver time, static vs. prophecy,
  sorted by static time.
- [`static.fun_data`](static.fun_data), [`prophecy.fun_data`](prophecy.fun_data) — the raw
  `name  solve_ms  status` records the plot is built from.

# 8. Relation to the literature

The encoding of §2 is a faithful transcription of the canonical model: RustHorn's borrow rule
(fresh prophecy, eager lender update, resolve at expiry) and Creusot's `(current, final)` record.
The simplicity those papers claim — and which §7 confirms this implementation inherits — is
simplicity of the *logical encoding*: no paths, no write-back ordering, no aliasing case analysis
in the verification conditions. It is purchased with static precision that, in the papers' setting,
the Rust type system provides for free. RustHornBelt's soundness argument rests on exclusivity:
while a borrow lives, the lender is not merely "not supposed to be observed" but *unobservable in
the source language*; and rustc hands the verifier exact borrow-end points. The Move Prover has
neither gift, and each mechanism this model needs beyond the three rules of §2 compensates for a
specific assumption Move breaks:

| Mechanism here | Assumption in the papers it compensates for |
| --- | --- |
| Observation syncs and path flags (§2.2) | Exclusivity: Rust cannot even express reading the lender mid-borrow; Move spec blocks can. |
| Binding-end taxonomy, cycle-robust traversals (§2.2a) | Borrow ends come from the borrow checker (NLL); here they are reconstructed from liveness over a temp-reusing IR. |
| Snapshot relocation for update invariants (§2.4) | Two-state, mid-function global invariants have no Rust analog; and RustHorn's CHC constraints are atemporal, while Boogie's assume-flow is forward-only. |
| The re-pin (§2.3) | Rust callees cannot touch borrowed memory *by type*; Move callees see all of global storage. |
| Loop back-edge incompleteness (§6) | CHC handles loops as recursion, where prophecies carry over freely; havoc-based loop-to-DAG severs the eager link. |

The deeper trade is against the static model itself. `WriteBack` is *self-guarding*: a misplaced
write-back is dynamically vacuous — its `IsParent` test fails in the logic — so the instrumentation
may be imprecise, and the solver pays for the guards on every query. Prophecy strips the dynamic
guards, so placement must be statically exact: a precision obligation moved from the solver's
search space into deterministic, testable instrumentation. Dynamic guarding does not grow under
this model; it shrinks and localizes. The only remaining runtime flags sit at lender-observing
spec sites (§2.2) and relocated update-invariant transitions (§2.4) — exactly the two features the
literature's source language does not have. That resolution placement is the intricate part is not
specific to this implementation: it is the subtlest part of Creusot's translation as well, *with*
rustc's borrow information (Denis's thesis treats it at length).

Two refinements from the literature apply here, one adopted and one open:

- **Final reborrows** (adopted by Creusot to tame prophecy chains): an assignment that is the
  *last use* of the source is a rename, not a reborrow — the destination inherits the prophecy,
  with no fresh prophecy and no resolve. This is adopted for the merged-exit moves of
  `NormalizeExits` (§2.2a, point 3), which are created after borrow analysis and so live outside
  the borrow graph; each rename saves a prophecy and a resolve per returned `&mut` per exit
  branch. It is deliberately *not* applied to in-body assignments, where the source stays on
  every ancestor chain of the destination and the dying-chain machinery would read the
  renamed-away temp's stale value (§2.2a). Nor does the rule thin the redefinition machinery,
  as one might hope: a shadowed binding is not assigned anywhere — it is redefined — so its
  end-of-binding resolve is irreducible.
- **Observation through the borrow** (Creusot's `*r`/`^r` operators in specifications): a
  spec-language form naming the borrow's current or final value directly would make new
  specifications independent of the sync brackets, leaving §2.2 as a compatibility layer for
  lender-naming specs.

For the wider design space: Aeneas takes the opposite branch — borrows become explicit *backward
functions* that propagate final values to the lender, a structured relative of the write-back idea
that avoids prophecies at the cost of explicit propagation structure. The prover has lived on that
side of the trade; the path machinery of §1 is what its unstructured form costs.

# References

- Y. Matsushita, T. Tsukada, N. Kobayashi. *RustHorn: CHC-based Verification for Rust Programs.* TOPLAS 2021.
- Y. Matsushita, X. Denis, J.-H. Jourdan, D. Dreyer. *RustHornBelt.* PLDI 2022.
- X. Denis, J.-H. Jourdan, C. Marché. *Creusot: a Foundry for the Deductive Verification of Rust Programs.* ICFEM 2022.
- X. Denis. *Deductive Verification of Rust Programs.* PhD thesis, Université Paris-Saclay, 2023.
- S. Ho, J. Protzenko. *Aeneas: Rust Verification by Functional Translation.* ICFP 2022.
- See also [`../fun_values_note.md`](../fun_values_note.md) for the function-value (closure) semantics this model builds on for free mutations.
