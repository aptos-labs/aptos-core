# The lowering, level by level

## Scope

How one function goes from source to plain Lean arithmetic, and what each
thing is called at every level on the way. This is a map across the other
designs, not a design of its own: each level names the document that defines
it. The excerpts are the verifier's output for the example below, shortened:
module prefixes are dropped, the generated nominal name of `Counter` is
written `Counter`, and unit tails are written `()`. A source reference links
to the declaration's line.

## The example

```
verex namespace lowering_demo where
  struct Counter where
    n : u32
  spec Counter where
    invariant n <= 100
  fun bump(c : &mut Counter) -> Unit := do
    (*c).n := (*c).n + 1u32
  spec bump where
    requires c.n < 100
    ensures c.n == old(c.n) + 1
```

## The map

Three things are lowered side by side: the body, the types and the
specification. Read down a column to follow one of them. The last column
names what produces the next level.

| Level | Code | Types | Specification | Lowered by |
|---|---|---|---|---|
| Source | `(*c).n := (*c).n + 1u32` | `&mut Counter`, `u32` | `requires c.n < 100`, `ensures c.n == old(c.n) + 1`, `invariant n <= 100` | a front end: VerexSL, Move, Rust |
| VIR (`eN`, `pN`, `tN`: an index into the arena of expressions, places, types) | `e6: assign p2 e5`, `e5: add e3 e4`, `e3: copy p5` | `t2`, `t5: reference mutable t4` | `e19: equal e13 e18`, `e16: old e15` | `compileFunction` for the body; the clause translator for the specification |
| Typed term | `Term.writePlace 27 none Var.here (Proj.field …).deref (Term.modular ModularOp.add …)`: a value of `Term unit ρ Γ τ` | `NTy` and `NRow`: `NRow.cons Counter.ref NRow.nil`, `NTy.int 32 false`, `NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 32 false) .nil)` | `requires := fun arguments state => Obligation … (arguments.fst.fst.fst.val < 100)`: a Lean proposition in a `Contract` record, with no term level | `Term.denoteIn` |
| Denotation | `Spec (Memory unit) Failure (Flow …)`, built from `Spec.bind`, `Spec.assume`, `certifyingAt` | `NTy.carrier τ` (certified) and `NTy.plain τ` (no certificate): Lean types computed from the code | `Satisfies (fun args => …) bump.typedContract` | unfolding `Satisfies` to `wp` |
| Verification condition | `⊢ wp (Flow.bind … (Term.denoteIn …)) ensures aborts initialState` | `c_n : NTy.plainOf … (NTy.int 32 false)`, `c_final : Counter.carrier`: the reference as its current and its prophecy | `permitted : c_n.val < 100`; `c_final.val.fst.val = c_n.val + 1` | the solver's walk |
| Leaf in source form | `(c_n.val + 1) % 4294967296` | an `Int` with `0 ≤ c_n.val` and `c_n.val ≤ 4294967295` as facts | `verexInvariant : c_n.val ≤ 100`; `⊢ (c_n.val + 1) % 4294967296 ≤ 100` | `omega` and the leaf deciders |

The theorem that results is
`lowering_demo.bump.typedVerified : … → Satisfies (fun args => …) lowering_demo.bump.typedContract`.

## The declarations the excerpts use

Sketches of the declarations in [`verex-ir/VerexIR/Proofs`](../../verex-ir/VerexIR/Proofs) that the levels
below are written in. Each is abridged to what the excerpts need: omitted
constructors and fields are marked `…`, and implicit arguments are dropped.

### Type codes and the Lean types of their values

A type of the source is a piece of data, a code. Functions compute from a
code the Lean type its values have.

