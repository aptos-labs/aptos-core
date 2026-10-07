-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# XAST mirror data structures

The Lean side of the XAST exchange format — the compiler-v2 typed AST of a
Move module with its specifications, produced by `aptos move exchange --format
ast`.  The normative schema is the `ast` module of the Rust crate
`move-model-exchange` (`third_party/move/move-model/exchange/src/ast.rs`);
these structures mirror it one to one, with Lean naming (`camelCase` fields,
`lowerCamel` constructors).  `LeanerMove.Frontend.Decode` reads the JSON wire form into
them.
-/

namespace LeanerMove.Frontend.Xast

/-- Schema identifier of an XAST document. -/
def schema : String := "move-xast-module"
/-- The XAST version this consumer reads. -/
def version : Nat := 10

/-- A source location: byte offsets into the module's file table. -/
structure Loc where
  file : Nat
  start : Nat
  stop : Nat
  deriving Repr, BEq, Inhabited

/-- Reference to a module. -/
structure ModuleRef where
  address : String
  addressAlias : Option String
  name : String
  deriving Repr, BEq, Inhabited, Ord

/-- Reference to a declaration of a module. -/
structure QualifiedName where
  module : ModuleRef
  name : String
  deriving Repr, BEq, Inhabited, Ord

structure NamedAddress where
  name : String
  address : String
  deriving Repr, BEq, Inhabited

/-- A state label by the number memory ranges refer to and its source name. -/
structure StateLabel where
  id : Nat
  name : String
  deriving Repr, BEq, Inhabited

/-- An ordinary source comment, delimiters included. -/
structure Comment where
  loc : Loc
  text : String
  /-- Whether only whitespace precedes the comment on its line. -/
  ownLine : Bool
  deriving Repr, BEq, Inhabited

inductive Ability where
  | copy | drop | store | key
  deriving Repr, BEq, Inhabited, DecidableEq

structure TypeParam where
  name : String
  abilities : List Ability
  isPhantom : Bool
  deriving Repr, BEq, Inhabited

/-- A Move or spec type. -/
inductive Ty where
  | bool | u8 | u16 | u32 | u64 | u128 | u256
  | i8 | i16 | i32 | i64 | i128 | i256
  | address | signer | num | range | eventStore
  | tuple (elements : List Ty)
  | vector (element : Ty)
  | struct (name : QualifiedName) (args : List Ty)
  /-- A Move function value. Its invocation is conservatively effectful in
  Leaner, so the printer renders the result under `Action`. -/
  | function (args result : Ty) (abilities : List Ability)
  | typeParam (index : Nat)
  | reference (mutable : Bool) (ty : Ty)
  | typeDomain (ty : Ty)
  | resourceDomain (name : QualifiedName) (args : Option (List Ty))
  | stateDomain
  deriving Repr, BEq, Inhabited

/-- A constant value.  Numbers are unbounded. -/
inductive Value where
  | address (hex : String)
  | number (value : Int)
  | bool (value : Bool)
  | vector (elements : List Value)
  | tuple (elements : List Value)
  deriving Repr, BEq, Inhabited

inductive PragmaValue where
  | value (value : Value)
  | name (name : String)
  | qualifiedName (name : String)
  deriving Repr, BEq, Inhabited

/-- A spec property: a pragma or a condition property. -/
structure Pragma where
  name : String
  value : PragmaValue
  deriving Repr, BEq, Inhabited

inductive AttributeValue where
  | value (value : Value)
  | name (module : Option ModuleRef) (name : String)
  deriving Repr, BEq, Inhabited

inductive Attribute where
  | apply (name : String) (args : List Attribute)
  | assign (name : String) (value : AttributeValue)
  deriving Repr, BEq, Inhabited

structure Field where
  name : String
  doc : String
  ty : Ty
  deriving Repr, BEq, Inhabited

structure Variant where
  name : String
  loc : Loc
  fields : List Field
  deriving Repr, BEq, Inhabited

inductive Visibility where
  | «private» | «public» | friend | package
  deriving Repr, BEq, Inhabited, DecidableEq

inductive FunctionKind where
  | regular | inlineRetained | native
  deriving Repr, BEq, Inhabited, DecidableEq

