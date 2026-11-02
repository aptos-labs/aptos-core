# VerexSL

VerexSL (VSL) is the profile-aware Verex source language for the shared
Verex IR, covering the semantic union represented by
[`VerexIR.Syntax`](../../verex-ir/VerexIR/Syntax.lean). This document
defines its boundary, grammar, forms, specification language, and the
canonical-source obligation. Open work:
[`roadmap.md`](../plan/roadmap.md).

The language has two audiences: people authoring profile-aware programs and
specifications in Lean, and a canonical source backend that prints every
validated structured VIR unit without changing its meaning. The second keeps
the first honest: ergonomic sugar may be partial, but every validated core
construct has an unambiguous canonical spelling.

## Purpose and boundary

Verex is one Lean-hosted language with explicitly selected semantic
profiles, not a family of unrelated Move, Rust, and extension dialects.

```text
Verex source --frontend--> RawUnit --validate--> ValidatedUnit
                                                 |
                                                 +--> execution
                                                 +--> verification
                                                 +--> executable backends
                                                 +--> canonical Verex source
```

The source language denotes the semantic contents of a `ValidatedUnit`:
namespaces, imports, declarations, generic binders and predicates,
structured bodies, places, patterns, operations, specifications, intrinsic
role graphs, attributes, pragmas, profile configuration, and checked
extensions. It does not author arena indexes, raw basic blocks, `goto`,
cleanup edges, structurization witnesses, source hashes, producer receipts,
alignment evidence, or derived validation facts; those belong to the raw
interchange or validation envelope. Canonical source always contains the
validated structured body; Verex has no `goto` fallback.

The coverage contract:

> Every semantic non-extension constructor reachable in a `ValidatedUnit`
> has a canonical Verex spelling. Every semantic extension constructor has
> one explicit, registry-checked profile spelling. No semantic node is
> silently erased or approximated to make source printing succeed.

Provenance-only fields use generated source maps and import receipts, not
tokens or optional annotations in program text.

### Three syntax layers

1. **Preferred surface syntax**: concise and profile-aware (operators,
   ordinary calls, structure literals, `&` borrows, familiar contract
   clauses), for authored programs.
2. **Canonical core syntax**: complete and unambiguous, the named `core.*`,
   `spec.*`, and `profile[...]` forms. A source backend falls back to it
   whenever a preferred spelling would lose information.
3. **Raw interchange syntax**: `RawUnit` JSON, which may contain CFGs,
   producer receipts, arena IDs, and structurization inputs. It is not a
   source language and is never printed as Verex code.

Surface and canonical syntax elaborate through the same frontend into the
same raw VIR constructors. Canonical forms are not escape hatches: they pass
through profile registration, structural validation, typing, capability
preparation, and borrow checking like any other source.

## Relationship to the Move profile

The Move profile's `module M at 0x42`, Move integer names, resources,
`Action`, `aborts_if`, and Move operators are profile-selected sugar over
this language. `module account at 0x42 where` is the Move shorthand for a
namespace with identity `0x42::account` and profile `move`. Move programs
do not become more verbose because the core language is general.
Conversely, a Rust-profile namespace never acquires Move arithmetic, abort,
reference, storage, or destruction semantics because their surface forms
look convenient: sugar is admitted only where the selected profile defines
the same VIR node and meaning.

## Hosting in Lean

