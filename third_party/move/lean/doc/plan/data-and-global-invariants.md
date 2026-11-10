# Data and global invariants: open problems

What is not done in the two invariant layers. The design of data invariants as
part of a type is [`../designs/data-invariants.md`](../designs/data-invariants.md);
the history of how it came to be is
[`transitions.md`](transitions.md), "Data invariants as carriers". The
global-invariant scheme below is a design of work that is entirely
unimplemented, which is why it is here and not in `designs/`; it follows the
Move Prover's paper (`doc/paper21/design.tex`, "Global Invariant Injection")
and passes (`bytecode-pipeline/src/global_invariant_analysis.rs`,
`global_invariant_instrumentation.rs`, `loop_analysis.rs`).

A declaration is cited by name, with its file relative to
`third_party/move/lean/verex-ir`. The Move Prover's sources and the fixtures
named as reproducers are relative to `third_party/move`.

## The stored-invariant layer still exists for the declarations the carriers do not state

**What is missing.** The whole machinery that states the data invariants of
stored values as a predicate over memory is still in the tree, and it is still
reachable. It is *not* emitted beside the carriers for a declaration whose
invariant the carriers state: `dataOwnTerms` (`VerexSL/Contract.lean`) sorts
every declaration with an invariant into the ones it carries, the ones whose
invariant reads the executable unit, a precondition or memory
(`invariantReads`), and the ones whose clauses do not translate, and every
piece of the second layer is restricted to the last two —
`storedInvariantTerm` is handed the carried handles and leaves them out at
every depth (`dataInvariantTerms`), `slotDataInvariantTerms` leaves them out of
the parameter and result boundary, `requiresTerm` states `storedAssumed` only
when some declaration is left (`storedInvariant.isSome`), the loop frame is
gated on the same, the data-invariant branch of `owesMemoryInvariants` selects
only a resource "holding a data invariant that is not part of the carrier of
its type", and `markedInvariant` (`VerexSL/Verify/Contracts.lean`) excludes the
carried declarations from the `constructedAt`/`mutationEndAt` conditions, so
those markers check nothing at a carried declaration's sites.

