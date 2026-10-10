-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast
import Transpiler.Names
import Transpiler.Effects
import Transpiler.Report
import Transpiler.Order
import Transpiler.Comments
import Transpiler.Intrinsics

/-!
# Leaner Move printer

Prints a decoded XAST module as textual Leaner Move (`leaner-move.md`).  The
printer owns all transpilation policy that is not naming or ordering:

- **Effects.**  Functions print with an `Action` result when
  `Transpiler.Effects` says so; bodies with statements print as `do` blocks
  (Leaner wraps a pure `do` in `Id.run`), single expressions as terms.
- **Hoisting.**  Leaner sequences `Action` operations explicitly, so an
  effectful sub-expression in value position (`*r`, `&mut R[a].f`, a call to
  an `Action` function) is hoisted into a `let tmp ← …` binding in evaluation
  order and replaced by the temporary.
- **Borrows.**  A Move `&mut v` temporary passed to `vector::push_back` prints
  as the pure update `v := v.push x`; `&v` passed to `length`/`is_empty` and
  `*vector::borrow(&v, i)` as `v.length`/`v.isEmpty`/`v.get i`; every other
  borrow becomes a named loan (`let tmp ← &mut place`), and a loan of a local
  that is passed to a callee is read back (`v ← *tmp`) so later uses of the
  local see the mutation.
- **Specs.**  Clauses print in Leaner's order (`requires; modifies; ensures;
  aborts_if …`), repeated `requires`/`ensures` conjoined, `ensures True` when
  absent, `pragma aborts_if_is_partial` and the loose frame `modifies *` per
  the design (`transpile-design.md`), `aborts_if_is_strict` as `aborts_if
  False`.
- **Unsupported constructs** raise an error naming the construct; the
  declaration printer catches it, emits the declaration commented out with the
  reason, and records it in the report.
-/

namespace Transpiler.Print

open Transpiler.Xast Transpiler.Names Transpiler.Effects Transpiler.Comments Transpiler.Order
open Std (Format)

-- -------------------------------------------------------------------------------------------------
-- State

/-- The printing context of one module. -/
structure Ctx where
  pkg : Package
  mod : Module
  effects : Table
  /-- The function being printed, if any. -/
  fn : Option Function := none
  /-- Names of the current function's parameters by index. -/
  params : Array String := #[]
  /-- Locals that are `let mut` in the current function. -/
  mutables : List String := []
  /-- Whether the current function prints with an `Action` result. -/
  inAction : Bool := false
  /-- The loop label per nesting level (innermost last). -/
  loopLabels : List (Option String) := []
  /-- Whether we are printing a spec term (Prop context). -/
  inSpec : Bool := false
  /-- Inside a struct invariant: `this` is the value. -/
  inStructInvariant : Bool := false
  /-- Names bound by quantifiers or spec lets in scope. -/
  specLocals : List String := []
  /-- Inside an `aborts_if` clause: global places read the pre-state and
  print as `old(R[a])`. -/
  preState : Bool := false
  /-- Type parameter names of the declaration being printed. -/
  tyParams : Array String := #[]
  /-- Spec context widened to `num`.  The specification elaborator supplies
  the mathematical interpretation, so bounded leaves keep their clean source
  spelling here. -/
  wideInts : Bool := false
  /-- Whether a widened arithmetic tree has mathematical `Int` semantics.
  Retained for precedence and target decisions; it no longer changes leaf
  spelling. -/
  wideAsInt : Bool := false
  /-- The expected type is known (a `let` with annotation): omit ascriptions. -/
  suppressAscription : Bool := false
  /-- Emit `verify f` after every `spec f` (proof campaigns; the default output
  leaves verification to the reader). -/
  emitVerify : Bool := false
  /-- Printing a specification inside a function body (a loop invariant):
  locals print under their body names. -/
  specInBody : Bool := false
  /-- The Lean module prefix of the generated files (`Transpiler.Tests.Programs.MoveStdlib`),
  used by the `import`s of transpiled dependencies. -/
  leanRoot : String := ""
  /-- Parameters rebound as mutable locals (reassigned or mutably borrowed
  in the body) print under a fresh name (`x'`): the parameter's own name
  stays in the signature and the spec binders, and the local never shadows
  a name the verifier tracks borrows of. -/
  paramRenames : List (String × String) := []
  /-- The names chosen for the bindings of the `let` being printed (its
  pattern prints them; its right-hand side still sees the old bindings). -/
  bindingOverrides : List (String × String) := []

structure St where
  fresh : Nat := 0
  comments : Pool
  report : Report := {}
  /-- Statement lines emitted for the block being printed, innermost last. -/
  lines : Array Format := #[]
  /-- Whether the last emitted line was a `let` (a block may not end on it). -/
  lastWasLet : Bool := false
  /-- The live loans of the block being printed: the textual place, the
  reference variable holding the loan, and whether the loan is mutable.  A
  loan is the access path of its place for the rest of the block: later
  borrows reuse it (Leaner's verifier requires borrows of a loaned place to
  chain through the live loan), and an owned local is read back through it
  after every mutation. -/
  loans : List (String × String × Bool) := []
  /-- Mutable locals bound in the block (Lean's `do` forbids shadowing a
  `let mut`), and the fresh names given to bindings that would shadow one. -/
  boundMuts : List String := []
  renames : List (String × String) := []
  /-- Locals declared without an initializer by a CFG frontend. Their first
  assignment is emitted as the binding that Lean syntax requires. -/
  uninitialized : List String := []
  /-- Declarations of the module that were not transpiled (their dependents
  are not either). -/
  failed : List String := []

abbrev PM := ReaderT Ctx (StateT St (Except String))

def unsupported {α : Type} (what : String) : PM α := throw s!"unsupported: {what}"

def record (e : Extension) : PM Unit :=
  modify fun s =>
    if s.report.extensions.contains e then s
    else { s with report := { s.report with extensions := s.report.extensions ++ [e] } }

def drop (what : String) : PM Unit :=
  modify fun s => { s with report := { s.report with dropped := s.report.dropped ++ [what] } }

def freshName (base : String) : PM String := do
  let n ← modifyGet fun s => (s.fresh, { s with fresh := s.fresh + 1 })
  pure s!"{base}{n}"

/-- Emits a statement line into the current block. -/
def emit (line : Format) : PM Unit :=
  let isLet := (Format.pretty line).startsWith "let "
  modify fun s => { s with lines := s.lines.push line, lastWasLet := isLet }