structure Param where
  name : String
  ty : Ty
  deriving Repr, BEq, Inhabited

inductive SurfaceSyntax where
  | receiverCall | indexNotation
  deriving Repr, BEq, Inhabited, DecidableEq

inductive QuantKind where
  | «forall» | «exists» | choose | chooseMin
  deriving Repr, BEq, Inhabited, DecidableEq

inductive RefKind where
  | immutable | mutable
  deriving Repr, BEq, Inhabited, DecidableEq

inductive AbortKind where
  | code | message
  deriving Repr, BEq, Inhabited, DecidableEq

inductive TraceKind where
  | user | auto | subAuto
  deriving Repr, BEq, Inhabited, DecidableEq

structure MemoryRange where
  pre : Option Nat
  post : Option Nat
  deriving Repr, BEq, Inhabited

inductive BehaviorKind where
  | requiresOf | abortsOf | ensuresOf | resultOf | unchangedOf | foldsOf
  | writeOf (index : Nat)
  deriving Repr, BEq, Inhabited, DecidableEq

/-- The operation of a call node. -/
inductive Operation where
  | moveFunction (name : QualifiedName)
  /-- A closure of a function: the call's arguments are the captured values,
  bound to the parameters whose bit is set in `mask`. -/
  | closure (name : QualifiedName) (mask : Nat)
  | pack (name : QualifiedName) (variant : Option String)
  | tuple
  | select (name : QualifiedName) (field : String)
  /-- The field of whichever listed variant the operand holds, as
  `(variant, field)` pairs. -/
  | selectVariants (name : QualifiedName) (fields : List (String × String))
  | testVariants (name : QualifiedName) (variants : List String)
  | specFunction (name : QualifiedName) (range : MemoryRange)
  | behavior (kind : BehaviorKind) (range : MemoryRange)
  | updateField (name : QualifiedName) (field : String)
  | result (index : Nat)
  | index | slice | range | implies | iff | identical
  | add | sub | mul | mod | div
  | bitOr | bitAnd | xor | shl | shr
  | and | or | eq | neq | lt | gt | le | ge
  | copy | move | not | cast | negate
  | exists (label : Option Nat)
  | borrowGlobal (kind : RefKind)
  | borrow (kind : RefKind)
  | deref | moveTo | moveFrom
  | freeze (explicit : Bool)
  | abort (kind : AbortKind)
  | vector | len | typeValue | typeDomain | resourceDomain | stateDomain
  | global (label : Option Nat)
  | canModify | old
  | saveStateAnchor (label : Nat)
  | withStateAnchor (label : Nat)
  | foldsCaptureAnchor (label : Nat)
  | inlineCallSummary
  | trace (kind : TraceKind)
  | specPublish (range : MemoryRange)
  | specRemove (range : MemoryRange)
  | specUpdate (range : MemoryRange)
  | emptyVec | singleVec | updateVec | concatVec | indexOfVec | containsVec
  | inRangeRange | inRangeVec | rangeVec
  | maxU8 | maxU16 | maxU32 | maxU64 | maxU128 | maxU256
  | bv2Int | int2Bv | abortFlag | abortCode | wellFormed
  | boxValue | unboxValue
  | emptyEventStore | extendEventStore | eventStoreIncludes | eventStoreIncludedIn
  | noOp
  deriving Repr, BEq, Inhabited

inductive ConditionKind where
  | letPost (name : String)
  | letPre (name : String)
  | assert | assume | decreases | abortsIf | abortsWith | succeedsIf | emits
  | ensures | requires | structInvariant | functionInvariant | loopInvariant
  | globalInvariant (typeParams : List String)
  | globalInvariantUpdate (typeParams : List String)
  | schemaInvariant
  | «axiom» (typeParams : List String)
  | update
  deriving Repr, BEq, Inhabited