A source file is an ordinary Lean file importing the Verex frontend
(`import Verex`, `open scoped Verex`); Verex declarations and bodies are
Lean command and term syntax. Lean parses, expands macros, and constructs
the frontend unit, but validated VIR, not retained Lean syntax, is the
semantic input to execution, verification, and backends. An expression
that cannot be lowered faithfully is a located frontend error; there is no
fallback that evaluates an unrecognized Lean term as VIR. Ordinary Lean
remains available for metaprogramming, proofs, and designated compile-time
computation. Elaboration can run metaprograms and host I/O, so tooling
treats Verex sources like trusted build scripts
([`vir-design.md`](vir-design.md#todo-secure-lean-source-discovery-and-elaboration)).

## Compilation units, profiles, and namespaces

A compilation unit contains one table snapshot, profile configurations,
owned namespaces, and checked dependency interfaces; source declares the
namespaces, the frontend constructs the tables.

```lean
verex namespace example::collections where   -- Rust profile
verex module 0x42::collections where         -- Move profile: address, one module name
verex namespace example::kernel using extension gpu where   -- registered extension

profile gpu version 2 where
  option target = "sm_90"
  option overflow = "trap"
```

The header keyword selects the profile; canonical source does not repeat it
with `using move` or `using rust`, though the explicit forms are accepted.
Profile options are semantic configuration: validation rejects unknown
names, versions, duplicates, and unsupported options. `using none` heads a
profileless namespace of shared declarations or dependency metadata;
executable and logical declarations still name a concrete profile, never
inferred from origin.

Namespace paths are ordered strings, `::`-separated, quoted segments for
names that are not Lean identifiers; a profile interprets the segments
(Move: address or package alias; Rust: crate and module), and the identity
is structural. A Move module path is `address::module`, its address a
literal or a named address (`address_alias std = 0x1`; `VerexSL.Addresses`
declares the Aptos ones). The alias is only a spelling: every Move path
leads with the canonical address, `std::vector` and `0x1::vector` name one
module, `@std` is `0x1`; the namespace reference keeps the alias
(`NamespaceRef.alias`, not identity), the printer spells it again, and a
rendering declares the aliases its environment lacks.

`use example::collections` imports a namespace, `use
example::collections::map` one symbol: a source-level alias for resolved
VIR names, separate from Lean's `import`. The printer infers symbol imports
when the final name is unique and falls back to a namespace import on
collision (`mem::replace`).

A namespace has one profile, which its declarations carry explicitly in
VIR. The authored language is profile-pure: declaration profile overrides
and cross-profile reference types, calls, and summaries are rejected until
validated adapter declarations exist.

## Names

Declarations have a namespace identity and an interned local spelling.
Source uses `name` for a declaration in the current namespace and
`path::to::namespace::name` for a qualified reference. Type names and value
names occupy the declaration families VIR validation enforces; trait
associated types and values may share a spelling because they occupy
separate associated namespaces. Duplicate names within one resolver family
are rejected, not resolved by declaration order.

Verex keywords use Lean's quoted identifier syntax (`«match»`).
Producer-private locals and generic binders are alpha-renamed to ordinary
identifiers instead of forcing guillemets. Numeric tuple fields keep their
`.0`, `0 : T`, and `{ 0 := value }` spellings. Semantic round trip is
alpha-equivalence for binders, not textual binder-name equality.

## Grammar overview

`[...]` is optional, `...` repetition, and capitalized words such as `Expr`
and `Type` stand for the categories below, not arbitrary Lean terms.

```text
CompilationUnit ::=
  ProfileConfig* Namespace+

ProfileConfig ::=
  "profile" Ident "version" Nat "where" ProfileOption*

Namespace ::=
  "verex" "module" MoveAddress "::" Ident "where" NamespaceItem*
  | "verex" "namespace" Path "where" NamespaceItem*
  | "verex" "namespace" Path "using" Profile "where" NamespaceItem*

Profile ::=
  "none" | "move" | "rust" | "extension" Ident

NamespaceItem ::=
  "use" Path
  | Attribute
  | Pragma
  | ConstantDecl
  | StructDecl
  | EnumDecl
  | TraitDecl
  | ImplDecl
  | FunctionDecl
  | SpecFunctionDecl
  | SpecVarDecl
  | NamespaceCondition
  | IntrinsicDecl

GenericBinder ::=
  "{" Ident ":" ("type" | "const" | "lifetime" | "evidence")
      [AbilityClause] "}"

GenericArgument ::=
  ["type"] Type
  | "const" Const
  | "lifetime" Lifetime
  | "evidence" Ident

WhereClause ::=
  "where" Predicate ("," Predicate)*

Type ::=
  "Unit" | "Never" | "Bool" | "Char" | "string" | "Bytes"
  | "Address" | "Signer"
  | "UInt" "<" Nat ">" | "SInt" "<" Nat ">"
  | "UPtr" | "IPtr" | "Nat" | "Int"
  | "(" Type,* ")"
  | "Vector" "<" Type ["," "const" Const] ">"
  | "Range" | "EventStore" | "TypeDomain" "<" Type ">"
  | "ResourceDomain" "<" Type ">" | "StateDomain"
  | Path ["::<" GenericArgument,* ">"]
  | "Fn" "(" Type,* ")" "->" Type [AbilityClause]
  | "&" ["[" Profile "]"] [Lifetime] ["mut"] Type
  | "profile_type" "[" Profile "," String "," String "]"

FunctionDecl ::=
  [Visibility] ["entry"] ["native" | "opaque"]
    "fun" Ident GenericBinder* Signature
    [WhereClause] [":=" Expr]

Visibility ::=
  "private" | "public" | "package" | "friend"

Signature ::=
  "(" Parameter,* ")" ["->" Type]

Parameter ::=
  ["mut"] Ident ":" Type

Pattern ::=
  "_" | Ident | "(" Pattern,* ")"
  | ConstructorPattern | Const | RangePattern

Place ::=
  Ident | "*" Place | Place "." Ident | Place "[" Expr "]"
  | Place "[" Nat ".." Nat "]"
  | Place "[" Nat "..^" Nat "]"
  | "downcast" Place "as" Path "::" Ident

Expr ::=
  Const | Path | Ident | Operation
  | "do" Statement* [Expr]
  | "let" ["mut"] Pattern [":" Type] [":=" Expr]
  | "if" Expr "then" Expr ["else" Expr]
  | "match" Expr "with" MatchArm+
  | "for" Ident "in" Expr ".." Expr "do" Statement*
  | "loop" [Label] Expr
  | "break" [Label] [Expr]
  | "continue" [Label]
  | "return" Expr
  | "abort" Expr,* | "panic" Expr,* | ProfileThrow
  | Place ":=" Expr | Pattern ":=" Expr
  | Quantifier | SpecBlock

MatchArm ::=
  "|" Pattern ["if" Expr] "=>" Expr

Operation ::=
  PreferredOperation | CoreOperation | SpecOperation | ProfileOperation
```

The grammar has no raw CFG production and exposes no interned IDs: names,
patterns, places, expressions, types, and lifetimes are resolved and
interned by frontend elaboration.

Canonical blocks use the offside rule and print no semicolons. When a branch
is itself a block, `do` stays on the branch header and the indented
sequence is its body:

```lean
if condition then do
  first()
  second()
else do
  fallback()
```

The parser also accepts semicolons as statement separators.

## Attributes, pragmas, documentation, and comments

Attributes and pragmas share one recursive semantic representation:

```lean
@[inline]
@[derive(Eq, Hash)]
@[layout = "transparent"]
pragma overflow(mode = "checked")
```

An attribute is a call with nested attribute arguments or an assignment
whose value is a constant, name, or qualified name. Placement distinguishes
a declaration attribute from a namespace, function, or specification
pragma; there is one attribute language.

Documentation and comments are provenance: one scanner, shared by file
elaboration and the formatter, lowers line and block comments to the VIR
comment table before Lean discards them, and canonical printing preserves
their text, multiplicity, and order near the declaration their location
selects. Move declaration documentation renders as `/-- ... -/`; namespace
documentation is owned by the header and renders as `/-! ... -/` before
`verex namespace` or `verex module`, inferred `use` declarations following
inside the body. Comments do not affect meaning. An operation may retain a
surface hint (receiver or index notation, an extension hint) that never
changes the operation; the printer honors it only when the sugar
re-elaborates to the same node.

## Generic declarations

Verex exposes the four VIR binder sorts: `{T}`, `{N : const}`,
`{'a : lifetime}`, `{D : evidence}`. The braces denote explicit generic
binders, not implicit Lean arguments. An unannotated binder is a type
binder; `: type` is written only for a phantom parameter
(`{T : phantom type}`). Type binders may carry the core abilities
(`{T has Copy, Drop, Store}`); other constraints use a `where` block:

```lean
where
  T implements collections::Iterator<Item = U8>,
  collections::Iterator<T>::LIMIT = 64,
  'a outlives 'b,
  N = 4
```

| Predicate | Meaning |
|---|---|
| `T has Copy` | Core ability constraint; the others are `Drop`, `Store`, `Key`. |
| `T implements Trait<...>` | Trait implementation obligation. |
| `Trait<...>::Item = U` | Associated type equality. |
| `Trait<...>::CONST = value` | Associated constant equality. |
| `'a outlives 'b` | Lifetime-outlives constraint. |
| `A = B` | Closed const equality. |
| `profile[p, "tag", "payload"]` | Registered profile predicate outside the known union. |

Generic applications take type, const, lifetime, and evidence arguments; the
canonical form makes non-type arguments explicit when their sort is
ambiguous: `Buffer::<type U8, const 32, lifetime 'a, evidence D>`.

A const binder `{N : const}` lowers to the canonical unsigned pointer-width
integer type `UPtr`. Evidence binders and arguments are ordinary authored
source: an evidence name resolves to a binder or to an entry of the unit's
checked evidence table, and application uses the explicit `evidence D`
argument. A numeric arena slot is never accepted or printed as proof of a
trait implementation.

## Types

Every type use carries a source location even when equal types share one
interned node.

| VIR type | Canonical spelling |
|---|---|
| Unit, Never, Boolean, Unicode scalar | `Unit`, `Never`, `Bool`, `Char` |
| Unicode string, byte string | `string`, `Bytes` |
| Address, Signer | `Address`, `Signer` |
| Fixed integers | `UInt<width>`, `SInt<width>` |
| Pointer integers | `UPtr`, `IPtr` |
| Unbounded integers | `Nat`, `Int` |
| Tuple | `(T₁, ..., Tₙ)` |
| Dynamic, fixed vector | `Vector<T>`, `Vector<T, const N>` |
| Logical range, event store | `Range`, `EventStore` |
| Type, resource, state domain | `TypeDomain<T>`, `ResourceDomain<R<...>>`, `StateDomain` |
| Nominal type | `path::Name::<...>` |
| Function value | `Fn(T₁, ..., Tₙ) -> R has A₁, ...` |
| Type parameter | its binder name |
| Shared, mutable reference | `&'a T`, `&'a mut T` |
| Profile extension | `profile_type[p, "tag", "payload"]` |

`UInt<width>` and `SInt<width>` accept every nonzero fixed width VIR
represents. Profile sugar includes Move's `U8` through `U256` and Rust's
`u8` through `u128`, `usize`, and signed counterparts.

The integer domain selected automatically by specifications is always
`Int`; `Nat` is available only when a model asks for it, and bounded
unsigned values are not implicitly widened to `Nat`.

A reference type records its semantic profile, which in the authored
language equals the enclosing namespace profile; the explicit cross-profile
annotation `&[rust] 'a mut T` is reserved for the adapter design.
Lifetimes are `'static`, named parameter lifetimes, `_` for inference, and
generated local lifetimes; canonical printing may give a local lifetime a
stable generated name when `_` would lose constraints.

Dynamic and fixed vectors share one VIR constructor; the optional length is
a closed nonnegative const. Tuples may be empty or unary. A function has one
packed result type in source; an imported physical result array is
normalized to it at the boundary. Logical domain types are
specification-only but first-class typed VIR nodes, never profile strings.

## Constants and literals

```text
()                         unit
true, false                Boolean
'x', '\u{1f980}'           Unicode scalar
0, -1, 0xff                integer
@0x1                       address
"text"                     string
b[0x00, 0xff]              bytes
#[a, b, c]                 profile-neutral constant vector
vector<u64>[a, b, c]       Move-profile vector
(a, b, c)                  constant tuple
profile[p, "tag", "data"] checked profile constant
```

The type at the use site determines an integer literal's width and
signedness; validation rejects out-of-range fixed-width literals and
invalid Unicode scalars. Address spelling is profile-checked, the core
keeping its textual identity. `string` and `Bytes` are distinct core types;
`Bytes` is not identified with `Vector<UInt<8>>`. `sourceConstant`
provenance may remember that a literal came from a named constant without
changing the value. A constant declaration, `const MAX : u64 := …`, has an
ordinary typed expression root checked under the namespace profile.

## Nominal data

```lean
struct Pair {T} has Copy, Drop where
  first : T
  second : T

enum Option {T} has Copy, Drop where
  | none = 0
  | some (value : T) = 1
```

A declaration is struct-shaped or enum-shaped. Enum discriminants are
optional integers and observable values, not array positions. Fields and
variants are ordered and may use named or positional syntax. Core abilities
follow `has`; profile-owned declaration properties use registered
annotations (`@[profile rust "repr" "C"]`). A data contract attaches
invariants to the declaration (`spec Pair where invariant this.first ==
this.second`). Move's certified-value interpretation of an invariant is a
Move-profile rule; the condition itself is shared VIR.

## Traits and implementations

```lean
trait Iterator {T} : Super<T> where
  type Item
  type Error := Never
  const LIMIT : Nat
  fun next (self : &'a mut T) : Option<Item>
  fun size_hint (self : &'a T) : UInt<64> := iterator_size_hint

impl {T} Iterator<T> for Cursor<T>
where
  T has Copy
where
  type Item := T
  const LIMIT := 64
  fun next := cursor_next
  fun size_hint := cursor_size_hint
```

Associated items are types (with bounds and defaults), constants (type and
optional default), and methods (signature, optional default naming an
ordinary function). Every associated item is owned once by its trait; an
implementation binds every required item. Validation checks ownership,
kinds, arity, first-order constraints, and default and binding types.
Implementation selection and evidence construction are part of the checked
frontend boundary; the language does not fabricate evidence because an
`impl` with a matching display name exists.

## Functions and signatures

```lean
fun swap_add {T} (left : u32, mut right : u32)
    -> (u32, Bool)
where
  T has Drop := do
  ...
```

Parameters are immutable unless `mut`. A signature has zero or one result
type; multiple physical results from an imported IR are canonicalized to
one tuple, which `return` supplies as one value. `opaque fun` covers an
absent body with an imported summary or unavailable implementation;
`native` covers a body supplied by the profile's runtime. The modifiers are
`private` (default), `public`, `package`, `friend` (mutually exclusive),
`entry`, and `native`. Inline hints, Rust ABI, and Rust safety are profile
data or registered attributes.

Declaration attributes accept nested calls and assigned values
(`@[resource_group (scope («global»)), randomness = 7]`); quoted
identifiers retain dotted names. Specification-function annotations use
the contract pragma slot and are restored to attributes by the printer.
`@[move_public]` is a compatibility spelling for a public function.
Intrinsic owners (`intrinsic_<model>`) and model-prefixed roles (`map_*`)
are reserved annotations lowering to the intrinsic role graph; malformed
owner or role annotations, and roles on nominal declarations, are rejected.

## Locals and lexical scope

Parameters occupy the leading local slots. Every local binding has a stable
identity, name, type, mutability, and location:

```lean
let x : T := value
let mut y : T := value
let mut later : T
```

An uninitialized declaration is permitted, but every read must pass
path-sensitive definite-initialization checking; branch joins keep only the
initialization common to every path. Moves and drops consume; writes and
matching bindings initialize. Shadowing creates a new identity; canonical
output may suffix the name to keep identities visible.

## Places and ownership operations

```text
p ::= x | *p | p.field | p[e] | p[start..stop] | p[start..^stop]
    | downcast p as Type::Variant
```

`p[start..^stop]` means `start .. length(p) - stop`. A downcast refines an
enum place to one variant before selecting its fields; tuple fields use
constant indexes. Ownership operations are explicit so partial moves and
non-consuming reads survive a fresh elaboration:

```lean
local          -- direct local access, determined by the declaration
move(place)    -- consume
copy(place)    -- Copy-qualified non-consuming load
read(place)    -- non-consuming place read (Rust CopyForDeref)
&place         -- shared reference
&mut place     -- mutable reference
place := value -- write through a place
drop(place)    -- destroy a Drop value
```

The profile and the ability and borrow checker decide which are legal;
none is inferred from origin. Assignment is structured syntax:
`place := value`, and `(left, right) := pair` as pattern assignment, whose
pattern binds no new locals.

## Patterns

```text
_                              wildcard
x                              binder or assignment target
(p1, ..., pn)                  tuple
Type::<...> { field := p }     struct constructor
Type::Variant::<...>(p1, ...)  enum constructor
literal                        closed literal
lo..hi, lo..=hi, ..hi, lo..    range
```

Patterns are typed and located; constructor patterns resolve the same
declarations, variants, generic arguments, abilities, and field types as
constructor operations. They occur in `let`, destructuring assignment, match
arms, destructors, and quantifier binders.

A match takes the first arm whose pattern fits and whose guard, evaluated
after binding, holds; a failed guard continues with the locals and state
before the arm. A value no arm fits, a `let` or assignment that does not
fit, and an access to a field of an enum value whose variant lacks it make
the profile's mismatch throw (`patternMismatchThrow?`): Move aborts with
compiler-v2's incomplete-match code `0xCA26CBD9BE0B0001`; a profile without
one has no outcome there. A binding fails atomically.

A pattern matched through a reference binds references to the fields it
names. A pass before validation rewrites such patterns into what Move's
compiler emits (`VerexIR.Import.elaborateReferencePatterns`): variant
tests, borrows of the bound fields, the mismatch throw, and arms tried in
order. `downcast p as Enum::Variant` is the value at `p` as that variant;
a field several variants give different types is reached through it.

## Structured expressions and control

```lean
do
  statement₁
  ...
  result

let pattern [: type] [:= value]
if condition then ... else ...
match value with
  | pattern [if guard] => body
while condition do
  body
where
  let logical_name := expression
  invariant condition
for iterator in lower..upper do
  body
loop[@label] ...
break[@label] [value]
continue[@label]
return value
abort arguments...
panic arguments...
throw profile[p, "tag", "payload"] arguments...
```

VIR stores `break` and `continue` targets as lexical nesting depths; source
uses labels (`loop@outer`, `break@outer`), and unknown labels are rejected.
Loops are expressions and may produce a value through `break`. A body that
can fall through must match its declared result type; a loop body, an `if`
without `else`, and a fall-through pattern assignment have type `Unit`.
Abrupt control is polymorphic through `Never`.

`for iterator in lower..upper do` is the half-open range loop. Each bound is
evaluated once, in order. Lowering expands it to typed locals, a `loop`, a
comparison, and the profile's increment, inserting the increment before each
current-loop `continue`; the printer recovers the `for` spelling from that
expansion, so imported Move range loops and re-elaborated source share one
fixed point.

A block's last entry decides how its `return` reads: without a semicolon it
supplies the block's value (how a function body spells its result); as
`return value;` it leaves the function. The printer emits the semicolon
exactly when the enclosing block does not itself yield the returned value.
A statement whose text ends with an indented block carries no separator.

`throw` has three classes, `abort`, `panic`, and registered profile throws;
the profile defines final-state behavior (Move rollback, Rust panic
cleanup), and source never substitutes one class for another. There is no
validated `goto`, raw switch, cleanup edge, resume, or unreachable
terminator; reducible frontend graphs become `if`, `match`, and `loop`, and
an unsupported graph is rejected before a source backend runs.

## Calls, construction, and closures

```lean
function(arguments...)
function::<RequiredTypeArguments>(arguments...)
(external::function(arguments...) : Result)
new Type { field := value }
new Type::Variant { field := value }
function[FnType](target, captures...)
invoke(callable, arguments...)
core.call_extension profile[...] targets[...] (arguments...)
```

Type arguments are omitted whenever argument types determine them. `f(args)`,
literals, destructuring patterns, and function-value application are sugar;
the canonical forms remain available where sugar would lose a distinction.
Field initialization shorthand `new Type { field }` applies when the value
is the local of the same name. A function whose first parameter is `self`
prefers receiver notation (`value.method()`), lowering inserting the needed
borrow; field, `.length`, and index notation accept a value or a reference
and insert the dereference, so canonical source uses `values.length` and
`values[index]`. The checked standard-library signature table supplies
receiver and result information for imported Move vector functions. All
declarations are predeclared before bodies elaborate, so forward and mutual
recursion need no `mutual` block; canonical declarations keep source order
after dependency constraints.

A closure names an ordinary function and captures its parameter prefix; its
result type is the residual function type. `invoke` takes the callable
first. Direct calls and closure construction substitute type, const,
lifetime, and evidence arguments and are checked against the full
signature. Caller and target share a profile; a cross-profile call needs an
explicit validated adapter.

## Primitive operations

Operators are sugar; the named canonical operation determines semantics.

| Family | Canonical operations |
|---|---|
| Aggregates | `tuple`, `vector`, `repeatVector`, `length`, `index`, `slice` |
| Modular arithmetic | `add`, `subtract`, `multiply`, `divide`, `modulo`, `negate` |
| Checked arithmetic | `checkedAdd`, `checkedSubtract`, `checkedMultiply`, `checkedDivide`, `checkedModulo`, `checkedNegate` |
| Overflow reporting | `overflowingAdd`, `overflowingSubtract`, `overflowingMultiply` |
| Bitwise | `bitwiseOr`, `bitwiseAnd`, `bitwiseXor`, `bitwiseNot` |
| Shifts | `shiftLeft`, `shiftRight`, `checkedShiftLeft`, `checkedShiftRight` |
| Boolean | `logicalAnd`, `logicalOr`, `logicalNot`, `implies`, `equivalent` |
| Comparison | `equal`, `notEqual`, `less`, `greater`, `lessEqual`, `greaterEqual`, `identical` |
| Conversion and ownership | `cast`, `checkedCast`, `copyValue`, `moveValue` |
| Logical range | `range` |

The printer uses operators and snake-case builtins (`left + right`,
`value as u8`, `overflowing_multiply(left, right)`, `slice(values, start,
stop)`) and never exposes `core.prim.*`. `length` accepts vectors, strings,
and bytes; string length is the number of UTF-8 bytes.

Checked operations carry their failure `ThrowKind`. Plain fixed-width
arithmetic is modular; plain division, modulo, and shifts are partial
outside their domain, and a frontend keeps the source language's assertion
as explicit control before the plain operation rather than swapping checked
and plain operations. The profile selects operator semantics: `+` lowers to
`checkedAdd[moveArithmeticError]` in Move and to modular `add` behind an
overflow assertion in Rust. Move arithmetic traps are the
`runtime.arithmetic_error` profile throw, which rolls back like other Move
failures but is distinct from `abort(code)`; contract clauses see
arithmetic and vector runtime errors as `EXECUTION_FAILURE` (`-1`).

## References and nominal data operations

```lean
core.ref.borrow(immutable, value)
core.ref.borrow(mut, value)
core.ref.dereference(reference)
core.ref.freeze(reference)
core.ref.freezeExplicit(reference)
core.ref.mutate(reference, value)
```

The frontend normalizes eligible value borrows to place-based borrows; where
that is impossible, capability preparation reports it rather than inventing
a temporary place.

Move-profile field and vector projections use Move 2's reference-transparent
syntax: `self.field`, `&mut self.field`, `values[index]`,
`&mut values[index]`. The canonical mutable element borrow is
`&mut self.values[index]`, not `vector::borrow_mut` and not
`&mut (*self).values[index]`. Reading `values[index]` is an immutable
borrow followed by a copy; assignment is `values[index] := value`. Move
indexes are `u64`; Rust indexes keep their pointer-sized type.

```lean
core.data.select[Type, field](value)
core.data.selectVariants[Type, Variant₁.field₁, ...](value)
core.data.testVariants[Type, Variant₁, ...](value)
core.data.discriminant[Type](value)
core.data.updateField(value, field, replacement)
```

The multi-variant operations express fields common to selected variants
and variant tests without integers; a value holding an unlisted variant is
a mismatch. `value.field` of an enum lists every variant declaring the
field, so the core form is printed only for a selection of some of them
(Move's `x.Variant.field`). Functional field updates are specification
expressions preserving the variant and the other fields; updates on
intrinsic maps and on fields absent from some variants are not supported.

## Global storage

The Move profile uses Move 2 index notation:

```lean
&Resource[address]
&mut Resource[address]
Resource[address]
Resource[address] := value
&mut Resource[address].field
Resource[address].field := value
```

A head resolving to an in-scope local is a vector place; a type-shaped head
denotes global storage. `exists<Resource>(address)`,
`move_from<Resource>(address)`, and `move_to<Resource>(signer, value)` keep
their builtins. `move_to` publishes under the address its signer holds,
`signer.address` (`core.prim.signerAddress(&signer)`), for a `Signer` or a
`&Signer`, in code and specifications alike. The first generic
instantiation identifies the stored resource family; the profile owns key
and storage rules (Move: an address key and a `Key` resource).

## Assertions and explicit drops

`assert condition;` is the typed core assertion: `Unit` when true, `abort`
with no arguments when false; it is not a raw MIR assertion terminator.
Raw assertions with other outcomes lower to structured conditional
`throw`s, and cleanup or unwind semantics are represented before
validation. The Move shape whose false branch aborts with a code is
`assert!(condition, code)`. Parameter and argument lists stay on one line
when they fit; otherwise the list starts on the next line, hangs two
columns, fills, and closes at the containing indentation. `drop(place)`
requires the profile's `Drop` rule and participates in initialization and
borrow checking; a backend omits visible drop syntax only when it proves the
target performs the same destruction.

## Profile extension nodes

`profile[profile-name, "tag", "payload"]` is the escape hatch for semantics
outside the Move/Rust union, admitted only in the extension positions VIR
has: constants, types, predicates, borrow, throw, and quantifier kinds, call
and operation extensions, surface provenance, declaration properties, and
profile-owned schema fields. An operation extension lists every declaration
target it may reach:

```lean
core.operation_extension profile[gpu, "shuffle", "xor"]
  targets[gpu::lane::shuffle](value, mask)
```

The registry validates profile identity, tag, payload, targets, types, and
capability. A known Move or Rust construct is promoted to a core node, never
hidden behind this syntax. The quoted family suffices for universal round
trip; a registered extension may add sugar but need not provide a parser
and printer pair.

## Specifications

Specifications share the executable expression, pattern, type, generic, and
name systems; logical-only types and operations extend that base.

### Mathematical integers

All specification arithmetic is over unbounded `Int`. Bounded executable
types (`UInt<64>`, Move `U64`, `SInt<32>`, Rust `i32`) widen to `Int`
automatically when their values enter a specification expression; authors
and canonical source print no `toInt`, `toNat`, or packing conversions.

```lean
fun increment (value : u64) -> u64 := value + 1

spec increment where
  ensures result = value + 1
  aborts_if value + 1 > MAX_U64
```

A specification names the bounds `MAX_U8` … `MAX_U256`, `MIN_I8` …
`MIN_I256`, and `MAX_I8` … `MAX_I256` (a local or module constant of the
same name takes precedence); canonical source prints 64-bit and wider bounds
by name and smaller ones as numerals. Widening is a specification
interpretation, not a `cast`, and cannot abort; it applies to equality,
ordering, shifts, vector lengths and indexes, resource fields, `old(...)`,
and specification-function arguments and results. Specification arithmetic
neither wraps nor reproduces executable failures; verification proves the
checked operation's success conditions separately.

A derived specification version of an executable function maps every direct
bounded integer parameter and result to `Int` (`increment.spec : Int ->
Int`). An authored specification function uses `Int` for direct integer
parameters and results (`spec fun distance (left : Int) (right : Int) :
Int := right - left`). A direct fixed-width parameter type is the
function's domain: the function is defined where each argument fits, and
its value elsewhere is unspecified (G15 in
[`prover-test-problems.md`](../status/prover-test-problems.md)); the body
reads such a parameter as an `Int`, a call passes any integer, and a
recursive definition unfolds inside the domain only. Widening applies at
integer leaves, not inside data types: `Vector<UInt<64>>` keeps its type,
and an extracted element widens where it enters arithmetic.

### Executable functions as specification functions

An executable `fun` is also a specification function when it has no effect
on global storage: it may read storage (the derived interpretation is then
evaluated against the specification caller's state, or its pre-state under
`old`), but not publish, remove, mutate, or mutably borrow a global. A call
from a specification position resolves to the derived interpretation; no
separate `spec fun` is needed. Where no translation exists, the use is an
error and the author provides a `spec fun` or an opaque model.

Abort behavior is not part of a derived specification function: assertions,
explicit aborts, and arithmetic failures impose no condition on its logical
result, and an aborting path has an unspecified value. The derived body is
lifted to `Int`, so `increment.spec : Int -> Int := fun value => value + 1`
describes the successful mathematical value; a caller needing success
proves the executable range and abort conditions in its contract.

### Specification declarations

```lean
spec fun f {T} (x : T) : Bool := expression
opaque spec fun predicate (x : T) : Bool
spec var ghost_count : Nat := 0
spec module where
  invariant proposition
  axiom proposition
```

A specification function with a body may omit its result type; the body's
type is then the result, and the function is not callable
(`VEREX-SPEC-RESULT-UNDECLARED`) nor part of the namespace's interface. A
proof step's expression lowers as such a function, in the scope of its
step.

Module invariants and axioms are members of one `spec module where` block.
An axiom is never an obligation; it is assumed at entry by every
verification of the unit, as the Move Prover states its axioms globally. An
invariant is assumed at entry and established at exit. A generic axiom,
`axiom {T} proposition`, is assumed at each instantiation at which the
verification applies a specification function the axiom applies, as the
Prover monomorphizes axioms. A generic invariant, `invariant {T} [update]
proposition`, holds at the instantiations that make a resource type it
reads one of the resource types the function reads or writes: a function
assumes those at entry and owes, after a write, those of the type written.
A type parameter a written type leaves undetermined is a ghost type
parameter of the function, proved over every frame; one that only read
memory leaves undetermined is not assumed. A Move spec variable `x` arrives
as the ghost resource `Ghost$x` the model backs it with, a struct with one
field `v` at address 0 whose existence is an axiom of the module;
`update x = e` is a write of that resource.

A specification function has a typed signature, optional body, locals,
contract, profile, origin, and profile data; a specification variable has
generic binders, type, profile, optional initializer, and scoped locals.
Namespace axioms and invariants are located `Condition`s with the same
`GenericBinder`s as other declarations. `NamespaceInvariant` is
semantically anonymous; a printer may generate a label, which is not
identity.

### Function contracts

```lean
spec f (x : T) where
  pragma partial
  requires pre
  ensures post
  aborts_if bad with code
  modifies resource_or_place
  reads T
```

| Group | Conditions |
|---|---|
| Bindings | `let_pre`, `let_post` |
| Local proof commands | `assert`, `assume`, `decreases`, `update` |
| Function behavior | `requires`, `ensures`, `aborts_if`, `aborts_with`, `succeeds_if`, `emits`, `function_invariant`, `loop_invariant` |
| Data and namespace | `struct_invariant`, `global_invariant`, `global_invariant_update`, `schema_invariant`, `axiom` |

A profile guide may adapt the surface vocabulary only when the adapted form
lowers to the same condition. A Rust-profile program never panics: every
Rust function has an implicit `aborts_if false`; `panic` stays
representable so the verifier proves it unreachable. A condition has one
principal expression plus named auxiliary expressions (an abort code, an
event payload); attributes on conditions preserve their properties.

A frame distinguishes omission from an explicitly empty frame:

```lean
modifies place_or_resource
modifies global<Resource>(addr), *
modifies *
reads Type
reads *
```

The mixed wildcard is a per-family frame: each listed family is closed at
its listed keys while unlisted families may change; `modifies *` opens
every family. The mixed spelling lowers to the modifies keys plus the
contract attribute `verex_loose_frame`, which the printer restores to the
surface marker. Invariant checking treats unlisted families as potentially
modified.

A function-typed parameter keeps global memory unless its contract declares
a footprint for it:

```lean
modifies_of<f>(a : Address) global<Resource>(a)
modifies_of<f> *
```

The formals bind an invocation's arguments; the clause lowers to a
`ParameterFrame`. A higher-order function's theorem assumes each
parameter's frame, and a caller establishes it of the value it passes.

In-body specification blocks use the same conditions at a program point
(`spec do` with `let`, `assert`, `assume`, `modifies`), or the inline `spec
assume …`, `spec assert …`, `spec invariant …`. Loop invariants attach to
the loop with `where`, logical `let` bindings preceding the invariants that
use them; a loop-owned `where` region is never an independent following
block. Verification checks an `assert` where it is stated and assumes it
afterwards; an `assume` is not checked. `spec assume
save_state_anchor!(n)` records the locals and state at its point, and
`with_state_anchor!(n, e)` in a later assertion reads them as `old` inside
`e`.

Pragmas are the leading entries of a contract. Verex does not support Move
schemas, so `include` is an ordinary identifier. Byte-vector pragma payloads
use byte strings (`pragma bv = b"0"` is the byte 48).

### Quantifiers

```lean
forall p in domain [where filter] [trigger (e₁, ...), ...], body
exists p in domain [where filter] [trigger (e₁, ...), ...], body
choose p in domain [where filter], body
choose_min p in domain [where filter], body
```

Each binder pairs a pattern with a domain; trigger groups are distinct
arrays. `forall` and `exists` produce `Bool`; `choose` and `choose_min`
produce the selected value. Registered profiles may add a checked
quantifier kind.

### Specification operations

The canonical `spec.*` vocabulary maps one-to-one to `SpecOperation`:

| Family | Operations |
|---|---|
| Calls and results | `functionCall`, `result` |
| Domains | `typeValue`, `typeDomain`, `resourceDomain`, `stateDomain` |
| State and frames | `global`, `canModify`, `old`, `final`, `saveStateAnchor`, `withStateAnchor`, `foldsCaptureAnchor`, `inlineCallSummary` |
| Function behavior | `requiresOf`, `abortsOf`, `ensuresOf`, `resultOf`, `unchangedOf`, `foldsOf`, `writeOf(index)` |
| Tracing | `traceUser`, `traceAutomatic`, `traceSubAutomatic` |
| Logical resources | `publish`, `remove`, `update` |
| Vectors | `emptyVector`, `singletonVector`, `updateVector`, `concatVector`, `indexOfVector`, `containsVector`, `inRange`, `inVectorRange`, `vectorRange` |
| Integers and bit vectors | `maxValue`, `bitVectorToInt`, `intToBitVector` |
| Execution observations | `abortFlag`, `abortCode`, `wellFormed` |
| Boxing | `boxValue`, `unboxValue` |
| Event stores | `emptyEventStore`, `extendEventStore`, `eventStoreIncludes`, `eventStoreIncludedIn` |
| Identity | `noOp` |

The printer uses standard spellings: `f::<T>(x)`,
`global<Resource>(address)`, `old(global<Resource>(address))`,
`requires_of<f>(x)`, `@before..@after |~ ensures_of<f>(x, result)`.
Function-behavior operations place the callable before its arguments in
VIR and keep their numeric memory-state anchors: `@state |~` is a single
pre-state, `@pre..@post |~` a two-state range. `write_of[index]` retains
the mutable-reference argument selected by `index` and is an internal
canonical form. `final(result)` names the final value of a returned mutable
reference, the value its lender holds once the caller's writes through it
end (`ensures cell.value == final(result)`); without `final`, a returned
reference reads as its current value. The shared checker types all
specification operations before a verification backend receives them.

## Intrinsics

An intrinsic declaration attaches a profile-owned model and two role graphs
to a nominal owner:

```lean
intrinsic map for Table using move where
  executable {
    new       := table::new
    borrow    := table::borrow
    borrowMut := table::borrow_mut
  }
  specification {
    model     := table::spec_model
    contains  := table::spec_contains
  }
```

Roles and targets are semantic data. Validation checks the owner, role
names, target policy, declaration categories, signatures, required roles,
and dependencies against the profile schema; a backend consumes the
validated graph and never rediscovers intrinsics from names.

## Semantics and validation

Frontend elaboration creates a located `RawUnit`. The shared validation and
preparation pipeline is authoritative for table bounds, acyclicity,
ownership, and uniqueness; type, pattern, place, operation, call, and
return checking; generic arity, binder sorts, abilities, and predicates;
declaration shape and associated-item completeness; structured control and
loop results; definite initialization, moves, copies, writes, and drops;
profile lifetime and borrow constraints; global and resource policy;
specification typing and intrinsic graphs; and semantic capability of every
reachable node. Lean elaboration may reject malformed syntax earlier, and
imported Rust has passed rustc admission, but neither replaces these
checks; a frontend never marks an unchecked assumption as a semantic fact.

Origin records where code came from; profile selects meaning. Two functions
with equal validated bodies and profile have equal VIR meaning whatever
their origin. Claims about a source artifact additionally require alignment
evidence.

## Canonical source

Canonical output consumes only `ValidatedUnit`, checked registries, and
source maps; prints headers, imports, binder sorts, and ambiguous operation
kinds; prefers sugar only where re-elaboration selects the same
constructor; names unnamed or colliding locals, lifetimes, and evidence
stably; preserves every semantic node; prints structured bodies; emits a
reverse source map from byte ranges to VIR IDs, provenance only in
sidecars; and reports a located capability error rather than dropping a
node. The required law is profile-aware alpha-equivalence:

```text
validate(frontend(printCanonical(validated))).semanticProjection
  ≈α
validated.semanticProjection
```

It does not require equal arena indexes, inferred lifetime IDs, generated
names, comments, formatting, origins, or receipts; those have separate
fidelity and freshness contracts.

## Profile guides

A profile guide specifies only what the profile owns: namespace conventions
and sugar; integer aliases; which operator spellings select modular,
checked, or asserted primitives; reference aliasing, lifetime, and escape
rules; ability derivation and destruction policy; global key and storage
rules; throw finalization and rollback or cleanup; admitted declaration
properties and extension nodes; backend compatibility. It does not redefine
common types, places, control, traits, conditions, or operations. The native
Rust source backend is a separate standard-Rust round trip.

## Further reading

- [`vir-design.md`](vir-design.md): the shared IR, profiles,
  frontend and backend contracts, round-trip laws.
- [`elaboration-design.md`](elaboration-design.md): runtime model, big-step
  semantics, interpreter.
- [`rust-mir-design.md`](rust-mir-design.md): Rust MIR frontend, Rust
  profile, native Rust source backend.
- [`VerexIR/Syntax.lean`](../../verex-ir/VerexIR/Syntax.lean): the core
  syntax inventory.
- [`VerexIR/Import/Raw.lean`](../../verex-ir/VerexIR/Import/Raw.lean): the
  raw frontend envelope and the non-source CFG representation.