/-- Runs `k` collecting the statement lines it emits, restoring the outer
block. -/
def collect (k : PM Unit) : PM (List Format) := do
  let saved ← get
  modify fun s => { s with lines := #[], lastWasLet := false }
  k
  let lines ← modifyGet fun s =>
    (s.lines, { s with
      lines := saved.lines
      lastWasLet := saved.lastWasLet
      loans := saved.loans
      boundMuts := saved.boundMuts
      renames := saved.renames
      uninitialized := saved.uninitialized })
  pure lines.toList

/-- The printed name of a local: its fresh name if it shadows a mutable
binding of the block, else the legalized Move name. -/
def localPrinted (n : String) : PM String := do
  match (← read).bindingOverrides.lookup n with
  | some printed => pure (legalize printed)
  | none => pure (legalize (((← get).renames.lookup n).getD n))

/-- The printed name a new binding of local `n` will have: a fresh name if
it would shadow a mutable local of the block (Lean's `do` forbids that). -/
def planBinding (n : String) : PM String := do
  let st ← get
  if st.boundMuts.contains n then
    let taken (m : String) := st.boundMuts.contains m || st.renames.any (·.2 == m)
    let rec fresh (candidate : String) (fuel : Nat) : String :=
      match fuel with
      | 0 => candidate
      | fuel + 1 => if taken candidate then fresh (candidate ++ "'") fuel else candidate
    pure (fresh (n ++ "'") 16)
  else pure n

/-- Registers the binding of local `n` under `printedName` (mutable if
`isMut`), after the statement that binds it has printed (its right-hand
side still sees the previous binding). -/
def commitBinding (n printedName : String) (isMut : Bool) : PM Unit :=
  modify fun s => { s with
    renames := (n, printedName) :: s.renames.filter (·.1 != n),
    boundMuts := if isMut then printedName :: s.boundMuts else s.boundMuts.filter (· != n) }

/-- The live loan of a place that can serve a borrow of `kind`: a mutable loan
serves both kinds (Leaner freezes `&mut` to `&` where `&` is expected), an
immutable loan only an immutable borrow. -/
def loanFor (kind : RefKind) (placeKey : String) : PM (Option String) := do
  let found := (← get).loans.find? fun (p, _, isMut) =>
    p == placeKey && (isMut || kind == .immutable)
  pure (found.map (·.2.1))

def registerLoan (placeKey refName : String) (isMut : Bool) : PM Unit :=
  modify fun s =>
    { s with loans := (placeKey, refName, isMut) :: s.loans.filter (·.1 != placeKey) }

/-- Forgets the loans rooted at a rebound local, and the loans held by a
rebound reference variable. -/
def forgetLoans (name : String) : PM Unit :=
  modify fun s => { s with loans := s.loans.filter fun (p, r, _) =>
    !(p == name || p.startsWith (name ++ ".") || p.startsWith (name ++ "[") || r == name) }

-- -------------------------------------------------------------------------------------------------
-- Format helpers

def text (s : String) : Format := Format.text s
def hard : Format := Format.line
def vcat (xs : List Format) : Format := Format.joinSep xs hard
def indent (n : Nat) (f : Format) : Format := Format.nest n f
def paren (f : Format) : Format := text "(" ++ f ++ text ")"
def sepBy (sep : String) (xs : List Format) : Format := Format.joinSep xs (text sep)
/-- `head arg₁ arg₂ …` as a wrapping group. -/
def app (head : Format) (args : List Format) : Format :=
  if args.isEmpty then head
  else Format.group (indent 2 (head ++ (args.foldl (fun acc a => acc ++ Format.line ++ a) (text ""))))
def block (header : Format) (lines : List Format) : Format :=
  header ++ indent 2 (hard ++ vcat lines)
/-- A `do` block: `do` followed by the lines, indented. -/
def doBlock (lines : List Format) : Format :=
  let statements := lines.filter fun l => !((Format.pretty l).trimLeft.startsWith "--")
  if lines.isEmpty then text "pure ()"
  else if statements.isEmpty then block (text "do") (lines ++ [text "pure ()"])
  else block (text "do") lines

-- -------------------------------------------------------------------------------------------------
-- Types and values

def intTyName : Ty → Option String
  | .u8 => some "U8" | .u16 => some "U16" | .u32 => some "U32" | .u64 => some "U64"
  | .u128 => some "U128" | .u256 => some "U256"
  | .i8 => some "I8" | .i16 => some "I16" | .i32 => some "I32" | .i64 => some "I64"
  | .i128 => some "I128" | .i256 => some "I256"
  | _ => none

def isIntTy (t : Ty) : Bool := (intTyName t).isSome
def isSignedTy : Ty → Bool
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => true
  | _ => false
def isUnitTy : Ty → Bool
  | .tuple [] => true
  | _ => false
def isRefTy : Ty → Bool
  | .reference _ _ => true
  | _ => false

/-- Whether a type contains a Move function value. The executable Leaner
boundary accepts these only on compile-time-only inline helpers. -/
partial def containsFunctionTy : Ty → Bool
  | .function _ _ _ => true
  | .tuple ts => ts.any containsFunctionTy
  | .vector t | .reference _ t | .typeDomain t => containsFunctionTy t
  | .struct _ args => args.any containsFunctionTy
  | .resourceDomain _ args => (args.getD []).any containsFunctionTy
  | _ => false
def derefTy : Ty → Ty
  | .reference _ t => t
  | t => t

/-- The type parameter names in scope: those of the current declaration. -/
def typeParamName (i : Nat) : PM String := do
  pure ((← read).tyParams[i]?.getD s!"T{i}")

/-- The `Action` monad, qualified where the module declares its own `Action`. -/
def actionName : PM String := do
  let m := (← read).mod
  pure (if m.structs.any (·.name == "Action") then "Move.Action" else "Action")

partial def printTy (t : Ty) : PM Format := do
  let ctxt ← read
  match t with
  | .bool => pure (text "Bool")
  | .address => pure (text "Address")
  | .signer => pure (text "Signer")
  | .num => pure (text "Int")
  | .range => unsupported "spec `range` type"
  | .eventStore => unsupported "spec `EventStore` type"
  | .tuple [] => pure (text "Unit")
  | .tuple ts => do
    let fs ← ts.mapM printTy
    pure (paren (sepBy " × " fs))
  | .vector e => do pure (text "Vector " ++ (← printTyAtom e))
  | .struct name args => do
    let head := text (qualify ctxt.mod.ref name)
    let fs ← args.mapM printTyAtom
    pure (if fs.isEmpty then head else head ++ text " " ++ sepBy " " fs)
  | .function args result _ => do
    let params := match args with
      | .tuple [] => [Ty.tuple []]
      | .tuple ts => ts
      | t => [t]
    let ps ← params.mapM printTyAtom
    let result ← printTy result
    let action ← actionName
    let codomain := text action ++ text " " ++ paren result
    pure (sepBy " → " (ps ++ [codomain]))
  | .typeParam i => do pure (text (← typeParamName i))
  | .reference mutable inner => do
    let f ← printTy inner
    pure (text (if mutable then "&mut " else "&") ++ f)
  | .typeDomain _ => unsupported "spec type domain"
  | .resourceDomain _ _ => unsupported "spec resource domain"
  | .stateDomain => unsupported "spec state domain"
  | other =>
    match intTyName other with
    | some n => pure (text n)
    | none => unsupported "type"
where
  printTyAtom (t : Ty) : PM Format := do
    match t with
    | .vector _ | .struct _ (_ :: _) | .function _ _ _ | .reference _ _ =>
      do pure (paren (← printTy t))
    | _ => printTy t

/-- A type in a purely logical spec position: references are erased and all
Move integer types use MSL's mathematical `Int` domain. -/
def printSpecTy (t : Ty) : PM Format := do
  let t := derefTy t
  if isIntTy t || t == .num then pure (text "Int") else printTy t

def hexOfByte (b : Nat) : String :=
  let digits := "0123456789ABCDEF".toList
  String.mk [digits[b / 16]!, digits[b % 16]!]

/-- A constant value of type `ty`. -/
partial def printValue (v : Value) (ty : Ty) : PM Format := do
  match v with
  | .bool b => pure (text (if b then "true" else "false"))
  | .address hex => pure (text ("@" ++ hex))
  | .number n =>
    if n < 0 then pure (paren (text (toString n))) else pure (text (toString n))
  | .vector elems =>
    match ty with
    | .vector .u8 =>
      let bytes := elems.filterMap fun | .number n => some n.toNat | _ => none
      if bytes.length == elems.length then
        if bytes.all fun b => b ≥ 0x20 && b < 0x7f && b != 0x22 && b != 0x5c then
          pure (text ("b\"" ++ String.mk (bytes.map Char.ofNat) ++ "\""))
        else pure (text ("x\"" ++ String.join (bytes.map hexOfByte) ++ "\""))
      else unsupported "non-numeric byte vector constant"
    | .vector elemTy =>
      let fs ← elems.mapM (printValue · elemTy)
      if fs.isEmpty then do pure (paren (text "vector![] : " ++ (← printTy ty)))
      else pure (text "vector![" ++ sepBy ", " fs ++ text "]")
    | _ => unsupported "vector constant of non-vector type"
  | .tuple elems =>
    match ty with
    | .tuple tys =>
      let fs ← (elems.zip tys).mapM fun (e, t) => printValue e t
      pure (paren (sepBy ", " fs))
    | _ => unsupported "tuple constant of non-tuple type"

-- -------------------------------------------------------------------------------------------------
-- Helpers over the AST

def unitExp : ExpNode := .call .tuple [] [] none

def isUnit (e : Exp) : Bool :=
  match e.node with
  | .call .tuple _ [] _ => true
  | .sequence [] => true
  | _ => false

def localName? (e : Exp) : PM (Option String) := do
  match e.node with
  | .«local» n => pure (some n)
  | .param i => pure ((← read).params[i]?)
  | _ => pure none

/-- Whether a name occurs free (as a local or parameter) in an expression. -/
partial def mentions (name : String) (paramIdx : Option Nat) (e : Exp) : Bool :=
  let go := mentions name paramIdx
  match e.node with
  | .«local» n => n == name
  | .param i => paramIdx == some i
  | .value _ _ | .loopCont _ _ => false
  | .call _ _ args _ => args.any go
  | .invoke function args => go function || args.any go
  | .block _ binding body => (binding.map go |>.getD false) || go body
  | .ite c t f => go c || go t || go f
  | .«match» s arms => go s || arms.any fun a => (a.guard.map go |>.getD false) || go a.body
  | .sequence es => es.any go
  | .loop b => go b
  | .«return» v => go v
  | .assign _ v => go v
  | .mutate t v => go t || go v
  | .specBlock _ => false
  | .quant _ rs _ c b => rs.any (fun r => go r.domain) || (c.map go |>.getD false) || go b

/-- Names bound by a pattern. -/
partial def patternNames (p : Pattern) : List String :=
  match p.node with
  | .var n => [n]
  | .tuple ps => ps.flatMap patternNames
  | .struct _ _ _ ps => ps.flatMap patternNames
  | _ => []

/-- Locals bound by `let` anywhere in an expression. -/
partial def boundLocals (e : Exp) : List String :=
  let go := boundLocals
  match e.node with
  | .block p binding body => patternNames p ++ (binding.map go |>.getD []) ++ go body
  | .call _ _ args _ => args.flatMap go
  | .invoke function args => go function ++ args.flatMap go
  | .ite c t f => go c ++ go t ++ go f
  | .«match» s arms => go s ++ arms.flatMap fun a => patternNames a.pattern ++ go a.body
  | .sequence es => es.flatMap go
  | .loop b => go b
  | .«return» v => go v
  | .assign _ v => go v
  | .mutate t v => go t ++ go v
  | _ => []

/-- Locals that must be `let mut`: assigned, or mutably borrowed. -/
partial def mutableLocals (e : Exp) : List String :=
  let go := mutableLocals
  match e.node with
  | .assign p v => patternNames p ++ go v
  | .call (.borrow .mutable) _ [arg] _ =>
    (match arg.node with | .«local» n => [n] | _ => []) ++ go arg
  | .call _ _ args _ => args.flatMap go
  | .invoke function args => go function ++ args.flatMap go
  | .block _ binding body => (binding.map go |>.getD []) ++ go body
  | .ite c t f => go c ++ go t ++ go f
  | .«match» s arms => go s ++ arms.flatMap fun a => (a.guard.map go |>.getD []) ++ go a.body
  | .sequence es => es.flatMap go
  | .loop b => go b
  | .«return» v => go v
  | .mutate t v => go t ++ go v
  | _ => []

/-- Parameters (by index) that are assigned or mutably borrowed. -/
partial def mutableParams (e : Exp) : List Nat :=
  let go := mutableParams
  match e.node with
  | .assign p v =>
    -- `Assign` to a parameter comes as a `Var` pattern with the parameter's
    -- name; resolved by the caller against the parameter names.
    go v
  | .call (.borrow .mutable) _ [arg] _ =>
    (match arg.node with | .param i => [i] | _ => []) ++ go arg
  | .call _ _ args _ => args.flatMap go
  | .invoke function args => go function ++ args.flatMap go
  | .block _ binding body => (binding.map go |>.getD []) ++ go body
  | .ite c t f => go c ++ go t ++ go f
  | .«match» s arms => go s ++ arms.flatMap fun a => (a.guard.map go |>.getD []) ++ go a.body
  | .sequence es => es.flatMap go
  | .loop b => go b
  | .«return» v => go v
  | .mutate t v => go t ++ go v
  | _ => []

/-- Whether a loop needs a label: its body contains, `k > 0` nested loops
deep, a `break`/`continue` whose `nest` is exactly `k` (so it targets this
loop from inside another). -/
partial def hasOuterCont (depth : Nat) (e : Exp) : Bool :=
  let go := hasOuterCont depth
  match e.node with
  | .loopCont nest _ => depth > 0 && nest == depth
  | .loop b => hasOuterCont (depth + 1) b
  | .call _ _ args _ => args.any go
  | .invoke function args => go function || args.any go
  | .block _ binding body => (binding.map go |>.getD false) || go body
  | .ite c t f => go c || go t || go f
  | .«match» s arms => go s || arms.any fun a => (a.guard.map go |>.getD false) || go a.body
  | .sequence es => es.any go
  | .«return» v => go v
  | .assign _ v => go v
  | .mutate t v => go t || go v
  | _ => false

-- -------------------------------------------------------------------------------------------------
-- Expressions

/-- How an expression renders: a pure term, or an `Action` whose value must be
bound with `←`. -/
inductive Kind where
  | pure | action
  deriving BEq, Repr

def calleeFacts (name : QualifiedName) : PM Facts := do
  pure ((← read).effects.get name)

/-- The render kind of a call by its operation and arguments. -/
partial def kindOf (e : Exp) : PM Kind := do
  match e.node with
  | .invoke _ _ => pure .action
  | .call op _ args _ =>
    match op with
    | .borrowGlobal _ | .«exists» _ | .moveTo | .moveFrom | .deref | .abort _ => pure .action
    | .freeze explicit => if explicit then pure .action else
        match args with
        | [a] => kindOf a
        | _ => pure .action
    | .borrow _ => pure .action
    -- Creating a certified value owes its invariant: `T.certify`, an `Action`.
    | .pack name none => pure (if (← read).pkg.certified name then .action else .pure)
    | .moveFunction name =>
      match vectorCall? op with
      | some vop =>
        match vop with
        -- These render as pure terms; a borrowed operand is hoisted into a
        -- named loan by the printer itself.
        | .pushBack | .length | .isEmpty => pure .pure
        | .empty | .singleton | .destroyEmpty | .contains | .indexOf => pure .pure
        | .borrow | .borrowMut => pure .action
        -- Mutations through a reference bind themselves (with the read-back
        -- of the borrowed local); they are term-producing for the caller.
        | .mutRef _ => pure .pure
      | none =>
        if isPrimitiveModule name.module then pure .pure
        else if (← calleeFacts name).isAction then
          -- A call that borrows a mutable local (read back afterwards) binds
          -- itself; so does a unit-typed call, emitted as a statement.
          let readsBack ← args.anyM fun a => do
            match a.node with
            | .call (.borrow .mutable) _ [inner] _ =>
              match ← localName? inner with
              | some n => pure ((← read).mutables.contains n)
              | none => pure false
            | _ => pure false
          pure (if readsBack || isUnitTy e.ty then .pure else .action)
        else pure .pure
    | _ => pure .pure
  | .mutate _ _ => pure .action
  | _ => pure .pure

/-- The borrow at the root of a field-selection chain (`R[a].f.g`,
`(&mut x).f`), if the chain starts with one. -/
partial def borrowRoot (e : Exp) : Option RefKind :=
  match e.node with
  | .call (.borrow kind) _ _ _ | .call (.borrowGlobal kind) _ _ _ => some kind
  | .call (.select _ _) _ [inner] _ | .call (.selectVariants _ _) _ [inner] _ => borrowRoot inner
  | _ => none

/-- The one field name of a variant field selection: Move's `x.f` on an enum
names the field `f` of every variant that has it. -/
def variantFieldName? (fields : List String) : Option String :=
  match fields with
  | [] => none
  | f :: rest => if rest.all (· == f) then some f else none

/-- Whether an expression is a place: a local/parameter, or a field/element
chain over one. -/
partial def isPlaceExp (e : Exp) : Bool :=
  match e.node with
  | .«local» _ | .param _ => true
  | .call (.select _ _) _ [inner] _ => isPlaceExp inner
  | .call (.selectVariants _ _) _ [inner] _ => isPlaceExp inner
  | .call (.borrowGlobal _) _ _ _ => true
  | .call (.borrow _) _ [inner] _ => isPlaceExp inner
  | .call (.deref) _ [inner] _ => isPlaceExp inner
  | .call (.freeze _) _ [inner] _ => isPlaceExp inner
  | .call op@(.moveFunction _) _ [v, _] _ =>
    match vectorCall? op with
    | some .borrow | some .borrowMut => isPlaceExp v
    | _ => false
  | _ => false

/-- Precedence for parenthesization: binary operators are always
parenthesized when nested under another binary operator, which keeps the
printer simple and the output unambiguous. -/
def atom (f : Format) (isCompound : Bool) : Format := if isCompound then paren f else f

/-- One step of a place after its root: a field or a vector element.  An
element step is `stable` when its index mentions only variables that never
change, so the textual place denotes one location for the rest of the block
(and a loan of it may be reused). -/
inductive PlaceStep where
  | field (name : String)
  | index (it : Format) (stable : Bool)

def renderSteps (steps : List PlaceStep) : Format :=
  steps.foldl (fun acc s => match s with
    | .field n => acc ++ text "." ++ text n
    | .index it _ => acc ++ text "[" ++ it ++ text "]") (text "")

def stepsStable (steps : List PlaceStep) : Bool :=
  steps.all fun | .field _ => true | .index _ stable => stable

/-- A place decomposed into its root — a local/parameter (`root`) or a global
`R[a]` — and the steps after it. -/
structure PlacePath where
  root : Exp
  rootText : Format
  rootStable : Bool
  steps : List PlaceStep

/-- The textual place, the key of its loans. -/
def PlacePath.key (p : PlacePath) : String := Format.pretty (p.rootText ++ renderSteps p.steps)
def PlacePath.stable (p : PlacePath) : Bool := p.rootStable && stepsStable p.steps

/-- The owned (non-reference) local or parameter at the root of a place. -/
partial def placeRootLocal (e : Exp) : PM (Option String) := do
  match e.node with
  | .«local» n => pure (if isRefTy e.ty then none else some n)
  | .param i =>
    match (← read).params[i]? with
    | some n => pure (if isRefTy e.ty then none else some n)
    | none => pure none
  | .call (.select _ _) _ [inner] _ | .call (.deref) _ [inner] _ | .call (.borrow _) _ [inner] _
  | .call (.freeze _) _ [inner] _ => placeRootLocal inner
  | .call op@(.moveFunction _) _ [v, _] _ =>
    match vectorCall? op with
    | some .borrow | some .borrowMut => placeRootLocal v
    | _ => pure none
  | _ => pure none


-- -------------------------------------------------------------------------------------------------
-- Specifications

/-- Precedence of the spec operators (Lean's): arithmetic above comparison
above the connectives. -/
def opPrec (sym : String) : Nat :=
  match sym with
  | "*" | "/" | "%" => 70
  | "+" | "-" => 65
  | "<<<" | ">>>" => 75
  | "&&&" => 60
  | "^^^" => 58
  | "|||" => 55
  | "=" | "≠" | "<" | ">" | "≤" | "≥" => 50
  | "∧" => 35
  | "∨" => 30
  | "→" => 25
  | "↔" => 20
  | _ => 50

/-- The precedence a spec expression prints at: `100` for atoms and
applications, the operator's precedence for binary operators, `0` for
binders and conditionals. -/
def specPrec (e : Exp) : Nat :=
  match e.node with
  | .invoke _ (_ :: _) => 90
  | .call op _ args _ =>
    match op, args with
    | .add, [_, _] => 65 | .sub, [_, _] => 65
    | .mul, [_, _] => 70 | .div, [_, _] => 70 | .mod, [_, _] => 70
    | .shl, [_, _] => 75 | .shr, [_, _] => 75
    | .bitAnd, [_, _] => 60 | .xor, [_, _] => 58 | .bitOr, [_, _] => 55
    | .eq, [_, _] | .neq, [_, _] | .lt, [_, _] | .gt, [_, _] | .le, [_, _] | .ge, [_, _]
    | .identical, [_, _] | .containsVec, [_, _] | .inRangeVec, [_, _] | .inRangeRange, [_, _] => 50
    | .and, [_, _] => 35
    | .or, [_, _] => 30
    | .implies, [_, _] => 25
    | .iff, [_, _] => 20
    | .specFunction _ _, (_ :: _) | .moveFunction _, (_ :: _) | .pack _ _, (_ :: _) => 90
    | .updateVec, _ | .concatVec, _ => 65
    | _, _ => 100
  | .quant _ _ _ _ _ | .ite _ _ _ | .block _ _ _ | .«match» _ _ => 0
  | _ => 100

/-- Whether a spec expression is a global place chain (`R[a]`, `R[a].f`). -/
partial def isGlobalPlace (e : Exp) : Bool :=
  match e.node with
  | .call (.global none) _ _ _ => true
  | .call (.select _ _) _ [inner] _ => isGlobalPlace inner
  | _ => false

/-- A vector length in a specification: the `len` operation, or the
specification version of `std::vector::length` (rendered as the list view's
length, a `Nat`, so it is read as `num` like `len`). -/
def isLengthCall (e : Exp) : Bool :=
  match e.node with
  | .call (.len) _ _ _ => true
  | .call (.specFunction name _) _ _ _ =>
    isPrimitiveModule name.module && companionFunction? name.name == some "length"
  | _ => false

/-- The effective type of a spec expression for widening decisions: an
arithmetic node over operands of one bounded type keeps that type (the model
types it `num`); a vector length is `num`; otherwise the node's own type. -/
partial def effTy (e : Exp) : Ty :=
  if isLengthCall e then .num else
  match e.node with
  | .call op _ args _ =>
    match op with
    | .add | .sub | .mul | .div | .mod =>
      match args.map effTy with
      | [a, b] => if a == b && isIntTy a then a else e.ty
      | _ => e.ty
    | .old | .copy | .move | .trace _ | .noOp | .deref | .borrow _ =>
      match args with
      | [a] => effTy a
      | _ => e.ty
    | _ => e.ty
  | _ => e.ty

/-- Whether a spec expression is arithmetic (its leaves, not the node, carry a
widening suffix), looking through the value forms (`if`, `match`, `let`) so
that every branch of a widened value widens. -/
partial def isArith (x : Exp) : Bool :=
  match x.node with
  | .call (.add) _ [_, _] _ | .call (.sub) _ [_, _] _ | .call (.mul) _ [_, _] _
  | .call (.div) _ [_, _] _ | .call (.mod) _ [_, _] _ | .call (.cast) _ [_] _
  | .call (.shl) _ [_, _] _ | .call (.shr) _ [_, _] _ => true
  | .ite _ t f => isArith t || isArith f
  | .«match» _ arms => arms.any fun a => isArith a.body
  | .block _ _ body => isArith body
  | .call (.old) _ [a] _ | .call (.copy) _ [a] _ | .call (.move) _ [a] _ => isArith a
  | _ => false

/-- A plain numeric literal (neutral: it adapts to `Nat`, `Int`, or a bounded
type). -/
def isNumLit (x : Exp) : Bool :=
  match x.node with
  | .value (.number _) none => true
  | _ => false

/-- Whether a widened arithmetic tree must print over `Int`: it subtracts or
negates (`Nat` subtraction truncates), or has a signed or `num`-valued leaf
other than a literal or `len`. -/
partial def treeNeedsInt (x : Exp) : Bool :=
  match x.node with
  | .call (.sub) _ _ _ | .call (.negate) _ _ _ => true
  | .call (.add) _ args _ | .call (.mul) _ args _ | .call (.div) _ args _
  | .call (.mod) _ args _ => args.any treeNeedsInt
  | .call (.shl) _ (a :: _) _ | .call (.shr) _ (a :: _) _ => treeNeedsInt a
  | .call (.cast) _ [a] _ => isSignedTy x.ty || treeNeedsInt a
  | .call (.old) _ [a] _ | .call (.copy) _ [a] _ | .call (.move) _ [a] _ => treeNeedsInt a
  | .ite _ t f => treeNeedsInt t || treeNeedsInt f
  | .«match» _ arms => arms.any fun a => treeNeedsInt a.body
  | .block _ _ body => treeNeedsInt body
  | .value _ _ => isSignedTy x.ty
  | _ => if isLengthCall x then false else isSignedTy (effTy x) || effTy x == .num

mutual

/-- A pure term for an expression, hoisting effectful parts into the current
block.  Returns the term and whether it is compound (needs parentheses as an
operand). -/
partial def term (e : Exp) : PM (Format × Bool) := do
  let ctxt ← read
  match e.node with
  | .value v constant =>
    match constant with
    | some n => pure (text (legalize n), false)
    | none => do pure (← printValue v e.ty, false)
  | .«local» n => do pure (text (← localPrinted n), false)
  | .param i =>
    match ctxt.params[i]? with
    | some n => pure (text (legalize n), false)
    | none => unsupported s!"parameter index {i}"
  | .invoke function args =>
    let a ← invokeCall function args
    let tmp ← freshName "tmp"
    emit (text s!"let {tmp} ← " ++ a)
    pure (text tmp, false)
  | .call op inst args _ => callTerm e op inst args
  | .block _ _ _ | .sequence _ =>
    -- A block in value position: its statements run in the enclosing block,
    -- its value is the term.
    blockValue e
  | .ite c t f =>
    let ct ← term c
    let tk ← isStatementLike t
    let fk ← isStatementLike f
    if !tk && !fk then
      let (tt, _) ← term t
      let (ft, _) ← term f
      pure (text "if " ++ ct.1 ++ text " then " ++ tt ++ text " else " ++ ft, true)
    else
      -- Branches with statements: bind the value of a statement-level `if`
      -- (each branch ends in its value).
      let tmp ← freshName "value"
      let tLines ← collect (blockTail t none)
      let fLines ← collect (blockTail f none)
      let ifDoc :=
        text "if " ++ ct.1 ++ text " then" ++ indent 2 (hard ++ vcat tLines) ++ hard ++
          text "else" ++ indent 2 (hard ++ vcat fLines)
      emit (text s!"let {tmp} ← " ++ ifDoc)
      pure (text tmp, false)
  | .«match» s arms =>
    let (st, _) ← scrutinee s arms
    let anyStatement ← arms.anyM fun a => isStatementLike a.body
    if !anyStatement then
      let armDocs ← arms.mapM fun a => do
        let p ← pattern a.pattern
        let g ← match a.guard with
          | some g => do pure (text " if " ++ (← term g).1)
          | none => pure (text "")
        let (b, _) ← term a.body
        pure (text "| " ++ p ++ g ++ text " => " ++ b)
      pure (text "match " ++ st ++ text " with" ++ indent 2 (hard ++ vcat armDocs), true)
    else
      let tmp ← freshName "value"
      let armDocs ← arms.mapM fun a => do
        let p ← pattern a.pattern
        let g ← match a.guard with
          | some g => do pure (text " if " ++ (← term g).1)
          | none => pure (text "")
        let lines ← collect (blockTail a.body none)
        pure (text "| " ++ p ++ g ++ text " =>" ++ indent 2 (hard ++ vcat lines))
      let doc := text "match " ++ st ++ text " with" ++ indent 2 (hard ++ vcat armDocs)
      emit (text s!"let {tmp} ← " ++ doc)
      pure (text tmp, false)
  | .loop _ | .loopCont _ _ | .«return» _ | .assign _ _ | .mutate _ _ | .specBlock _ =>
    -- Statement forms in value position: run them, value is unit.
    stmt e
    pure (text "()", false)
  | .quant _ _ _ _ _ => unsupported "quantifier in code"

/-- Whether an expression needs statement-level printing (it is not a pure
term even after hoisting its effects into the enclosing block) — a
`sequence`/`block` with statements, a loop, a return, …  Used to decide the
shape of `if`/`match` branches. -/
partial def isStatementLike (e : Exp) : PM Bool := do
  match e.node with
  | .block _ _ _ | .sequence (_ :: _ :: _) | .loop _ | .«return» _ | .assign _ _
  | .mutate _ _ | .specBlock _ | .loopCont _ _ => pure true
  | .sequence [x] => isStatementLike x
  | .sequence [] => pure false
  | .call (.abort _) _ _ _ => pure true
  | .call _ _ _ _ => pure ((← kindOf e) == .action)
  | .invoke _ _ => pure true
  | .ite c t f => do pure ((← isStatementLike t) || (← isStatementLike f) || (← isStatementLike c))
  | .«match» _ arms => arms.anyM fun a => isStatementLike a.body
  | _ => pure false

/-- The reference variable holding the reference-typed expression `e`: the
local or parameter itself, the loan of a borrowed place, else a fresh
binding of the value. -/
partial def referenceVariable (kind : RefKind) (e : Exp) : PM String := do
  match e.node with
  | .«local» n => localPrinted n
  | .param i =>
    match (← read).params[i]? with
    | some n => pure (legalize n)
    | none => unsupported "parameter index"
  | _ =>
    if isPlaceExp e then borrowRef kind e
    else
      let (t, _) ← valueTerm e
      let r ← freshName "ref"
      emit (text s!"let {r} ← " ++ t)
      pure r

/-- The scrutinee of a `match`: a tuple prints as `a, b`.  Through a
reference, a `match` whose arms bind payload variables binds them by
reference — Leaner's `match` on a reference local (its arms borrow the
payload through it) — so the scrutinee is the reference variable; a
variable-free match through a reference matches the referent instead. -/
partial def scrutinee (s : Exp) (arms : List MatchArm) : PM (Format × Bool) := do
  match s.node with
  | .call .tuple _ args@(_ :: _ :: _) _ => do
    let fs ← args.mapM fun a => do pure (← term a).1
    pure (sepBy ", " fs, false)
  | _ =>
    match s.ty with
    | .reference mutable _ =>
      if arms.any (fun a => !(patternNames a.pattern).isEmpty) then
        let r ← referenceVariable (if mutable then .mutable else .immutable) s
        pure (text r, false)
      else
        let (rt, _) ← valueTerm s
        let tmp ← freshName "value"
        emit (text s!"let {tmp} ← *" ++ rt)
        pure (text tmp, false)
    | _ => term s

/-- The value of a block/sequence expression: statements are emitted into the
enclosing block; the last expression is the value. -/
partial def blockValue (e : Exp) : PM (Format × Bool) := do
  match e.node with
  | .block p binding body =>
    letStmt p binding
    blockValue body
  | .sequence [] => pure (text "()", false)
  | .sequence [x] => blockValue x
  | .sequence (x :: rest) =>
    stmt x
    blockValue (⟨e.ty, e.loc, .sequence rest⟩)
  | _ => term e

/-- A term with Action position: if the expression is an action, it is bound
to a temporary first. -/
partial def valueTerm (e : Exp) : PM (Format × Bool) := do
  match e.node with
  | .call (.borrow kind) _ [inner] _ =>
    -- A borrow in value position is its reference variable (the loan is
    -- emitted), not an action to bind.
    pure (text (← materializedRef kind inner), false)
  | _ =>
  match ← kindOf e with
  | .pure => term e
  | .action =>
    let a ← actionDoc e
    let tmp ← freshName "tmp"
    emit (text s!"let {tmp} ← " ++ a)
    pure (text tmp, false)

/-- The document of an action expression (to be bound with `←` or run as a
statement). -/
partial def actionDoc (e : Exp) : PM Format := do
  match e.node with
  | .call op inst args _ => actionCall e op inst args
  | .invoke function args => invokeCall function args
  | .mutate target value => do mutateStmt target value; pure (text "pure ()")
  | _ => do pure (← term e).1

/-- Invoke a function-valued expression. Move function types carry no effect
row, so the corresponding Lean value returns `Action`; a nullary Move
function receives Lean's unit value explicitly. -/
partial def invokeCall (function : Exp) (args : List Exp) : PM Format := do
  let (ft, fc) ← valueTerm function
  let docs ← args.mapM fun arg => do
    let (t, c) ← valueTerm arg
    pure (atom t c)
  let docs := if docs.isEmpty then [text "()"] else docs
  pure (app (atom ft fc) docs)

/-- Whether a term mentions only variables that never change in the
function, so its text denotes one value for the rest of the block. -/
partial def stableTerm (e : Exp) : PM Bool := do
  let ctxt ← read
  match e.node with
  | .value _ _ => pure true
  | .«local» n => pure (!ctxt.mutables.contains n)
  | .param i => pure ((ctxt.params[i]?.map fun n => !ctxt.mutables.contains n).getD false)
  | _ => pure false

/-- Decomposes a place; index and address terms are printed (and their
effects hoisted) once here. -/
partial def placePath (e : Exp) : PM PlacePath := do
  match e.node with
  | .«local» n => do pure ⟨e, text (← localPrinted n), true, []⟩
  | .param i =>
    match (← read).params[i]? with
    | some n => pure ⟨e, text (legalize n), true, []⟩
    | none => unsupported "parameter index"
  | .call (.borrowGlobal _) inst [addr] _ =>
    let (a, _) ← valueTerm addr
    let r ← resourceHead inst
    pure ⟨e, r ++ text "[" ++ a ++ text "]", ← stableTerm addr, []⟩
  | .call (.select _ field) _ [inner] _ =>
    let p ← placePath inner
    pure { p with steps := p.steps ++ [.field (legalize field)] }
  | .call (.selectVariants _ fields) _ [inner] _ =>
    -- The payload field of an enum: one name across the variants that have
    -- it; Leaner borrows it through the reference (`&r.f`), the VM's variant
    -- check included.
    let some field := variantFieldName? fields | unsupported "variant field selection across differently named fields"
    let p ← placePath inner
    pure { p with steps := p.steps ++ [.field (legalize field)] }
  | .call (.deref) _ [inner] _ => placePath inner
  | .call (.borrow _) _ [inner] _ => placePath inner
  | .call (.freeze _) _ [inner] _ => placePath inner
  | .call op@(.moveFunction _) _ [v, i] _ =>
    match vectorCall? op with
    | some .borrow | some .borrowMut =>
      let p ← placePath v
      let (it, _) ← valueTerm i
      pure { p with steps := p.steps ++ [.index it (← stableTerm i)] }
    | _ => unsupported "call in place position"
  | _ => unsupported "expression in place position"

/-- A place: a local, a global `R[a]`, a field path, a vector element. -/
partial def place (e : Exp) : PM Format := do
  let p ← placePath e
  pure (p.rootText ++ renderSteps p.steps)

/-- The operand of `&`/`&mut` for a `kind`-borrow of a place, or (`inl`) the
live loan that already holds the place.  Leaner's place grammar accepts an
element step only directly after the root and borrows fields and elements
through references only, and its verifier requires the borrows of a place to
chain through its live loan; so the place is rebased onto its longest loaned
prefix, a borrow inside an owned local first loans the local itself (the loan
becomes the local's access path, and the local is read back through it after
mutations), and the path is split into fresh loans at every later element
step. -/
partial def resolvePath (kind : RefKind) (path : PlacePath) : PM (String ⊕ Format) := do
  let isMut := kind == .mutable
  let amp := if isMut then "&mut " else "&"
  let keyOf (k : Nat) : String := Format.pretty (path.rootText ++ renderSteps (path.steps.take k))
  let stableTo (k : Nat) : Bool := path.rootStable && stepsStable (path.steps.take k)
  let refRoot := match path.root.node with
    | .«local» _ | .param _ => isRefTy path.root.ty
    | _ => false
  -- A reborrow of a reference variable is the variable.
  if path.steps.isEmpty && refRoot then return .inl (keyOf 0)
  let mut base := path.rootText
  let mut baseKey := keyOf 0
  let mut baseStable := path.rootStable
  let mut rest := path.steps
  let mut found := false
  for k in (List.range (path.steps.length + 1)).reverse do
    if !found then
      if let some loan ← loanFor kind (keyOf k) then
        if k == path.steps.length then return .inl loan
        base := text loan; baseKey := keyOf k; baseStable := stableTo k
        rest := path.steps.drop k; found := true
  let ownedRoot := match path.root.node with
    | .«local» _ | .param _ => !refRoot
    | _ => false
  if !found && ownedRoot && !path.steps.isEmpty then
    -- Leaner borrows fields and elements through references only: loan the
    -- local itself first; the sub-borrow chains through the live loan, and the
    -- local is read back through it after mutations.
    let r ← freshName "ref"
    emit (text s!"let {r} ← {amp}" ++ path.rootText)
    registerLoan baseKey r isMut
    base := text r
  let mut pending : List PlaceStep := []
  for step in rest do
    if let .index _ _ := step then
      if !pending.isEmpty then
        let r ← freshName "ref"
        emit (text s!"let {r} ← {amp}" ++ base ++ renderSteps pending)
        baseKey := Format.pretty (text baseKey ++ renderSteps pending)
        baseStable := baseStable && stepsStable pending
        if baseStable then registerLoan baseKey r isMut
        base := text r; pending := []
    pending := pending ++ [step]
  pure (.inr (base ++ renderSteps pending))

/-- The reference variable holding a `kind`-borrow of the place `e`: its live
loan, else a fresh named loan. -/
partial def borrowRef (kind : RefKind) (e : Exp) (base : String := "ref") : PM String := do
  let path ← placePath e
  match ← resolvePath kind path with
  | .inl loan => pure loan
  | .inr p =>
    let r ← freshName base
    emit (text s!"let {r} ← " ++ text (if kind == .mutable then "&mut " else "&") ++ p)
    if path.stable then registerLoan path.key r (kind == .mutable)
    pure r

/-- A reference for a `kind`-borrow of `e`: of its place if it is one, else of
a fresh local bound to its value (Move borrows temporaries, `&2`; Leaner
borrows places). -/
partial def materializedRef (kind : RefKind) (e : Exp) : PM String := do
  if isPlaceExp e then borrowRef kind e
  else
    let (t, _) ← valueTerm e
    let tmp ← freshName "value"
    emit (text s!"let {tmp} : " ++ (← printTy e.ty) ++ text " := " ++ t)
    let r ← freshName "ref"
    emit (text s!"let {r} ← " ++ text (if kind == .mutable then "&mut " else "&") ++ text tmp)
    pure r

/-- The read-back of an owned mutable local after a mutation through a loan
rooted at it (`x ← *loan`), if the place is rooted at one. -/
partial def readBackFor (e : Exp) : PM (Option Format) := do
  match ← placeRootLocal e with
  | some n =>
    if (← read).mutables.contains n then
      let printed ← localPrinted n
      match ← loanFor .mutable printed with
      | some loan => pure (some (text s!"{printed} ← *{loan}"))
      | none => pure none
    else pure none
  | none => pure none

/-- The head of a resource family from a type instantiation: `R` or `(R T)`. -/
partial def resourceHead (inst : List Ty) : PM Format := do
  match inst with
  | [.struct name []] => pure (text (qualify (← read).mod.ref name))
  | [t@(.struct _ _)] => do pure (paren (← printTy t))
  | [t] => printTy t
  | _ => unsupported "resource instantiation"

/-- `*target = value`, emitted as statements. -/
partial def mutateStmt (target value : Exp) : PM Unit := do
  match target.node with
  | .«local» _ | .param _ =>
    -- Writing through a reference variable; `*r = *r op e` keeps Leaner's
    -- sequenced `r := *r op e` shape.
    let tname ← localName? target
    let sequenced ← match value.node with
      | .call op _ [l, r] _ =>
        match l.node with
        | .call .deref _ [inner] _ =>
          let iname ← localName? inner
          if iname.isSome && iname == tname then
            let sym := match op with
              | .add => some "+" | .sub => some "-" | .mul => some "*" | .div => some "/"
              | .mod => some "%" | _ => none
            match sym with
            | some sym => do
              let (rt, rc) ← valueTerm r
              pure (some (text s!" := *{legalize tname.get!} {sym} " ++ atom rt rc))
            | none => pure none
          else pure none
        | _ => pure none
      | _ => pure none
    match sequenced with
    | some tail => do emit ((← place target) ++ tail)
    | none =>
      let (vt, _) ← valueTerm value
      emit ((← place target) ++ text " := " ++ vt)
  | _ =>
    if isPlaceExp target then
      -- A field (or element) place, through a local or a reference: written
      -- through its loan; `place = place op e` keeps the `r := *r op e` shape.
      let path ← placePath target
      let ref ← match ← resolvePath .mutable path with
        | .inl loan => pure loan
        | .inr p =>
          let r ← freshName "ref"
          emit (text s!"let {r} ← &mut " ++ p)
          if path.stable then registerLoan path.key r true
          pure r
      let sequenced ← match value.node with
        | .call op _ [l, r] _ =>
          if isPlaceExp l && (← placePath l).key == path.key then
            let sym := match op with
              | .add => some "+" | .sub => some "-" | .mul => some "*" | .div => some "/"
              | .mod => some "%" | _ => none
            match sym with
            | some sym => do
              let (rt, rc) ← valueTerm r
              pure (some (text s!"{ref} := *{ref} {sym} " ++ atom rt rc))
            | none => pure none
          else pure none
        | _ => pure none
      match sequenced with
      | some line => emit line
      | none =>
        let (vt, _) ← valueTerm value
        emit (text ref ++ text " := " ++ vt)
      if let some rb ← readBackFor target then emit rb
    else
      let (vt, _) ← valueTerm value
      let (rt, _) ← valueTerm target
      emit (rt ++ text " := " ++ vt)

partial def callTerm (e : Exp) (op : Operation) (inst : List Ty) (args : List Exp) :
    PM (Format × Bool) := do
  let ctxt ← read
  let binop (sym : String) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let (lhs, lc) ← valueTerm a
      let (bt, bc) ← valueTerm b
      pure (atom lhs lc ++ text s!" {sym} " ++ atom bt bc, true)
    | _ => unsupported s!"arity of `{sym}`"
  -- A shift amount is a `U8`; a literal amount needs the ascription (the
  -- amount's type is not determined by the shifted operand).
  let shiftOp (sym : String) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let (lhs, lc) ← match a.node with
        | .value _ none => do pure (paren ((← valueTerm a).1 ++ text " : " ++ (← printTy e.ty)), false)
        | _ => valueTerm a
      let bt ← match b.node with
        | .value _ none => do pure (paren ((← valueTerm b).1 ++ text " : U8"))
        | _ => do let (t, c) ← valueTerm b; pure (atom t c)
      pure (atom lhs lc ++ text s!" {sym} " ++ bt, true)
    | _ => unsupported s!"arity of `{sym}`"
  match op with
  | .add => binop "+"
  | .sub => binop "-"
  | .mul => binop "*"
  | .div => binop "/"
  | .mod => binop "%"
  | .bitOr => binop "|||"
  | .bitAnd => binop "&&&"
  | .xor => binop "^^^"
  | .shl => shiftOp "<<<"
  | .shr => shiftOp ">>>"
  | .and | .or =>
    match args with
    | [a, b] =>
      -- Move's `&&`/`||` short-circuit: an effectful right operand is
      -- evaluated only when the left one does not decide the result.
      let rhsKind ← kindOf b
      let isAnd := op matches .and
      if rhsKind == .pure then binop (if isAnd then "&&" else "||")
      else
        -- `let mut t := a; if t then t := b` (resp. `if !t then t := b`).
        let (lhs, _) ← valueTerm a
        let tmp ← freshName "value"
        let lines ← collect (blockTail b (some tmp))
        emit (text s!"let mut {tmp} : Bool := " ++ lhs)
        emit (text (if isAnd then s!"if {tmp} then" else s!"if !{tmp} then") ++
          indent 2 (hard ++ vcat lines))
        pure (text tmp, false)
    | _ => unsupported "arity of boolean connective"
  | .eq => binop "=="
  | .neq => binop "!="
  | .lt => binop "<"
  | .gt => binop ">"
  | .le => binop "<="
  | .ge => binop ">="
  | .not =>
    match args with
    | [a] => do let (lhs, lc) ← valueTerm a; pure (text "!" ++ atom lhs lc, false)
    | _ => unsupported "arity of `!`"
  | .negate =>
    match args with
    | [a] => do let (lhs, lc) ← valueTerm a; pure (text "-" ++ atom lhs lc, false)
    | _ => unsupported "arity of negation"
  | .cast =>
    match args with
    | [a] => do
      match a.node with
      | .value _ none =>
        -- A cast of a literal is the literal at the target type.
        let (lit, _) ← valueTerm a
        pure (paren (lit ++ text " : " ++ (← printTy e.ty)), false)
      | .«local» _ | .param _ =>
        let (lhs, lc) ← valueTerm a
        pure (paren (atom lhs lc ++ text ".cast : " ++ (← printTy e.ty)), false)
      | _ =>
        -- The verifier reads a cast's operand as a local: bind it first.
        let (lhs, _) ← valueTerm a
        let tmp ← freshName "value"
        emit (text s!"let {tmp} := " ++ lhs)
        pure (paren (text s!"{tmp}.cast : " ++ (← printTy e.ty)), false)
    | _ => unsupported "arity of cast"
  | .copy | .move | .freeze false =>
    match args with
    | [a] => term a
    | _ => unsupported "arity"
  | .tuple =>
    match args with
    | [single] => term single
    | _ =>
      let fs ← args.mapM fun a => do pure (← valueTerm a).1
      if fs.isEmpty then pure (text "()", false) else pure (paren (sepBy ", " fs), false)
  | .vector =>
    let fs ← args.mapM fun a => do pure (← valueTerm a).1
    if fs.isEmpty then
      if (← read).suppressAscription then pure (text "vector![]", false)
      else do pure (paren (text "vector![] : " ++ (← printTy e.ty)), false)
    else pure (text "vector![" ++ sepBy ", " fs ++ text "]", false)
  | .pack name variant =>
    -- In code, a certified struct is created through `T.certify`, an
    -- `Action` bound to a temporary; in specifications and for other structs
    -- the literal is the value.
    if variant.isNone && !ctxt.inSpec && ctxt.pkg.certified name then
      let tmp ← freshName "tmp"
      emit (text s!"let {tmp} ← " ++ (← certifyTerm e name inst args))
      pure (text tmp, false)
    else packTerm e name variant inst args
  | .select _ field =>
    match args with
    | [inner] =>
      match borrowRoot inner with
      | some kind =>
        -- A field read through a borrow: borrow the whole field place, read it.
        let ref ← borrowRef kind e
        let v ← freshName "value"
        emit (text s!"let {v} ← *{ref}")
        pure (text v, false)
      | none =>
      if isRefTy inner.ty then
        if isPlaceExp e then
          -- A field read through a reference place: borrow the field, read it.
          let ref ← borrowRef .immutable e
          let v ← freshName "value"
          emit (text s!"let {v} ← *{ref}")
          pure (text v, false)
        else
          -- Reading a field of a reference value: read the referent first.
          let (rt, _) ← valueTerm inner
          let tmp ← freshName "value"
          emit (text s!"let {tmp} ← *" ++ rt)
          pure (text tmp ++ text "." ++ text (legalize field), false)
      else
        let (it, ic) ← valueTerm inner
        pure (atom it ic ++ text "." ++ text (legalize field), false)
    | [] =>
      -- A bare field in a struct invariant: the field of the value itself.
      if ctxt.inStructInvariant then pure (text s!"this.{legalize field}", false)
      else unsupported "field selection without an owner"
    | _ => unsupported "arity of field selection"
  | .selectVariants _ fields =>
    -- The payload field of whichever variant the owner is: borrowed through
    -- the owner (Leaner's `&r.f` on an enum referent performs the VM's
    -- variant check), then read.
    match args with
    | [inner] =>
      let some _ := variantFieldName? fields
        | unsupported "variant field selection across differently named fields"
      match borrowRoot inner with
      | some kind =>
        let ref ← borrowRef kind e
        let v ← freshName "value"
        emit (text s!"let {v} ← *{ref}")
        pure (text v, false)
      | none =>
        if isPlaceExp e then
          let ref ← borrowRef .immutable e
          let v ← freshName "value"
          emit (text s!"let {v} ← *{ref}")
          pure (text v, false)
        else unsupported "variant field selection of a value that is not a place"
    | _ => unsupported "arity of variant field selection"
  | .testVariants name variants =>
    match args with
    | [inner] =>
      let (it, ic) ← if isRefTy inner.ty then do
          -- `r is V`: the test is on the referent.
          let (rt, _) ← valueTerm inner
          let tmp ← freshName "value"
          emit (text s!"let {tmp} ← *" ++ rt)
          pure (text tmp, false)
        else valueTerm inner
      let head := qualify ctxt.mod.ref name
      let tests := variants.map fun v => atom it ic ++ text s!" is {head}.{legalize v}"
      pure (sepBy " || " tests, true)
    | _ => unsupported "arity of `is`"
  | .moveFunction name =>
    match vectorCall? op with
    | some vop => vectorTerm e vop inst args
    | none =>
      if isPrimitiveModule name.module then
        unsupported s!"std::{name.module.name}::{name.name}"
      else
        let kind ← kindOf e
        let doc ← userCall name args (e.node matches .call _ _ _ (some .receiverCall))
        match kind with
        | .action =>
          let tmp ← freshName "tmp"
          emit (text s!"let {tmp} ← " ++ doc)
          pure (text tmp, false)
        | .pure => pure (doc, !(Format.pretty doc == "()") && !args.isEmpty)
  | .borrow _ | .borrowGlobal _ | .deref | .«exists» _ | .moveTo | .moveFrom | .freeze true
  | .abort _ =>
    let a ← actionCall e op inst args
    let tmp ← freshName "tmp"
    emit (text s!"let {tmp} ← " ++ a)
    pure (text tmp, false)
  | .specFunction _ _ | .behavior _ _ | .old | .global _ | .len | .index | .slice |
      .range | .implies | .iff
  | .identical | .result _ | .updateField _ _ | .canModify | .typeValue | .typeDomain
  | .resourceDomain | .stateDomain | .emptyVec | .singleVec | .updateVec | .concatVec
  | .indexOfVec | .containsVec | .inRangeRange | .inRangeVec | .rangeVec | .maxU8 | .maxU16
  | .maxU32 | .maxU64 | .maxU128 | .maxU256 | .bv2Int | .int2Bv | .abortFlag | .abortCode
  | .wellFormed | .boxValue | .unboxValue | .emptyEventStore | .extendEventStore
  | .eventStoreIncludes | .eventStoreIncludedIn | .saveStateAnchor _ | .withStateAnchor _
  | .foldsCaptureAnchor _ | .inlineCallSummary | .specPublish _ | .specRemove _
  | .specUpdate _ => unsupported "spec operation in code"
  | .trace _ | .noOp =>
    match args with
    | [a] => term a
    | [] => pure (text "()", false)
    | _ => unsupported "arity"

/-- The creation of a certified struct value: `T.certify f₁ … fₙ`, the
positional field values (ascribed with the type when it is instantiated). -/
partial def certifyTerm (e : Exp) (name : QualifiedName) (inst : List Ty)
    (args : List Exp) : PM Format := do
  let ctxt ← read
  let fs ← args.mapM fun a => do
    pure (← withReader (fun c => { c with suppressAscription := true }) (valueTerm a))
  let doc := app (text s!"{qualify ctxt.mod.ref name}.certify") (fs.map fun (f, c) => atom f c)
  if inst.isEmpty then pure doc
  else do pure (paren (doc ++ text " : Action " ++ paren (← printTy e.ty)))

/-- A struct or enum value. -/
partial def packTerm (e : Exp) (name : QualifiedName) (variant : Option String) (inst : List Ty)
    (args : List Exp) : PM (Format × Bool) := do
  let ctxt ← read
  -- Field values have a known expected type: no ascriptions inside.
  let fs ← args.mapM fun a => do
    pure (← withReader (fun c => { c with suppressAscription := true }) (valueTerm a))
  let head := qualify ctxt.mod.ref name
  match variant with
  | some v =>
    let doc := app (text s!"{head}.{legalize v}") (fs.map fun (f, c) => atom f c)
    if inst.isEmpty then pure (doc, !fs.isEmpty)
    else do pure (paren (doc ++ text " : " ++ (← printTy e.ty)), false)
  | none =>
    match ctxt.pkg.struct? name with
    | some s =>
      if isPositionalFields s.fields then
        let doc := app (text s!"{head}.mk") (fs.map fun (f, c) => atom f c)
        if inst.isEmpty then pure (doc, !fs.isEmpty)
        else do pure (paren (doc ++ text " : " ++ (← printTy e.ty)), false)
      else
        let inits := (s.fields.zip fs).map fun (fld, (f, _)) =>
          text (legalize fld.name) ++ text " := " ++ f
        let body := text "{ " ++ sepBy ", " inits ++ text " }"
        pure (paren (body ++ text " : " ++ (← printTy e.ty)), false)
    | none => unsupported s!"struct `{name.name}` of an unknown module"

/-- A call to a user function: `f args` or `x.f args` in receiver style; a
`&mut local` temporary becomes a named loan that is read back afterwards. -/
partial def userCall (name : QualifiedName) (args : List Exp) (receiver : Bool) : PM Format := do
  let ctxt ← read
  let head := qualify ctxt.mod.ref name
  let mut docs : List Format := []
  let mut readBacks : List Format := []
  for a in args do
    match a.node with
    | .call (.borrow kind) _ [inner] _ =>
      let r ← materializedRef kind inner
      docs := docs ++ [text r]
      if kind == .mutable then
        if let some rb ← readBackFor inner then readBacks := readBacks ++ [rb]
    | _ =>
      let (t, c) ← valueTerm a
      docs := docs ++ [atom t c]
  -- Receiver style (`x.f args`) needs the callee in the type's namespace,
  -- which the printer does not emit yet (E1); calls print in prefix form.
  let _ := receiver
  let doc := app (text head) docs
  let facts ← calleeFacts name
  let isUnit := match ctxt.pkg.function? name with
    | some callee => isUnitTy callee.result
    | none => false
  if facts.isAction && isUnit then
    -- A unit-typed effectful call is a statement; its read-backs follow.
    emit doc
    for rb in readBacks do emit rb
    pure (text "()")
  else if readBacks.isEmpty then pure doc
  else do
    -- Bind the call now so the read-backs follow it in order.
    if facts.isAction then
      let tmp ← freshName "tmp"
      emit (text s!"let {tmp} ← " ++ doc)
      for rb in readBacks do emit rb
      pure (text tmp)
    else
      let tmp ← freshName "tmp"
      emit (text s!"let {tmp} := " ++ doc)
      for rb in readBacks do emit rb
      pure (text tmp)

/-- A `std::vector` call.  `bindTo` names the `let` the result is bound to,
so a mutation through a reference binds its result directly. -/
partial def vectorTerm (e : Exp) (vop : VecOp) (inst : List Ty) (args : List Exp)
    (bindTo : Option String := none) : PM (Format × Bool) := do
  let operand (a : Exp) : PM Format := do
    -- `&v` / `&mut v` of a local: the local itself; a borrow of any other
    -- place: a named loan; a reference variable: itself.
    match a.node with
    | .call (.borrow kind) _ [inner] _ =>
      match ← localName? inner with
      | some n => pure (text (legalize n))
      | none => do pure (text (← materializedRef kind inner))
    | _ => do pure (← valueTerm a).1
  let elemTy : PM Format := do
    match inst with
    | [t] => printTy t
    | _ => unsupported "vector instantiation"
  match vop, args with
  | .empty, [] =>
    if (← read).suppressAscription then pure (text "Move.Vector.empty", false)
    else do pure (paren (text "Move.Vector.empty : Vector " ++ (← elemTy)), false)
  | .singleton, [x] => do
    let (xt, xc) ← valueTerm x
    pure (app (text "Move.Vector.singleton") [atom xt xc], true)
  | .length, [v] => do pure ((← operand v) ++ text ".length", false)
  | .isEmpty, [v] => do pure ((← operand v) ++ text ".isEmpty", false)
  | .pushBack, [v, x] =>
    let (xt, _) ← valueTerm x
    let localLoan ← match v.node with
      | .call (.borrow _) _ [inner] _ =>
        match ← localName? inner with
        | some n => loanFor .mutable (legalize n)
        | none => pure none
      | _ => pure none
    if isLocalBorrow v && localLoan.isNone then
      let vt ← operand v
      emit (vt ++ text " := " ++ vt ++ text ".push " ++ xt)
      pure (text "()", false)
    else
      -- Through a reference (a variable, the local's live loan, or a loan of a
      -- non-local place): read, push, write back.
      let rt ← match localLoan with
        | some loan => pure (text loan)
        | none => operand v
      let tmp ← freshName "value"
      emit (text s!"let {tmp} ← *" ++ rt)
      emit (rt ++ text s!" := {tmp}.push " ++ xt)
      match v.node with
      | .call (.borrow _) _ [inner] _ => if let some rb ← readBackFor inner then emit rb
      | _ => pure ()
      pure (text "()", false)
  | .borrow, [v, i] | .borrowMut, [v, i] =>
    let _ := (v, i)
    let kind := if vop == .borrowMut then RefKind.mutable else RefKind.immutable
    pure (text (← borrowRef kind e (base := "elem")), false)
  | .destroyEmpty, [v] => do
    let (vt, vc) ← valueTerm v
    pure (app (text "Move.Vector.destroyEmpty") [atom vt vc], true)
  | .contains, [v, x] | .indexOf, [v, x] =>
    let refOf (a : Exp) : PM Format := do
      match a.node with
      | .call (.borrow kind) _ [inner] _ => do pure (text (← materializedRef kind inner))
      | _ => do pure (← valueTerm a).1
    let vr ← refOf v
    let xr ← refOf x
    let fn := if vop == .contains then "Move.Vector.contains" else "Move.Vector.indexOf"
    pure (app (text fn) [vr, xr], true)
  | .mutRef leanName, v :: rest =>
    let restDocs ← rest.mapM fun a => do let (t, c) ← valueTerm a; pure (atom t c)
    let recv ← match v.node with
      | .call (.borrow .mutable) _ [inner] _ =>
        let r ← borrowRef .mutable inner
        -- read back a local after the mutation
        pure (text r, ← readBackFor inner)
      | _ => do pure ((← valueTerm v).1, none)
    let call := app (text s!"Move.Vector.{leanName}") (recv.1 :: restDocs)
    if isUnitTy e.ty then
      emit call
      if let some rb := recv.2 then emit rb
      pure (text "()", false)
    else
      let name ← match bindTo with
        | some n => pure n
        | none => freshName "tmp"
      emit (text s!"let {legalize name} ← " ++ call)
      if let some rb := recv.2 then emit rb
      pure (text (legalize name), false)
  | _, _ => unsupported s!"vector operation {repr vop} arity"

/-- The document of an action call (borrow, global storage, abort, ...). -/
partial def actionCall (e : Exp) (op : Operation) (inst : List Ty) (args : List Exp) :
    PM Format := do
  match op, args with
  | .borrow kind, [inner] => do pure (text s!"pure {← materializedRef kind inner}")
  | .borrowGlobal kind, [addr] => do
    let (a, _) ← valueTerm addr
    pure (text (if kind == .mutable then "&mut " else "&") ++ (← resourceHead inst) ++
      text "[" ++ a ++ text "]")
  | .deref, [inner] =>
    -- `*vector::borrow(&v, i)` is the pure `v.get i`; any other read through
    -- a borrow of a place reads through the place's loan.  A dereference that
    -- stands as a statement prints in parentheses (`(*r)`): Lean's `do` would
    -- read a leading `*` as a continuation of the previous line.
    let doc ← match inner.node with
    | .call iop@(.moveFunction _) _ [v, i] _ =>
      match vectorCall? iop with
      | some .borrow =>
        if isLocalBorrow v then
          let (it, _) ← valueTerm i
          let vt ← match v.node with
            | .call _ _ [l] _ => do pure (← place l)
            | _ => do pure (← valueTerm v).1
          pure (text "pure (" ++ vt ++ text ".get " ++ it ++ text ")")
        else do pure (text s!"*{← borrowRef .immutable inner (base := "elem")}")
      | some .borrowMut => do pure (text s!"*{← borrowRef .mutable inner (base := "elem")}")
      | _ => do pure (text "*" ++ (← valueTerm inner).1)
    | .call (.borrow kind) _ [p] _ => do pure (text s!"*{← materializedRef kind p}")
    | _ => do pure (text "*" ++ (← valueTerm inner).1)
    pure (if (Format.pretty doc).startsWith "*" then paren doc else doc)
  | .«exists» _, [addr] => do
    let (a, _) ← valueTerm addr
    pure (app (text "existsAt") [← resourceHead inst, a])
  | .moveTo, [signer, value] => do
    let (st, sc) ← valueTerm signer
    let (vt, vc) ← valueTerm value
    pure (app (text "moveTo") [atom st sc, atom vt vc])
  | .moveFrom, [addr] => do
    let (a, _) ← valueTerm addr
    pure (app (text "moveFrom") [← resourceHead inst, a])
  | .freeze _, [inner] => do
    let (it, ic) ← valueTerm inner
    pure (app (text "freeze") [atom it ic])
  | .abort kind, [code] => do
    let (ct, cc) ← valueTerm code
    match kind with
    | .code => pure (app (text "abort") [atom ct cc])
    | .message => pure (app (text "abort") [atom ct cc])
  | .moveFunction name, _ =>
    match vectorCall? op, args with
    | some .borrow, [v, i] | some .borrowMut, [v, i] =>
      let _ := (v, i)
      let kind := if vectorCall? op == some .borrowMut then RefKind.mutable else RefKind.immutable
      pure (text s!"pure {← borrowRef kind e (base := "elem")}")
    | some vop, _ => do pure (← vectorTerm e vop inst args).1
    | none, _ => userCall name args (e.node matches .call _ _ _ (some .receiverCall))
  | .pack name none, _ =>
    if (← read).pkg.certified name then certifyTerm e name inst args
    else do pure (← term e).1
  | _, _ => do pure (← term e).1

/-- A pattern. -/
partial def pattern (p : Pattern) : PM Format := do
  let ctxt ← read
  match p.node with
  | .var n => do pure (text (← localPrinted n))
  | .wildcard => pure (text "_")
  | .tuple [single] => pattern single
  | .tuple ps => do
    let fs ← ps.mapM pattern
    pure (paren (sepBy ", " fs))
  | .struct name _ variant fields =>
    let fs ← fields.mapM pattern
    match variant with
    | some v =>
      let head := if name.module == ctxt.mod.ref then "." ++ legalize v
        else qualify ctxt.mod.ref name ++ "." ++ legalize v
      pure (app (text head) fs)
    | none =>
      match ctxt.pkg.struct? name with
      | some s =>
        if isPositionalFields s.fields then
          pure (text (qualify ctxt.mod.ref name) ++ paren (sepBy ", " fs))
        else pure (text "⟨" ++ sepBy ", " fs ++ text "⟩")
      | none => pure (text "⟨" ++ sepBy ", " fs ++ text "⟩")
  | .literal v => printValue v p.ty
  | .range lower upper inclusive =>
    let lo ← match lower with
      | some v => printValue v p.ty
      | none => pure (text "")
    let hi ← match upper with
      | some v => printValue v p.ty
      | none => pure (text "")
    pure (lo ++ text (if inclusive then "..=" else "..") ++ hi)

-- -------------------------------------------------------------------------------------------------
-- Statements

/-- `let pattern [:=|←] binding`. -/
partial def letStmt (p : Pattern) (binding : Option Exp) : PM Unit := do
  let ctxt ← read
  let names := patternNames p
  let isMut := names.any ctxt.mutables.contains
  let mutKw := if isMut then "mut " else ""
  match binding with
  | none =>
    -- A declaration without a value (`let x;`): Leaner needs a value; use the
    -- type's default via a later assignment — the first assignment will bind.
    -- Move requires assignment before use, so the declaration is dropped and
    -- the first `assign` prints as a `let mut`.
    modify fun s => { s with
      uninitialized := names.foldl (fun pending name =>
        if pending.contains name then pending else name :: pending) s.uninitialized }
  | some b =>
    let planned ← names.mapM fun n => do pure (n, ← planBinding n)
    let pat ← withReader (fun c => { c with bindingOverrides := planned }) (pattern p)
    bindAfter planned isMut do
    match b.node with
    | .call (.borrow kind) _ [inner] _ =>
      if isPlaceExp inner then namedLoan mutKw pat kind inner
      else letGeneric mutKw pat b
    | .call op@(.moveFunction _) inst args _ =>
      match vectorCall? op, p.node with
      | some (.mutRef _), .var n =>
        -- `let x = vector::pop_back(&mut v)` binds the result directly.
        let _ ← vectorTerm b (VecOp.mutRef ((vectorCall? op).map (fun | .mutRef l => l | _ => "") |>.getD "")) inst args (some n)
        pure ()
      | some .borrow, .var _ => namedLoan mutKw pat .immutable b
      | some .borrowMut, .var _ => namedLoan mutKw pat .mutable b
      | _, _ => letGeneric mutKw pat b
    | _ => letGeneric mutKw pat b
where
  /-- Runs the statement printer `k`, then registers the planned bindings
  (forgetting the loans of rebound names). -/
  bindAfter (planned : List (String × String)) (isMut : Bool) (k : PM Unit) : PM Unit := do
    k
    for (n, printedName) in planned do
      forgetLoans (legalize printedName)
      commitBinding n printedName isMut
  /-- A let-bound reference: a named loan of a place, or an alias of the
  place's live loan. -/
  namedLoan (mutKw : String) (pat : Format) (kind : RefKind) (placeExp : Exp) : PM Unit := do
    let path ← placePath placeExp
    match ← resolvePath kind path with
    | .inl loan => emit (text s!"let {mutKw}" ++ pat ++ text s!" := {loan}")
    | .inr p =>
      emit (text s!"let {mutKw}" ++ pat ++ text " ← " ++
        text (if kind == .mutable then "&mut " else "&") ++ p)
      if path.stable then registerLoan path.key (Format.pretty pat) (kind == .mutable)
  letGeneric (mutKw : String) (pat : Format) (b : Exp) : PM Unit := do
    match ← kindOf b with
    | .action =>
      let a ← actionDoc b
      emit (text s!"let {mutKw}" ++ pat ++ text " ← " ++ a)
    | .pure =>
      -- Literal-rooted bindings need the type: Leaner infers widths from
      -- the expected type.
      let annotate ← needsAnnotation b
      if annotate then
        let (t, _) ← withReader (fun c => { c with suppressAscription := true }) (term b)
        emit (text s!"let {mutKw}" ++ pat ++ text " : " ++ (← printTy b.ty) ++ text " := " ++ t)
      else
        let (t, _) ← term b
        emit (text s!"let {mutKw}" ++ pat ++ text " := " ++ t)

partial def needsAnnotation (e : Exp) : PM Bool := do
  match e.node with
  | .value _ none => pure true
  | .value _ (some _) => pure false
  | .call .vector _ _ _ => pure true
  | .call .cast _ _ _ => pure false
  | .call op@(.moveFunction _) _ [] _ => pure (vectorCall? op == some .empty)
  | _ => pure false

/-- `x := v` for a local: through its live mutable loan when it has one (the
loan is the local's access path for the rest of the block), followed by the
read-back. -/
partial def assignLocal (n : String) (vt : Format) (type? : Option Ty := none) : PM Unit := do
  if (← get).uninitialized.contains n then
    let printedName ← planBinding n
    let annotation ← match type? with
      | some sourceType => pure (text " : " ++ (← printTy sourceType))
      | none => pure (text "")
    emit (text s!"let mut {legalize printedName}" ++ annotation ++ text " := " ++ vt)
    commitBinding n printedName true
    modify fun s => { s with uninitialized := s.uninitialized.filter (· != n) }
    return
  let name ← match (← read).paramRenames.lookup n with
    | some renamed => pure (legalize renamed)
    | none => localPrinted n
  match ← loanFor .mutable name with
  | some loan =>
    emit (text s!"{loan} := " ++ vt)
    emit (text s!"{name} ← *{loan}")
  | none => emit (text name ++ text " := " ++ vt)

/-- A statement. -/
partial def stmt (e : Exp) : PM Unit := do
  let ctxt ← read
  match e.node with
  | .sequence es => for x in es do stmt x
  | .block p binding body =>
    letStmt p binding
    stmt body
  | .ite c t f =>
    -- assert! shapes
    match assertShape c t f with
    | some (cond, code, negated) =>
      let condDoc ← if negated then negatedTerm cond else do pure (← valueTerm cond).1
      let (codeT, _) ← valueTerm code
      emit (text "assert!(" ++ condDoc ++ text ", " ++ codeT ++ text ")")
    | none =>
      let (ct, _) ← valueTerm c
      emit (← ifChain ct t f (fun x => collect (stmt x)))
  | .«match» s arms =>
    let (st, _) ← scrutinee s arms
    let armDocs ← arms.mapM fun a => do
      let p ← pattern a.pattern
      let g ← match a.guard with
        | some g => do pure (text " if " ++ (← term g).1)
        | none => pure (text "")
      let lines ← collect (stmt a.body)
      pure (text "| " ++ p ++ g ++ text " =>" ++ indent 2 (hard ++ vcat (if lines.isEmpty then [text "pure ()"] else lines)))
    emit (text "match " ++ st ++ text " with" ++ indent 2 (hard ++ vcat armDocs))
  | .loop body => loopStmt body
  | .loopCont nest isContinue =>
    let labels := ctxt.loopLabels
    let label := if nest == 0 then none else labels.reverse[nest]? |>.join
    let kw := if isContinue then "continue" else "break"
    match label with
    | some l => emit (text s!"{kw}@{l}")
    | none => emit (text kw)
  | .«return» v =>
    if isUnit v then emit (text "return ()")
    else
      let (vt, _) ← valueTerm v
      emit (text "return " ++ vt)
  | .assign p v =>
    match p.node with
    | .var n =>
      let (vt, _) ← valueTerm v
      assignLocal n vt (some p.ty)
    | .wildcard =>
      match ← kindOf v with
      | .action => emit (text "let _ ← " ++ (← actionDoc v))
      | .pure => do let (vt, _) ← term v; emit (text "let _ := " ++ vt)
    | .tuple ps =>
      let (vt, _) ← valueTerm v
      let tmps ← ps.mapM fun _ => freshName "part"
      emit (text "let (" ++ sepBy ", " (tmps.map text) ++ text ") := " ++ vt)
      for (sub, tmp) in ps.zip tmps do
        match sub.node with
        | .var n => assignLocal n (text tmp) (some sub.ty)
        | .wildcard => pure ()
        | _ => unsupported "nested assignment pattern"
    | .struct .. =>
      if (patternNames p).all (← get).uninitialized.contains then
        letStmt p (some v)
      else unsupported "assignment to an initialized constructor pattern"
    | _ => unsupported "assignment pattern"
  | .mutate target value => mutateStmt target value
  | .specBlock spec =>
    for c in spec.conditions do
      match c.kind with
      | .assert =>
        try
          let (term, _) ← withReader (fun cx => { cx with inSpec := true, specInBody := true })
            (specTerm c.exp)
          record .specStatement
          emit (text "assert " ++ term)
        catch e =>
          drop s!"spec assert in function `{(ctxt.fn.map (·.name)).getD "?"}`: {e}"
          emit (text s!"-- spec assert dropped: {e}")
      | .assume =>
        match c.exp.node with
        | .call (.foldsCaptureAnchor label) _ [] _
        | .call (.saveStateAnchor label) _ [] _ =>
          -- The model compiler inserts this before an inlined fold.  Its
          -- anchored `old` uses become `oldAt[label](…)` below, and Leaner's
          -- source-spec translator snapshots their values at this point.
          record .specStatement
          emit (text s!"__moveSpecCapture {label}")
        | _ =>
          try
            let (term, _) ← withReader (fun cx => { cx with inSpec := true, specInBody := true })
              (specTerm c.exp)
            record .specStatement
            emit (text "assume " ++ term)
          catch e =>
            drop s!"spec assume in function `{(ctxt.fn.map (·.name)).getD "?"}`: {e}"
            emit (text s!"-- spec assume dropped: {e}")
      | .loopInvariant =>
        -- Loop-head blocks are removed by `splitLoopInvariants` before this
        -- statement printer runs.  A remaining invariant is not attached to
        -- any loop and cannot be given a sound source meaning.
        record .loopInvariant
        drop s!"loop invariant outside a loop in function `{(ctxt.fn.map (·.name)).getD "?"}`"
        emit (text "-- loop invariant dropped: not at a loop head")
      | kind =>
        drop s!"spec condition `{repr kind}` in function `{(ctxt.fn.map (·.name)).getD "?"}`"
        emit (text s!"-- spec condition dropped: {repr kind}")
    if !spec.pragmas.isEmpty || spec.frame.isSome then
      drop s!"spec block metadata in function `{(ctxt.fn.map (·.name)).getD "?"}`"
  | .call (.abort kind) _ [code] _ =>
    let (ct, cc) ← valueTerm code
    let _ := kind
    emit (app (text "abort") [atom ct cc])
  | .call op inst args _ =>
    if isUnit e then pure ()
    else if let some vop@(.mutRef _) := vectorCall? op then
      -- A vector mutation whose result is discarded: `let _ ← …`.
      let _ ← vectorTerm e vop inst args (some "_")
    else
      match ← kindOf e with
      | .action =>
        let a ← actionDoc e
        if isUnitTy e.ty then emit a else emit (text "let _ ← " ++ a)
      | .pure =>
        let (t, _) ← term e
        if isUnitTy e.ty then pure () else emit (text "let _ := " ++ t)
  | .invoke function args =>
    let call ← invokeCall function args
    if isUnitTy e.ty then emit call else emit (text "let _ ← " ++ call)
  | .value _ _ | .«local» _ | .param _ => pure ()
  | .quant _ _ _ _ _ => unsupported "quantifier in code"

/-- An `if` statement over printed branches: a single short `then` line is
inlined (`if c then break`), an `else` that is itself an `if` chains as
`else if`. -/
partial def ifChain (ct : Format) (t f : Exp) (branch : Exp → PM (List Format)) : PM Format := do
  let tLines ← branch t
  let thenDoc := match tLines with
    | [single] =>
      let s := Format.pretty single
      if s.length < 40 && !(s.contains '\n') then text " then " ++ single
      else text " then" ++ indent 2 (hard ++ single)
    | [] => text " then" ++ indent 2 (hard ++ text "pure ()")
    | _ => text " then" ++ indent 2 (hard ++ vcat tLines)
  let head := text "if " ++ ct ++ thenDoc
  if isUnit f then pure head
  else
    match f.node with
    | .ite c2 t2 f2 =>
      match assertShape c2 t2 f2 with
      | some _ => do
        let fLines ← branch f
        pure (head ++ hard ++ text "else" ++ indent 2 (hard ++ vcat fLines))
      | none =>
        let (ct2, _) ← valueTerm c2
        let rest ← ifChain ct2 t2 f2 branch
        pure (head ++ hard ++ text "else " ++ rest)
    | .sequence [x] =>
      match x.node with
      | .ite _ _ _ => ifChain ct t x branch
      | _ => do
        let fLines ← branch f
        pure (head ++ hard ++ text "else" ++ indent 2 (hard ++ vcat fLines))
    | _ => do
      let fLines ← branch f
      match fLines with
      | [single] =>
        let str := Format.pretty single
        if str.length < 40 && !(str.contains '\n') then pure (head ++ hard ++ text "else " ++ single)
        else pure (head ++ hard ++ text "else" ++ indent 2 (hard ++ single))
      | _ => pure (head ++ hard ++ text "else" ++ indent 2 (hard ++ vcat fLines))

/-- `¬c` as a term: a comparison flips, `!x` drops the negation, anything
else is `!(…)`. -/
partial def negatedTerm (c : Exp) : PM Format := do
  match c.node with
  | .call op _ [a, b] _ =>
    let flipped := match op with
      | .lt => some ">=" | .le => some ">" | .gt => some "<=" | .ge => some "<"
      | .eq => some "!=" | .neq => some "==" | _ => none
    match flipped with
    | some sym =>
      let (lhs, lc) ← valueTerm a
      let (bt, bc) ← valueTerm b
      pure (atom lhs lc ++ text s!" {sym} " ++ atom bt bc)
    | none => do pure (text "!" ++ paren (← valueTerm c).1)
  | .call .not _ [a] _ => do pure (← valueTerm a).1
  | _ => do pure (text "!" ++ paren (← valueTerm c).1)

/-- The `assert!(c, code)` shapes: `if c {} else abort code` and
`if !c abort code else {}`. -/
partial def assertShape (c t f : Exp) : Option (Exp × Exp × Bool) :=
  match t.node, f.node with
  | _, .call (.abort _) _ [code] _ => if isUnit t then some (c, code, false) else none
  | .call (.abort _) _ [code] _, _ => if isUnit f then
      match c.node with
      | .call .not _ [inner] _ => some (inner, code, false)
      | _ => some (c, code, true)
    else none
  | _, _ => none

/-- The loop invariants of a sequence's leading spec block, and the
sequence without them.  Other spec statements of that block stay, to be
reported by `stmt`. -/
partial def stripLeadingInvariants (e : Exp) : List Condition × Exp :=
  match e.node with
  | .sequence (x :: rest) =>
    match x.node with
    | .specBlock spec =>
      let (invariants, others) := spec.conditions.partition fun c => c.kind == .loopInvariant
      if invariants.isEmpty then ([], e)
      else
        let rest := if others.isEmpty then rest else
          (⟨x.ty, x.loc, .specBlock (.mk spec.loc spec.pragmas others spec.frame)⟩ : Exp) :: rest
        let remainder : Exp := match rest with
          | [single] => single
          | _ => ⟨e.ty, e.loc, .sequence rest⟩
        (invariants, remainder)
    | _ => ([], e)
  | _ => ([], e)

/-- The loop invariants at the head of a loop body — the spec block the Move
compiler places at the loop head: before the first statement of a `loop`
body, or in the condition block of a `while` (`while ({spec {…}; c})`) —
and the body without them. -/
partial def splitLoopInvariants (body : Exp) : List Condition × Exp :=
  let (invariants, body) := stripLeadingInvariants body
  if !invariants.isEmpty then (invariants, body) else
  let fromCondition (c t f : Exp) (wrap : Exp → Exp) : List Condition × Exp :=
    let (invariants, c) := stripLeadingInvariants c
    if invariants.isEmpty then ([], body)
    else (invariants, wrap ⟨body.ty, body.loc, .ite c t f⟩)
  match body.node with
  | .ite c t f => fromCondition c t f id
  | .sequence [x] =>
    match x.node with
    | .ite c t f => fromCondition c t f fun e => ⟨body.ty, body.loc, .sequence [e]⟩
    | _ => ([], body)
  | _ => ([], body)

/-- A loop: `while c do …` when the body is `if c { … } else break`, else
`loop`; labeled when an inner `break`/`continue` targets it. -/
partial def loopStmt (body : Exp) : PM Unit := do
  let ctxt ← read
  -- The loop invariants print first, as `invariant P` statements over the
  -- body's locals; one the spec printer cannot express is reported.
  let (invariants, body) := splitLoopInvariants body
  let invariantLines ← invariants.filterMapM fun c => do
    try
      let (t, _) ← withReader (fun cx => { cx with inSpec := true, specInBody := true })
        (specTerm c.exp)
      record .loopInvariant
      pure (some (text "invariant " ++ t))
    catch e =>
      record .loopInvariant
      drop s!"loop invariant in function `{(ctxt.fn.map (·.name)).getD "?"}`: {e}"
      pure none
  let labeled := hasOuterCont 0 body
  let label ← if labeled then some <$> freshName "loop" else pure none
  let labels := ctxt.loopLabels ++ [label]
  let whileShape : Option (Exp × Exp) :=
    match body.node with
    | .ite c t f =>
      match f.node with
      | .loopCont 0 false => some (c, t)
      | _ => none
    | .sequence [x] =>
      match x.node with
      | .ite c t f =>
        match f.node with
        | .loopCont 0 false => some (c, t)
        | _ => none
      | _ => none
    | _ => none
  let withLabels {α : Type} (k : PM α) : PM α := withReader (fun c => { c with loopLabels := labels }) k
  match whileShape, label with
  | some (c, t), none =>
    -- The condition must be pure to be a `while` condition.
    match ← kindOf c with
    | .pure =>
      let condLines ← collect (do let _ ← term c; pure ())
      if condLines.isEmpty then
        let (ct, _) ← term c
        let lines ← withLabels (collect (stmt t))
        let lines := invariantLines ++ (if lines.isEmpty then [text "pure ()"] else lines)
        emit (text "while " ++ ct ++ text " do" ++ indent 2 (hard ++ vcat lines))
      else loopWithTest c t label labels invariantLines
    | .action => loopWithTest c t label labels invariantLines
  | some (c, t), some _ => loopWithTest c t label labels invariantLines
  | none, _ =>
    let lines ← withLabels (collect (stmt body))
    let lines := invariantLines ++ (if lines.isEmpty then [text "pure ()"] else lines)
    let head := match label with
      | some l => s!"loop@{l}"
      | none => "loop"
    emit (text head ++ indent 2 (hard ++ vcat lines))
where
  loopWithTest (c t : Exp) (label : Option String) (labels : List (Option String))
      (invariantLines : List Format) : PM Unit := do
    let lines ← withReader (fun cx => { cx with loopLabels := labels }) <| collect do
      for line in invariantLines do emit line
      let (ct, _) ← valueTerm c
      let brk := match label with
        | some l => s!"break@{l}"
        | none => "break"
      emit (text "if !" ++ paren ct ++ text s!" then {brk}")
      stmt t
    let head := match label with
      | some l => s!"loop@{l}"
      | none => "loop"
    emit (text head ++ indent 2 (hard ++ vcat lines))

/-- The tail of a block whose value is consumed: binds the value to `target`
(`target := value`) if given, else it is the block's result (`pure v` in an
`Action` function, `v` in a pure one). -/
partial def blockTail (e : Exp) (target : Option String) : PM Unit := do
  let ctxt ← read
  match e.node with
  | .block p binding body =>
    letStmt p binding
    blockTail body target
  | .sequence [] => tailValue (text "()") false target
  | .sequence [x] => blockTail x target
  | .sequence (x :: rest) =>
    stmt x
    blockTail ⟨e.ty, e.loc, .sequence rest⟩ target
  | .«return» _ | .loopCont _ _ => stmt e
  | .call (.abort _) _ _ _ => stmt e
  | .ite c t f =>
    match assertShape c t f with
    | some _ => stmt e; tailValue (text "()") false target
    | none =>
      let (ct, _) ← valueTerm c
      emit (← ifChain ct t f (fun x => collect (blockTail x target)))
  | .«match» s arms =>
    let (st, _) ← scrutinee s arms
    let armDocs ← arms.mapM fun a => do
      let p ← pattern a.pattern
      let g ← match a.guard with
        | some g => do pure (text " if " ++ (← term g).1)
        | none => pure (text "")
      let lines ← collect (blockTail a.body target)
      pure (text "| " ++ p ++ g ++ text " =>" ++ indent 2 (hard ++ vcat lines))
    emit (text "match " ++ st ++ text " with" ++ indent 2 (hard ++ vcat armDocs))
  | _ =>
    if isUnit e then tailValue (text "()") false target
    else
      match ← kindOf e with
      | .action =>
        let a ← actionDoc e
        match target with
        | some t => emit (text s!"{t} ← " ++ a)
        | none => emit a
      | .pure =>
        let (t, c) ← term e
        tailValue t c target
where
  tailValue (v : Format) (compound : Bool) (target : Option String) : PM Unit := do
    let ctxt ← read
    let isUnitValue := Format.pretty v == "()"
    match target with
    | some t => emit (text s!"{t} := " ++ v)
    | none =>
      let st ← get
      -- A unit value after a complete statement adds nothing; after a `let`
      -- (or in an empty block) the block still needs its value.
      if isUnitValue && !st.lines.isEmpty && !st.lastWasLet then pure ()
      else if ctxt.inAction then emit (text "pure " ++ (if compound then paren v else v))
      else emit v


/-- A spec term printed in a mathematical-integer context.  Bounded leaves
remain bare: clause elaboration supplies their `Int` interpretation. -/
partial def widenedTerm (asInt : Bool) (x : Exp) : PM Format := do
  let (xt, _) ← withReader (fun c => { c with wideInts := true, wideAsInt := asInt }) (specTerm x)
  pure xt

/-- A spec value for a position of type `target`: a `num` or arithmetic value
(a mathematical number in MSL, assumed in range) narrows to a bounded target
through `MoveInt.ofInt`; anything else prints as is. -/
partial def narrowedTerm (target : Ty) (x : Exp) : PM (Format × Bool) := do
  if isIntTy target && (effTy x == .num || isArith x) && !(isNumLit x) then
    let asInt := x.ty == .num || treeNeedsInt x
    let xt ← widenedTerm asInt x
    pure (paren (text "MoveInt.ofInt " ++ paren xt ++ text " : " ++ (← printTy target)), false)
  else specTerm x

/-- The value of a spec `let` with declared type `ty`: a `num` let is an
`Int` over the mathematical values of its leaves; a bounded let narrows an
arithmetic value back to its type. -/
partial def specLetValue (ty : Ty) (x : Exp) : PM (Format × Option String) := do
  if ty == .num then
    let xt ← widenedTerm true x
    pure (xt, some "Int")
  else
    let (xt, _) ← narrowedTerm ty x
    pure (xt, none)

/-- The spec term printer (Prop/value context). -/
partial def specTerm (e : Exp) : PM (Format × Bool) := do
  let ctxt ← read
  -- A sequence is its value (the model types the sequence at the position's
  -- type, the value keeps its own).
  if let .sequence (_ :: _) := e.node then
    if let some last := (match e.node with | .sequence es => es.getLast? | _ => none) then
      return ← specTerm last
  if ctxt.wideInts && isIntTy (effTy e) && !(isArith e) && !(isNumLit e) then
    withReader (fun cx => { cx with wideInts := false }) (specTermCore e)
  else if ctxt.wideAsInt && isLengthCall e then
    specTermCore e
  else specTermCore e

/-- The spec term printer proper. -/
partial def specTermCore (e : Exp) : PM (Format × Bool) := do
  let ctxt ← read
  -- Binary operators are printed by precedence: an operand is parenthesized
  -- only if it binds looser than the operator (or equally, on the side the
  -- operator does not associate to).
  let binPrec (sym : String) (prec : Nat) (rightAssoc : Bool) (args : List Exp) :
      PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let (lhs, _) ← specTerm a
      let (bt, _) ← specTerm b
      let pa := specPrec a
      let pb := specPrec b
      let lhs := if pa < prec || (pa == prec && rightAssoc) then paren lhs else lhs
      let bt := if pb < prec || (pb == prec && !rightAssoc) then paren bt else bt
      pure (lhs ++ text s!" {sym} " ++ bt, true)
    | _ => unsupported s!"arity of spec `{sym}`"
  let bin (sym : String) (args : List Exp) : PM (Format × Bool) :=
    binPrec sym (opPrec sym) false args
  -- The Move prover types bitwise trees as `num` while their operands retain
  -- the selected bounded integer width.  Specification-function parameters,
  -- however, print as mathematical `Int`, which has shifts but no and/or/xor
  -- operations in Lean.  Reify each operand temporarily at the source width,
  -- perform the exact finite-width operation, and project its result to the
  -- logical `Int` domain again.
  let bitwise (sym : String) (args : List Exp) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      unless isIntTy a.ty && isIntTy b.ty do
        unsupported s!"bitwise `{sym}` over non-bounded integers"
      unless a.ty == b.ty do
        unsupported s!"bitwise `{sym}` over different integer widths"
      let operand (x : Exp) : PM Format := do
        let (xt, xc) ← specTerm x
        pure (paren (text "MoveInt.ofInt " ++ paren (text "Move.Spec.int " ++ atom xt xc) ++
          text " : " ++ (← printTy x.ty)))
      let lhs ← operand a
      let rhs ← operand b
      pure (paren (lhs ++ text s!" {sym} " ++ rhs) ++ text ".toInt", false)
    | _ => unsupported s!"arity of bitwise `{sym}`"
  -- Arithmetic is unbounded in specifications (MSL evaluates every integer
  -- type as `num`).  The clause rewriter selects the mathematical operators;
  -- this printer only preserves the readable surface and precedence.
  let widened := widenedTerm
  let narrowed := narrowedTerm
  let arith (sym : String) (args : List Exp) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let asInt := ctxt.wideAsInt || treeNeedsInt e
      let lhs ← widened asInt a
      let bt ← widened asInt b
      let prec := opPrec sym
      let lhs := if specPrec a < prec then paren lhs else lhs
      let bt := if specPrec b ≤ prec then paren bt else bt
      pure (lhs ++ text s!" {sym} " ++ bt, true)
    | _ => unsupported s!"arity of spec `{sym}`"
  -- A shift is mathematical too; the clause rewriter interprets its amount.
  let shiftArith (sym : String) (args : List Exp) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let asInt := ctxt.wideAsInt || treeNeedsInt e
      let lhs ← widened asInt a
      let (bt, _) ← withReader (fun c => { c with wideInts := true, wideAsInt := false }) (specTerm b)
      let prec := opPrec sym
      let lhs := if specPrec a < prec then paren lhs else lhs
      let bt := if specPrec b ≤ prec then paren bt else bt
      pure (lhs ++ text s!" {sym} " ++ bt, true)
    | _ => unsupported s!"arity of spec `{sym}`"
  let cmp (sym : String) (args : List Exp) : PM (Format × Bool) := do
    match args with
    | [a, b] =>
      let ta := effTy a
      let tb := effTy b
      let isLit (x : Exp) := isNumLit x
      let mixed := (isIntTy ta != isIntTy tb) && !(isLit a || isLit b)
      if isArith a || isArith b || mixed then
        let asInt := ctxt.wideAsInt || treeNeedsInt a || treeNeedsInt b
        let lhs ← widened asInt a
        let bt ← widened asInt b
        let prec := opPrec sym
        let lhs := if specPrec a ≤ prec then paren lhs else lhs
        let bt := if specPrec b ≤ prec then paren bt else bt
        pure (lhs ++ text s!" {sym} " ++ bt, true)
      else bin sym args
    | _ => unsupported s!"arity of spec `{sym}`"
  match e.node with
  | .value v constant =>
    match constant with
    | some n => pure (text (legalize n), false)
    | none => do pure (← printValue v e.ty, false)
  | .«local» n =>
    if ctxt.inStructInvariant && (n == "$self" || n == "self") then pure (text "this", false)
    else if ctxt.specInBody then pure (text (← localPrinted n), false)
    else pure (text (legalize n), false)
  | .param i =>
    if ctxt.inStructInvariant && i == 0 then pure (text "this", false)
    else
      match ctxt.params[i]? with
      | some n => pure (text (legalize n), false)
      | none => unsupported s!"parameter index {i} in spec"
  | .ite c t f =>
    let (ct, _) ← specTerm c
    let (tt, _) ← specTerm t
    let (ft, _) ← specTerm f
    pure (text "if " ++ ct ++ text " then " ++ tt ++ text " else " ++ ft, true)
  | .block p binding body =>
    match binding with
    | some b =>
      let pat ← pattern p
      let (bt, ascription) ← specLetValue p.ty b
      let (bodyT, _) ← specTerm body
      let typed := match ascription with
        | some t => text s!" : {t}"
        | none => text ""
      pure (text "let " ++ pat ++ typed ++ text " := " ++ bt ++ text "; " ++ bodyT, true)
    | none => specTerm body
  | .quant kind ranges _ cond body =>
    let binders ← ranges.mapM fun r => do
      let names := patternNames r.pattern
      let name := names.head?.getD "_"
      match r.domain.node with
      | .call .typeDomain _ [] _ =>
        match r.domain.ty with
        | .typeDomain t => do pure (text (legalize name) ++ text " : " ++ (← printSpecTy t), none)
        | _ => pure (text (legalize name), none)
      | .call .range _ [lo, hi] _ =>
        -- `x in lo..hi` ranges over `num`: an `Int`, with the bounds over the
        -- mathematical values of their leaves.
        let lt ← widenedTerm true lo
        let ht ← widenedTerm true hi
        pure (paren (text (legalize name) ++ text " : Int"),
          some (lt ++ text " ≤ " ++ text (legalize name) ++ text " ∧ " ++
            text (legalize name) ++ text " < " ++ ht))
      | _ =>
        -- `forall x in v`: logical membership in the vector.
        let (dt, dc) ← specTerm r.domain
        pure (text (legalize name) ++ text " ∈ " ++ atom dt dc, none)
    let (bt, _) ← specTerm body
    let guard ← match cond with
      | some c => do pure (some (← specTerm c).1)
      | none => pure none
    let guards := binders.filterMap (·.2) ++ (guard.map ([·]) |>.getD [])
    let bindersDoc := sepBy " " (binders.map (·.1))
    let connective := match kind with
      | .«forall» => " → "
      | .«exists» => " ∧ "
      | _ => " → "
    let body := if guards.isEmpty then bt
      else sepBy connective (guards ++ [bt])
    match kind with
    | .«forall» => pure (text "∀ " ++ bindersDoc ++ text ", " ++ body, true)
    | .«exists» => pure (text "∃ " ++ bindersDoc ++ text ", " ++ body, true)
    | _ => unsupported "choose quantifier"
  | .call op inst args _ =>
    match op with
    | .add => arith "+" args
    | .sub => arith "-" args
    | .mul => arith "*" args
    | .div => arith "/" args
    | .mod => arith "%" args
    | .bitOr => bitwise "|||" args
    | .bitAnd => bitwise "&&&" args
    | .xor => bitwise "^^^" args
    | .shl => shiftArith "<<<" args
    | .shr => shiftArith ">>>" args
    | .and => bin "∧" args
    | .or => bin "∨" args
    | .implies => binPrec "→" (opPrec "→") true args
    | .iff => bin "↔" args
    | .eq | .identical => cmp "=" args
    | .neq => cmp "≠" args
    | .lt => cmp "<" args
    | .gt => cmp ">" args
    | .le => cmp "≤" args
    | .ge => cmp "≥" args
    | .not =>
      match args with
      | [a] => do
        let (lhs, _) ← specTerm a
        pure (text "¬" ++ (if specPrec a < 100 then paren lhs else lhs), false)
      | _ => unsupported "arity of `!`"
    | .negate =>
      match args with
      | [a] => do
        let (lhs, _) ← specTerm a
        pure (text "-" ++ (if specPrec a < 100 then paren lhs else lhs), false)
      | _ => unsupported "arity"
    | .cast =>
      match args with
      | [a] =>
        -- The specification elaborator interprets every Move integer type in
        -- the mathematical `Int` domain, so integer casts are identities here.
        -- Concrete fields and values are reified later at their type boundary.
        specTerm a
      | _ => unsupported "arity of cast"
    | .copy | .move | .freeze _ | .trace _ | .noOp =>
      match args with
      | [a] => specTerm a
      | _ => unsupported "arity"
    | .old =>
      match args with
      | [a] => do
        let (lhs, _) ← withReader (fun c => { c with preState := false }) (specTerm a)
        pure (text "old(" ++ lhs ++ text ")", false)
      | _ => unsupported "arity of old"
    | .behavior _ _ =>
      unsupported "function-value behavior predicate in the legacy Lean backend"
    | .withStateAnchor label =>
      match args with
      | [anchored] =>
        match anchored.node with
        | .call .old _ [value] _ => do
          let (captured, capturedCompound) ← withReader
            (fun c => { c with preState := false }) (specTerm value)
          pure (text s!"oldAt[{label}](" ++ captured ++ text ")", capturedCompound)
        | _ => unsupported "state anchor without an `old` observation"
      | _ => unsupported "arity of state anchor"
    | .global label =>
      match args with
      | [addr] =>
        let (lhs, _) ← specTerm addr
        let r ← resourceHead inst
        let place := r ++ text "[" ++ lhs ++ text "]"
        match label with
        | some _ => pure (text "old(" ++ place ++ text ")", false)
        | none =>
          if ctxt.preState then pure (text "old(" ++ place ++ text ")", false)
          else pure (place, false)
      | _ => unsupported "arity of global"
    | .«exists» label =>
      match args with
      | [addr] =>
        let (lhs, _) ← specTerm addr
        let r ← match inst with
          | [.struct name []] => pure (text (qualify ctxt.mod.ref name))
          | [t] => printTy t
          | _ => unsupported "exists instantiation"
        let doc := text "existsAt<" ++ r ++ text ">(" ++ lhs ++ text ")"
        match label with
        | some _ => pure (text "old(" ++ doc ++ text ")", false)
        | none => pure (doc, false)
      | _ => unsupported "arity of exists"
    | .select _ field =>
      match args with
      | [inner] =>
        if ctxt.preState && isGlobalPlace inner then
          -- `old(R[a].f.g)`: the whole place chain under one `old`.
          let (it, _) ← withReader (fun c => { c with preState := false }) (specTerm inner)
          pure (text "old(" ++ it ++ text "." ++ text (legalize field) ++ text ")", false)
        else
          let (it, ic) ← specTerm inner
          if ctxt.inStructInvariant && Format.pretty it == "this" then
            pure (text "." ++ text (legalize field), false)
          else pure (atom it ic ++ text "." ++ text (legalize field), false)
      | [] =>
        -- A bare field in a struct invariant: the field of the value itself.
        if ctxt.inStructInvariant then pure (text "." ++ text (legalize field), false)
        else unsupported "field selection without an owner"
      | _ => unsupported "arity of field selection"
    | .result i =>
      -- `result` or the i-th component of a tuple result.
      match ctxt.fn with
      | some f =>
        match f.result with
        | .tuple ts =>
          if ts.length ≤ 1 then pure (text "result", false)
          else
            -- nested pair projections: result.1, result.2.1, ...
            let rec proj (k : Nat) (last : Bool) : String :=
              match k with
              | 0 => if last then "" else ".1"
              | k + 1 => ".2" ++ proj k last
            let isLast := i + 1 == ts.length
            pure (text ("result" ++ proj i isLast), false)
        | _ => pure (text "result", false)
      | none => pure (text "result", false)
    | .len =>
      match args with
      | [v] => do let (vt, vc) ← specTerm v; pure (atom vt vc ++ text ".length", false)
      | _ => unsupported "arity of len"
    | .index =>
      match args with
      | [v, i] => do
        let (vt, vc) ← specTerm v
        let (it, _) ← specTerm i
        pure (atom vt vc ++ text "[" ++ it ++ text "]!", false)
      | _ => unsupported "arity of index"
    | .containsVec =>
      match args with
      | [v, x] => do
        let (vt, vc) ← specTerm v
        let (xt, xc) ← specTerm x
        pure (atom xt xc ++ text " ∈ " ++ atom vt vc, true)
      | _ => unsupported "arity of contains"
    | .inRangeVec =>
      match args with
      | [v, i] => do
        let (vt, vc) ← specTerm v
        let (it, ic) ← specTerm i
        pure (atom it ic ++ text " < " ++ atom vt vc ++ text ".length", true)
      | _ => unsupported "arity of in_range"
    | .inRangeRange =>
      match args with
      | [r, i] =>
        match r.node with
        | .call .range _ [lo, hi] _ => do
          let (lt, _) ← specTerm lo
          let (ht, _) ← specTerm hi
          let (it, _) ← specTerm i
          pure (lt ++ text " ≤ " ++ it ++ text " ∧ " ++ it ++ text " < " ++ ht, true)
        | _ => unsupported "in_range over a non-literal range"
      | _ => unsupported "arity of in_range"
    | .updateVec =>
      match args with
      | [v, i, x] => do
        let (vt, vc) ← specTerm v
        let (it, _) ← specTerm i
        let (xt, xc) ← specTerm x
        pure (app (text "Move.Spec.vectorSet") [atom vt vc, it, atom xt xc], true)
      | _ => unsupported "arity of update"
    | .concatVec => do
      match args with
      | [a, b] => do
        let (lhs, lc) ← specTerm a
        let (bt, bc) ← specTerm b
        pure (app (text "Move.Spec.vectorAppend") [atom lhs lc, atom bt bc], true)
      | _ => unsupported "arity of concat"
    | .emptyVec => pure (text "[]", false)
    | .singleVec =>
      match args with
      | [x] => do let (xt, _) ← specTerm x; pure (text "[" ++ xt ++ text "]", false)
      | _ => unsupported "arity of vec"
    | .maxU8 => pure (text "255", false)
    | .maxU16 => pure (text "65535", false)
    | .maxU32 => pure (text "4294967295", false)
    | .maxU64 => pure (text "18446744073709551615", false)
    | .maxU128 => pure (text "340282366920938463463374607431768211455", false)
    | .maxU256 => pure (text "115792089237316195423570985008687907853269984665640564039457584007913129639935", false)
    | .int2Bv | .bv2Int =>
      -- These operations switch between the prover's integer and bit-vector
      -- encodings without changing the source-level value.  Lean represents
      -- that value as Nat/Int until a surrounding cast or declaration result
      -- reifies it at a bounded Move integer type, so the conversion disappears.
      match args with
      | [a] => specTerm a
      | _ => unsupported s!"arity of {repr op}"
    | .specFunction name _ =>
      -- A companion (`$f`, the specification version of the Move function
      -- `f`) is applied as `f`: Leaner derives that meaning of `f` in
      -- specifications from the function's own body.  `std::vector`'s
      -- operations have curated renderings (the module is Leaner's).
      if let some base := companionFunction? name.name then
        if name.module == ctxt.mod.ref && (← get).failed.contains base then
          unsupported s!"uses `{base}`, which was not transpiled"
        if let some rendering ← curatedSpecCall? e { name with name := base } args then
          return rendering
      else if name.module == ctxt.mod.ref && (← get).failed.contains name.name then
        unsupported s!"uses `{name.name}`, which was not transpiled"
      let specParamTys : List Ty := match ctxt.pkg.find? name.module with
        | some m => ((m.specFuns.find? fun (sf : SpecFun) => sf.name == name.name).map fun sf => sf.params.map fun (p : Param) => p.ty).getD []
        | none => []
      let fs ← (args.zipIdx).mapM fun (a, i) => do
        match specParamTys[i]? with
        | some pty =>
          if pty == .num || isIntTy pty then
            -- Integer parameters of specification functions are declared in
            -- the logical `Int` domain.  The application rewrite inserts the
            -- projection, leaving the emitted call conversion-free.
            do let (t, c) ← specTerm a; pure (atom t c)
          else
            do let (t, c) ← narrowed pty a; pure (atom t c)
        | none => do let (t, c) ← specTerm a; pure (atom t c)
      let head ← specFunHead name
      pure (app (text head) fs, !fs.isEmpty)
    | .moveFunction name =>
      -- A Move function the spec rewriter left in place (it resolves the
      -- calls of specifications to companions, so this is a curated
      -- function, or a call the rewriter does not reach).
      if let some rendering ← curatedSpecCall? e name args then
        return rendering
      unsupported s!"Move function `{name.name}` applied in a specification outside the spec rewriter's reach"
    | .pack name variant =>
      -- Values in specs: same spelling as in code (pure); a widened (`num`
      -- or arithmetic) value lands in a bounded field through `MoveInt.ofInt`.
      let fieldTys : List Ty := match variant, ctxt.pkg.struct? name with
        | none, some s => s.fields.map (·.ty)
        | some v, some s => ((s.variants.getD []).find? (·.name == v)).map (·.fields.map (·.ty)) |>.getD []
        | _, _ => []
      let fs ← (args.zipIdx).mapM fun (a, i) => do
        match fieldTys[i]?, a.node with
        | some (.vector _), .call (.vector) _ _ _ =>
          -- A vector literal in a field: the vector value, not its list view.
          term a
        | some fty, _ => narrowed fty a
        | none, _ => specTerm a
      let head := qualify ctxt.mod.ref name
      match variant with
      | some v => pure (app (text s!"{head}.{legalize v}") (fs.map fun (f, c) => atom f c), !fs.isEmpty)
      | none =>
        match ctxt.pkg.struct? name with
        | some s =>
          if isPositionalFields s.fields then
            pure (app (text s!"{head}.mk") (fs.map fun (f, c) => atom f c), !fs.isEmpty)
          else
            let inits := (s.fields.zip fs).map fun (fld, (f, _)) =>
              text (legalize fld.name) ++ text " := " ++ f
            pure (paren (text "{ " ++ sepBy ", " inits ++ text " } : " ++ (← printTy e.ty)), false)
        | none => unsupported "struct of unknown module in spec"
    | .updateField _ field =>
      match args with
      | [s, x] => do
        let (st, _) ← specTerm s
        let (xt, _) ← specTerm x
        pure (text "{ " ++ st ++ text " with " ++ text (legalize field) ++ text " := " ++ xt ++ text " }", false)
      | _ => unsupported "arity of update_field"
    | .tuple =>
      let fs ← args.mapM fun a => do pure (← specTerm a).1
      if fs.isEmpty then pure (text "()", false) else pure (paren (sepBy ", " fs), false)
    | .vector =>
      -- A vector literal in a specification is a `Move.Vector` value, as in
      -- code (vector-typed specification values are `Move.Vector`s; the
      -- list view is taken where a list operation is applied).
      let fs ← args.mapM fun a => do pure (← specTerm a).1
      if fs.isEmpty then
        if ctxt.suppressAscription then pure (text "vector![]", false)
        else do pure (paren (text "vector![] : " ++ (← printTy e.ty)), false)
      else pure (text "vector![" ++ sepBy ", " fs ++ text "]", false)
    | .testVariants name variants =>
      match args with
      | [inner] =>
        let (it, ic) ← specTerm inner
        let head := qualify ctxt.mod.ref name
        pure (sepBy " ∨ " (variants.map fun v => atom it ic ++ text s!" is {head}.{legalize v}"), true)
      | _ => unsupported "arity of `is`"
    | .selectVariants _ fields =>
      -- `x.f` on an enum value: Leaner's field selection on an enum value
      -- is the field of whichever variant has it, an unspecified value
      -- otherwise — the Move Prover's reading (`translate_select_variant`).
      match args with
      | [inner] =>
        let some field := variantFieldName? fields
          | unsupported "variant field selection across differently named fields"
        let (it, ic) ← specTerm inner
        pure (atom it ic ++ text "." ++ text (legalize field), false)
      | _ => unsupported "arity of variant field selection"
    | .abort _ =>
      -- An abort in a specification expression: an unspecified value, fixed
      -- per site (the Move Prover's `$Arbitrary_value_of`).
      pure (text s!"Move.Spec.arbitrary _ {e.loc.start}", true)
    | .deref | .borrow _ | .borrowGlobal _ =>
      -- References are erased in specs.
      match args with
      | [a] => specTerm a
      | _ => unsupported "arity"
    | .abortCode => pure (text "abortCode", false)
    | .typeValue | .typeDomain | .resourceDomain | .stateDomain | .canModify | .slice | .range
    | .indexOfVec | .rangeVec | .abortFlag | .wellFormed | .boxValue
    | .unboxValue | .emptyEventStore | .extendEventStore | .eventStoreIncludes
    | .eventStoreIncludedIn | .saveStateAnchor _ | .foldsCaptureAnchor _
    | .inlineCallSummary | .specPublish _ | .specRemove _ | .specUpdate _
    | .moveTo | .moveFrom =>
      unsupported s!"spec operation {repr op}"
  | .«match» s arms =>
    let (st, _) ← specTerm s
    let armDocs ← arms.mapM fun a => do
      let p ← pattern a.pattern
      let (b, _) ← specTerm a.body
      pure (text "| " ++ p ++ text " => " ++ b)
    pure (text "match " ++ st ++ text " with" ++ indent 2 (hard ++ vcat armDocs), true)
  | .sequence [x] => specTerm x
  | _ => unsupported "expression form in spec"
where
  -- A spec function under its Move name; a companion `$f` as `f`, the Move
  -- function it is the specification version of (Leaner resolves `f` in a
  -- specification to the version derived from `f`'s body).
  specFunHead (name : QualifiedName) : PM String := do
    let ctxt ← read
    match companionFunction? name.name with
    | some base => pure (qualify ctxt.mod.ref { name with name := base })
    | none => pure (qualify ctxt.mod.ref name)
  -- The curated renderings of Move functions in specifications.  Scoped
  -- logical vector instances keep the list representation out of the text.
  curatedSpecCall? (e : Exp) (name : QualifiedName) (args : List Exp) : PM (Option (Format × Bool)) := do
    let ctxt ← read
    if let some vop := vectorCall? (.moveFunction name) then
      match vop, args with
      | .length, [v] => do
        let (vt, vc) ← specTerm v
        return some (atom vt vc ++ text ".length", false)
      | .isEmpty, [v] => do
        let (vt, vc) ← specTerm v
        return some (atom vt vc ++ text ".isEmpty", false)
      | .contains, [v, x] => do
        let (vt, vc) ← specTerm v
        let (xt, xc) ← specTerm x
        return some (atom xt xc ++ text " ∈ " ++ atom vt vc, true)
      | .borrow, [v, i] => do
        let (vt, vc) ← specTerm v
        let (it, _) ← specTerm i
        return some (atom vt vc ++ text "[" ++ it ++ text "]!", false)
      -- Vector values (`vector[]`, `vector[x]`) are `Move.Vector`s, as in
      -- code; the list view is taken where a list operation is applied.
      | .empty, [] =>
        if ctxt.suppressAscription then return some (text "vector![]", false)
        else return some (paren (text "vector![] : " ++ (← printTy (derefTy e.ty))), false)
      | .singleton, [x] => do
        let (xt, _) ← specTerm x
        return some (text "vector![" ++ xt ++ text "]", false)
      | _, _ => unsupported s!"vector operation {repr vop} in a specification"
    return none

end

-- -------------------------------------------------------------------------------------------------
-- Declarations

def docComment (doc : String) : List Format :=
  if doc.trim.isEmpty then []
  else
    let body := doc.trim
    [text ("/-- " ++ body ++ " -/")]

def moduleDocComment (doc : String) : List Format :=
  if doc.trim.isEmpty then [] else [text ("/-! " ++ doc.trim ++ " -/")]

/-- Move attributes in Leaner's attribute grammar. -/
partial def attributeDoc (a : Attribute) : PM (Option Format) := do
  match a with
  | .apply name args =>
    let name := name.replace "::" "."
    if name == "test" || name == "test_only" || name == "verify_only" then pure none
    else
      let argDocs ← args.filterMapM attributeArg
      pure (some (app (text name) argDocs))
  | .assign name value =>
    let name := name.replace "::" "."
    match ← attributeValueDoc value with
    | some v => pure (some (paren (text name ++ text " " ++ v)))
    | none => pure none
where
  attributeArg (a : Attribute) : PM (Option Format) := do
    match a with
    | .apply name args =>
      let name := name.replace "::" "."
      let argDocs ← args.filterMapM attributeArg
      if argDocs.isEmpty then pure (some (text name)) else pure (some (paren (app (text name) argDocs)))
    | .assign name value =>
      match ← attributeValueDoc value with
      | some v => pure (some (paren (text (name.replace "::" ".") ++ text " " ++ v)))
      | none => pure none
  attributeValueDoc (v : AttributeValue) : PM (Option Format) := do
    match v with
    | .value (.number n) => pure (some (text (toString n)))
    | .value (.bool b) => pure (some (text (if b then "true" else "false")))
    | .value _ => do drop "attribute value of unsupported kind"; pure none
    | .name module n =>
      match module with
      | some m => pure (some (text (legalize m.name ++ "." ++ legalize n)))
      | none => pure (some (text (legalize n)))

def attributesDoc (attrs : List Attribute) : PM (List Format) := do
  let docs ← attrs.filterMapM attributeDoc
  if docs.isEmpty then pure [] else pure [text "@[" ++ sepBy ", " docs ++ text "]"]

def abilitiesDoc (abilities : List Ability) : String :=
  if abilities.isEmpty then ""
  else " has " ++ ", ".intercalate (abilities.map fun
    | .copy => "Copy" | .drop => "Drop" | .store => "Store" | .key => "Key")

def typeParamsDoc (tps : List TypeParam) (binderOpen binderClose : String) : String :=
  String.join <| tps.map fun tp =>
    let bounds := if tp.abilities.isEmpty then "" else " : " ++ ", ".intercalate (tp.abilities.map fun
      | .copy => "Copy" | .drop => "Drop" | .store => "Store" | .key => "Key")
    " " ++ binderOpen ++ legalize tp.name ++ bounds ++ binderClose

/-- A struct or enum declaration, with its data invariant. -/
def structDecl (s : Struct) : PM (List Format) :=
  withReader (fun c => { c with tyParams := s.typeParams.map (·.name) |>.toArray }) do
  let attrs ← attributesDoc s.attributes
  let tps := typeParamsDoc s.typeParams "(" ")"
  let header ← match s.variants with
    | none =>
      if isPositionalFields s.fields then
        let tys ← s.fields.mapM fun f => printTy f.ty
        pure [text (s!"struct {legalize s.name}{tps}") ++ paren (sepBy ", " tys) ++
          text (abilitiesDoc s.abilities)]
      else
        let fields ← s.fields.mapM fun f => do
          let d := docComment f.doc
          pure (d ++ [text (legalize f.name ++ " : ") ++ (← printTy f.ty)])
        pure [block (text s!"struct {legalize s.name}{tps}{abilitiesDoc s.abilities} where") fields.flatten]
    | some variants =>
      let vs ← variants.mapM fun v => do
        let fs ← v.fields.mapM fun f => do
          pure (paren (text (legalize f.name ++ " : ") ++ (← printTy f.ty)))
        pure (text ("| " ++ legalize v.name) ++ (if fs.isEmpty then text "" else text " " ++ sepBy " " fs))
      pure [block (text s!"enum {legalize s.name}{tps}{abilitiesDoc s.abilities} where") vs]
  -- Data invariants.
  let invariants ← s.spec.conditions.filterMapM fun c => do
    match c.kind with
    | .structInvariant =>
      let (t, _) ← withReader (fun cx => { cx with inStructInvariant := true, inSpec := true, fn := none, params := #[] }) (specTerm c.exp)
      pure (some (text "invariant " ++ t))
    | _ => do drop s!"struct `{s.name}` spec condition {repr c.kind}"; pure none
  for p in s.spec.pragmas do drop s!"struct `{s.name}` pragma {p.name}"
  let invDoc := if invariants.isEmpty then [] else
    let clauses := invariants.zipIdx.map fun (inv, i) =>
      if i + 1 == invariants.length then inv else inv ++ text ";"
    [block (text s!"spec {legalize s.name}{typeParamsDoc s.typeParams "{" "}"} where") clauses]
  pure (docComment s.doc ++ attrs ++ header ++ invDoc)

def constantDecl (c : Constant) : PM (List Format) := do
  let v ← printValue c.value c.ty
  pure (docComment c.doc ++ [text (s!"def {legalize c.name} : ") ++ (← printTy c.ty) ++ text " := " ++ v])

/-- The function header keyword sequence. -/
def functionKeywords (f : Function) (recursive : Bool) : String :=
  let vis := match f.visibility with
    | .«public» => if f.isEntry then "" else "public "
    | .friend => "friend "
    | .package => "package "
    | .«private» => ""
  let kind := match f.kind with
    | .native => "native "
    | .inlineRetained => "inline "
    | .regular => ""
  let entry := if f.isEntry then "entry " else ""
  let partialKw := if recursive then "partial " else ""
  -- Leaner: modifiers [public] [partial] [entry | friend] fun; `package`/`native` are item heads.
  if f.kind == .native then s!"{vis}{kind}fun"
  else if f.visibility == .package then s!"{partialKw}package fun"
  else s!"{vis}{partialKw}{kind}{entry}{if f.visibility == .friend then "" else ""}fun"

/-- Binders of a function signature; `fun` type parameters carry no ability
bounds in Leaner. -/
def paramBinders (f : Function) : PM Format := do
  let tps := typeParamsDoc (f.typeParams.map fun tp => { tp with abilities := [] }) "{" "}"
  let ps ← f.params.mapM fun p => do
    pure (paren (text (legalize p.name ++ " : ") ++ (← printTy p.ty)))
  pure (text tps ++ (if ps.isEmpty then text "" else text " " ++ sepBy " " ps))

/-- The `modifies` clause per the design's rendering rule. -/
def modifiesClause (f : Function) (facts : Facts) : PM (Option Format) := do
  let isOpaque := pragmaTrue f.pragmas "opaque"
  let frame := f.spec.frame
  let targets ← match frame with
    | some fr => fr.modifies.mapM fun t => do
      match t.node with
      | .call (.global _) inst [addr] _ =>
        let (lhs, _) ← withReader (fun c => { c with inSpec := true }) (specTerm addr)
        pure ((← resourceHead inst) ++ text "[" ++ lhs ++ text "]")
      | _ => unsupported "modifies target"
    | none => pure []
  let modifiesAll := frame.map (·.modifiesAll) |>.getD false
  if modifiesAll then pure (some (text "modifies *"))
  else if targets.isEmpty then
    if facts.writesGlobal then pure (some (text "modifies *"))
    else pure none
  else if isOpaque then pure (some (text "modifies " ++ sepBy ", " targets))
  else pure (some (text "modifies " ++ sepBy ", " (targets ++ [text "*"])))

/-- The header of a function's `spec`: binders follow Leaner's asymmetry
(immutable references become values, except `&signer`). -/
def specHeader (f : Function) : PM Format := do
  let tps := typeParamsDoc (f.typeParams.map fun tp => { tp with abilities := [] }) "{" "}"
  let ps ← f.params.mapM fun p => do
    let ty := match p.ty with
      | .reference false .signer => Ty.reference false .signer
      | .reference false t => t
      | t => t
    pure (paren (text (legalize p.name ++ " : ") ++ (← printTy ty)))
  pure (text s!"spec {legalize f.name}{tps}" ++ (if ps.isEmpty then text "" else text " " ++ sepBy " " ps) ++ text " where")

/-- The `spec f` block, if the function has one worth printing. -/
def specDecl (f : Function) (facts : Facts) : PM (List Format) := do
  -- Inline helpers are compile-time-only: Leaner inlines their bodies into a
  -- deployable caller before compilation and source-spec derivation. A
  -- standalone contract would instead try to assign runtime semantics to the
  -- function-valued parameters which bytecode never sees.
  if f.kind == .inlineRetained then return []
  -- `[abstract]` conditions describe the opaque abstraction, `[concrete]`
  -- ones the body.  A function with a body is verified against, and
  -- summarized by, the concrete contract; a native has only its abstraction.
  let isNative := f.body.isNone
  let (droppedConds, conds) := f.spec.conditions.partition fun c =>
    c.properties.any fun p => p.name == (if isNative then "concrete" else "abstract")
  for c in droppedConds do
    drop s!"function `{f.name}` [{if isNative then "concrete" else "abstract"}] {repr c.kind} condition"
  -- A native without a specification of its own: the intrinsic model of the
  -- prover's prelude, when there is one (an uninterpreted spec function with
  -- its axioms, and the native's contract relating the call to it).
  if isNative && conds.isEmpty then
    let ctxt ← read
    match Intrinsics.model? { module := ctxt.mod.ref, name := f.name } with
    | some model =>
      for a in model.axioms do
        modify fun s => { s with report := { s.report with axioms := s.report.axioms ++ [s!"`{a}` (intrinsic model of `{f.name}`)"] } }
      let header ← specHeader f
      let clauseDocs := model.clauses.map text
      let clauseLines := (clauseDocs.zipIdx).map fun (c, i) =>
        if i + 1 == clauseDocs.length then c else c ++ text ";"
      -- A native has no body to verify against the model.
      return (model.decls.map text) ++ (if model.decls.isEmpty then [] else [text ""]) ++
        [header ++ indent 2 (hard ++ vcat clauseLines)]
    | none => pure ()
  -- A function without conditions or pragmas has no `spec`: Leaner derives
  -- its semantics from the body, for callers in any module, exactly when
  -- there is no `spec` block -- a dummy `ensures true` would replace that
  -- derivation by an uninterpreted contract.  A native without conditions
  -- has its intrinsic model above, or nothing.
  let hasSemantics := !conds.isEmpty || pragmaTrue f.pragmas "aborts_if_is_strict" ||
    pragmaTrue f.pragmas "aborts_if_is_partial"
  if !hasSemantics then pure [] else do
  let specCtx {α : Type} (k : PM α) : PM α :=
    withReader (fun c => { c with inSpec := true }) k
  -- Spec `let`s: prefixed to the clauses that mention them.
  let lets := conds.filterMap fun c => match c.kind with
    | .letPre n => some (n, c.exp, false)
    | .letPost n => some (n, c.exp, true)
    | _ => none
  let withLets (e : Exp) (post : Bool) : PM Format := do
    -- A boolean literal condition is the proposition.
    if let .value (.bool b) _ := e.node then
      return text (if b then "True" else "False")
    let (t, _) ← specCtx (specTerm e)
    let t := if specPrec e < 35 then paren t else t
    -- The lets the condition mentions, and transitively the lets those
    -- mention, in declaration order.
    let rec closure (needed : List String) (fuel : Nat) : List String :=
      match fuel with
      | 0 => needed
      | fuel + 1 =>
        let more := lets.filter fun (n, v, _) =>
          !needed.contains n && needed.any fun m => (lets.find? (·.1 == m)).any fun (_, mv, _) => mentions n none mv
        if more.isEmpty then needed else closure (needed ++ more.map (·.1)) fuel
    let direct := lets.filter (fun (n, _, isPost) => (!isPost || post) && mentions n none e) |>.map (·.1)
    let needed := closure direct lets.length
    let used := lets.filter fun (n, _, isPost) => (!isPost || post) && needed.contains n
    let prefixes ← used.mapM fun (n, v, _) => do
      let (vt, ascription) ← specCtx (specLetValue v.ty v)
      let typed := match ascription with
        | some t => s!" : {t}"
        | none => ""
      pure (text s!"let {legalize n}{typed} := " ++ vt ++ text "; ")
    pure (prefixes.foldl (· ++ ·) (text "") ++ t)
  let mut clauses : List Format := []
  -- Pragmas.  `aborts_if_is_partial` is transported as written.  A pure
  -- function whose spec declares no abort clause (and is not strict) gets the
  -- same pragma: it states the production default — abort behavior
  -- uninterpreted — and selects Leaner's relational contract shape, whose
  -- value-contract alternative reads aborting executions with wrapping values.
  let hasAbortClause := conds.any fun c => match c.kind with
    | .abortsIf | .abortsWith => true
    | _ => false
  let isStrict := pragmaTrue f.pragmas "aborts_if_is_strict"
  if pragmaTrue f.pragmas "aborts_if_is_partial" ||
      (!facts.isAction && !hasAbortClause && !isStrict) then
    clauses := clauses ++ [text "pragma aborts_if_is_partial"]
  -- `pragma opaque`: Leaner summarizes the function by its contract for its
  -- callers (and still verifies the body when it can).
  if pragmaTrue f.pragmas "opaque" then
    clauses := clauses ++ [text "pragma opaque"]
  for p in f.pragmas do
    unless ["aborts_if_is_partial", "aborts_if_is_strict", "opaque", "verify", "intrinsic"].contains p.name do
      drop s!"function `{f.name}` pragma {p.name}"
  -- requires
  let reqs ← conds.filterMapM fun c => match c.kind with
    | .requires => some <$> withLets c.exp false
    | _ => pure none
  if !reqs.isEmpty then
    clauses := clauses ++ [text "requires " ++ sepBy " ∧ " reqs]
  -- modifies
  match ← modifiesClause f facts with
  | some m => clauses := clauses ++ [m]
  | none => pure ()
  -- ensures
  let ens ← conds.filterMapM fun c => match c.kind with
    | .ensures => some <$> withLets c.exp true
    | _ => pure none
  clauses := clauses ++ [text "ensures " ++ (if ens.isEmpty then text "True" else sepBy " ∧ " ens)]
  -- aborts_if
  let mut aborts : List Format := []
  for c in conds do
    match c.kind with
    | .abortsIf =>
      let t ← withReader (fun cx => { cx with preState := true }) (withLets c.exp false)
      match c.abortCode with
      | some code =>
        let (ct, _) ← specCtx (specTerm code)
        aborts := aborts ++ [text "aborts_if " ++ t ++ text " with " ++ ct]
      | none => aborts := aborts ++ [text "aborts_if " ++ t]
    | .abortsWith => drop s!"function `{f.name}` aborts_with"
    | .emits => drop s!"function `{f.name}` emits"
    | .requires | .ensures | .letPre _ | .letPost _ => pure ()
    | k => drop s!"function `{f.name}` spec condition {repr k}"
  if aborts.isEmpty && pragmaTrue f.pragmas "aborts_if_is_strict" then
    aborts := [text "aborts_if False"]
  clauses := clauses ++ aborts
  let header ← specHeader f
  let body := clauses.zipIdx.map fun (c, i) => if i + 1 == clauses.length then c else c ++ text ";"
  -- `verify f`: the automatic proof of the contract against the body; a
  -- native has no body (its contract is assumed, as the summary).
  let verifyCmd := if (← read).emitVerify && !isNative then
      [text "", text s!"verify {legalize f.name}"]
    else []
  pure ([block header body] ++ verifyCmd)

/-- A function declaration with its spec. -/
def functionDecl (f : Function) (recursive : Bool) : PM (List Format) := do
  if f.kind != .inlineRetained &&
      (f.params.any (containsFunctionTy ·.ty) || containsFunctionTy f.result) then
    unsupported "function-typed parameters/results outside a retained inline helper"
  let ctxt ← read
  let facts := ctxt.effects.get { module := ctxt.mod.ref, name := f.name }
  let params := f.params.map (·.name) |>.toArray
  -- Parameters that are reassigned (`Assign` to a `Var` with the parameter's
  -- name, unless a local of that name shadows the parameter) or mutably
  -- borrowed are rebound as mutable locals under a fresh name.
  let bound := f.body.map boundLocals |>.getD []
  let assignedParams := (f.body.map mutableLocals |>.getD []).filter fun n =>
    params.contains n && !bound.contains n
  let borrowedParams := (f.body.map mutableParams |>.getD []).filterMap fun i => params[i]?
  let rebound := (assignedParams ++ borrowedParams).eraseDups
  let inner : PM (List Format) := do
    let attrs ← attributesDoc f.attributes
    let binders ← paramBinders f
    let resultTy ← printTy f.result
    let actionNm ← actionName
    let result := if facts.isAction then text (actionNm ++ " ") ++ (match f.result with
      | .tuple [] => text "Unit"
      | .struct _ (_ :: _) | .vector _ | .function _ _ _ | .reference _ _ | .tuple _ => paren resultTy
      | _ => resultTy) else resultTy
    let keywords := functionKeywords f recursive
    let header := text keywords ++ text " " ++ text (legalize f.name) ++ binders ++ text " : " ++ result
    match f.body with
    | none =>
      pure (docComment f.doc ++ attrs ++ [header])
    | some body =>
      -- A pure single expression prints as a term; anything with statements
      -- (or an `Action` function) as a `do` block.
      let statementLike ← isStatementLike body
      let pureTerm ← if !facts.isAction && rebound.isEmpty && !statementLike then
          let saved ← get
          let (t, _) ← term body
          let st ← get
          if st.lines.isEmpty then pure (some t)
          else do set saved; pure none
        else pure none
      let bodyDoc ← match pureTerm with
        | some t => pure (indent 2 (Format.group (Format.line ++ t)) )
        | none =>
          let lines ← collect do
            for p in rebound do
              emit (text s!"let mut {legalize (p ++ "'")} := {legalize p}")
            blockTail body none
          pure (text " " ++ doBlock lines)
      let fnDoc := header ++ text " :=" ++ bodyDoc
      pure (docComment f.doc ++ attrs ++ [fnDoc])
  withReader (functionCtx f facts) inner
where
  /-- The printing context of a function's body: its parameters (a rebound
  parameter under its fresh name), mutable locals, effect, type parameters. -/
  functionCtx (f : Function) (facts : Facts) (c : Ctx) : Ctx :=
    let params := f.params.map (·.name) |>.toArray
    let mutables := f.body.map mutableLocals |>.getD []
    let bound := f.body.map boundLocals |>.getD []
    let assignedParams := mutables.filter fun n => params.contains n && !bound.contains n
    let borrowedParams := (f.body.map mutableParams |>.getD []).filterMap fun i => params[i]?
    let rebound := (assignedParams ++ borrowedParams).eraseDups
    let renames := rebound.map fun n => (n, n ++ "'")
    let printed := params.map fun n => (renames.lookup n).getD n
    let tyParams := (f.typeParams.map (·.name)).toArray
    { c with
      fn := some f, params := printed, paramRenames := renames,
      mutables := mutables ++ rebound ++ rebound.map (· ++ "'"),
      inAction := facts.isAction, tyParams := tyParams }

/-- A spec function as Leaner's `spec fun` declaration — a specification
function under its Move name; an uninterpreted one is `spec opaque`.  A
spec function may read global memory (`spec fun` is stateful then); `old`
has no meaning in one.  A companion (`$f`, the spec rewriter's specification
version of the Move function `f`) prints nothing: Leaner derives the
specification version of `f` from `f`'s own body (at `fun f`), which is what
a specification's `f args` denotes. -/
def specFunDecl (sf : SpecFun) : PM (List Format) :=
  withReader (fun c => { c with tyParams := sf.typeParams.map (·.name) |>.toArray }) do
  if sf.params.any (containsFunctionTy ·.ty) || containsFunctionTy sf.result then
    unsupported "function-typed specification function"
  if sf.isMoveFun then return []
  let name := sf.name
  let tps := typeParamsDoc sf.typeParams "{" "}"
  let ps ← sf.params.mapM fun p => do
    pure (paren (text (legalize p.name ++ " : ") ++ (← printSpecParamTy p.ty)))
  let resultTy ← match sf.result with
    | .bool => pure (text "Prop")
    | t => printSpecTy t
  let binders := if ps.isEmpty then text "" else text " " ++ sepBy " " ps
  let fnAsFunction : Function := {
    name := sf.name, doc := sf.doc, loc := sf.loc, visibility := .«private», isEntry := false
    kind := .regular, isReceiver := false, attributes := [], typeParams := sf.typeParams
    params := sf.params, result := sf.result, pragmas := [], spec := .empty, body := none }
  match sf.body with
  | some body =>
    if sf.usesOld then
      unsupported "`old` in a spec function (Leaner's specification functions read the state of the clause that applies them)"
    let paramNames := sf.params.map (·.name) |>.toArray
    let (t, _) ← withReader (fun c => { c with inSpec := true, fn := some fnAsFunction, params := paramNames })
      (specTerm body)
    let header := text s!"spec fun {legalize name}{tps}" ++ binders ++ text " : " ++ resultTy
    pure (docComment sf.doc ++ [header ++ text " :=" ++ indent 2 (hard ++ t)])
  | none =>
    -- A bodyless spec function is normally just an uninterpreted symbol.  Its
    -- semantic constraints live in module-level `axiom` declarations, which
    -- `invariantsDecl` emits below.  Only report an attachment when the XAST
    -- actually carries one that this declaration cannot express.
    if !sf.spec.pragmas.isEmpty || !sf.spec.conditions.isEmpty || sf.spec.frame.isSome then
      drop s!"uninterpreted spec function `{sf.name}` (attached conditions not transported)"
    pure (docComment sf.doc ++ [text "spec opaque " ++ text s!"{legalize name}{tps}" ++ binders ++
      text " : " ++ resultTy])
where
  -- A parameter of a spec function: references are erased to the value
  -- (`printSpecTy`); a `signer` is `&Signer`, the form specification binders
  -- keep for it (`specHeader`), so a signer passed on type-checks.
  printSpecParamTy (t : Ty) : PM Format :=
    match derefTy t with
    | .signer => pure (text "&Signer")
    | t' => printSpecTy t'

/-- Module axioms, followed by global invariants in one `spec module where` block.

Move axioms have no source-level name. Give each emitted Lean axiom a stable,
collision-free module-local name so the report can identify the trusted
assumption. -/
def invariantsDecl (m : Module) : PM (List Format) := do
  if m.invariants.isEmpty then pure [] else do
  let mut taken := m.constants.map (·.name) ++ m.structs.map (·.name) ++
    m.functions.map (·.name) ++ m.specFuns.map (·.name) ++ m.specVars.map (·.name)
  let mut axioms : List Format := []
  let mut clauses : List Format := []
  for inv in m.invariants do
    match inv.kind with
    | .«axiom» =>
      let name := fresh "move_axiom" taken
      taken := name :: taken
      -- Like every generated generic Move declaration, a source axiom needs
      -- the internal inhabitance evidence that Leaner associates with each
      -- Move type parameter.  It is required here to apply an uninterpreted
      -- `spec opaque` whose type parameter has the same implicit evidence.
      let typeParams := String.join <| inv.typeParams.flatMap fun tp =>
        [" {" ++ legalize tp ++ "}", " [Inhabited " ++ legalize tp ++ "]"]
      let (t, _) ← withReader (fun c => {
        c with inSpec := true, fn := none, params := #[], tyParams := inv.typeParams.toArray
      }) (specTerm inv.exp)
      modify fun s => { s with report := { s.report with axioms := s.report.axioms ++
        [s!"`{name}` (source axiom)"] } }
      axioms := axioms ++ [text s!"axiom {legalize name}{typeParams} : " ++ t]
    | .global | .globalUpdate =>
      let (t, _) ← withReader (fun c => { c with inSpec := true, fn := none, params := #[] }) (specTerm inv.exp)
      let kw := if inv.kind == .globalUpdate then "invariant update " else "invariant "
      clauses := clauses ++ [text kw ++ t]
  if clauses.isEmpty then pure axioms else
    let body := clauses.zipIdx.map fun (c, i) => if i + 1 == clauses.length then c else c ++ text ";"
    pure (axioms ++ [block (text "spec module where") body])

-- -------------------------------------------------------------------------------------------------
-- Module

/-- The modules a type references. -/
partial def refsTy : Ty → List ModuleRef
  | .struct n args => n.module :: args.flatMap refsTy
  | .tuple ts => ts.flatMap refsTy
  | .vector t => refsTy t
  | .function args result _ => refsTy args ++ refsTy result
  | .reference _ t => refsTy t
  | .typeDomain t => refsTy t
  | .resourceDomain n args => n.module :: (args.getD []).flatMap refsTy
  | _ => []

mutual
  /-- The modules an expression references. -/
  partial def refsExp (e : Exp) : List ModuleRef :=
    refsTy e.ty ++ match e.node with
    | .invoke function args => refsExp function ++ args.flatMap refsExp
    | .call op inst args _ =>
      (match op with
       | .moveFunction n | .pack n _ | .select n _ | .selectVariants n _ | .testVariants n _
       | .specFunction n _ | .updateField n _ => [n.module]
       | _ => []) ++ inst.flatMap refsTy ++ args.flatMap refsExp
    | .block p b body => refsPat p ++ (b.map refsExp |>.getD []) ++ refsExp body
    | .ite c t f => refsExp c ++ refsExp t ++ refsExp f
    | .«match» s arms => refsExp s ++ arms.flatMap fun a => refsPat a.pattern ++ (a.guard.map refsExp |>.getD []) ++ refsExp a.body
    | .sequence es => es.flatMap refsExp
    | .loop b => refsExp b
    | .«return» v => refsExp v
    | .assign p v => refsPat p ++ refsExp v
    | .mutate t v => refsExp t ++ refsExp v
    | .quant _ rs _ c b => rs.flatMap (fun r => refsPat r.pattern ++ refsExp r.domain) ++ (c.map refsExp |>.getD []) ++ refsExp b
    | _ => []
  partial def refsPat (p : Pattern) : List ModuleRef :=
    refsTy p.ty ++ match p.node with
    | .struct n inst _ fs => n.module :: inst.flatMap refsTy ++ fs.flatMap refsPat
    | .tuple ps => ps.flatMap refsPat
    | _ => []
end

/-- The modules a module references through qualified names (types,
calls, patterns, friends). -/
def referencedModules (m : Module) : List ModuleRef :=
  let ty := refsTy
  let exp := refsExp
  let spec (s : Spec) : List ModuleRef :=
    s.conditions.flatMap fun c => exp c.exp ++ (c.abortCode.map exp |>.getD [])
  (m.friends ++
    m.structs.flatMap (fun s => s.fields.flatMap (fun f => ty f.ty) ++
      (s.variants.getD []).flatMap (fun v => v.fields.flatMap fun f => ty f.ty) ++ spec s.spec) ++
    m.functions.flatMap (fun f => f.params.flatMap (fun p => ty p.ty) ++ ty f.result ++
      (f.body.map exp |>.getD []) ++ spec f.spec) ++
    m.specFuns.flatMap (fun f => (f.body.map exp |>.getD []) ++ f.params.flatMap fun p => ty p.ty) ++
    m.invariants.flatMap (fun i => exp i.exp)).eraseDups

/-- Conventional Aptos named addresses registered by `Move.ConventionalAddresses`. -/
def conventionalAliases : List String :=
  ["vm", "vm_reserved", "std", "aptos_std", "aptos_framework", "aptos_token", "aptos_token_objects",
   "aptos_trading", "aptos_experimental", "aptos_fungible_asset", "core_resources"]

/-- The declarations of a module, as a list of items (each a list of lines),
with unsupported declarations commented out. -/
def moduleItems : PM (List (List Format)) := do
  let ctxt ← read
  let m := ctxt.mod
  let items := order m
  let guardItem (name : String) (k : PM (List Format)) : PM (List Format) := do
    let saved ← get
    try k catch e =>
      let e := " ".intercalate (e.splitOn "\n" |>.map String.trimAscii |>.map (·.toString))
      set { saved with
        report := { saved.report with unsupported := saved.report.unsupported ++ [s!"{name}: {e}"] },
        failed := saved.failed ++ [name] }
      pure [text s!"-- `{name}` not transpiled: {e}"]
  let mut out : List (List Format) := []
  for item in items do
    match item with
    | .constant c => out := out ++ [← guardItem c.name (constantDecl c)]
    | .struct s => out := out ++ [← guardItem s.name (structDecl s)]
    | .specFun sf =>
      -- A companion no specification uses prints nothing.
      let decl ← guardItem sf.name (specFunDecl sf)
      if !decl.isEmpty then out := out ++ [decl]
    | .specVar v => do drop s!"ghost variable `{v.name}`"; out := out ++ [[text s!"-- ghost variable `{v.name}` dropped (E18)"]]
    | .functions group recursive =>
      match group with
      | [f] => out := out ++ [← guardItem f.name (functionDecl f recursive)]
      | fs =>
        let docs ← fs.mapM fun f => guardItem f.name (functionDecl f true)
        out := out ++ [[text "mutual"] ++ (docs.map fun d => indent 2 (hard ++ vcat d)).map (fun d => d) ++ [text "end"]]
    | .spec f =>
      if (← get).failed.contains f.name then pure ()  -- its function was not transpiled
      else
        let facts := ctxt.effects.get { module := m.ref, name := f.name }
        -- Specifications see the parameters under their own names (a rebound
        -- parameter's fresh name belongs to the body alone).
        let specCtx (c : Ctx) : Ctx :=
          { functionDecl.functionCtx f facts c with
            params := (f.params.map (·.name)).toArray, paramRenames := [] }
        let spec ← guardItem s!"spec {f.name}" (withReader specCtx (specDecl f facts))
        if !spec.isEmpty then out := out ++ [spec]
    | .invariant _ => pure ()
  let invs ← guardItem "module invariants" (invariantsDecl m)
  if !invs.isEmpty then out := out ++ [invs]
  pure out

/-- The whole generated file for a module. -/
def moduleDoc : PM Format := do
  let ctxt ← read
  let m := ctxt.mod
  let sourceName := ((m.sources[m.loc.file]? <|> m.sources.head?).getD "?").splitOn "/" |>.getLast!
  let header := [text s!"-- generated by move-to-lean from {sourceName}", text "import Move"]
  -- Imports of transpiled dependencies: the package modules this module references.
  let referenced := referencedModules m
  let deps := ctxt.pkg.modules.filter fun d =>
    d.ref != m.ref && !isPrimitiveModule d.ref && referenced.contains d.ref
  let rootPrefix := if ctxt.leanRoot.isEmpty then "" else ctxt.leanRoot ++ "."
  let imports := deps.map fun d => text s!"import {rootPrefix}{leanModulePath d.ref}"
  let opens := [text "open Move", text "open scoped Move Move.Spec"]
  let aliases := match m.addressAlias with
    | some a => if conventionalAliases.contains a then [] else [text s!"address_alias {a} = {m.address}"]
    | none => []
  let items ← moduleItems
  let root := rootNamespace m.ref
  let atAddr := match m.addressAlias with
    | some a => a
    | none => m.address
  let moduleHeader := text s!"module {legalize m.name} at {atAddr} where"
  let friends := m.friends.map fun f =>
    text s!"friend {f.addressAlias.getD f.address}::{legalize f.name};"
  let itemSep := hard ++ hard
  let bodyLines := ((friends.map fun f => [f]) ++ items).map vcat
  let bodyDoc := Format.joinSep bodyLines itemSep
  let moduleBlock := if bodyLines.isEmpty then moduleHeader
    else moduleHeader ++ indent 2 (hard ++ hard ++ bodyDoc)
  let wrapped := match root with
    | some r => [text s!"namespace {r}", text ""] ++ [moduleBlock] ++ [text "", text s!"end {r}"]
    | none => [moduleBlock]
  if root.isSome then record .nestedNamespace
  let all := header ++ imports ++ [text ""] ++ opens ++ [text ""] ++ aliases ++
    (if aliases.isEmpty then [] else [text ""]) ++ moduleDocComment m.doc ++ wrapped
  pure (vcat all ++ hard)

/-- Prints a module of the package; returns the text and the report. -/
def printModule (pkg : Package) (m : Module) (effects : Table) (width : Nat := 100)
    (emitVerify : Bool := false) (leanRoot : String := "") (initialReport : Report := {}) :
    Except String (String × Report) := do
  if let some owner := m.structs.find? fun s => s.intrinsic.isSome then
    let model := owner.intrinsic.map (·.name) |>.getD "unknown"
    throw s!"unsupported: intrinsic type `{owner.name}` uses model `{model}`; E16 is suspended until intrinsic validation is implemented on the unified LIR"
  let ctxt : Ctx := { pkg, mod := m, effects, emitVerify, leanRoot }
  let st : St := { comments := Pool.ofModule m, report := initialReport }
  let (doc, st) ← (moduleDoc ctxt).run st
  let rendered := Format.pretty doc width
  let stripped := "\n".intercalate <| (rendered.splitOn "\n").map fun line =>
    String.ofList (line.toList.reverse.dropWhile (· == ' ') |>.reverse)
  pure (stripped, st.report)

end Transpiler.Print
