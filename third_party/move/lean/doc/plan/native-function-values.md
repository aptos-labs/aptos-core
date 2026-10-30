# Native function values

Approved 2026-10-25. Extends the typed carrier of
[`higher-order-functions.md`](../designs/higher-order-functions.md).
Ordered against [`proof-language.md`](proof-language.md): step 2a comes
before that plan's L0; steps 2b and 3 run interleaved with its L0–L2; its
function-value constructs (L4) come after 2b and 3.

Two places still hold runtime values after
[`native-verification.md`](../designs/native-verification.md): the runtime
family's implementation of a function type's carrier, typed runtime
closures whose captures are runtime values (step 2a moved every contract,
leaf and solver rule onto the carrier's interface), and the requires table
`FunctionHandle → Array RuntimeValue → Memory unit → Prop`. Both become
native data, following the Prover's
representation of function values (`higher-order-paper-26`, "Representing
Function Values") and its monomorphization, this stack's D3.

**Closures are data.** A closure is its target and its captured arguments
as native values; invocation dispatches on the target with the captures
woven into the arguments. Per verified instance, the generator computes
the closure sites (target `f`, instantiation `θ`, mask `m`) of each
function type `τ` and generates one mutual block of inductives:

```
inductive FV_τ
  | closure_{f,θ,m} (captures : <carrier of the captured row>)  -- one per site
  | foreign (key : ForeignClosure τ)
```

`Carriers` gains `functionCarrier`, and `NTy.carrier (.function …)` is
`FV_τ`; at an induced family it is the outer family's at the substituted
type, so transports stay identities. Typing holds by construction: the
subtype, `ClosureTypedAt` and the closure rules' typing side conditions
go. A carrier defined by recursion on the type cannot hold native captures
(the captured row is not part of the type, and captures may be closures),
hence generated finite inductives.

**Values from parameters and storage** are variables of `FV_τ`; a value of
a closure site is `closure_{f,θ,m} c` with `c` unknown. A target the
instance does not contain is `foreign k`: `ForeignClosure τ` is the one
sealed piece (handle, instantiation, Move's equality and order, storage
key), and its meaning is a hypothesis, the declared behavioral predicates,
as the Prover's parameter and field variants are.

**Invocation, equality, order.** `invoke v args` matches: a closure arm is
the target's meaning at the woven arguments, handled by the call rule and
the target's theorem as a direct call; `foreign k` is `invokeForeign k
args`, a hypothesis stated as the model's `InvokeMeaning` is (definedness
under the declared `requires_of`, coherence across frames, the declared
predicates and frame). The weave and composition lemmas
(`closureMeaning_closureOf_*`, `requiresOf_closureOf`) become unnecessary;
the call rules and `KeepsMemoryAt` stay. Equality is structural up to
ghosts, constructors of different sites differ; order is Move's
lexicographic order (handle rank, handle, instantiation, mask, captures).
A value is storable where its captured rows are reference-free.

**The requires table**, per constructor over native rows:
`RequiresOf (closure_{f,θ,m} c) args := requires_f θ (weave_m.compose c
args)`; `RequiresOf (foreign k)` is the uninterpreted `requiresForeign k`
of the `invokeForeign` hypothesis. Defining it as `True` would make every
foreign closure invocable at every argument.

**Rust.** A closure type is a struct of its upvars (a by-reference upvar a
`.ref` field) with a `self`-taking body, and a call through a known
closure type is a direct call; `dyn Fn*` values are `FV_τ` over the link
set's closure structs plus `foreign`; function pointers are nullary
constructors per target and instantiation, `foreign` outside the unit.

**Failures (done).** `Failure := ThrowKind × Array Int`, related to the
runtime failure by the injective `Failure.encode`; the public theorem
assumes `FailuresIntegral` (the assumption ledger of
[`denotation.md`](../designs/denotation.md)). It stays an assumption, since
failures will carry more than integers later; establishing it would take a
validation rule on a throw's operands, a big-step lemma, and an obligation
of the natives registry.

**Where the runtime form remains**: the generated decoder and encoder of
`FV_τ`, `ForeignClosure`'s implementation, `closureMeaning`,
`propheticRun`, `Failure.encode`, the codecs and the agreement; the
native-only check flags the runtime closure vocabulary anywhere else
(`ClosureValue`, `RuntimeFunction`, `closureOf`, `closureRun`, the
interface's `closure` and `ofClosure?`).

**Steps**, one commit each. 1: failures (done). 2a (done): the interface,
`Carriers.functionCarrier` with `TypedFunctions` (a value's runtime closure
and the value a typed closure is; the codec, equality key and order derive
from them), construction (`functionOf`, typed by `FunctionTyped`, a
literal's typing decided at the leaf by one check the kernel evaluates
once, an auxiliary theorem of the target's proof, `FunctionTyped.ofCheck`), invocation (`InvokeMeaning` over values of the
carrier, `closureMeaning` the run of the value's closure), and the
behavioral predicates and frames over those values; the runtime family
implements it by typed closures in a type of their own (`RuntimeFunction`),
so no leaf unfolds to a closure. A function type marks its shared-reference
results as it marks its parameters (`Sharing`); a mutable-reference result
reads as itself. The interface is also what the proof
language's elaborator spells a function-typed expression as
([`proof-language.md`](proof-language.md), "Semantics and lowering", 2),
so its construction, invocation, equality and order operations stay
through 2b and 3; the datatypes implement them, they do not replace
them. 2b: the data, the per-instance datatypes implementing it, with
`Carriers.runtime unit` taking the function carriers as a parameter so
the library and the agreement stay one definition. Gates: Check/Closures,
Check/Storage's stored closures, the prover's closure tests,
`DenotePerformance`, the native inventory empty. 3: the requires table per
constructor (gates: the `requires_of` fixtures, `behavior_manual_facts`,
`specialize_generic_caller`); the leaf's provenance records the
constructor's target under the invocation site the proof language's L0
records, the one piece of the solver both plans touch. The authored
proofs over closures the gates cover (`calculator`, the paper example
`amm`, `stored_fun_values`) are written in the proof language, once, after
3; where a gate needs one of them before the language carries function
values, it is ported under `lean =>` and listed in the language's L4, and
no proof spells an `FV_τ` constructor outside `split` or `lean =>`. 4:
Rust function pointers in the mapper. Open: the sharing marks in
`NTy.function` are Move's calling convention inside the type and should
become derived data of the target.