mutual
  /-- A typed expression node. -/
  inductive Exp where
    | mk (ty : Ty) (loc : Loc) (node : ExpNode)

  inductive ExpNode where
    /-- `defaultedNum` preserves the compiler's defaulted literal type, distinct
    from an explicitly suffixed integer of the same value and width. -/
    | value (value : Value) (constant : Option String) (defaultedNum : Bool := false)
    | «local» (name : String)
    | param (index : Nat)
    | call (op : Operation) (inst : List Ty) (args : List Exp) (surface : Option SurfaceSyntax)
    | invoke (function : Exp) (args : List Exp)
    | block (pattern : Pattern) (binding : Option Exp) (body : Exp)
    | ite (cond thenBranch elseBranch : Exp)
    | «match» (scrutinee : Exp) (arms : List MatchArm)
    | sequence (exps : List Exp)
    | loop (body : Exp)
    | loopCont (nest : Nat) (isContinue : Bool)
    | «return» (value : Exp)
    | assign (pattern : Pattern) (value : Exp)
    | mutate (target value : Exp)
    | specBlock (spec : Spec)
    | quant (kind : QuantKind) (ranges : List QuantRange) (triggers : List (List Exp))
        (condition : Option Exp) (body : Exp)

  inductive MatchArm where
    | mk (loc : Loc) (pattern : Pattern) (guard : Option Exp) (body : Exp)

  /-- One binder of a quantifier with its domain; a binder over the state
  domain (`exists S in *`) names the state label it binds. -/
  inductive QuantRange where
    | mk (pattern : Pattern) (domain : Exp) (label : Option Nat)

  /-- A typed pattern node. -/
  inductive Pattern where
    | mk (ty : Ty) (loc : Loc) (node : PatternNode)

  inductive PatternNode where
    | var (name : String)
    | wildcard
    | tuple (elements : List Pattern)
    | struct (name : QualifiedName) (inst : List Ty) (variant : Option String)
        (fields : List Pattern)
    | literal (value : Value)
    | range (lower upper : Option Value) (inclusive : Bool)

  /-- A specification block, with the `proof` block of a function's. -/
  inductive Spec where
    | mk (loc : Option Loc) (pragmas : List Pragma) (conditions : List Condition)
        (frame : Option Frame) (accessOf : List AccessOf) (proof : Option Proof)

  /-- A statement of a structured proof (`proof { ... }`). -/
  inductive Proof where
    | «let» (loc : Loc) (name : String) (exp : Exp)
    | ite (loc : Loc) (cond : Exp) (thenProof : Proof) (elseProof : Option Proof)
    | block (loc : Loc) (proofs : List Proof)
    | «assert» (loc : Loc) (exp : Exp)
    | «assume» (loc : Loc) (exp : Exp)
    | apply (loc : Loc) (application : LemmaApplication)
    /-- `forall binders [triggers] [weight = n] apply lemma(args)`. -/
    | forallApply (loc : Loc) (binders : List Param) (triggers : List (List Exp))
        (weight : Option Nat) (application : LemmaApplication)
    /-- `calc(e1 op e2 op ...)`: each step as the comparison it asserts. -/
    | calc (loc : Loc) (steps : List Exp)
    /-- `post proof`: run at each return instead of at entry. -/
    | post (loc : Loc) (proof : Proof)
    | split (loc : Loc) (exp : Exp)

  /-- A lemma applied to arguments, with its type arguments. -/
  inductive LemmaApplication where
    | mk (lemma : QualifiedName) (inst : List Ty) (args : List Exp)

  inductive Condition where
    | mk (kind : ConditionKind) (loc : Loc) (properties : List Pragma) (exp : Exp)
        (abortCode : Option Exp) (additionalCodes : List Exp) (emitsHandle : Option Exp)
        (emitsCondition : Option Exp) (updateTarget : Option Exp)

  inductive Frame where
    | mk (modifies : List Exp) (reads : List Ty) (modifiesAll : Bool) (readsAll : Bool)

  /-- The access declaration of a function-typed parameter or field `f`:
  `modifies_of<f>(formals) targets` and `reads_of<f> types`. -/
  inductive AccessOf where
    | mk (loc : Loc) (name : String) (formals : List Param) (modifies : List Exp)
        (reads : List Ty) (modifiesAll : Bool) (readsAll : Bool)
end

namespace Exp
def ty : Exp → Ty | .mk t _ _ => t
def loc : Exp → Loc | .mk _ l _ => l
def node : Exp → ExpNode | .mk _ _ n => n
end Exp

