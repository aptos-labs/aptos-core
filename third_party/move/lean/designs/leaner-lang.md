# The Leaner Language

## Status

This document is the first design of the profile-aware Leaner source language
for the shared Leaner IR. It generalizes the Move-profile language described in
[`../move/Move/leaner-move.md`](../move/Move/leaner-move.md) to the complete
semantic union represented by
[`LeanerIR.Syntax`](../leaner-ir/LeanerIR/Syntax.lean).

The semantic boundary and the principal spellings in this document are design
decisions; much of the general surface syntax is not yet implemented. The
canonical Move `leaner module` and Rust `leaner namespace` headers, together
with a growing profile-selected expression and declaration subset, are
implemented. Remaining `core.*`, `spec.*`, extension-profile, and dependency
forms state the target language unless their sections say otherwise.

The language has two audiences:

1. People authoring profile-aware programs and specifications in Lean.
2. A canonical Leaner source backend which must be able to print every
   validated structured LIR unit without changing its meaning.

The second requirement keeps the first honest. Ergonomic sugar may be partial,
but every validated core construct needs an unambiguous canonical spelling.

## Purpose and boundary

Leaner is one Lean-hosted language with explicitly selected semantic profiles.
It is not a family of unrelated Move, Rust, and extension dialects.

```text
Leaner source --frontend--> RawUnit --validate--> ValidatedUnit
                                                 |
                                                 +--> execution
                                                 +--> verification
                                                 +--> executable backends
                                                 +--> canonical Leaner source
```

The source language denotes the semantic contents of a `ValidatedUnit`:

- namespaces and imports;
- constants, nominal data, traits, implementations, and functions;
- generic binders and predicates;
- structured executable bodies, places, patterns, and operations;
- specification functions and variables, contracts, frames, axioms, and
  invariants;
- intrinsic role graphs;
- attributes, pragmas, profile configuration, and checked profile extensions.

The following are not authored language constructs:

- arena indexes and strong IDs;
- raw basic blocks, `goto`, cleanup edges, and structurization witnesses;
- source hashes and byte offsets;
- imported-producer receipts and alignment evidence; and
- derived validation, initialization, lifetime, or borrow-checking facts.

Those belong to the raw interchange or validation envelope. A frontend may
accept a raw CFG, but canonical Leaner source always contains the validated
structured body. In particular, Leaner has no `goto` or program-counter
fallback.

The target coverage contract is:

> Every semantic non-extension constructor reachable in a `ValidatedUnit` has
> a canonical Leaner spelling. Every semantic extension constructor has one
> explicit, registry-checked profile spelling. No semantic node is silently
> erased or approximated to make source printing succeed.

Provenance-only fields use generated source maps and import receipts rather
than tokens or optional annotations in program text. The schema gaps identified
later in this document must be closed before an implementation can claim the
target coverage contract.

### Three syntax layers

The design distinguishes three layers which should not be conflated:

1. **Preferred surface syntax** is concise and profile-aware: operators,
   ordinary calls, structure literals, `&` borrows, and familiar contract
   clauses. It is intended for authored programs.
2. **Canonical core syntax** is complete and unambiguous: the named `core.*`,
   `spec.*`, and `profile[...]` forms in this document. These names, rather
   than an `lir%` quotation or ordinary Lean implementation names, are the
   stable canonical notation. A source backend falls back to it whenever a
   preferred spelling would lose information.
3. **Raw interchange syntax** is `RawUnit` JSON and may contain CFGs,
   producer receipts, arena IDs, and structurization inputs. It is not a
   source language and is never printed as Leaner code.

Both surface and canonical syntax elaborate through the same frontend into the
same raw LIR constructors. Canonical forms are not unsafe escape hatches: they
still pass through ordinary profile registration, structural validation,
typing, capability preparation, and borrow checking.

## Relationship to Leaner Move

[`leaner-move.md`](../move/Move/leaner-move.md) remains the user guide for the
Move profile. Its `module M at 0x42`, Move integer names, resources, `Action`,
`aborts_if`, and Move operators are profile-selected sugar over this language.
Whether that guide should eventually be reduced to a short profile supplement
is deliberately deferred; it remains a complete standalone manual for now.

For example, the existing header

```lean
module account at 0x42 where
```

is the Move shorthand for a namespace whose identity is `0x42::account` and
whose semantic profile is `move`. Existing Move programs should not become
more verbose merely because the core language becomes more general.

Conversely, a Rust-profile namespace must not acquire Move arithmetic, abort,
reference, storage, or destruction semantics just because their surface forms
look convenient. Sugar is admitted only where the selected profile defines the
same LIR node and meaning.

## Hosting in Lean

A source file is an ordinary Lean file importing the Leaner frontend:

```lean
import Leaner

open scoped Leaner
```

Leaner declarations and bodies are recognized Lean command and term syntax.
Lean performs parsing, macro expansion, dependency lookup, and frontend
construction, but validated LIR—not retained Lean syntax—is the semantic input
to execution, verification, and backends.

The language is hosted in Lean; it is not arbitrary Lean with best-effort
translation. An executable or specification expression which cannot be lowered
faithfully is a located frontend error. Ordinary Lean remains available for
metaprogramming, proofs, theorem statements, and explicitly designated
compile-time computation, but there is no semantic fallback which evaluates an
unrecognized Lean term and pretends it was LIR.

