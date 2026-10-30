# Higher-order functions

Open work: [`roadmap.md`](../plan/roadmap.md). The replacement of the
closure carrier's runtime captures and of the requires table by native data
is planned in
[`native-function-values.md`](../plan/native-function-values.md).

Move function values (language 2.2) are closures: a function, a mask saying
which of its parameters are bound, and the bound values. Their
specifications (language 2.4) use behavioral predicates (`requires_of`,
`aborts_of`, `ensures_of`, `result_of`) and state labels
([`state-labels.md`](state-labels.md)). This design carries them through
VIR, execution, the denotation, and verification, and connects them to Move
sources, Move bytecode, and Rust function pointers.

Sources: the Move Book ("Function Values", `spec-compositional.md`), the
Prover's paper (`move-prover/doc/higher-order-paper-26`) and implementation
(`bytecode_translator.rs`, `mono_analysis.rs`), the bytecode instructions
(`PackClosure`, `PackClosureGeneric`, `CallClosure`, `ClosureMask`), and
`mono-move/docs/closure_design.md`.

## Move semantics

- A function type `|T1, …| R has A` has abilities from `copy`, `drop`,
  `store`. Naming a function gives `copy + drop`, plus `store` when the
  function is persistent (public, entry, or `#[persistent]`).
- `PackClosure(f, mask, c1..cn)` binds the parameters of `f` whose mask bit is
  set, in any positions, to the captured values; its type is the function
  type of the remaining parameters. Capturing moves the values. A closure's
  abilities are `copy + drop` intersected with those of every captured value,
  plus `store` if `f` is persistent (a partial application of a persistent
  function keeps `store`).
- `CallClosure` consumes the closure and calls
  `f(mask.compose(captures, arguments))`.
- References cannot be captured. Lambdas are lifted to functions by the
  compiler before any backend sees them.
- Equality and ordering compare the module, the function name, the type
  arguments, the mask, then the captured values (`values_impl.rs`,
  `LazyLoadedFunction::cmp_dyn`), never behavior.
- Calling back into a module already on the call stack locks that module's
  resources: resource operations on them abort. `#[module_lock]` locks every
  re-entered module. Neither the Prover nor MonoVM models this
  (`mono-move/runtime/src/interpreter.rs`, the reentrancy TODO).

## VIR

**Masks.** `CallKind.closure` carries the mask, a bit set over the target's
parameters as in Move; a prefix is the mask with the low `n` bits set. The
alternative, adapter functions that reorder parameters, would add functions
the source does not have and break the VerexSL round trip. `compose` and
`extract` are defined once in `VerexIR` and used by validation, the
runtime, and the denotation.

**Instantiation.** A closure of a generic target fixes its type arguments at
construction. `RuntimeValue.closure` carries the handle, the mask, the
captures, and the instantiation that `callTypeInstantiation` computes from
the constructing frame; `invoke` calls under that instantiation, not the
caller's.

**Validation.** For every profile, until Rust closures need more:

- no captured type contains a reference;
- the function type's abilities are within those the captures and the
  target allow (above); Move's persistence rule is a Move profile rule
  (`ProfileSchema.storableTarget`, defined once as
  `VerexSL.moveStorableTarget` over both spellings of the modifiers and
  used by the VerexSL and Move registries, `VIR-CLOSURE-STORE`);
- `invoke` requires the operand's function type, arity, and argument types
  (exists), and consumes a non-`copy` function value;
- the borrow analysis treats `invoke` as a call: reference arguments are lent
  for its duration, and a returned reference is a loan of the invocation
  site, registered as for a direct call.

**Values.** `ValueHasType` for a closure requires the target's signature,
instantiated and with the mask applied, to match the function type, and the
captures to have their parameter types. Closures compare as the VM compares
them, by qualified function name first; the runtime reads names through the
unit, not handle indexes.

## Runtime

`closureValue` evaluates the captures and builds the closure with its mask
and instantiation. `invoke` composes captures and arguments by the mask,
calls the target under the closure's instantiation, and registers a returned
reference as `callReturned` does. The interpreter follows the rules; the
soundness, fuel, and completeness proofs extend case by case.