namespace Pattern
def ty : Pattern → Ty | .mk t _ _ => t
def loc : Pattern → Loc | .mk _ l _ => l
def node : Pattern → PatternNode | .mk _ _ n => n
end Pattern

namespace MatchArm
def loc : MatchArm → Loc | .mk l _ _ _ => l
def pattern : MatchArm → Pattern | .mk _ p _ _ => p
def guard : MatchArm → Option Exp | .mk _ _ g _ => g
def body : MatchArm → Exp | .mk _ _ _ b => b
end MatchArm

namespace QuantRange
def pattern : QuantRange → Pattern | .mk p _ _ => p
def domain : QuantRange → Exp | .mk _ d _ => d
def label : QuantRange → Option Nat | .mk _ _ l => l
end QuantRange

namespace Spec
def loc : Spec → Option Loc | .mk l _ _ _ _ _ => l
def pragmas : Spec → List Pragma | .mk _ p _ _ _ _ => p
def conditions : Spec → List Condition | .mk _ _ c _ _ _ => c
def frame : Spec → Option Frame | .mk _ _ _ f _ _ => f
def accessOf : Spec → List AccessOf | .mk _ _ _ _ a _ => a
def proof : Spec → Option Proof | .mk _ _ _ _ _ p => p
def empty : Spec := .mk none [] [] none [] none
end Spec

namespace LemmaApplication
def lemma : LemmaApplication → QualifiedName | .mk l _ _ => l
def inst : LemmaApplication → List Ty | .mk _ i _ => i
def args : LemmaApplication → List Exp | .mk _ _ a => a
end LemmaApplication

namespace Condition
def kind : Condition → ConditionKind | .mk k _ _ _ _ _ _ _ _ => k
def loc : Condition → Loc | .mk _ l _ _ _ _ _ _ _ => l
def properties : Condition → List Pragma | .mk _ _ p _ _ _ _ _ _ => p
def exp : Condition → Exp | .mk _ _ _ e _ _ _ _ _ => e
def abortCode : Condition → Option Exp | .mk _ _ _ _ c _ _ _ _ => c
def additionalCodes : Condition → List Exp | .mk _ _ _ _ _ c _ _ _ => c
def emitsHandle : Condition → Option Exp | .mk _ _ _ _ _ _ h _ _ => h
def emitsCondition : Condition → Option Exp | .mk _ _ _ _ _ _ _ c _ => c
def updateTarget : Condition → Option Exp | .mk _ _ _ _ _ _ _ _ t => t
end Condition

namespace Frame
def modifies : Frame → List Exp | .mk m _ _ _ => m
def reads : Frame → List Ty | .mk _ r _ _ => r
def modifiesAll : Frame → Bool | .mk _ _ m _ => m
def readsAll : Frame → Bool | .mk _ _ _ r => r
end Frame

namespace AccessOf
def loc : AccessOf → Loc | .mk l _ _ _ _ _ _ => l
def name : AccessOf → String | .mk _ n _ _ _ _ _ => n
def formals : AccessOf → List Param | .mk _ _ f _ _ _ _ => f
def modifies : AccessOf → List Exp | .mk _ _ _ m _ _ _ => m
def reads : AccessOf → List Ty | .mk _ _ _ _ r _ _ => r
def modifiesAll : AccessOf → Bool | .mk _ _ _ _ _ m _ => m
def readsAll : AccessOf → Bool | .mk _ _ _ _ _ _ r => r
end AccessOf

instance : Inhabited Exp := ⟨.mk .bool ⟨0, 0, 0⟩ (.value (.bool false) none)⟩
instance : Inhabited Pattern := ⟨.mk .bool ⟨0, 0, 0⟩ .wildcard⟩
instance : Inhabited Spec := ⟨Spec.empty⟩

structure Constant where
  name : String
  doc : String
  loc : Loc
  ty : Ty
  value : Value
  deriving Inhabited

/-- A resolved function assigned to one semantic role of an intrinsic type. -/
structure IntrinsicBinding where
  role : String
  target : QualifiedName
  deriving Repr, BEq, Inhabited

