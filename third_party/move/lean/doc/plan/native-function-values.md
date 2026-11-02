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
args`, a hypothesis stated as the model's `InvokeMeaning` is (definedness,
coherence across frames, the declared predicates and frame). The weave and composition lemmas
(`closureMeaning_closureOf_*`, `requiresOf_closureOf`) become unnecessary;
the call rules and `KeepsMemoryAt` stay. Equality is structural up to
ghosts, constructors of different sites differ; order is Move's
lexicographic order (handle rank, handle, instantiation, mask, captures).
A value is storable where its captured rows are reference-free.

**The requires meaning** (`functionRequires`, generated per link set),
per constructor over native rows: `RequiresOf (closure_{f,θ,m} c) args :=
requires_f θ (weave_m.compose c args)`; `RequiresOf (foreign k)` is the
uninterpreted `requiresForeign`, which gives meaning to the `requires_of`
predicate only. Calling a function value needs no `requires_of`, as in the
Move Prover, which asserts none at a call of a function value (caller
obligations are stated in the specification, not checked at the call): a
model unit's invocation of a value is defined where its site's target's
precondition holds and, of a `foreign` value, always (the definedness
meaning `functionRequires (fun … => True)`); an executable unit's by
typing. A link set none of whose sites' targets declares a precondition
proves every invocation defined (`functionRequires_everywhere`). Off the
base frame, `requires_of` of a construction is read at the base frame
(`requiresOf_construct`); at rows without type parameters its arguments
reach the target's precondition (`toRuntimeFree`).

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
`DenotePerformance`, the native inventory empty. 3 (done): the requires meaning per
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

**Step 2b, decided** (2026-10-28):

- A generic target is verified at a skolem frame, where every function
  value is a variable and only the interface's laws apply (construction,
  invocation, equality, order); closure sites are enumerable where
  monomorphization says, at the base frame. A unit's function values are a
  class, `FunctionCarriers unit` (a carrier per function type, its
  `TypedFunctions`, and two laws: every value's closure is one the unit
  types, every closure the unit types decodes), with exactly one instance
  per unit constant, declared where the unit is registered, before any
  theorem. `Carriers.runtime unit`, `Skolems unit`, `Memory unit` and
  `Comp unit` keep their spellings under the instance: storage holds the
  unit's function values, the base frame's transports stay identities, and
  `Carriers.instantiate` keeps forwarding function carriers to the outer
  family at the substituted rows. A field of the frame would allow two
  frames of one unit at different function values, where memory needs one.
  `FunctionCarriers.runtime unit` (`RuntimeFunction`) is a definition, not
  an instance: the instance of a unit without a generated block and of the
  library's tests. The agreement holds for every unit and instance.
- Generated per link set (the unit and the dependencies its theorems
  relocate), once, before the unit's first target theorem
  (`ensureFunctionCarriers`): one indexed
  family `FV : NRow → Sharing → NRow → Type`, a constructor per closure site
  at its closed rows (a site inside a generic function once per
  instantiation the link set uses) and `foreign (key : ForeignClosure unit p
  s r)` at every index. A capture's carrier is spelled concretely (`NTy.carrier`
  at the family under definition is not a strictly positive occurrence) and
  checked once by `rfl` against `NTy.carrier` at the base frame; the
  encoder and decoder match on the index and the sites, and the
  `TypedFunctions` laws are generated per constructor. A `split` of a
  function value at the base frame is `cases`, whose index selects the sites
  of its type.
- Construction and dispatch at the base frame are generated equations of
  the normal form: `functionOf` of a site's target, mask and instantiation is
  its constructor, the meanings' invocation of a constructor is the target's
  invocation at the woven arguments, of `foreign k` the hypothesis
  `invokeForeign k`. At an abstract frame nothing reduces.
- Per function type one hypothesis bundle for `foreign` (`invokeForeign`:
  definedness without a precondition, coherence across
  frames, the declared predicates and frame) replaces `verexInvoke`, its
  definedness and coherence binders and the per-member routing; generic
  targets are constructors at their instantiation. `FunctionTyped`,
  `ClosureTypedAt` and the solver's carrier typing rule go: typing holds by
  construction.

Sub-steps, one commit each, each with the step's gates: (i) the class
`FunctionCarriers`, its laws for the runtime package, the declarations
under its instance, and the runtime instance emitted at each unit's
registration, a refactor that changes no outcome or cost; (ii) closure sites
(target, instantiation, mask, captured row, function type, grouped by type
and mutual where captures hold closures, over the link set), with a fixture
printing them; (iii) the generated datatypes and construction, `.closure`
and `.closureGeneric` denoting constructors, the typing machinery deleted;
(iv) invocation by the generated match, the foreign bundle, the
`closureMeaning_closureOf_*` lemmas deleted, the two proof files under
`lean =>` updated; (v) equality, order and storage derived from the
generated structure. Kept for step 3: `RequiresOf` through
`ClosureMask.compose` and the requires table.

Status: (i) to (iv) done; (v) done except where noted. The library
holds the family interface (`SiteFamily`): the generator supplies the
inductive with typed captures, the structural encoder and the site table,
and the library derives the decoder (by closure size, over the codec
parameterized by its function arm, `NTy.decodeWith`), the four laws and
the instance. A closure whose captures do not decode at its site's row is
`foreign`, so every admitted closure decodes. Construction is a frame
operation, `Skolems.construct`, typed by its site (`SiteTyped`, what a
closure node carries; a clause's literal by the unit's check,
`SiteTyped.ofCheck`, whose proof the clause binds), irreducible; no
`Carriers` field, so the ground family keeps no function values. The sites
include the literal function values specifications write. Per site
without type arguments the generator proves its constructor's closure, the
unit's check, its invocation as its target's call at the woven arguments,
and the typing of the value naming its target, and registers them
(`SiteEquations`); the normalizer rewrites construction and invocation at
the base frame by them (`constructSite`, `invokeSite`), and the solver's
dispatch rules read a construction and a site's constructor alike
(`LiteralValue`). `FunctionTyped`, `ClosureTypedAt`, the literal typing
rule and the leaf-time evaluation of a literal's check are gone; the
unseen-invocation rule takes its typing from the carrier
(`FunctionCarriers.admits` through the frame's transport, where the
readings agree), and a function value at a skolem frame invokes by it.
Equality and order of known function values (a construction, a site's
constructor) are read off the generated structure by the normalizer
(`knownFunctionsEqual`, `knownFunctionsOrder`): at one key, the captures'
equality without ghosts and the captures' order; at different keys,
unequal, and ordered by the keys (`closureKeyOrder`); a value the leaf does
not know keeps the interface's opaque key and order. Storage needs nothing
further: a stored function value is encoded and decoded by the generated
codec. Decided for (iv) (2026-10-29): definedness stays `verexInvokeDefined` under
the requires table for every value (`requiresForeign` and the
per-constructor table are step 3); executable units keep `closureMeaning`,
`foreign k` opaque there; a model unit keeps `verexInvoke` with its
definedness and coherence, its routing restated per closure site at the
base frame. Open: the invocation equations of sites with type arguments
(they keep `functionOfGeneric` and its invocation rule); function values
under a vector or map in a capture. A carrier whose length certificate is a
proof over its elements cannot nest the family in Lean, so (decided
2026-10-29) the vector and map carriers drop their certificate; the bound
becomes a leaf fact like an integer's (supplied where a value enters a
leaf, required by the encoding). Postponed as an open gap; until it is done
the generator rejects such captures.