So what is left is the machinery itself, kept alive by the declarations the
carriers do not state: an invariant that states the behaviour of a stored
function value, one that reads a precondition, one that reads memory (recorded
as unsupported with the reason "its data invariant reads memory, which a
value's type cannot state"), and any whose clauses fail to translate.

**Why it matters.** Two obligation systems for one language feature: each of
them has to be read, scheduled, reported and maintained, and a reader of a term
cannot tell from the term which one owes what. The second layer's reading side
is also the shape that was expensive before the carriers existed — on
`primary_fungible_store::deposit`, three lines of Move with `modifies *`,
1 660M of 2 374M elaborator heartbeats were two leaves targeting
`DataInvariants` after 34 join arrivals had failed to prove it
([`transitions.md`](transitions.md)) — so any declaration that falls back to it
pays that shape again. That figure is historical and is not a measurement of
the present cost; the present cost is unmeasured, and measuring it on a unit
with a behavioural declaration is part of this problem.

**What is known.** The declarations to remove, and where they live:

| Declaration | Still in |
|---|---|
| `DataInvariants`, `DataInvariants.set_some` | `VerexIR/Proofs/Denote/{Resources,Types,Solve,SharedContinuations}.lean`, `Solve/{Base,Decide}.lean`, `VerexSL/Contract.lean`, `VerexSL/Verify/{Contracts,Members,Steps,FunctionValues}.lean` |
| `StoredCase`, `StoredCase.atFrame` | `VerexIR/Proofs/Denote/{Types,Resources}.lean`, `Solve/Decide.lean`, `VerexSL/Contract.lean` |
| `storedInvariantTerm`, `ensureStoredInvariant`, `keepsStoredInvariants`, `storedAssumed` | `VerexSL/Contract.lean`, `VerexSL/Verify/Contracts.lean` |
| `verex_data_invariants`, `verex_stored_cases`, `verex_stored_reads`, `storedInvariantsProof?`, `storedInvariantsStep`, `proveStoredInvariants` | `Solve/Decide.lean`, `VerexIR/Proofs/Denote/Solve.lean`, `Solve/Invoke.lean` |
| `lemma_stored_invariants_frame` | `VerexSL/Verify/Steps.lean`, `VerexIR/Proofs/Denote/FunctionValueLemmas.lean` |
| `owesMemoryInvariants`, `handlesDataInvariants` | `VerexSL/Contract.lean`, `VerexSL/Verify.lean` |
| the `DataInvariants` case of `exposeNormalEntries`, and the `DataInvariants` conjuncts of derived summaries and loop frames | `VerexIR/Proofs/Denote/Solve.lean`, `VerexSL/Verify/Contracts.lean` |

Every item goes when no declaration selects it, which makes this problem
downstream of two others: behavioural clauses inside a data invariant (below),
and an invariant that reads memory, which is a global invariant wearing a data
invariant's syntax and belongs to the global-invariant scheme.
`handlesDataInvariants` stays, because it is also what selects a
specification-less function that mutates a carried value for verification.
Fifteen source comments mark their declaration as retiring with
this removal and name it `step 3.4`, after a step list that no longer exists;
they want the problem's name instead
(`VerexIR/Proofs/Denote/Resources.lean` four, `Solve/Decide.lean` seven,
`VerexIR/Proofs/Denote/Solve.lean` three, `VerexSL/Contract.lean` one).

**The gate.** Keep: `data_invariant_reads` (all five reads clean, the control
reporting), `data_invariant_call_during_loan` (both clean),
`data_invariant_global_mut` at its baseline; `deposit` verifies at 300M
heartbeats within bounded wall time; no verified target of
`primary_fungible_store`, `fungible_asset`, `coin` and `multisig_account` lost.
Kill: a read loses its invariant, or a fixture changes outcome.

## `constructedAt` and `mutationEndAt` are emitted where they check nothing

**What is missing.** `certifyingAt`, with `.construction` and `.mutationEnd`,
is what owes a carried data invariant. The markers `Term.constructed` and
`Term.mutationEnd` (`VerexIR/Proofs/Denote/Term.lean`, with the steps
`constructedAt` and `mutationEndAt` and the cut rules `wp_constructedAt` and
`wp_mutationEndAt`) are nevertheless emitted at every pack and every mutation
end of a type carrying an invariant, carried or not
(`ns.carriesInvariant` decides in `VerexIR/Proofs/Denote/Compile.lean`), and
where the declaration is carried the conditions tables have no entry for the
site, so the marker checks nothing.

**Why it matters.** An empty marker is a node in every such term and a lookup
in the solver's conditions tables at every one of its sites, and it is the
reason a reader of a term cannot tell which layer owes what.

**What is known.** The markers carry no values, so removing them changes no
statement. Their obligation, where they have one, is the second layer's:
`mutationConditions` and `constructionConditions`
(`VerexSL/Verify/Contracts.lean`) build a condition only for the declarations
`markedInvariant` selects, which are exactly the ones `Carriers.own` does not
carry. Nothing else reads those two tables — the module-invariant side states
its obligation at `Term.memoryWritten` through `memoryWriteInvariants`, a
different marker that `deathEffect` emits beside this one — so the markers
retire with the second layer and nothing about them waits on the
global-invariant scheme. A narrower first step that needs nothing else: emit
them only where `markedInvariant` holds, which removes the empty marker from
every carried declaration's term. The retirement is marked in the source at
`VerexIR/Proofs/Denote/Solve.lean`, `Solve/Decide.lean` and
`VerexSL/Contract.lean`.

## Global invariants are not implemented

**What is missing.** All of it. There is no relevance by unification of
instantiated resource types, no invariant assumed at a loop head, no suspension,
no cluster induction. `reachFrom` (`VerexSL/Contract.lean`) records
`StructHandle`s without type arguments, so an invariant on `S<u64>` is selected
for a function touching `S<u8>` — sound, an over-approximation —
and `memoryWriteInvariants` instantiates the obligation at the written type.
Loop frames carry no namespace invariant.

**Why it matters.** `invariant` in a module spec is a documented Move
specification form, and a verifier that does not implement it either rejects
the module or verifies it against less than it says.

**What is known.** The scheme is the Move Prover's, and this is the design to
implement:

- **Relevance by usage.** `accessed(f)` and `modified(f)` are the memory a
  function reads and writes, transitively through callees; `accessed(I)` the
  memory an invariant reads. `I` is assumed at the entry of `f` when
  `accessed(f)` overlaps `accessed(I)`, decided by unification of the
  instantiated resource types (`I<T>` on `S<T>` applies to a function on
  `S<u8>` at `T := u8`, not `I` on `S<u64>`).
- **Owed after the write, on that branch** (`Term.memoryWritten` after a global
  loan's death, `move_to`, `move_from`, a ghost store): exactly the invariants
  the function carries that read the resource written; an update invariant
  compares with the memory before the write.
- **Suspension.** `[suspendable]` invariants and
  `delegate_invariants_to_caller` move the check to the call site
  (`CallChecked`); `disable_invariants_in_body` defers it to the exit; public
  and entry functions carry their invariants themselves.
- **Loops.** The loop frame assumes, at each iteration's head, the invariants
  the function carries that read memory the body writes, as the Prover
  re-assumes them after its loop-header havoc
  (`global_invariant_instrumentation.rs`); they are owed at the back edge,
  which the in-body checks make a fact lookup.
- **Clusters.** `invariantRelatedModules` links to a unit every module whose
  invariants read memory its functions reach and every earlier module whose
  functions write memory its invariants read, and verifies them together.
- **The ledger entry.** The induction over memory has three parts: genesis
  memory satisfies the invariants; every verified function re-establishes them
  after each write of memory they read (the per-function theorem, which
  exists); a run that writes no such memory leaves them unchanged. It is one
  hypothesis of `satisfies_prophetic`, in the form the ledger uses for
  `GlobalsPreserved`: the genesis condition and the cluster condition (no
  module outside the verified cluster writes memory the invariants read) are
  assumptions about the system, while the frame part is to be established by a
  theorem over runs, from `Memory.set` and the soundness of the usage analysis.

**The gate.** Fixtures: a generic invariant at a specific instantiation, a
suspended invariant restored at the call site, an update invariant after a
write, a loop writing memory an invariant reads. Keep: each reports at the
Prover's site. Kill: an invariant owed at exit that the Prover owes after a
write.

## The native models are not audited for data invariants

**What is missing.** A statement, per native model and per opaque callee of
another unit, of whether it returns or stores a value of a type carrying a data
invariant, and the `ensures` it then owes.

**Why it matters.** It is one of the four conditions under which
`GlobalsPreserved` holds of a unit
([`../designs/data-invariants.md`](../designs/data-invariants.md), "The storage
ledger entry"). A native that stores a value breaking an invariant breaks the
hypothesis silently, and nothing in the tree checks it.

**What is known.** The models are the `NativeModel` cases registered per unit;
the condition is stated in the design and nowhere enforced. The audit's output
is a list of models and the clause each needs, which is also what tells the
ledger entry whether it is still an assumption about the system or now a
checked property of the unit.

## Behavioural clauses inside a data invariant

**What is missing.** A data invariant whose clause states the behaviour of a
stored function value is not carried by the type; such declarations keep the
stored-invariant layer.

**Why it matters.** It is the one case that stops the second layer from being
removed, so the whole cost of that layer is paid for this one form.

**What is known.** Such a clause is closed under quantification over states —
explicitly in `amm_example` (`invariant forall S in *, …: S |~
!aborts_of<self.0>(…)`), implicitly in `function_wrappers`
(`!aborts_of<self.0>(v)`, read as holding in every memory). That is the right
semantics; what keeps it out of the carrier is only *where* `S` ranges: over
`Memory unit`, a function into carriers, which a carrier cannot mention.
Inside a data invariant, `S` would range over runtime states instead, and `S |~
aborts_of<f>(…)` would be the big-step run of the closure from `S`
(`functionSpecAt executable`, the form `FailuresIntegral` takes). Nothing is
lost by that reading: an invocation's denotation is already that run from a
state encoding the memory (`closureRun`, `VerexIR/Proofs/Denote/Term.lean`), so
the runtime statement implies
the typed one at every memory, and a data invariant observes `S` through
behavioural predicates only, never by reading a resource of it. The clause then
mentions no carrier and goes into the type with the rest.

What it costs is a translation of such clauses over runtime values
(`PlainValue.reads` is that reading) and the bridge lemma from `closureRun`.
`storedInvariantReadsUnit` (`VerexSL/Contract/Values.lean`) is the marker that
still singles these clauses out.

**The gate.** `amm_example` and `function_wrappers` unchanged in outcome.

## `data_invariant_in_map::no_violation` is falsely rejected

**What is missing.** `no_violation` is rejected although it breaks nothing.

**Why it matters.** It is a Move Prover fixture in the smoke set
(`scripts/smoke.sh`), so the false alarm is in front of everyone who runs the
smoke suite, and it is the measured cost of a deliberate over-approximation.

**What is known.** A holder resolves once per loan death, and every loan fact
naming the holder is treated as a possible origin (`holderResolves`,
`VerexIR/Proofs/Denote/Compile.lean`). The narrower rule — excluding loans whose
borrow lends another referent type — was measured and rejected: in this same
fixture the obligation that rejects `violation_1` is the referent certification
that a `&mut Table<address, S>` holder owes on behalf of the `&mut R` global
borrow naming it, and excluding that loan by type lets the violation through.
So an ancestor loan naming the holder is asked for an obligation it does not
owe, which is sound and incomplete. The fix belongs to the stored-value
obligation in the plain layer, not to loan resolution: a holder that is a table
should owe the invariant of what it stores, not the invariant of a resource an
ancestor loan reaches.

A second, smaller conservatism in the same mechanism: `lenderIdentity` reports
`incomparable` for a path containing a variant's choices (`Proj.steps?` returns
`none` there), so two such paths never merge and each contributes its own
obligation. Comparing `Choices` exactly would retire it.

What is not settled is whether the over-approximation is what fails here. The
undecided leaf of `no_violation` (solver debug at 821af8289ba) is
`¬entry.snd.fst.val = 0` for an entry ranging over
`(address.table <decl>).forget (SpecMap.writeBack …)`, while the map facts are
stated over the certified value's components (`val.val.fst.snd.snd.fst`): the
shape of "An element type spelled at two layers" below. Which of the two the
false alarm needs fixed is the first question of this problem.

**Reproducer.** `move-prover/tests/sources/functional/data_invariant_in_map.move`,
`no_violation`; the regenerated `.lean_exp` records it as failing with the data
invariant `invariant value != 0` not holding where a mutation ends, at the
declaration's clause.

## A returned reborrow of a field the invariant reads is rejected

**What is missing.** A function that returns a mutable reference into a `&mut`
parameter whose type carries an invariant, where the returned component is one
the invariant reads, is rejected although it preserves the invariant —
`fun ref_free(s: &mut S): &mut u64 { s.v = 0; &mut s.v }` with `invariant v > 0`
on `S`, and its shape.

**Why it matters.** Returning a reference to a field is a common Move shape,
and the rejection is incompleteness, not unsoundness.

**What is known.** A `u64` referent is certificate-free, so `certifiedResult`
emits no `Term.returned` for the result; the loan the result carries is
excluded from the deaths recorded at the function's exit, so its prophecy is
unresolved there ([`../designs/prophetic-references.md`](../designs/prophetic-references.md),
"Returned references"); and the parameter's own exit obligation
(`Mutables.resolve`, `certifyingAt .exit`) is stated of its current. A
reborrow of a component the invariant does not read verifies:
`move-prover/tests/sources/regression/mut_ref_returned_data_invariant.move`'s
`return_free_reborrow_ok` returns `&mut o.n` beside `o.inner`'s invariant and
is not in its `.lean_exp`. That the failing case is the exit obligation seeing
the component the reborrow holds open is a reconstruction and has not been read
off a leaf.

**Reproducer.** None isolates the shape; the fixture this problem needs is a
single unconditional reborrow of the field the invariant reads, returned, with
no join. The nearest are confounded by joins:
`move-prover/tests/sources/functional/mut_ref.move`'s
`return_ref_different_root`, `return_ref_different_path` and
`return_ref_different_path_vec2` return a `&mut u64` out of a parameter whose
type declares `invariant value > 0` and fail with that clause not established
(the Move Prover's own `.exp` reports only
`call_return_ref_different_path_vec2_incorrect`), and each branches between two
borrows, so the joined-holder rule is at least as likely a cause. At
821af8289ba the `mut_ref` targets carry 7 headlines that main does not, at T's
invariant at the end of such a returned reference's loan.

## A lender reads its index operands where the loan ends

**What is missing.** A `place` lender records the path a borrow was taken at,
index operands included, as a path and not as values: `PlaceIndex.slot` names a
local and `PlaceIndex.resolve?` reads it from the environment
(`VerexIR/Proofs/Denote/Term.lean`). `Lender.resolve` applies that path to the
environment at the loan's death, so a borrow `&mut v[i]` writes back to the
element `i` names *then*, not the element it named at the borrow.

**Why it matters.** Because the two readings differ once the index local is
written, the compiler refuses every function that may write an index local
while a mutable loan indexing by it may still be resolved
(`writesIndexUnderLoan`, read by `compileFunctionIn`,
`VerexIR/Proofs/Denote/Compile.lean`; decided by `indexWriteDiagnostics` in
`VerexIR/Validation/Borrowing.lean` at an assignment, a move-out, a place
write, a mutable borrow of the whole local, a destructuring assignment and a
`let`, against the analysis's holder-liveness active set). The refusal is
sound and leaves such a function without a denotation: it is incompleteness,
and a Move program that reassigns or rebinds its index while a reference it
took is still read — a loop that keeps a reference across the back edge and
rebinds the index with `let` in the body — is not verified at all.
[`../designs/data-invariants.md`](../designs/data-invariants.md), "A borrowed
path", describes the refusal.

**What is known.** The compiler resolves the place before recording the lender,
so two borrows of one place compare equal and the operand is part of that
identity (`lenderIdentity`); that identity would have to be by the operand's
value at the borrow too. The principled fix is for a lender to carry its
operands as evaluated at the borrow, so that nothing about the path depends on
the environment at the death; the refusal then goes.

**Reproducer.** `verex-ir/VerexSL/Tests/SoundnessCounterexamples.lean`: the
negatives of `review2_index`, `review2_write_paths`, `review2_loops` (among
them `rebind_in_loop`) and `carried_holder` report "no denotation … a write to
a local a live mutable borrow uses as an element index", beside the controls
of `review2_liveness` and `review2_loop_control`, which verify.
Through the Move Prover:
`move-prover/tests/sources/regression/index_write_under_loan.move`, whose
`write_under_loan_call`, `write_under_loan_index` and
`write_under_loan_destructure` are refused while `clear_all` (the loop of
`bit_vector::shift_left_for_verification_only`) and `write_after_loan` verify.
That fixture's `move_under_loan` is not refused through the package path: its
regenerated `.lean_exp` records `ensures false` as not established instead,
against the fixture's own comment.

## Failures the certified carriers introduced against main

The four problems below are what the certified carriers cost in completeness.
Their counts come from one measurement: every diagnostic present at
821af8289ba and absent at main (7a89e8311c5), across the Move Prover tests and
the end-to-end fixtures, each failing function rerun with solver debug and
classified by its undecided leaf. A count is of such headlines (clause
diagnostics plus function summaries), of groups (one per function) and of
tests. 3bd43a06681, which lets `grind` see related linear atoms and the frame
instance (`atomsRelatedByCongruence`, `unresolvedInstances`,
`VerexIR/Proofs/Denote/Solve/Base.lean`), was measured to remove errors from
some of these tests, named in each problem; the counts are not re-taken after
it. The reproducers are chosen outside that set.

### An element type spelled at two layers

**What is missing.** One value read at a certified owner and at its plain
layer is two terms to the leaf: an element default spelled
`<decl>.inhabitant …` against `plainInhabitantOf …`, a vector read through
`NTy.plainOf toPlain …` against the carrier, `<decl>.vector.forget v` against
`v`, a map read as `table.forget (SpecMap.writeBack (unread …) …)` against
plain `ofEntries`. No equation of the leaf relates the two spellings, so a
fact at one does not decide a goal at the other.

**Why it matters.** 55 headlines (35 clauses), 23 groups, 7 tests;
`verify_vector` alone has 30. A loop invariant over a vector mutated through a
`&mut`, `∀ j in 0..i, v[j] == …`, fails this way and needs no case split on the
written index: the same targets verify where the read default has one spelling.
3bd43a06681 was measured to make `table_option` verify and to remove two of
`bp_forwarding`'s seven errors.

**What is known.** [`../designs/data-invariants.md`](../designs/data-invariants.md),
"Data invariants", states the rule the producers are meant to follow: a
certified case spells its plain layer out rather than through the wrapper
abbreviations, because a subtype stores its carrier type and one type read at
two spellings is two terms to a congruence closure. The leaves above are
producers that do not follow it, or follow it at one side only — defaults
(`NTy.inhabitant`, `typedInhabitant?` in `VerexSL/Contract/Values.lean`),
`NTy.forget` at certificate-free element types, and map write-backs. The fix
belongs at the producers, one spelling per type, not in a rewrite at the leaf.

**Reproducer.** `move-prover/tests/sources/functional/bv_vector_element.move`,
`get_byte`: the leaf has `heq : b_data.values[i]? = some v` with
`b_data : NTy.plainOf toPlain (int 8).vector` and the goal
`i.val < b_data.values.size` over the other spelling.

### A declaration's constant against its structural row

**What is missing.** A value of a declared struct or enum is spelled through
the declaration's constant on one side of a leaf and as its structural row on
the other, and the leaf does not open the one into the other: struct and
string equality as `<decl>.eqb (<decl>.ofDeepHolds …) …` against field
equality; path decisions and `find` ensures as
`variantPlainName […] (NRows …)` against `s.val = Sum.inr (Sum.inl a)`;
`variantPlain.deepHolds` and `variantPlain.equal` over declaration rows
against the enum's fields; `Which.projectCertified? (<decl>.ofDeepHolds …)`
against `Which.project? ⟨…⟩`; map keys as
`SpecMap.ofEntries (<decl>.readPlain …)` against key fields; and
`serializedBytes [SpecTypeArgument.native <decl>]` against field-wise
equality.

**Why it matters.** 41 headlines (25 clauses), 19 groups, 16 tests. 3bd43a06681
was measured to remove `BcsSerialization`'s `same_bytes` errors.

**What is known.** Each producer above is a spelling choice of one translator
or normal-form rule, and every one of them reads a declaration constant where
the other side reads the row the constant reduces to. Either the producers
emit one of the two, or the normal form opens a declaration constant applied to
a known row; the choice belongs with the nominal declaration descriptors
([`../designs/native-verification.md`](../designs/native-verification.md#nominal-declaration-descriptors)).
`resources::move_from_addr_to_sender`, where the constant is only a resource
type tag and an `if` in a hypothesis is never split, is a different problem.

**Reproducer.** `move-prover/tests/sources/functional/abort_in_fun.move`,
`call_abort_generic_struct`: the leaf has
`pathDecision : (<decl> (param 0)).eqb (<decl>.ofDeepHolds (lent x, ())) (<decl>.ofDeepHolds (lent y, ())) = false`
and the goal `¬x = y`.

### A certified local is not split

**What is missing.** A local whose type is a certified struct stays whole in
the leaf: its facts are stated over projections of `.val`
(`value.val.snd.snd.fst`), the call-return cut asserts
`invariantHolds : True` because the invariant is in the certificate, and the
goal is over another name for the same component. The split loop's row rule
(`splitHypotheses`, `VerexIR/Proofs/Denote/Solve/Base.lean`) matches a `Prod`
and not the `Subtype` carrier; `destructureCarrier`, reached only through the
pruned cases, accepts the `Subtype`.

**Why it matters.** 40 headlines (26 clauses), 14 groups, 11 tests.
3bd43a06681 was measured to make `labeled_state_arg_mismatch` and
`spec_fun_generic_labeled` verify and to remove the `inc_twice` errors of
`StateLabels` and `spec_fun_old_param_labeled`.

**What is known.** A split of the certified local was measured, as a second
pass of `splitHypotheses` over certified carrier locals (summary parameters
left whole, since splitting them unlinks an inlined callee's summary), with the
certificate replaced by its normal form and dropped where it reduces to
`True`. It recovers `MutableTemporaries::read_deref` and the state-label
targets, and it costs: `bitwise_table::add_fail_exists` runs out of its budget
(33M to 59M heartbeats, with an extra path leaf `¬k1.val = k2.val`), the
`mut_ref` target's cost doubles (103M to 213M) and adds `ensures !b` to
`call_return_ref_different_path_vec2_incorrect`, and
`stored_maps_at_frames::proposal_count_u64` fails. Splitting only at the leaf
keeps one gain and the same regressions. The open question is how to keep the
certificate parts of the split rows from multiplying the walk's cost and
losing path facts.

**Reproducer.** `move-prover/tests/sources/functional/bitwise_table.move`,
`remove`: the facts are over `value.val.snd.snd.fst` with
`invariantHolds : True`, the goal over `value_0`.

### A selected declaration clause the split exposes is not opened

**What is missing.** Splitting a certificate exposes a declaration's selected
predicate as a fact `<unit>.dataOwn.caseN row`. `assertCertifiedClauses`
(`VerexIR/Proofs/Denote/Solve/Base.lean`) opens the selected clauses of the
certified values the leaf names once, at its boundary, through
`selectedOwnClauses?`, and the quantified element instances are opened as they
are observed (`observedOwnInstances`); a `dataOwn.caseN` fact the split loop
exposes later is not opened, and the leaf cannot use its clauses.

**Why it matters.** Opening those facts in the split loop was measured to make
these errors disappear: `inlined_start_model`'s `walk`, `establishes` and
`invariant v <= 10` ×3, `table_stored_resource::write`, `bitwise_table::add`,
and `enum_invariants`' `invariant this.bar > 0` ×2.

**What is known.** Opening there duplicates. One declaration's clauses are
read from several certificates of one value — a parameter, a callee's result
and a field — and an opener in the split loop opens them once per certificate,
so `pool_u64::buy_in`'s context held four to six copies of every `Pool` clause,
and its authored `on _ { }` decide failed with "maximum recursion depth" in the
full module. De-duplicating per call, or keeping the folded fact as evidence and
opening each once, does not remove the copies, because the opened conjunctions
are split into parts before the next copy is compared, and the parts also
repeat `assertCertifiedClauses`' own facts. The opener also adds about 1.1M
heartbeats to `intrinsic_map_rank_bulk::bulk_add`'s invariant step, which
tips it over its budget under solver debug. The principled form is to open
each selected case once, where `assertCertifiedClauses` already does, and to
find which walk-time obligation needs the opened clauses for the gains above.

**Reproducer.** The failing targets: `verex-e2e-tests/VerexE2ETests/SourceVerify/inlined_start_model.move`
(`walk`) and `verex-e2e-tests/VerexE2ETests/SourceVerify/table_stored_resource.move`
(`write`). The duplication shows on
`aptos_std::pool_u64`'s `buy_in` with an opener added to the split loop.