The re-entrancy lock needs the modules on the call stack in the state and a
locked set that resource operations consult, with MonoVM's own support.
Until then the semantics, like the Prover's, does not abort on re-entrant
resource access, and MonoVM differential fixtures avoid it.

## Denotation

A function value is a value of its function type's carrier, which the
family provides (`Carriers.functionCarrier`, "Closures in memory" below):
construction, invocation, equality, and order go through that interface,
and the runtime family implements it by the closures the runtime holds
(handle, mask, instantiation, and captures as runtime values that hold no
loan), so equality, ordering, storage in structs, and passing are those of
any value. `Term.closure handle weave results captures` builds the function
value of a target without type arguments (`Skolems.construct`, a frame
operation seen in leaves only through its two equations),
`Term.closureGeneric` one with them (`functionOfGeneric`, construction at
the induced instantiation); the `Weave`
relates the target's parameter row to the captured and supplied rows, and
the compiler checks that its mask is the program's. `Term.invoke shape f
args` denotes `closureMeaning executable σs shared shape f args`: the
prophetic meaning of the value's closure's target under its instantiation,
with its captures composed into the lent arguments by its mask
(`closureRun`).

A known closure invokes as a call of its target (`Denote/Closures.lean`):
`closureRun_closureOf` equates the run with the target's
`propheticMeaning` on the woven arguments; for a generic target,
`propheticMeaning_subst` carries the meaning to the family the type
arguments induce, the form a direct call with type arguments has. At the
base frame a closure site of a target without type arguments is a
constructor of the link set's generated family: construction is the
constructor and invoking the constructor is the target's call
(`construct…`, `invoke…`, registered as `SiteEquations` and applied by the
normalizer), a function value at an abstract frame reduces no further. The
solver reads a known function value, a construction or a site's
constructor, by the proof that its closure is its target's at its captures
(`LiteralValue`), so the target's theorem is used as for a direct call,
and closure targets join a function's callees. Since
`invoke`'s denotation is defined from the big-step meaning, its case of
the agreement is by definition.

## Relation to the Prover's encoding

The Prover represents the function values of a type `τ` by a datatype of the
program's sources (`encoding.tex`, "Representing Function Values"): a
constructor `C_f(c̄)` per closure site, `P` per function-typed parameter of the
verification target, and `F(n)` per function-typed struct field. The datatype
does three things:

1. **First-order representation.** SMT has no function values. The
   constructors stand for them, and uninterpreted per-variant predicates
   (`requires_P`, `aborts_P`, `result_P`) stand for an unknown function's
   behavior, tied together by axioms: the one-directional result axiom, and
   `ensures_P` defined as the graph of `result_P`.
2. **Dispatch.** `invoke_τ` branches on the constructor: a closure calls its
   function, and a parameter or field goes through the behavioral predicates.
3. **Closed world.** `τ` has no other values, so a verification condition
   sees only the function values derivable from the program, and a value
   whose source it cannot see is still one of the program's closures.

Lean is higher order, which removes the need for the first, keeps the second,
and requires the third to be stated as a theorem.

**Representation.** A Lean proposition can quantify over closures and state
the meaning of an arbitrary one, so a function value is its closure, and the
behavioral predicates are definitions over its meaning (next section) rather
than uninterpreted functions with axioms. What the axioms had to get right
holds by construction: `result_of` is the result the function computes, since
the big-step relation is deterministic, and a target without a specification
has exact predicates rather than a vacuous default.

**Dispatch.** The schema stays, applied to terms instead of constructor tags.
At an invocation, a literal closure is `C_f`: it reduces to the direct call. A
parameter is `P` and a value read from a field is `F`: their predicates are
opaque atoms, constrained by the contract's `requires` and by data
invariants. The solver's discipline mirrors the Prover's triggers: a
predicate is unfolded only for a literal closure, through its target's
theorem.

**Closed world.** During an execution of the verified function, a value of
`τ` comes from a closure site that the execution runs, from the arguments,
directly or nested in data, or from the initial state; a callee's results
derive from the same three. The Prover's datatype has `C` variants for the
sites in the verification root's slice, `P` only for parameters of function
type, and `F` only for fields of function type; every other value is forced
into the closed world, and two probes verify specifications the same code
violates at run time (aptos-core #20650): `apply_first` invoking a vector
element verifies `aborts_if false`, and `use_make` invoking the closure an
opaque `make()` returns takes it for the one site in the root's slice.

In Lean the closed world is a theorem: every closure an execution creates
is a closure of a site of the executable unit; values from the arguments,
however nested, and from the initial state are arbitrary, where a type
without `store` cannot occur in the initial state. It is proved once, by
induction over the big-step relation, and the solver uses it on demand
where a proof cannot see a value's provenance: the value is one of the
unit's closures of `τ`, a case split whose cases are direct calls, or it
came from an input and is known through contracts only. Under the modular
discipline a callee's contract describes the closures it returns, so the
case split is a fallback rather than the representation.

## Behavioral predicates

Three predicates are defined from the closure's meaning, exactly and without
axioms; the fourth from its target's contract. The meaning is the
invocation meaning of the unit (`closureMeaning executable`, or a model
unit's `invoke`) on native arguments from a start with the globals of `S`
(`Proofs/Behavior.lean`):

| Predicate | Definition at `x`, pre-state `S`, post-state `T` |
|---|---|
| `aborts_of<f>(x)` | the invocation aborts from a start at `S` |
| `ensures_of<f>(x, r)` | the invocation returns `r` from a start at `S`, leaving the globals of `T` |
| `result_of<f>(x)` | the result the invocation returns from a start at `S`, read at every default where it returns none |
| `requires_of<f>(x)` | the target's declared precondition at the captures and `x`, read by the link set's requires meaning (`functionRequires`); of a value no closure site encodes, the uninterpreted `requiresForeign` |

A contract states partial correctness, not that a run exists, so
`result_of` of a known function value is its target's result only where
the invocation returns, which `¬aborts_of` does not give. The theorem of a
function whose contract, or a callee's at any depth, states `result_of`
takes the hypothesis `Terminating` (runs of the unit's functions end, the
Move Prover's assumption), under which an invocation that does not abort
returns (`returns_of_terminating`); a theorem whose verified body states
`aborts_of` or `result_of` in an invariant, assertion or assumption
assumes the same (`bodyReads`).

`requires_of` names a contract: a Move function's meaning has no
precondition, but a caller needs the target's precondition to use its
verified contract. The requires meaning maps each closure site's
constructor to its target's `requires` at the woven arguments
(`buildDeclaredRequires`), one `vir_denote_norm` equation per constructor;
a precondition stating a behavioral predicate would read the meaning
itself, and a contract reading it is rejected. Calling a function value
asserts no `requires_of`, as in the Move Prover: definedness of a call
needs a site's precondition, of a value no site encodes nothing. A verified target's
theorem then yields what the Prover asserts as compliance: under
`requires_of`, `ensures_of` implies the postcondition and `aborts_of` the
`aborts_if`.

**Literal closures.** Of a closure whose target, weave, and captures a leaf
sees, the predicates reduce to the target: `requires_of` to its declared
precondition at the woven arguments (the generated `requires` equations,
off the base frame through `requiresOf_construct`), `ensures_of` and
`aborts_of` to runs of its prophetic meaning (`ensuresOf_closureOf`,
`abortsOf_closureOf`), which the solver reads by the target's theorem, or,
for a target without one, by its body, as the Move Prover infers the
specification of such a lambda: a hypothesis stating `ensures_of` or
`aborts_of` of it becomes the weakest precondition of its meaning under
which the leaf holds of every run or abort (`forall_ok_of_wp`,
`of_aborts_of_wp`). The aborts of an inlined target are read by its body
even where it has a theorem, since a specification without `aborts_if`
does not state them. `result_of` of a literal closure whose abort is
undecided is decided case by case on the abort. A parameter of a shared
reference type is the value it observes (`observedType`). A closure that
captures values behaves as the function value naming its target at the
woven arguments (`abortsOf_closureOf_captured`, `verex_woven_behaviors`).

**Mutable-reference arguments** follow the prophetic model, in the Prover's
argument layout (`spec_translator.rs`, `bytecode_translator.rs`): a
predicate's inputs are the arguments' values, a `&mut` one's its entry
value; `ensures_of` takes after them the declared results and then the final
value of each `&mut` argument, in order (`ensures_of<f>(old(x), x)` for
`f: |&mut T|`); `result_of` names the declared results alone, and
`requires_of` and `aborts_of` the inputs alone. A `.ref` position of the
predicates is an `(entry, final)` pair: the invocation runs with each
`&mut` argument lent under a loan of its own, and the final value is the
one the run exports for the loan, with the holes of returned references
filled, as `argumentsResolve` reads a call. A contract reads a `&mut` input
at the invocation's pre-state (its entry value, or its copy at a pre-state
label), as the Prover reads `old` of it, and a final value of `ensures_of`
at the post-state (`Contract.mutableAt`). A result that is a mutable
reference is read as its referent's value at return (`readResults`). Lent
invocations are deterministic up to loan identities
([`static-typing.md`](../historical/static-typing.md), "Loan
independence"), so `result_of` is the results every run returns where those
hold no loan, and a returning run refutes `aborts_of`.

A call of a target whose parameters and result are each a mutable
reference or hold none is a run of the function value naming it
(`wp_named`, `wp_named_typed`): where its returned reference is settled
(`ResultShape.Settled`), the continuation assumes `ensures_of` at the
call's entry values, the result's current value, and the arguments' final
values, that `result_of` is that result, and the absence of aborts. The
solver names such a call where the goal owes a behavioral predicate
(`obligesBehavior`).

**Frames.** Without `modifies_of`, Move treats a function parameter as not
modifying global memory, and the compiler checks every closure passed for it.
Here the frame is a precondition of the higher-order function: `f` may
change global memory only within the declared resources and addresses.
The theorem of a verified body with a function-typed parameter assumes,
beside its typing, that it keeps memory (`KeepsMemoryAt`: every run from
typed memory at reference-free arguments ends in the memory it started
from), so a loop whose body invokes only parameters it does not reassign
keeps its state. A caller establishes the frame of a literal closure from
its target's theorem (`KeepsMemoryAt.ofVerified`) or from the target's
body (`KeepsMemoryAt.ofBody`), as the Move Prover derives a closure's
footprint from its code. The Move frontend reads an invocation of such a
parameter as writing no global memory, so a higher-order function's frame
is closed. `modifies_of<f>(a) R[a]` reaches VIR as a parameter frame: the
theorem then assumes `FramedAt`, the closed frame but for the targets with
the formals bound to an invocation's arguments, established the same two
ways; `modifies_of<f> *` assumes nothing, and the frontend reads the
invocation as a write. A frame a hypothesis states establishes any wider
one (`FramedAt.mono`); a stored closure's frame a generic callee requires
is read in the caller's view (`FramedAt.of_instantiate`).

Default frames of directly function-typed Move fields are implicit data
invariants (`KeepsMemoryAtOf` on the field's value), checked or carried by
construction, mutation, parameters, results and stored resources; an
invocation of a closure read from storage recovers its frame from
`MemoryInvariants.read`. Explicit struct-field write frames use
`FramedAtOf`: the formals bind invocation arguments, resource types are
instantiated in the enclosing nominal's scope, addresses are evaluated in
the invocation's pre-state, and a wildcard frame imposes no restriction.
Neither constrains function values supplied as type arguments or nested in
vectors. Dependency interfaces omit nominal contracts, so an explicit write
frame at that boundary is rejected; importing it as an empty frame would be
unsound. `reads_of` is left out (non-goals).

**Verification.** In a higher-order function's proof the predicates over its
parameters are opaque atoms. The solver rewrites `invoke` on a variable with
one lemma, `wp` of the call as the predicates, and then works with the
contract's clauses about them. `invoke` on a literal closure reduces to the
direct call, and closure targets join the callees whose theorems a proof
assumes, so a caller of a higher-order function uses the target's contract as
it would for a direct call. Generic higher-order functions need nothing
further: the carrier does not depend on type parameters.

**Decisions.** `wp` of an invocation of an unknown closure excludes the
outcome in which the target's results do not decode at the function type
by preservation of runtime typing for `EvalFunction`, which the agreement
proof also needs, not by an assumption on every function-typed input (it
would have to be conditioned on `requires_of` and fails for targets
without a theorem). `requires_of` is a case split over the values a proof
can name, a definition generated per link set mapping each closure site's
constructor to its target's `requires` at the woven arguments and every
other value to the uninterpreted `requiresForeign`; the closed-world theorem restricts the split to the unit's
closures. A contract using a predicate takes the executable unit as a
parameter, which the theorem already quantifies, since the meaning is the
unit's.

## VerexSL

`Fn(A, B) -> R has copy, drop`, `function[F](target, captures…)`,
`invoke(f, args…)`, the predicates, and `@n` labels, plus:

- a mask by position: `function[Fn(u64) -> bool](f, _, c)` binds the second
  parameter; `_` marks a parameter supplied at invocation, and a prefix needs
  none;
- generic targets: `function[Fn(T) -> T](f::<T>, …)`;
- `f(x)` for a function-typed local, printed back as `f(x)`;
- `modifies_of<f>(a: address) R[a]` in a specification, for the frame above.

Lambdas stay out; lifted Move lambdas arrive as ordinary functions.

## Move frontend and backend

- **Exchange.** The producer exports `Operation::Closure(module, function,
  mask)` with its type arguments and captures as a closure node, encoded as
  `CallKind.closure`; lifted `__lambda__` functions export as ordinary
  functions, and `modifies_of`/`reads_of` of function parameters with the
  specification. A skipped declaration carries an `inline` flag: retained
  inline functions are passed over, any other skip is an unsupported
  declaration.
- **XIR.** The function type and the operations `closure`/`closure_inst`
  (target, mask, type arguments; the captures are the operands) and
  `invoke` (the function value last, as in stackless bytecode); a
  closure's target is a used, not a called, function of its creator.

## Rust

Function pointers reach VIR as closures without captures. Capturing
closures take their environment by reference (`Fn`, `FnMut`) or by value
(`FnOnce`); the reference cases need captured references, which the
prophetic model can express but this design excludes. They are a later
milestone of the Rust frontend, with the `Fn*` traits specialized as other
traits are.

## Closures in memory: a typed carrier

**Why types come up.** A theorem quantifies over values of their native
carriers. A data type's carrier states its static type (a `u64` is an
integer with its range proof, `SpecInt`; a structure a row of typed
fields), so every quantified value is well typed and no proof reasons about
types. A runtime closure, a target and captures, may name a target of any
signature: as a carrier it would be the one that does not state its type. Without more, a closure read from memory or passed in is
typed only by a hypothesis, and memory with a function-typed field is not
typed for free.

**Decision.** A function type gets a typed carrier, as `u64` has one: the
closures whose target has the type's signature at its frame, with captures
typed. The typing is a static fact about the unit and erased at run time;
nothing is carried by the runtime, and no proof obligation states it. Then
no function value, a parameter, a field, a stored closure, an invocation's
operand, needs reasoning about types, and the rule for an unseen closure
takes its typing from its carrier.

**Typing.** A closure inhabits a function type at a unit in the
denotation's own types: `.function ps rs` holds the closures whose target's
signature at their frame compiles to a captured row, `ps`, and `rs`
(`closureRowsIn?`, the compiler's reading of the target's types), and whose
captures inhabit the captured row. Inhabiting a native type is decided
structurally on the runtime value (`NTy.admits`), a closure capturing
closures recursing into its captures; every encoding of a carrier value
admits its type. A type parameter of the runtime family admits any runtime
value.

Native types erase what semantic types state: a shared reference is the
value it observes. A function type keeps which of its parameters and
results are shared references (`NTy.function parameters shared results`,
`shared : Sharing`), so `|&u64| bool` and `|u64| bool` are distinct types,
as are `|u64| &u64` and `|u64| u64`, and a nominal type keeps its
type arguments as native types beside its rows (`NTy.struct source
arguments fields`, enums alike), so `|Cap<A>|` and `|Cap<B>|` of
`Cap<phantom F>` are distinct. A native type then reads as one semantic
type, and a closure's native typing fixes its function type's semantic
signature exactly, as `HasType.closure` requires. Runtime values carry no
type arguments, so admission and the carriers ignore them.

A shared-reference result reads as the value it observes, as a parameter
does; a mutable reference result reads as itself (`.ref`). A function type as a
type argument (`Option<|u64|u64>`) is not carried either: the frame a
generic call induces defaults each parameter to an inhabitant of its
argument (`Carriers.default_instantiate`), which a function type does not
have on its own (`NTy.inhabitable`), its values being the unit's typed
closures; the fix is a default the unit provides, or a family without
defaults.

**Families at a unit.** `NTy.carrier (.function ps shared rs)` is the
family's function carrier (`Carriers.functionCarrier`), and the family
provides its interface (`TypedFunctions`): each value's runtime closure, and
the value a runtime closure is where the family types it; the codec,
equality without ghost slots (an opaque key, `erased`), and order derive
from them; of known function values the normalizer reads them off the
closure's target, mask and instantiation and its captures as native values
(`erased_eq_closureOf`, `order_closureOf`, and at different keys
`erased_eq_closureOf_ne`, `order_closureOf_ne`). The runtime family's carrier is the closures the unit types
(`RuntimeFunction`, its own type so that no verification condition unfolds
a function value to its closure), an induced family's the outer family's at
the substituted rows, and the ground family has none. A closure's
construction is a value of the carrier where the family types it
(`Skolems.construct`, typed by its site, `SiteTyped`), invocation an invocation meaning over
values of the carrier (`InvokeMeaning`), and the behavioral predicates are
stated over those values. The runtime family is built at a unit
(`Carriers.runtime unit`), and so is every frame (`Skolems unit`), so
`Memory`, `Comp`, and `Meanings` take the unit. An executable unit is
indexed by the validated unit it executes (`ExecutableUnit unit`), so a
theorem's frames, memory, and carriers are at the module's unit constant:
closed terms, whose reductions Lean's caches keep (`whnf` caches no term with
a free variable). A frame's closure typing is the runtime family's at the
rows it resolves, as its transports to the runtime family force
(`Skolems.ofClosure?_of_runtime`).

**Typed by construction.** Typing is never a proof obligation. Compiled
code is indexed by its unit (`Function unit`, `Term unit`), and a closure
node carries the proof that its target's signature at the unit splits by its
mask into its rows, which `compileFunction` establishes once by evaluation.
A closure is created typed from that proof and two facts the runtime
meaning holds: the unit's readings agree (`TypesAgree`, the compiler's
native type and the checker's semantic type of every type identifier are
one type, decided once per unit by evaluation), and the target's frame is
coherent (below). `propheticMeaning` has an outcome only where both hold,
so taking them at creation leaves the agreement as stated. Decoding a
runtime closure checks its typing, so the codec stays tight; an inhabitant
of a function type is a typed closure of the unit, if one exists
(`NTy.inhabitable`), not a default.

A closure is constructed with its site's typing (`SiteTyped`: its target's
rows and sharing at its instantiation are the frame's resolution of its
rows, and the instantiation is faithful), which a closure node carries. A
specification is generic in its unit, so a literal function value it writes
is typed by the unit's check (`SiteTyped.ofCheck`, `closureTypingCheck`)
whose proof it binds (`bindCuts`); a literal a specification writes is a
closure site of the link set, whose check the generated block proves once
by evaluation, and the leaf reads the binding off that theorem.

**Coherent frames.** A generic function is verified once, at a skolem
frame (D3, [`denotation.md`](denotation.md)). Its type parameters appear
there twice: as the frame's types, and in the runtime type instantiation
that its closures and storage keys carry. The Move Prover has one type per
parameter; a frame is coherent when its two agree: its runtime
instantiation is faithful (`FrameInstantiation`) to semantic arguments that
are its types' semantic types, wherever its types have one. This
generalizes `Coherent`. At a coherent caller, a closure's target frame is
coherent by static typing (`FrameInstantiation.call`), which the agreement
proof carries. No theorem shape changes, and no call owes coherence.

`Coherent` speaks of the types the frame reads, as the runtime reads them:
the types it requires (`requiredAt`, the resource types of its storage
operations and the type arguments of its calls and closures, closed under
its calls) and its type parameters, each read at its node through the
instantiation. The runtime instantiation maps only the types static typing
interns, exactly the required ones, so a clause over every type of the
namespace would fail at real frames. Over the required types and the
parameters, coherence holds at the runtime frame, and a coherent caller's
calls and closures have coherent frames. A closure's rows at its
instantiation are its target's own rows with each type parameter replaced
by its argument (`ClosureTyped`); at a coherent target frame they are the
frame's resolution of the target's rows.

**Public theorem.** A function without type parameters is stated at the
empty instantiation. A generic function's public theorem is stated at
every type instantiation a call gives it (`FrameOf`) and type arguments
`θ` whose induced frame is coherent with it, its contract read at the
arguments' types (`Contract.ofSkolem`, `satisfies_generic_at`); a caller at
concrete type arguments discharges the coherence by evaluation.

**Semantic typing from the native.** The runtime's typing (`HasType`) is
what an unseen invocation and typed memory need; carriers state the native
one. Three pieces relate them.

- *The readings agree* (`TypesAgree`). `NTy.TypedAs` relates a native type
  to the semantic type its encodings inhabit (a function type to the
  function type of its related rows, a parameter marked shared to a shared
  reference); a native type reads as at most one semantic type
  (`TypedAs.unique`), and both readings commute with substitution
  (`TypedAs.subst`). A unit's readings agree when every closed type
  identifier the compiler reads natively (`ntyOf`) is `TypedAs` its
  semantic type, and every function's signature is, over its generic
  environment; decided once per unit by evaluation. A unit whose readings
  do not agree is reported as not memory-typed.
- *Faithful closures.* `HasType.closure` types a closure only under an
  instantiation faithful to its arguments (`FrameInstantiation`), which
  the native typing also checks in its decidable form; a closure the
  runtime builds at a faithful frame is faithful (`FrameInstantiation.call`).
- *Closed frames.* A carrier value's encoding has the semantic type of its
  native type when that type is closed; a coherent frame of a faithful
  instantiation resolves each parameter with a node to a closed native
  type, so a generic body's proof has the semantic types of its
  parameters' values wherever its frame is coherent.

Then a carrier value of a closed type at the runtime family is typed at
the semantic type its native type is `TypedAs` (`NTy.encode_hasType`).
Memory is typed for every resource type the readings agree on, function
fields included, and an unseen invocation's typing is derived from the
invoked value's carrier, so contracts do not assume it of function
parameters. The rule for an invocation of a function value a proof does
not see (`wp_closureMeaning_unseen_at`) takes the closure's typing
(`ClosureTyped`) from the carrier (`FunctionCarriers.admits` through the
frame's transport, where the readings agree), and its arguments' semantic types and its results' decoding (`RowTyped`,
`RowDecodes`), facts of the rows at the frame, decided at rows the frame
resolves to scalars (`ScalarAt`); a generic function whose function-typed
parameter's rows mention its type parameters assumes that its frame
resolves them to scalars, which a call discharges at its type arguments.

## Non-goals

- Lambdas in VerexSL.
- Captured references, for Move and Rust alike.
- `reads_of`: a meaning is exact, so reads need no declaration to be sound;
  what it adds, independence of the result from other memory, can follow.
- `unchanged_of` and `folds_of`, which exist only for lambda arguments of
  inline functions, expanded before export.
- Enumerating closures of code outside the executable unit: such closures
  reach a verified function only through its inputs, which are arbitrary.