Elaborating a Lean source file can run metaprograms and host I/O. Tooling must
treat Leaner sources like trusted build scripts until the authorization and
sandboxing work described in
[`lir-design.md`](lir-design.md#todo-secure-lean-source-discovery-and-elaboration)
is complete.

## Compilation units, profiles, and namespaces

A compilation unit contains one table snapshot, one or more profile
configurations, owned namespaces, and checked dependency interfaces. Source
normally declares the owned namespaces; the frontend constructs the tables.

The canonical Rust-profile namespace header is:

```lean
leaner namespace example::collections where
  ...
```

The canonical Move-profile header uses `module`, a hexadecimal
address, and exactly one following module name:

```lean
leaner module 0x42::collections where
  ...
```

The header keyword selects the known profile; canonical source does not repeat
it with `using move` or `using rust`. The legacy explicit forms remain accepted
while generated and authored fixtures migrate. Registered extensions will use
a named, versioned configuration:

```lean
profile gpu version 2 where
  option target = "sm_90"
  option overflow = "trap"

leaner namespace example::kernel using extension gpu where
  ...
```

Profile options are semantic configuration, not arbitrary metadata. Validation
rejects unknown names, versions, duplicate configurations, and unsupported
options.

`using none` is the proposed header for a profileless namespace containing
only shared declarations or dependency metadata. Executable functions and
logical declarations still name a concrete profile; source must not infer one
from their origin.

Namespace paths are ordered arrays of strings. `::` is the normal source
separator; quoted segments cover names which are not Lean identifiers. A
profile interprets the segments—for example, Move treats leading segments as
an address or package alias, while Rust treats them as crate/module identity.
The underlying identity remains structural and is not reconstructed from a
display string.

Dependencies are explicit:

```lean
leaner namespace example::app where
  use example::collections
  use example::collections::map
  ...
```

`use` may import either a namespace or one symbol from a namespace. It creates
a source-level alias for already resolved LIR names; it is separate from Lean's
`import`, which loads the frontend. The canonical printer infers symbol imports
when the final name is unique and falls back to an unambiguous namespace import
when an unqualified symbol would collide, for example `mem::replace`.

A namespace has one profile. Each executable function and logical declaration
also carries its profile explicitly in LIR and inherits the namespace profile.
The initial authored language is profile-pure: declaration profile overrides,
cross-profile reference types, calls, and summaries are rejected. Their
surface design is deferred until validated adapter declarations exist. Origin
alone never selects a profile.

## Names

Declarations have a namespace identity and an interned local spelling. Source
uses `name` for a declaration in the current namespace and
`path::to::namespace::name` for a qualified reference.

Type names and value names occupy the declaration families enforced by LIR
validation. Trait associated types and associated values may share a spelling
because they occupy separate associated namespaces. Duplicate names within one
resolver family are rejected rather than resolved by declaration order.

Leaner keywords use Lean's quoted identifier syntax:

```lean
«match»
```

Producer-private locals and generic binders are alpha-renamed to ordinary
identifiers instead of forcing guillemets. Numeric tuple fields retain their
ordinary `.0`, `0 : T`, and `{ 0 := value }` spellings. Semantic round-trip is
alpha-equivalence for binders, not textual binder-name equality.

## Grammar overview

This is a compact grammar of the target general surface. Later sections
define typing, lowering, canonical fallback forms, and profile-selected sugar.
`[...]` means optional grammar, `...` means repetition, and capitalized words
such as `Expr` and `Type` stand for the categories described below rather than
arbitrary Lean terms.

```text
CompilationUnit ::=
  ProfileConfig* Namespace+

ProfileConfig ::=
  "profile" Ident "version" Nat "where" ProfileOption*

Namespace ::=
  "leaner" "module" MoveAddress "::" Ident "where" NamespaceItem*
  | "leaner" "namespace" Path "where" NamespaceItem*
  | "leaner" "namespace" Path "using" Profile "where" NamespaceItem*

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

The grammar deliberately has no raw CFG production. It also does not expose
interned IDs: names, patterns, places, expressions, types, and lifetimes are
resolved and interned by frontend elaboration.

Canonical blocks use the offside rule and do not print semicolons. When a
branch is itself a block, `do` stays on the branch header and the indented
sequence is its body:

```lean
if condition then do
  first()
  second()
else do
  fallback()
```

The parser continues to accept semicolons in migrated source where an
unambiguous legacy separator is useful.

## Attributes, pragmas, documentation, and comments

Attributes and pragmas share one recursive semantic representation. The
proposed source forms are:

```lean
@[inline]
@[derive(Eq, Hash)]
@[layout = "transparent"]
pragma overflow(mode = "checked")
```

An attribute is either a call with nested attribute arguments or an assignment
whose value is a constant, name, or qualified name. Placement distinguishes a
declaration attribute from a namespace/function/specification pragma; it does
not create a second attribute language.

Documentation and comments are provenance. Before Lean discards comment trivia,
ordinary file elaboration and the standalone formatter use the same scanner to
lower line and nested block comments to the LIR comment table. Canonical
printing preserves their text, multiplicity, and source order near the
declaration selected by their source location. Move declaration documentation
is rendered as adjacent `/-- ... -/` documentation comments, while ordinary
Rust-exporter comments remain Lean comments. Their text does not affect program
meaning, and semantic round-trip does not require byte-identical whitespace.

Namespace and module documentation is owned by the header rather than the
first declaration in its body. Canonical source therefore renders it as a
`/-! ... -/` module-doc comment immediately before `leaner namespace` or
`leaner module`; inferred `use` declarations remain inside the body, directly
after that header. The source scanner recovers this contiguous leading comment
group into the namespace documentation field, so a format or LIR round trip
cannot move it back below the header.

An operation may also retain a preferred surface hint: receiver-call notation,
index notation, or a registered extension hint. The hint never changes the
operation. A canonical printer honors readable sugar when doing so
re-elaborates to the same node; otherwise it prints the unambiguous named form.
Source maps retain the hint when exact presentation fidelity matters.

## Generic declarations

Leaner exposes all four LIR binder sorts:

```lean
{T}
{N : const}
{'a : lifetime}
{D : evidence}
```

The braces denote explicit generic binders, not implicit Lean arguments. Every
unannotated binder is a type binder, so canonical source omits the redundant
`: type`; the other binder sorts remain explicit.

Type binders may carry the four core abilities:

```lean
{T : type has Copy, Drop, Store}
```

Other constraints use a `where` block:

```lean
where
  T implements collections::Iterator<Item = U8>,
  collections::Iterator<T>::LIMIT = 64,
  'a outlives 'b,
  N = 4
```

The core predicate forms are:

| Predicate | Meaning |
|---|---|
| `T has Copy` | Core ability constraint. The other abilities are `Drop`, `Store`, and `Key`. |
| `T implements Trait<...>` | Trait implementation obligation. |
| `Trait<...>::Item = U` | Associated type equality. |
| `Trait<...>::CONST = value` | Associated constant equality. |
| `'a outlives 'b` | Lifetime-outlives constraint. |
| `A = B` | Closed const equality. |
| `profile[p, "tag", "payload"]` | Registered profile predicate outside the known union. |

Generic applications may contain type, const, lifetime, and evidence
arguments. The canonical form makes non-type arguments explicit when their
sort is otherwise ambiguous:

```lean
Buffer::<type U8, const 32, lifetime 'a, evidence D>
```

LIR const binders carry a required `TypeUse`. The initial general source keeps
the compact `{N : const}` spelling and lowers it to the canonical unsigned
pointer-width integer type (`UPtr`). A future surface extension may expose an
explicit binder type when a profile needs more than this portable first slice.

Evidence binders and arguments are part of ordinary authored source from the
first version; they are not restricted to generated canonical output. An
evidence name must resolve to a binder or to an entry in the unit's checked
evidence table, and application uses the explicit `evidence D` argument shown
above. Consequently the frontend work includes adding that table and the
authoritative implementation-selection boundary. Existing opaque
`EvidenceId`s must be migrated to table entries with stable generated names
before their units can claim canonical source round-trip; a numeric arena slot
is never accepted or printed as proof of a trait implementation.

## Types

Every type use carries a source location even when equal types share one
interned node. The canonical type vocabulary is:

| LIR type | Canonical Leaner spelling |
|---|---|
| Unit | `Unit` |
| Never | `Never` |
| Boolean | `Bool` |
| Unicode scalar | `Char` |
| Unicode string | `string` |
| Byte string | `Bytes` |
| Address | `Address` |
| Signer | `Signer` |
| Unsigned fixed integer | `UInt<width>` |
| Signed fixed integer | `SInt<width>` |
| Unsigned pointer integer | `UPtr` |
| Signed pointer integer | `IPtr` |
| Unsigned unbounded integer | `Nat` |
| Signed unbounded integer | `Int` |
| Tuple | `(T₁, ..., Tₙ)` |
| Dynamic vector | `Vector<T>` |
| Fixed vector | `Vector<T, const N>` |
| Logical range | `Range` |
| Logical event store | `EventStore` |
| Type domain | `TypeDomain<T>` |
| Resource domain | `ResourceDomain<R<...>>` |
| State domain | `StateDomain` |
| Nominal type | `path::Name::<...>` |
| Function value | `Fn(T₁, ..., Tₙ) -> R has A₁, ...` |
| Type parameter | its binder name |
| Shared reference | `&'a T` |
| Mutable reference | `&'a mut T` |
| Profile extension | `profile_type[p, "tag", "payload"]` |

`UInt<width>` and `SInt<width>` accept every nonzero fixed width representable
by LIR, not just the widths of one source language. Profile sugar includes
Move's `U8` through `U256` and Rust's `u8` through `u128`, `usize`, and signed
counterparts.

Although the core type union can represent both unbounded `Nat` and `Int`, the
integer domain selected automatically by specifications is always `Int`.
`Nat` is available only when a model explicitly asks for it; bounded unsigned
values are not implicitly widened to `Nat`.

A reference type also records its semantic profile. In the initial authored
language it must equal the enclosing namespace profile. The following explicit
cross-profile annotation is reserved for the later adapter design and is not
yet admitted:

```lean
&[rust] 'a mut T
```

Lifetimes are `'static`, named parameter lifetimes, `_` for inference, and
generated local lifetimes. Canonical printing may give a stable generated name
to a local lifetime when `_` would lose constraints.

Dynamic vectors and fixed vectors share one LIR type constructor. The optional
length is a closed const value; validation currently requires a nonnegative
integer. Tuples may be empty or unary. A function has one packed result type in
Leaner source; any imported physical result array is normalized to that type at
the source boundary.

Logical domain types are specification-only until their shared interpretation
is implemented. They are nevertheless first-class typed LIR nodes and must not
be replaced with profile strings.

## Constants and literals

Closed constants are:

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

The type at the use site determines an integer literal's width and signedness.
Validation rejects out-of-range fixed-width literals and invalid Unicode
scalars. Address spelling is profile-checked; the core retains its textual
identity rather than assuming one address parser for every profile.

`string` and `Bytes` join the known core type union. A string literal has type
`string`; a byte literal has type `Bytes`. They are distinct semantic types:
in particular, `Bytes` is not silently identified with
`Vector<UInt<8>>`. `Ty`, declarative `ValueHasType`, validation, JSON, and the
LeanerLang frontend/canonical printer all implement these cases as core nodes,
not profile-owned literal-typing hooks.

`sourceConstant` provenance may remember that a literal came from a named
source constant, but it does not change the value. A constant declaration is:

```lean
const MAX : u64 := 18446744073709551615
```

Its initializer is an ordinary typed expression root and is checked under the
owning namespace profile.

## Nominal data

Structures and enums use one nominal declaration family:

```lean
struct Pair {T} has Copy, Drop where
  first : T
  second : T

enum Option {T} has Copy, Drop where
  | none = 0
  | some (value : T) = 1
```

A declaration is exclusively struct-shaped or enum-shaped. Structs have
fields and no variants; enums have variants and no top-level fields. Enum
discriminants are optional integers and are observable values, not array
positions. Fields and variants are ordered and may use named or positional
source syntax.

Core abilities appear after `has`. Profile-owned declaration properties use
explicit registered annotations:

```lean
@[profile rust "repr" "C"]
```

A data contract attaches invariants to the nominal declaration:

```lean
spec Pair where
  invariant this.first == this.second
```

Move's certified-value interpretation is a Move-profile rule. Other profiles
may give data invariants a different admission or proof role, but the invariant
condition itself is shared LIR.

## Traits and implementations

Traits and implementations are first-class core declarations:

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

Associated items have three kinds: type, constant, and method. Types may have
bounds and defaults; constants have a type and optional expression default;
methods have a signature and may name an ordinary function as their default
implementation. Implementation bindings likewise name a type, expression, or
ordinary function.

Every associated item is owned exactly once by its trait. A local
implementation binds every required item. Validation checks ownership, kinds,
arity, first-order constraints, and default/binding types. Authoritative
implementation selection and evidence construction are part of the checked
frontend boundary required by authored evidence arguments; the source language
does not fabricate evidence merely because an `impl` with a matching display
name exists.

## Functions and signatures

An executable function declares a profile-aware signature and a structured
body:

```lean
fun swap_add {T} (left : u32, mut right : u32)
    -> (u32, Bool)
where
  T has Drop := do
  ...
```

Parameters are immutable unless marked `mut`. A signature may have zero or one
source-level result type. Multiple physical results from an imported IR are
canonicalized to one tuple type, and `return` supplies that tuple as one value.
The frontend/backend boundary performs the corresponding packing and
unpacking; Leaner source never exposes declaration-level result arrays.

Functions without an executable body are explicit:

```lean
opaque fun external_hash (bytes : Bytes) -> UInt<256>
```

`opaque` covers an absent body with an imported summary or unavailable
implementation. `native` covers an absent body supplied by the selected
profile's runtime. Checked profile data and alignment evidence retain any
additional implementation facts.

The common function modifiers are `private` (the default), `public`, `package`,
`friend`, `entry`, and `native`. The four visibility choices are mutually
exclusive; `entry` and `native` are independent flags, and a native function
has no executable body. These properties belong in the next typed core schema
revision rather than in opaque profile strings. Inline hints, Rust ABI, and
Rust safety remain profile data or registered attributes until a later design
shows that they need shared semantic fields. `opaque` remains the
profile-neutral spelling for an absent body whose reason is not specifically
native execution.

Function contracts are described below. Declaration pragmas, attributes,
profile data, origin, and alignment do not alter the structured body.

## Locals and lexical scope

Parameters occupy the leading local slots. Every local binding has a stable
declaration identity, name, type, mutability, and location:

```lean
let x : T := value
let mut y : T := value
let mut later : T
```

An uninitialized declaration is permitted in raw structured source, but every
read must pass path-sensitive definite-initialization checking. Branch joins
retain only initialization common to every fallthrough path. Direct-local
moves and drops consume; writes and matching bindings initialize.

Shadowing creates a new local identity. Canonical output may suffix the source
name to make distinct identities visible, but ordinary authored source uses
lexical scoping.

## Places and ownership operations

A place is storage, not a computed value. Its canonical grammar is:

```text
p ::= x
    | *p
    | p.field
    | p[e]
    | p[start..stop]
    | p[start..^stop]
    | downcast p as Type::Variant
```

`p[start..^stop]` means `start .. length(p) - stop`. A downcast refines an enum
place to one variant before selecting its fields. Tuple fields use constant
indexes through the same indexed-place representation.

The checked LIR ownership distinction is explicit. Canonical source keeps a
direct local ergonomic when its declaration already determines the access,
and spells projected ownership operations explicitly so partial moves and
non-consuming reads survive a fresh elaboration:

```lean
local                        -- direct local access inferred from checked LIR
move(place)                  -- consume this place
copy(place)                  -- Copy-qualified non-consuming load
read(place)                  -- Rust CopyForDeref/non-consuming place read
&place                       -- create a shared reference
&mut place                   -- create a mutable reference
place := value               -- write through a place
drop(place)                  -- explicitly destroy a Drop value
```

These operations are not inferred from a declaration's origin. The profile
and ability/borrow checker determine which are legal. Value-level copy/move
operations over an already evaluated operand remain distinct from consuming
or reading storage through a place.

Assignment is structured syntax rather than a generic operation:

```lean
place := value
(left, right) := pair
```

The second form is pattern assignment. Its pattern binds no new locals and is
checked against the existing local identities.

## Patterns

The shared pattern language is:

```text
_                              wildcard
x                              variable binder or assignment target
(p1, ..., pn)                  tuple
Type::<...> { field := p }     struct constructor
Type::Variant::<...>(p1, ...)  enum constructor
literal                        closed literal
lo..hi, lo..=hi, ..hi, lo..    range
```

Every pattern is typed and located. Constructor patterns resolve the same
nominal declarations, variants, generic arguments, abilities, and substituted
field types as constructor operations. Range endpoints are optional closed
constants; the inclusive flag is explicit.

Patterns occur in `let`, destructuring assignment, match arms, destructors,
and quantifier binders.

## Structured expressions and control

All validated bodies use the following tree forms:

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
loop ['label] ...
break ['label] [value]
continue ['label]
return value
abort arguments...
panic arguments...
throw profile[p, "tag", "payload"] arguments...
```

LIR stores `break` and `continue` targets as lexical nesting depths. Source
uses labels where needed; frontend resolution computes the depth. Loops are
expressions and may produce a value through `break`. A body which can fall
through must match its declared result type. A loop body, an `if` without
`else`, and a pattern assignment which can fall through must have `Unit` type.
Abrupt control is polymorphic through `Never`.

`for iterator in lower..upper do ...` is the canonical half-open range-loop
surface. Each bound is evaluated exactly once, in source order. Lowering keeps
core LIR small by expanding the surface form to typed locals, a `loop`, a
comparison, and the profile-selected increment operation. The printer recovers
the `for` spelling from that stable compiler expansion, so imported Move 2
range loops and re-elaborated Leaner source share one fixed point.
Lowering also inserts the implicit increment before each current-loop
`continue`; nested loops retain their own continue target. The printer hides
that administrative increment when it recovers the surface loop.

A block's last entry decides how its `return` reads. Written without a
semicolon it supplies that block's value, which is how a function body spells
its result; written as `return value;` it is a statement that leaves the
function. The two readings coincide in a function's own tail and differ
everywhere else, so the printer emits the semicolon exactly when the enclosing
block does not itself yield the returned value.

A statement or declaration whose own text ends with an indented block carries
no separator: a semicolon there attaches to that block's last entry, which
turns the block's value into a function exit.

`throw` has three shared classes: `abort`, `panic`, and registered profile
throws. The profile defines final-state behavior such as Move rollback or Rust
panic cleanup. Source syntax never substitutes one throw class for another.

There is no validated `goto`, raw switch, cleanup edge, resume, or unreachable
terminator. Reducible frontend graphs become `if`, `match`, and `loop`; an
unsupported graph is rejected before a source backend runs.

## Calls, construction, and closures

Canonical source uses ordinary calls and standard constructor/function-value
forms. Type arguments are omitted whenever argument types determine them:

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

Ordinary `f(args)`, structure/variant literals, destructuring patterns, and
function-value application are ergonomic spellings for these nodes. Canonical
forms remain available whenever sugar would lose the distinction.

When a field's value is the local with the same name, construction uses field
initialization shorthand: `new Type { field }`. A function whose first
parameter is named `self` prefers receiver notation, such as `value.method()`;
lowering inserts the required shared or mutable borrow when the receiver is a
value. Field, `.length`, and index receiver notation similarly accepts either
a collection value or a reference and inserts the required dereference, so
canonical source uses `values.length` and `values[index]`, not explicit
`(*values)` scaffolding. The checked standard-library signature table supplies
the same receiver and result information for imported Move vector functions
until dependency interfaces carry their authoritative signatures; this keeps
`values.contains(x)` and `values.push_back(x)` round-trippable without result
ascriptions. All declarations are predeclared before bodies are elaborated, so
forward and mutually recursive calls do not require a source `mutual` block.
After dependency constraints, canonical declarations retain source-location
order.

A closure names an ordinary function and captures its parameter prefix. Its
result type is the residual function type. `invoke` takes the callable as its
first operand. Direct calls and closure construction substitute type, const,
lifetime, and evidence arguments and are checked against the full signature.

Calls currently require caller and target to share a profile. Cross-profile
calls need an explicit validated boundary adapter rather than an origin-based
exception.

## Primitive operations

Operators are sugar; the named canonical operation determines semantics. The
complete shared primitive vocabulary is:

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

The integrated printer uses operators, mixins, and standard builtin names:

```lean
left + right
value >> distance
value as u8
overflowing_multiply(left, right)
slice(values, start, stop)
```

`length` accepts vectors, strings, and byte sequences. String length is the
number of UTF-8 bytes, so it agrees with Rust `str::len` and serialized Move
text rather than counting Unicode scalar values.

Checked operations carry their failure `ThrowKind` in the operation itself.
Plain fixed-width arithmetic is modular. Plain division, modulo, and shifts
are partial outside their admitted domain; a frontend such as Rust preserves
the source-language assertion as explicit control before emitting the plain
operation. It must not silently replace a plain operation with a checked Move
operation or vice versa.

The profile selects operator semantics. For example, the same `+` token may
lower to `checkedAdd[abort]` in Move and modular `add` behind a Rust overflow
assertion in Rust. When an operator alone would not determine the LIR
constructor, canonical source uses a standard snake-case builtin rather than
exposing `core.prim.*`.

## References and nominal data operations

Value-level reference operations are:

```lean
core.ref.borrow(immutable, value)
core.ref.borrow(mut, value)
core.ref.dereference(reference)
core.ref.freeze(reference)
core.ref.freezeExplicit(reference)
core.ref.mutate(reference, value)
```

The frontend normalizes eligible value borrows to stronger place-based borrows.
If that normalization is impossible, capability preparation reports it rather
than inventing a temporary place.

Move-profile field and vector projections use Move 2's reference-transparent
surface syntax. An explicit `*` is not needed merely to project through a
reference, and the borrow mode belongs on the complete projected place:

```lean
self.field
&self.field
&mut self.field
values[index]
&values[index]
&mut values[index]
```

Thus the canonical spelling of a mutable vector-element borrow is
`&mut self.values[index]`, not a call to `vector::borrow_mut` and not
`&mut (*self).values[index]`. Reading `values[index]` denotes the corresponding
immutable borrow followed by a copy; assignment uses `values[index] := value`.
The Move profile uses `u64` indexes, while Rust-profile indexing retains its
pointer-sized integer type.

Typed nominal data operations use qualified targets:

```lean
core.data.select[Type, field](value)
core.data.selectVariants[Type, field₁, ...](value)
core.data.testVariants[Type, Variant₁, ...](value)
core.data.discriminant[Type](value)
core.data.updateField[Type, field](value, replacement)
```

The multi-variant operations express fields common to selected variants and
variant tests without lowering through integers. A `selectVariants` field is
listed for each variant that defines the selected payload; fieldless variants
are omitted. `discriminant` returns the declared observable discriminant.

## Global storage

The Move profile uses Move 2 index notation for global borrows, reads, writes,
and field projections:

```lean
&Resource[address]
&mut Resource[address]
Resource[address]
Resource[address] := value
&Resource[address].field
&mut Resource[address].field
Resource[address].field
Resource[address].field := value
```

`Resource[address]` is context-sensitive with ordinary vector indexing. A
head which resolves to an in-scope local is a vector place; otherwise a
type-shaped head denotes global storage. This is resolved during lowering and
round-trips to the same LIR global-borrow and reference operations.

Operations without a Move 2 index equivalent retain their standard builtins:

```lean
exists<Resource>(address)
move_from<Resource>(address)
move_to<Resource>(signer, value)
```

The first generic instantiation identifies the stored resource family. The
profile owns key and storage rules. Move requires an address key and a resource
with `Key`; a Rust or extension profile may define different checked rules.
The syntax does not call every keyed global a Move resource.

## Assertions and explicit drops

Structured LIR has a typed assertion operation and explicit place destruction:

```lean
assert condition;
drop(place)
```

The typed core assertion returns `Unit` when its Boolean operand is true and
throws `abort` with no arguments when it is false. It is not a raw MIR
assertion terminator. Frontends lower supported raw assertions with other
outcomes to structured conditional `throw` behavior. Cleanup/unwind semantics
must be represented before validation; they are never hidden in source
provenance.

For the common Move assertion shape whose false branch aborts with a code,
canonical source uses the equivalent compact spelling:

```lean
assert!(condition, code)
```

Parameter and call argument lists remain on one line when the complete list
fits. Otherwise the list starts on the following line, hangs two columns from
the containing declaration or expression, fills as many entries as fit on a
line, and places the closing parenthesis at the containing indentation.

Drop requires the profile's `Drop` rule and participates in initialization and
borrow checking. A backend may omit visible drop syntax only when it proves the
target language performs the same destruction at that point.

## Profile extension nodes

`ProfileValue` is the escape hatch for semantics outside the known Move/Rust
union. Its canonical value spelling is:

```lean
profile[profile-name, "tag", "payload"]
```

It may appear only in the extension positions admitted by LIR: constants,
types, predicates, borrow/throw/quantifier kinds, call and operation extensions,
surface provenance, declaration properties/metadata, and profile-owned schema
fields.

An operation extension also lists every qualified declaration target it may
reach:

```lean
core.operation_extension profile[gpu, "shuffle", "xor"]
  targets[gpu::lane::shuffle](value, mask)
```

The active registry validates profile identity, tag, payload, targets, types,
and semantic capability. A known Move or Rust construct must be promoted to a
core node rather than hidden behind this syntax. The quoted
`profile[p, "tag", "payload"]` family is sufficient for universal round-trip;
a registered extension may add readable sugar, but it is not required to
provide a dedicated parser/printer pair.

## Specifications

Specifications share the executable expression, pattern, type, generic, and
name systems. Logical-only types and operations extend that common base; they
do not form a detached untyped AST.

### Mathematical integers

All integer arithmetic in the specification language is over mathematical,
unbounded `Int`. Bounded executable types such as `UInt<64>`/Move `U64` and
`SInt<32>`/Rust `i32` are widened to `Int` automatically when their values
enter a specification expression. Authors and canonical source do not print
routine `toInt`, `toNat`, or packing conversions.

```lean
fun increment (value : u64) -> u64 := value + 1

spec increment where
  ensures result = value + 1
  aborts_if value + 1 > spec.maxValue[64]
```

Inside both clauses, `value`, `result`, `1`, and the fixed-width maximum
participate in `Int` arithmetic. The source types on the attached executable
function remain bounded; the specification elaborator inserts the logical
projections at the boundary. This widening is a specification interpretation,
not an executable `cast`, and it cannot abort.

Explicit widening casts to `Int` are omitted as well: widening is implicit and
cannot fail. The same rule applies to equality and ordering, shifts, vector lengths and
indexes, resource fields, `old(...)`, and arguments/results of specification
functions. Specification arithmetic neither wraps nor reproduces executable
overflow, division, shift, or cast failures. Contracts state mathematical
values; verification separately proves the checked executable operation's
success conditions and declared throw behavior.

A derived specification version of an executable function maps every direct
bounded integer parameter and result to `Int`. For example, the effective
logical signature of `increment` above is:

```lean
increment.spec : Int -> Int
```

An authored specification function must use `Int` explicitly for direct
integer parameters and results:

```lean
spec fun distance (left : Int) (right : Int) : Int := right - left
```

Declaring a direct `UInt<n>`, `SInt<n>`, `UPtr`, or `IPtr` parameter/result on
`spec fun` is an error. There is no implicit bounded re-packing on a
specification-function call: an `Int` expression remains mathematical.

Widening is applied at integer leaves, not by recursively replacing every
integer nested in a data type. `Vector<UInt<64>>` remains that vector type, and
a nominal field retains its declared bounded type; an extracted element or
field widens when it participates in specification arithmetic, comparison, or
an `Int` argument. Explicit representation views may remain available for
low-level models, but they are compatibility/implementation vocabulary rather
than the normal specification language.

### Executable functions as specification functions

A regular executable `fun` may also be used as a specification function when
it has no effect on global storage. In particular, its execution must not
publish, remove, or mutate a global value, nor obtain a mutable global borrow.
The frontend derives a logical interpretation of the function body and a call
to the regular function from a specification position resolves to that
interpretation. No separate `spec fun` declaration is required.

Reading global storage does not itself disqualify a function: a derived
interpretation that reads storage is stateful and is evaluated against the
specification caller's current state (or its pre-state beneath `old`). It still
cannot change that state. Profiles may reject additional executable effects
that they cannot translate into a pure logical value. If no such translation
exists, using the regular function in a specification is an error and the
author must provide an explicit `spec fun` or opaque logical model.

Abort behavior is deliberately not part of a derived specification function.
Assertions and explicit aborts impose no condition on its logical result; a
path that would abort executably has an unspecified value. Likewise, the
derived interpretation does not reproduce overflow, underflow, division,
shift, cast, or other arithmetic aborts. A function contract's `aborts_if` and
checked executable semantics specify and verify those behaviors separately.

All arithmetic in the derived body is lifted to mathematical `Int`, and every
direct bounded-integer parameter and result has the widened logical type
described above. For example:

```lean
fun increment (value : u64) -> u64 := value + 1

-- Conceptual derived interpretation; normally not printed or authored.
increment.spec : Int -> Int := fun value => value + 1
```

The derived function therefore describes the successful mathematical value,
not whether the executable function succeeds. A caller that needs to establish
success must prove the executable operation's range and abort conditions in
its contract.

### Specification declarations

The declaration families are:

```lean
spec fun f {T} (x : T) : Bool := expression
opaque spec fun predicate (x : T) : Bool
spec var ghost_count : Nat := 0
axiom name {T} where proposition
spec namespace where invariant proposition
```

A specification function has a physical typed signature, optional body,
locals, contract, profile, origin, and profile data. A specification variable
has generic binders, type, profile, optional initializer, and scoped locals.
Namespace axioms and global/update invariants are located `Condition`s rather
than untyped declarations.

Namespace axioms and global invariants use the same strong `GenericBinder`s as
other declarations. The current schema's string binders must therefore be
promoted in the specification-binder migration. `NamespaceInvariant` remains
semantically anonymous; a canonical printer may generate a stable local label,
but that label is not declaration identity and need not survive semantic round
trip. The `name` in the `axiom` surface form is such a proof-local label.

### Function contracts

A function contract contains ordered conditions, an optional frame, and
pragmas:

```lean
spec f (x : T) where
  pragma partial
  requires pre
  ensures post
  aborts_if bad with code
  modifies resource_or_place
  reads T
```

The complete shared condition roles are:

| Group | Conditions |
|---|---|
| Bindings | `let_pre`, `let_post` |
| Local proof commands | `assert`, `assume`, `decreases`, `update` |
| Function behavior | `requires`, `ensures`, `aborts_if`, `aborts_with`, `succeeds_if`, `emits`, `function_invariant`, `loop_invariant` |
| Data and namespace | `struct_invariant`, `global_invariant`, `global_invariant_update`, `schema_invariant`, `axiom` |

These common roles keep the established Move clause spellings where they read
naturally. A profile guide may adapt their surface vocabulary and resource
notation, but only when the adapted form lowers to the same shared condition.
In particular, a Rust-profile program is required never to panic: every Rust
function has an implicit `aborts_if false` covering panic and other abrupt
failure. An imported or authored `panic` remains representable so that the
verifier can prove it unreachable; it is not an admitted behavior that a Rust
contract may weaken or omit.

A condition has one principal expression plus named auxiliary expressions.
For example, an abort code and an event payload are auxiliary operands rather
than positional text understood only by a printer. Attributes on conditions
preserve condition properties.

A frame distinguishes omission from an explicitly empty frame:

```lean
modifies place_or_resource
modifies *
reads Type
reads *
```

In-body specification blocks use the same conditions, pragmas, and frame at a
structured program point:

```lean
spec do
  let logical_name := expression
  assert condition
  assume condition
  modifies place
```

Loop invariants are in-body conditions attached to the loop program point, not
comments recovered from a printer. Canonical source attaches their complete
specification region with `where`; logical `let` bindings precede the
invariants which use them:

```lean
while condition do
  body
where
  let upper := expression
  invariant value <= upper
```

A single in-body condition uses the inline `spec assume ...`, `spec assert
...`, or `spec invariant ...` form; multiple independent conditions use
`spec do`. A loop-owned `where` region contains one or more logical `let`
bindings and loop invariants and is never interpreted as an independent
following specification block.
Pragmas are always the leading entries of a declaration contract. Leaner does
not support Move schemas, so `include` is an ordinary identifier rather than a
soft keyword. Printable byte-vector pragma payloads use byte strings—for
example, `pragma bv = b"0"` denotes the single byte with value 48.

### Quantifiers

Quantifiers bind typed patterns over explicit domains:

```lean
forall p in domain [where filter] [trigger (e₁, ...), ...], body
exists p in domain [where filter] [trigger (e₁, ...), ...], body
choose p in domain [where filter], body
choose_min p in domain [where filter], body
```

Each binder pairs a pattern with a domain expression. Trigger groups remain
distinct arrays. `forall` and `exists` produce `Bool`; `choose` and
`choose_min` produce the selected value. Registered profiles may add a checked
quantifier kind.

### Specification operations

The canonical `spec.*` vocabulary maps one-to-one to `SpecOperation`:

| Family | Operations |
|---|---|
| Calls and results | `functionCall`, `result` |
| Domains | `typeValue`, `typeDomain`, `resourceDomain`, `stateDomain` |
| State and frames | `global`, `canModify`, `old`, `saveStateAnchor`, `withStateAnchor`, `foldsCaptureAnchor`, `inlineCallSummary` |
| Function behavior | `requiresOf`, `abortsOf`, `ensuresOf`, `resultOf`, `unchangedOf`, `foldsOf`, `writeOf(index)` |
| Tracing | `traceUser`, `traceAutomatic`, `traceSubAutomatic` |
| Logical resources | `publish`, `remove`, `update` |
| Vectors | `emptyVector`, `singletonVector`, `updateVector`, `concatVector`, `indexOfVector`, `containsVector`, `inRange`, `inVectorRange`, `vectorRange` |
| Integers and bit vectors | `maxValue`, `bitVectorToInt`, `intToBitVector` |
| Execution observations | `abortFlag`, `abortCode`, `wellFormed` |
| Boxing | `boxValue`, `unboxValue` |
| Event stores | `emptyEventStore`, `extendEventStore`, `eventStoreIncludes`, `eventStoreIncludedIn` |
| Identity | `noOp` |

The integrated source printer uses standard function/global spellings for the
currently accepted range-free subset:

```lean
f::<T>(x)
global<Resource>(address)
old(global<Resource>(address))
requires_of<f>(x)
@before..@after |~ ensures_of<f>(x, result)
```

Forms such as `old(e)`, `result`, vector notation, and resource indexing lower
to these nodes. Function-behavior operations place the callable before its
arguments in LIR and retain their numeric pre/post memory-state anchors. A
single pre-state is written `@state |~`; an explicit two-state range is written
`@pre..@post |~`. `write_of[index]` retains the mutable-reference argument
selected by `index`; it is an internal canonical form rather than a Move source
operator.

The shared checker types all specification operations before a verification
backend receives them. Interpretation of the complete logical vocabulary is
still an implementation milestone; surface representability does not claim
that every well-typed specification can already be proved.

## Intrinsics

An intrinsic declaration attaches a profile-owned model and two explicit role
graphs to a nominal owner:

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

Roles and targets are semantic data. Validation checks the owner, unique role
names, unique/shared-target policy, declaration categories, signatures,
required roles, and role dependencies against the selected profile schema.
A backend consumes the validated graph; it does not rediscover intrinsics from
names or source pragmas.

## Semantics and validation

Frontend elaboration creates a located `RawUnit`. The shared validation and
semantic-preparation pipeline is authoritative for:

- table bounds, acyclicity, ownership, and name uniqueness;
- type, pattern, place, operation, call, and return checking;
- generic arity, binder sorts, abilities, and first-order predicates;
- declaration shape and associated-item completeness;
- structured control and loop-result checking;
- definite initialization, moves, copies, writes, and drops;
- profile lifetime and borrow constraints;
- global/resource policy;
- specification typing and intrinsic graphs; and
- semantic capability for every reachable core or extension node.

Lean elaboration may reject malformed surface syntax earlier, and imported
Rust must already have passed rustc admission, but neither replaces these
shared checks. A source frontend must not mark an unchecked assumption as a
semantic fact.

Origin records where code came from; profile selects meaning. Two functions
with equal validated bodies and profile have equal LIR meaning even if one came
from Move, one from Leaner source, and one from imported MIR. Claims about the
source artifact additionally require alignment evidence.

## Canonical source

Canonical Leaner output obeys these rules:

1. It consumes only `ValidatedUnit` plus checked profile registries and source
   maps.
2. It prints profile-selecting namespace/module headers, imports, generic
   binder sorts, and ambiguous operation kinds.
3. It prefers readable profile sugar whenever re-elaboration selects exactly
   the same LIR constructor under the printed profile, and otherwise falls
   back to the named canonical core form.
4. It gives stable generated names to locals, lifetimes, and evidence when
   authored names are absent or collide.
5. It preserves all semantic declarations, conditions, frames, intrinsics,
   anchors, and extension nodes.
6. It prints a structured body and never raw CFG control.
7. It emits a reverse source map from generated UTF-8 byte ranges to LIR
   namespace/declaration/node IDs and `LocId`s. Origins, alignment, and import
   receipts remain exclusively in sidecars; canonical program text has no
   optional provenance annotations.
8. It reports a located capability error rather than dropping a node for
   which the selected frontend has no round-trip spelling.

The required law is semantic, profile-aware alpha-equivalence:

```text
validate(frontend(printCanonical(validated))).semanticProjection
  ≈α
validated.semanticProjection
```

It does not require equal arena indexes, inferred lifetime IDs, generated local
names, comments, formatting, origins, or alignment receipts. Those have
separate fidelity and freshness contracts.

## Profile guides

A profile guide specifies only the choices genuinely owned by that profile:

- admitted namespace conventions and source sugar;
- fixed and pointer integer aliases;
- which operator spellings select modular, checked, or asserted primitives;
- reference aliasing, lifetime, and escape rules;
- ability derivation and destruction policy;
- global key/storage rules;
- throw finalization and state rollback/cleanup;
- admitted declaration properties and extension nodes; and
- backend compatibility.

It does not redefine common types, places, structured control, traits,
conditions, or known operations. The Move guide is
[`leaner-move.md`](../move/Move/leaner-move.md). A corresponding Rust-profile
guide should be written when the Leaner Rust surface is implemented; the native
Rust source backend remains a separate standard-Rust round trip.

## Completeness against `LeanerIR.Syntax`

The intended source coverage is summarized below.

| LIR family | Source section | Status |
|---|---|---|
| Profiles, namespaces, names, imports | Compilation units, profiles, and namespaces | Canonical Move `leaner module 0xADDRESS::name` and Rust `leaner namespace path` headers, structural paths, inferred `use` aliases, and checked profile defaults are implemented; additional profile forms remain |
| Cross-profile declarations and references | Compilation units, profiles, and namespaces; Types | Authored adapters are deferred; the initial language is profile-pure |
| Core and extension types/constants | Types; Constants and literals | Core scalar/tuple/vector syntax includes checked `string`/`Bytes`; remaining core and extension forms need surface coverage |
| Generic binders, arguments, predicates | Generic declarations | Four binder sorts parse; const binders lower with canonical `UPtr` type; evidence still needs a checked table and applications/predicates need full surface coverage |
| Structures and enums | Nominal data | Profile-general frontend and canonical-printer subsets cover fields, variants, discriminants, and abilities |
| Traits, associated items, implementations | Traits and implementations | Schema/checking first slice exists; Leaner surface proposed |
| Places and patterns | Places and ownership operations; Patterns | Schema/checking implemented; profile-general canonical spelling decided |
| Structured expression/control forms | Structured expressions and control | Schema/semantics, Move surface, canonical `while`, and half-open `for` lowering/recovery are implemented; labels and remaining profile-specific recovery need completion |
| Calls, closures, globals, references, data | Operation sections | Schema/checking/executable slices exist; canonical named spelling decided |
| Primitive operations | Primitive operations | Closed core inventory implemented; `core.*` named syntax decided |
| Conditions, frames, spec blocks | Specifications | Schema/type checking exists; full interpretation remains M4/M5 work |
| Quantifiers and spec operations | Specifications | Closed core inventory/type checking exists; full interpretation deferred |
| Constants, functions, spec declarations, spec vars, invariants | Declarations and Specifications | Source model decided; invariant binders still need promotion to `GenericBinder` |
| Function results and modifiers | Functions and signatures | Tuple-normalized results and visibility/entry/native/opaque source roundtrip are implemented over current checked metadata; typed core fields remain a schema cleanup |
| Intrinsic declarations | Intrinsics | Move map schema implemented; general syntax proposed |
| Attributes, pragmas, comments | Attributes, pragmas, documentation, and comments | Schema, shared Lean-file/formatter comment extraction, Rust source-comment import, Move documentation emission, source-order placement, and pragma-first contracts are implemented |
| Profile extension positions | Profile extension nodes | Registry mechanism implemented; universal quoted spelling decided |
| Raw CFG forms and evidence envelopes | Purpose and boundary | Deliberately not source language |

## Further reading

- [`../move/Move/leaner-move.md`](../move/Move/leaner-move.md) — implemented
  Move-profile language.
- [`../move/Move/int-widening-design.md`](../move/Move/int-widening-design.md) —
  conversion-free `Int` arithmetic at the specification boundary.
- [`lir-design.md`](lir-design.md) — shared IR architecture,
  profiles, frontend/backend contracts, and round-trip laws.
- [`elaboration-design.md`](elaboration-design.md) — migration to LIR-owned
  execution, verification, and generated Lean declarations.
- [`rust-mir-design.md`](rust-mir-design.md) — Rust MIR frontend, Rust
  profile, and native Rust/Leaner source backend requirements.
- [`../leaner-ir/LeanerIR/Syntax.lean`](../leaner-ir/LeanerIR/Syntax.lean) —
  authoritative current core
  syntax inventory.
- [`../leaner-ir/LeanerIR/Import/Raw.lean`](../leaner-ir/LeanerIR/Import/Raw.lean)
  — raw frontend envelope
  and deliberately non-source CFG representation.