/-- A validated intrinsic-type declaration and its resolved role bindings. -/
structure Intrinsic where
  name : String
  moveFunctions : List IntrinsicBinding
  specFunctions : List IntrinsicBinding
  deriving Repr, BEq, Inhabited

structure Struct where
  name : String
  doc : String
  loc : Loc
  abilities : List Ability
  typeParams : List TypeParam
  attributes : List Attribute
  isNative : Bool
  fields : List Field
  variants : Option (List Variant)
  spec : Spec
  intrinsic : Option Intrinsic
  deriving Inhabited

structure Function where
  name : String
  doc : String
  loc : Loc
  visibility : Visibility
  isEntry : Bool
  kind : FunctionKind
  isReceiver : Bool
  attributes : List Attribute
  typeParams : List TypeParam
  params : List Param
  result : Ty
  /-- Resolved pragmas (module-level ones inherited). -/
  pragmas : List Pragma
  spec : Spec
  body : Option Exp
  deriving Inhabited

structure SpecFun where
  name : String
  doc : String
  loc : Loc
  typeParams : List TypeParam
  params : List Param
  result : Ty
  uninterpreted : Bool
  isNative : Bool
  isMoveFun : Bool
  usesOld : Bool
  body : Option Exp
  spec : Spec
  deriving Inhabited

structure SpecVar where
  name : String
  loc : Loc
  typeParams : List TypeParam
  ty : Ty
  init : Option Exp
  deriving Inhabited

/-- A lemma: `lemma name<T>(params) { requires ..; ensures ..; } proof { .. }`. -/
structure Lemma where
  name : String
  loc : Loc
  typeParams : List TypeParam
  params : List Param
  /-- The `requires` and `ensures` conditions. -/
  conditions : List Condition
  /-- The declared measure (`decreases`), if any. -/
  decreases : Option (List Exp)
  proof : Option Proof
  deriving Inhabited

inductive InvariantKind where
  | global | globalUpdate | «axiom»
  deriving Repr, BEq, Inhabited, DecidableEq

structure Invariant where
  kind : InvariantKind
  loc : Loc
  typeParams : List String
  properties : List Pragma
  exp : Exp
  deriving Inhabited

/-- A declaration the producer left out (function-value construction), with the reason. -/
structure Skipped where
  name : String
  reason : String
  /-- Whether the declaration is an inline function, or the specification
  version of one: the compiler has expanded it where it is called. -/
  inline : Bool
  deriving Repr, Inhabited

/-- A module's typed AST: the top-level object of an XAST document. -/
structure Module where
  address : String
  addressAlias : Option String
  name : String
  doc : String
  loc : Loc
  namedAddresses : List NamedAddress
  friends : List ModuleRef
  pragmas : List Pragma
  constants : List Constant
  structs : List Struct
  functions : List Function
  specFuns : List SpecFun
  specVars : List SpecVar
  invariants : List Invariant
  lemmas : List Lemma := []
  /-- The state labels the module's specifications name. -/
  labels : List StateLabel := []
  skipped : List Skipped := []
  sources : List String
  comments : List Comment
  /-- Whether the module's functions are verification targets. A dependency
  read with its bodies, so its calls inline, is not. -/
  isTarget : Bool := true
  /-- The proof file beside the module's source, whose `verify` items and
  lemmas are spliced into the module; named on it as the `proof_file`
  pragma whether or not the file exists. -/
  proofFile : Option String := none
  deriving Inhabited

/-- The declarations the export lost. Compiler-v2 has already expanded an
inline function where it is called, so leaving one out loses nothing; any
other declaration left out is unsupported. -/
def Module.omitted (m : Module) : List Skipped :=
  m.skipped.filter (!·.inline)

/-- The module's own reference. -/
def Module.ref (m : Module) : ModuleRef :=
  { address := m.address, addressAlias := m.addressAlias, name := m.name }

/-- Whether a pragma bag sets a boolean property to `true`. -/
def pragmaTrue (pragmas : List Pragma) (name : String) : Bool :=
  pragmas.any fun p => p.name == name && p.value == .value (.bool true)

end LeanerMove.Frontend.Xast