Sources: [`NTy`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L46), [`NRow`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L102), [`SpecInt`](../../verex-ir/VerexIR/Proofs/Representation.lean#L31), [`NTy.plain`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L3505), [`NTy.carrier`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L3657), [`HList`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L3691), [`HEnv`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L7164), [`ResultShape`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L7395), [`ResultShape.carrier`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L7400), [`Memory`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L7648).

```
inductive NTy where                    -- a type code
  | unit | bool
  | int (width : Nat) (signed : Bool)
  | ref (referent : NTy)               -- &mut
  | struct (source : StructHandle) (arguments fields : NRow)
  | …
inductive NRow where                   -- a row of codes: fields, parameters, locals
  | nil
  | cons (head : NTy) (tail : NRow)

structure SpecInt (width : IntWidth) (signed : Bool) where
  val  : Int                           -- a mathematical integer
  fits : IntegerValueFits width signed val                   -- in range

def NTy.plain   : NTy → Type           -- the shape alone
def NTy.carrier : NTy → Type           -- the shape with its data invariants certified
  --  .bool             ↦ Bool                       in both
  --  .int w s          ↦ SpecInt (.bits w) s        in both
  --  .struct _ _ fs    ↦ the fields' product        in plain;
  --                      { v : plain // invariant } in carrier, hence `.val`
  --  .ref τ            ↦ τ.plain × τ.carrier        current, prophecy

def HList : NRow → Type                -- a row of values: the arguments
  | .nil         => Unit
  | .cons τ rest => τ.carrier × HList rest
def HEnv : NRow → Type                 -- the locals; a moved-out slot is `none`
  | .nil         => Unit
  | .cons τ Γ    => Option τ.carrier × HEnv Γ

inductive ResultShape where            -- what a function returns
  | none
  | one (τ : NTy)
def ResultShape.carrier : ResultShape → Type
  | .none  => Unit
  | .one τ => τ.carrier

def Memory (unit : ValidatedUnit) : Type :=    -- global storage of the unit `unit`
  (resource : ResourceType) → StorageKey → Option (resource.carrier unit)
```

`unit` in `Memory unit`, `Term unit …` and `lowering_demo.unit` is the
validated unit being verified, a value of [`ValidatedUnit`](../../verex-ir/VerexIR/Validation/Validated.lean#L461). It is not Lean's
type `Unit`.

### The typed syntax

Sources: [`Var`](../../verex-ir/VerexIR/Proofs/Denote/Types.lean#L7169), [`Proj`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L501), [`Term`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L1964).

```
inductive Var : NRow → NTy → Type where        -- a position in a row, with its type
  | here  : Var (.cons τ Γ) τ
  | there (rest : Var Γ τ) : Var (.cons σ Γ) τ

inductive Proj : NTy → NTy → Type where        -- a path from a value into a component
  | nil   : Proj τ τ
  | deref (rest : Proj τ σ) : Proj (.ref τ) σ                  -- through a reference's current
  | field (x : Var fields σ) (rest : Proj σ τ) : Proj (.struct source arguments fields) τ
  | …

inductive Term (unit : ValidatedUnit) (ρ : ResultShape) : NRow → NTy → Type where
  | lit (value : τ.groundCarrier) (literal : τ.literal = true) : Term unit ρ Γ τ
  | modular (op : ModularOp) (left right : Term unit ρ Γ (.int w s)) : Term unit ρ Γ (.int w s)
  | drop (value : Term unit ρ Γ σ) (body : Term unit ρ Γ τ) : Term unit ρ Γ τ      -- value; body
  | copyPlace (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ σ) (free : …) :
      Term unit ρ Γ σ
  | writePlace (site : Nat) (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ σ)
      (value : Term unit ρ Γ σ) : Term unit ρ Γ .unit
  | assumeCurrent (x : Var Γ (.ref σ)) : Term unit ρ Γ .unit
  | …
```

In `Term unit ρ Γ τ`, `ρ` is the enclosing function's result shape, `Γ` the
row of its parameters and locals, and `τ` the type of this term. A `site` is
the number of the source position an obligation is reported at.

### Meanings

Sources: [`Spec`](../../verex-ir/VerexIR/Proofs/Spec.lean#L23), [`Spec.pure`](../../verex-ir/VerexIR/Proofs/Spec.lean#L99), [`Spec.bind`](../../verex-ir/VerexIR/Proofs/Spec.lean#L103), [`Spec.assume`](../../verex-ir/VerexIR/Proofs/Spec.lean#L138), [`Flow`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L38), [`Term.denoteIn`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L4231), [`certifyingAt`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L139).

```
structure Spec (σ ε α : Type) where            -- a computation, as relations
  ok        : σ → α → σ → Prop                 -- initial state, result, final state
  aborts    : σ → ε → Prop                     -- initial state, failure
  undefined : σ → Prop := fun _ => False       -- a proof is owed and unmet

def Spec.pure (value : α) : Spec σ ε α                             -- return a value
def Spec.bind (action : Spec σ ε α) (next : α → Spec σ ε β) : Spec σ ε β   -- sequence
def Spec.assume (proposition : Prop) : Spec σ ε Unit
  -- continues only where the proposition holds; otherwise no outcome at all

inductive Flow (ρ : ResultShape) (Γ : NRow) (α : Type) where       -- what a term yields
  | value (value : α) (env : HEnv Γ)           -- a value and the locals after it
  | return_ (result : ρ.carrier) (env : HEnv Γ)
  | break_ … | continue_ …

def Term.denoteIn (meanings : InterpreterOps unit C State) :
    Term unit ρ Γ τ → HEnv Γ → Spec State Failure (Flow ρ Γ τ.carrier)

def certifyingAt (site : Nat) (kind : CertifyKind) (τ : NTy) (value : τ.plain) :
    Spec State Failure τ.carrier               -- the check of a data invariant
```

### Contracts and the statement proved

Sources: [`Contract`](../../verex-ir/VerexIR/Proofs/Contract.lean#L23), [`Satisfies`](../../verex-ir/VerexIR/Proofs/Contract.lean#L57), [`wp`](../../verex-ir/VerexIR/Proofs/Contract.lean#L223), [`Obligation`](../../verex-ir/VerexIR/Proofs/Contract.lean#L155).

```
structure Contract (σ ε Args Result : Type) where
  requires  : Args → σ → Prop
  assumes   : Args → σ → Prop := fun _ _ => True
  ensures   : Args → σ → Result → σ → Prop     -- arguments, state before, result, state after
  aborts    : Args → σ → ε → Prop              -- the failures permitted
  mayAbort  : Args → σ → Prop                  -- where a declared failure excuses `ensures`
  mustAbort : Args → σ → Prop := fun _ _ => False
  frame     : Args → σ → Result → σ → Prop := fun _ initial _ final => final = initial

def Satisfies (function : Args → Spec σ ε Result) (contract : Contract σ ε Args Result) : Prop :=
  ∀ args initial, contract.assumes args initial → contract.requires args initial →
    (∀ result final, (function args).ok initial result final →
      (¬contract.mayAbort args initial → contract.ensures args initial result final) ∧
      contract.frame args initial result final ∧
      ¬contract.mustAbort args initial) ∧
    (∀ error, (function args).aborts initial error → contract.aborts args initial error) ∧
    ¬(function args).undefined initial

def wp (action : Spec σ ε Result) (ensures : Result → σ → Prop) (aborts : ε → Prop)
    (initial : σ) : Prop :=
  (∀ result final, action.ok initial result final → ensures result final) ∧
  (∀ error, action.aborts initial error → aborts error) ∧
  ¬action.undefined initial

def Obligation (file : String) (startByte endByte : Nat) (p : Prop) : Prop := p
  -- `p`, tagged with the byte range of the source clause it came from
```

Two rules of `wp` are what the walk of level 5 applies; the second is
[`wp_certifyingAt`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L143):

```
wp (Spec.bind (Spec.assume P) k) ensures aborts s  ↔  P → wp (k ()) ensures aborts s
wp (certifyingAt site kind τ v) ensures aborts s   ↔
  NTy.deepHolds τ v ∧ ∀ holds, ensures (NTy.ofDeepHolds τ v holds) s
```

An assumption becomes a hypothesis. A check becomes an obligation, here the
value's data invariants `NTy.deepHolds τ v`, and a continuation that
receives the value as a certified one.

## The levels

### 1. VIR


A [`ValidatedUnit`](../../verex-ir/VerexIR/Validation/Validated.lean#L461), registered per namespace
([`vir-design.md`](vir-design.md)); its nodes are [`Expr`](../../verex-ir/VerexIR/Syntax.lean#L1082) with [`ExprKind`](../../verex-ir/VerexIR/Syntax.lean#L1045),
[`Place`](../../verex-ir/VerexIR/Syntax.lean#L921) and [`Ty`](../../verex-ir/VerexIR/Syntax.lean#L328). Every front end ends here. The unit keeps
one arena per kind of node, and a node names another by its index in the
arena of its kind: `eN` an expression, `pN` a place, `tN` a type, `lN` a
local. The body and the specification clauses are both expressions. The
borrow checker's facts, the loans and where they die, are stored beside them.

```
t0: integer unbounded          -- a specification's integer
t1: bool
t2: integer 32 unsigned        -- u32
t3: unit
t4: nominal 0                  -- Counter
t5: reference mutable t4       -- &mut Counter

p0: localVar l0                -- c
p1: deref p0                   -- *c
p2: field p1 "n"               -- (*c).n, where it is written
p3: localVar l0
p4: deref p3
p5: field p4 "n"               -- (*c).n, where it is read

-- the body
e3: copy p5            : t2    -- (*c).n
e4: integer 1          : t2    -- 1u32
e5: add e3 e4          : t2
e6: assign p2 e5       : t3    -- (*c).n := …
e7: block [e6]         : t3

-- ensures c.n == old(c.n) + 1
e12: localVar l0       : t4    -- c
e13: select "n" e12    : t0    -- c.n
e14: localVar l0       : t4
e15: select "n" e14    : t0
e16: old e15           : t0    -- old(c.n)
e17: integer 1         : t0
e18: add e16 e17       : t0
e19: equal e13 e18     : t1
```

The remaining expressions are the invariant (`e0` to `e2`), the precondition
(`e8` to `e11`) and the abort condition (`e20`). A specification reads `c` at
the value's type `t4`, with the reference erased, and its integers are
unbounded.

### 2. Typed term

[`compileFunction`](../../verex-ir/VerexIR/Proofs/Denote/Compile.lean#L3076), quoted into Lean
by [`quoteTerm`](../../verex-ir/VerexSL/Verify/Contracts.lean#L296) ([`denotation.md`](denotation.md)). The body becomes one value
of an intrinsically typed syntax, in which an ill-typed term cannot be built.
Places become a variable and a path. The data-invariant protocol enters the
program here: the entry assumption of a reference's current, and the site
number each check reports at ([`data-invariants.md`](data-invariants.md)).

```
def lowering_demo.bump.body :
    Term lowering_demo.unit ResultShape.none (NRow.cons Counter.ref NRow.nil) NTy.unit :=
  (Term.assumeCurrent Var.here).drop
    ((Term.writePlace 27 none Var.here (Proj.field Var.here Proj.nil).deref
        (Term.modular ModularOp.add
          (Term.copyPlace none Var.here (Proj.field Var.here Proj.nil).deref ⋯)
          (Term.lit { val := 1, fits := ⋯ } _))).drop
      (Term.lit () _))
```

[`compiled.body.describe`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L5740) gives the short form:
`(assumeCurrent local0; ((local0.path := (local0.path add 1 wrapping)); ()))`.

### 2′. Contract

The clause translator, [`VerexSL/Contract.lean`](../../verex-ir/VerexSL/Contract.lean) and [`VerexSL/Contract/`](../../verex-ir/VerexSL/Contract). The
specification does not pass through the typed term. A metaprogram writes each
clause directly as a Lean proposition over the argument row, wrapped in a
marker that carries the clause's source range for messages.

```
def lowering_demo.bump.typedContract :
    Contract (Memory unit) Failure (HList (NRow.cons Counter.ref NRow.nil))
      ResultShape.none.carrier :=
  { requires := fun arguments state =>
      Obligation "Demo.lean" 303 321 (arguments.fst.fst.fst.val < 100),
    ensures := fun arguments state result final =>
      Obligation "Demo.lean" 326 353
        ((Var.here.selectCertified arguments.fst.snd).val = arguments.fst.fst.fst.val + 1),
    aborts := fun arguments state failure => Obligation "Demo.lean" 283 353 False,
    mayAbort := fun arguments state => False }
```

`arguments.fst` is the reference. Its `.fst` is the current value on entry,
which `old(c.n)` and the precondition read. Its `.snd` is the prophecy, the
value at exit, which `c.n` in the postcondition reads
([`prophetic-references.md`](prophetic-references.md)).

### 3. Denotation and the theorem

[`Term.denoteIn`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L4231); [`Satisfies`](../../verex-ir/VerexIR/Proofs/Contract.lean#L57). The term is given a meaning as a
specification monad over memory, and the statement proved is that this
meaning satisfies the contract. The link from the big-step semantics of VIR
to this denotation is the assumed agreement of
[`denotation.md`](denotation.md).

```
theorem lowering_demo.bump.typedVerified :
  prepareExecution registry lowering_demo.unit = Except.ok executable →
  ∀ [CarriersNonempty (Carriers.runtime lowering_demo.unit)],
    Satisfies
      (fun args =>
        Spec.bind
          (Term.denote (closedMeanings executable #[]) lowering_demo.bump.body
            (initialEnv lowering_demo.bump.params lowering_demo.bump.locals … args))
          (ResultShape.finish lowering_demo.bump.exitSite lowering_demo.bump.mutables
            lowering_demo.bump.shape))
      lowering_demo.bump.typedContract
```

### 4. Verification condition

After the arguments are introduced and named from the source, the goal is a
weakest precondition over the still-folded term. The argument row has become
two binders, and the contract's clauses have become a hypothesis and the two
continuations of `wp`.

```
c_final   : Counter.carrier
c_n       : NTy.plainOf Carriers.toPlain (NTy.int 32 false)
permitted : c_n.val < 100
⊢ wp
    ((Flow.bind
        ((Spec.assume (Counter.Holds (c_n, ()))).bind fun _ =>
          Spec.pure (Flow.value () (initialEnv … (((c_n, ()), c_final), ()))))
        fun _ env =>
          Term.denoteIn (closedMeanings executable #[]).interpreterOps
            ((Term.writePlace 27 none Var.here (Proj.field Var.here Proj.nil).deref
                (Term.modular ModularOp.add (Term.copyPlace …) (Term.lit …))).drop
              (Term.lit () _))
            env).bind
      (ResultShape.finish exitSite mutables shape))
    (fun result final =>
      Obligation "Demo.lean" 326 353 (c_final.val.fst.val = c_n.val + 1) ∧
        final = initialState)
    (fun failure => Obligation "Demo.lean" 283 353 False)
    initialState
```

### 5. The walk

[`VerexIR/Proofs/Denote/Solve.lean`](../../verex-ir/VerexIR/Proofs/Denote/Solve.lean) and [`Solve/`](../../verex-ir/VerexIR/Proofs/Denote/Solve)
([`native-verification.md`](native-verification.md)). The solver unfolds the
denotation one step at a time and normalizes as it goes. A check
([`certifyingAt`](../../verex-ir/VerexIR/Proofs/Denote/Term.lean#L139), taken by [`dataInvariantStep`](../../verex-ir/VerexIR/Proofs/Denote/Solve.lean#L1988)) is a cut: it splits off an obligation and continues with the
fact. Each finished path is a leaf. For `bump`: four binds, two splits, one
invariant cut, two leaves.

```
→ wp (Spec.bind (Flow.bind (Spec.bind (Spec.assume))))   entry: assume the current's invariant
→ wp (Spec.bind (certifyingAt))                          exit: the reference's current is checked
→ Obligation                                             leaf 1: the invariant after the write
→ wp (Spec.bind (Spec.assume))                           the prophecy equation, current = final
→ Obligation                                             leaf 2: the ensures clause
```

### 6. Leaves in source form

[`verex_prepare`](../../verex-ir/VerexIR/Proofs/Denote/Solve.lean#L5322), which is what an authored proof sees
([`proof-language.md`](proof-language.md)). Each leaf is restated over the
source's names. Machine integers are integers with their bounds as
hypotheses, invariants are opened to their clauses, and the obligation
markers are removed. What remains is ordinary Lean, here closed by `omega`.

```
case data_invariant_1
permitted      : c_n.val < 100
verexInvariant : c_n.val ≤ 100
c_n_lower      : 0 ≤ c_n.val
c_n_upper      : c_n.val ≤ 4294967295
⊢ (c_n.val + 1) % 4294967296 ≤ 100

case ensures_1
permitted      : c_n.val < 100
c_n_lower      : 0 ≤ c_n.val
c_n_upper      : c_n.val ≤ 4294967295
⊢ (c_n.val + 1) % 4294967296 = c_n.val + 1
```

## Printing a level

For a function `f` of a namespace `m`:

| Level | How to print it |
|---|---|
| VIR | in a `run_cmd`: [``VerexSL.registeredUnit? env `m``](../../verex-ir/VerexSL/Registry.lean#L137), then `repr` of `unit.namespaces[0].expressions` |
| Typed term | `#print m.f.body` after `verify`; `(compileFunction unit handle).body.describe` for the short form |
| Contract | `#print m.f.typedContract` |
| Theorem | `#check @m.f.typedVerified` (named by [`typedVerifiedName`](../../verex-ir/VerexSL/Contract.lean#L4484)) |
| Verification condition and walk | [`set_option verex.solverDebug true in verify m::f`](../../verex-ir/VerexIR/Proofs/Denote/Solve/Base.lean#L1685) |
| Leaves | `verify m::f by lean => verex_obligations; all_goals (verex_prepare <;> trace_state)` ([`verex_obligations`](../../verex-ir/VerexIR/Proofs/Denote/Solve.lean#L4503), [`verex_prepare`](../../verex-ir/VerexIR/Proofs/Denote/Solve.lean#L5322)) |
| Time per stage | [`set_option verex.stageLog "path"`](../../verex-ir/VerexSL/Options.lean#L69) |
