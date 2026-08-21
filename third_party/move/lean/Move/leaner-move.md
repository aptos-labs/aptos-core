# The Leaner Move Language

Leaner Move is a Move dialect embedded in Lean 4. A Leaner module is ordinary
Lean source: every construct below is a Lean syntax fragment, elaborated by
Lean and lowered by the Leaner compiler to Move IR and bytecode. Host-language
constructs outside this definition (closures, `Nat`, recursive data, `IO`,
dependent runtime values, ...) are rejected at the compilation boundary with
source-positioned diagnostics rather than reinterpreted.

This document defines the deployable language: grammar, types, values,
expressions, functions, and modules. The specification language (`spec`,
`verify`) is summarized under [Modules](#specifications); its semantics is
defined in [`verification-design.md`](verification-design.md). Example-driven
introductions are in [`README.md`](README.md).

## Grammar

The EBNF uses `[x]` for option, `{x}` for repetition, `|` for alternatives,
and quoted terminals. `ident`, `nat`, and `doc-comment` are Lean's lexical
classes; layout (newlines and indentation) follows Lean's `do` and command
conventions. Where a production says "Lean expression grammar applies", the
listed forms are the Move-recognized subset within Lean's precedence rules.

```ebnf
module          = "move_module" ident "where" { item } ;

item            = struct-decl | enum-decl | fun-decl
                | spec-decl | verify-decl
                | lean-decl                     (* Lean-only helper: def, theorem, ... *)
                | "namespace" ident { item } "end" ident
                | compile-directive ;

(* ---- attributes ---- *)

attributes      = "@[" attribute { "," attribute } "]" ;
attribute       = ident { attr-arg } ;
attr-arg        = ident | nat
                | "(" ident { attr-arg } ")" ;    (* instantiated type *)

(* ---- data types ---- *)

struct-decl     = [ doc-comment ] [ attributes ]
                  "struct" ident { type-param } "where"
                  { ident ":" type } [ deriving-abilities ] ;

enum-decl       = [ doc-comment ] [ attributes ]
                  "enum" ident { type-param } "where"
                  { "|" ident ( { "(" ident { ident } ":" type ")" }
                              | ":" type { "→" type } ) }
                  [ deriving-abilities ] ;

type-param      = "(" ident { ident } [ ":" "Type" ] ")" ;
deriving-abilities = "deriving" ability { "," ability } ;
ability         = "Copy" | "Drop" | "Store" | "Key" ;

(* ---- functions ---- *)

fun-decl        = modifiers [ "entry" | "friend" ] "fun" ident
                  { generic-param } { value-param }
                  ":" result-type ":=" body ;
modifiers       = [ doc-comment ] [ attributes ] [ "public" ] [ "partial" ] ;
generic-param   = "{" ident { ident } [ ":" "Type" ] "}" ;
value-param     = "(" ident { ident } ":" param-type ")" ;
param-type      = type | "&" type | "&mut " type ;
result-type     = type | "&" type | "&mut " type
                | "Action" type-atom ;
body            = expr | do-block ;

(* ---- types ---- *)

type            = int-type | "Bool" | "Address" | "Signer"
                | "Move.Vector" type-atom
                | ident { type-atom }           (* struct/enum instantiation *)
                | "&" type | "&mut " type
                | "(" type ")" ;
type-atom       = ident | "(" type ")" ;
int-type        = "U8" | "U16" | "U32" | "U64" | "U128" | "U256" ;

(* ---- statements ---- *)

do-block        = "do" do-seq ;
do-seq          = do-elem { do-elem } ;         (* newline/indentation separated *)
do-elem         = "let" [ "mut" ] ident [ ":" type ] ":=" expr
                | "let" [ "mut" ] ident "←" expr
                | ident ":=" expr               (* write / local reassignment *)
                | "if" expr "then" do-seq [ "else" do-seq ]
                | "while" expr "do" do-seq
                | "loop" [ "@" ident ] do-seq
                | "break" [ "@" ident ]
                | "continue" [ "@" ident ]
                | "return" expr
                | expr ;                        (* call, abort, tail continue, result *)

(* ---- expressions (Lean expression grammar applies) ---- *)

expr            = literal | ident | "(" expr ")"
                | expr arith-op expr
                | expr bit-op expr
                | expr cmp-op expr
                | expr bool-op expr
                | "(" expr ".cast" ":" int-type ")"  (* checked width cast *)
                | borrow | "*" expr
                | "abort" expr
                | "pure" expr                   (* Action result *)
                | "vector![" [ expr { "," expr } ] "]"
                | struct-value | enum-value
                | call | receiver-call | field-access
                | "continue" ident { expr }     (* marked tail self-call *)
                | "if" expr "then" expr "else" expr
                | "match" expr "with" { "|" pattern "=>" expr } ;

arith-op        = "+" | "-" | "*" | "/" | "%" ;
bit-op          = "&&&" | "|||" | "^^^" | "<<<" | ">>>" ;
cmp-op          = "<" | "<=" | "==" ;
bool-op         = "&&" | "||" ;

borrow          = ( "&" | "&mut " ) place ;
place           = ident                         (* local *)
                | ident "[" expr "]"            (* vector element or Resource[addr] *)
                | place "." ident ;             (* field *)

call            = qualified-ident { "(" ident ":=" type ")" } { expr } ;
receiver-call   = expr "." ident { expr } ;
field-access    = expr "." ident ;
struct-value    = "{" field-init { "," field-init } "}" ;
field-init      = ident ":=" expr | ident ;     (* shorthand for f := f *)
enum-value      = "." ident { expr } | qualified-ident { expr } ;
pattern         = "." ident { ( ident | "_" ) } | ident | "_" ;
literal         = nat | "true" | "false" ;

(* ---- specifications ---- *)

spec-decl       = "spec" ident { spec-binder } "where" spec-clauses ;
spec-binder     = "{" ident [ ":" "Type" ] "}"
                | "[" term "]"                  (* instance assumption *)
                | "(" ident ":" param-type ")" ;
spec-clauses    = "ensures" term                (* only a postcondition:
                                                   pure value predicate, or an
                                                   effectful contract if `f`
                                                   returns `Action _` *)
                | [ "requires" term ";" ] [ modifies-clause ]
                  "ensures" term
                  [ ";" aborts-clause ] ;      (* omitted: uninterpreted *)
modifies-clause = "modifies" modifies-target { "," modifies-target } ";" ;
modifies-target = ident [ "[" expr "]" ] ;      (* family, or one address *)
aborts-clause   = "aborts_if" term [ "with" term ]
                  { ";" "aborts_if" term "with" term }
                | "aborts" term ;
data-invariant  = "spec" ident spec-binder* "where"
                  "invariant" term { ";" "invariant" term } ;
                  (* struct or enum type; `this` is the value, `.field` is
                     `this.field`; patterns of an enum with an invariant
                     bind its proof with a trailing `_` *)
global-invariant = "spec" "global" "where"
                  "invariant" invariant-pred { ";" "invariant" invariant-pred } ;
invariant-pred  = "(" "all" ident ":" term ")"          (* regular *)
                | "update" "(" "all" ident ":" term ")" ; (* update *)
                  (* `term` ranges over the address `ident`, using `R[a]` and
                     `exists<R>(a)`; the `update` form may also use `old(R[a])`.
                     Semantics under Specifications. *)
verify-decl     = "verify" ident [ "by" tactic-seq ] ;
                                                (* tactic-seq: Lean tactics *)

(* ---- compilation directives ---- *)

compile-directive = "#export_leaner" string [ selection ]
                | "def" ident ":" "MModule" ":="
                  "move_module%" string [ selection ]
                | "#emit_leaner_xir" ident ;
selection       = "structs" "[" [ ident { "," ident } ] "]"
                  "functions" "[" [ ident { "," ident } ] "]" ;
```

## Types

The base types and type constructors are:

| Type | Meaning |
|---|---|
| `U8` ... `U256` | unsigned integers with aborting (non-wrapping) arithmetic |
| `Bool` | Move boolean |
| `Address` | account address |
| `Signer` | transaction signer capability |
| `Move.Vector T` | homogeneous growable vector, length certified within the `u64` domain |
| `&T`, `&mut T` | immutable resp. mutable reference (parameter/result positions) |
| `Action T` | effectful computation returning `T` |

All six Move widths (`U8`, `U16`, `U32`, `U64`, `U128`, `U256`) are
supported. They are one generic type: `UInt W` is the subtype of signed
unbounded integers within the width's range
(`{ x : Int // 0 ≤ x ∧ x < 2 ^ bits }`), indexed by a type-level width name
`W8` ... `W256` — the compiler's intermediate representation erases value
indices from types, so the width is carried as a type. A value's range bound
is certified at construction and available in every proof (`x.toNat < 2^n`
with no hypothesis tracking). These are dedicated types — deliberately not
Lean's wrapping `UInt64`. `Address` and `Signer` are opaque; `Signer` values
enter only as entry-function arguments.

**Structures.** `struct Name ... where ... deriving ...` declares a Move
struct. Fields must be Move-representable; recursive structures are rejected.
Abilities are exactly the derived `Copy`, `Drop`, `Store`, `Key` markers —
`struct` carries no implicit abilities. A *resource* is a structure
deriving `Key`. Generic ability constraints are inferred from non-phantom
field usage: deriving `Copy, Drop, Store` bounds each used parameter by those
abilities, deriving `Key` bounds used parameters by `Store`, and phantom
parameters receive no bounds. Every Move structure implicitly derives Lean's
`Inhabited`; this host detail is erased during lowering.

**Enums.** `enum Name ... where` declares a native Move enum with
constructor payloads in either named-binder or arrow form. Recursive, indexed,
and empty enums are rejected; non-recursive generic enums are supported.
Borrowing a field directly out of an enum variant is not supported.

**References.** `&T` and `&mut T` elaborate to the opaque `Move.Ref T` and
`Move.MutRef T`. References appear in parameter and result positions only:
they cannot be stored in structures or vectors, and entry functions cannot
take or return them where Move forbids it. Passing a `&mut T` where `&T` is
expected inserts an implicit freeze (lowered to `freeze_ref`).

**Generics.** Function type parameters are declared `{T}` and struct or enum
parameters `(T)`; the `: Type` ascription may be written but is implied. Each
type parameter of a `fun` implicitly requires `Inhabited` (erased during
lowering). Generic functions, structs, enums, and resources are compiled as
true generics — executable compilation does not monomorphize. Function type parameters currently receive
the conservative `copy + drop + store` bound.

**Move-representability.** Only the types above, annotated non-recursive
structs/enums over them, and references to such values cross the compilation
boundary. Lean `Nat`, `Int`, `UInt*`, recursive inductives, functions, proofs,
`IO`, and dependent runtime values are rejected.

## Values and Constants

| Form | Meaning |
|---|---|
| `0`, `1`, `42`, ... | `U64` literals via Lean numeric literals |
| `true`, `false` | `Bool` literals |
| `vector![a, b, c]` | vector value; expands to `Vector.push` chains from `Vector.empty` |
| `{ f := e, g := e' }` | struct value; `{ key, value }` abbreviates `f := f` field puns |
| `.ctor e ...` / `T.ctor e ...` | enum value |
| `()` | the `Unit` value |

Numeric literal instances are compiler-recognized primitives; their Lean
definitions are not a competing wrapping semantics. There are no source-level
`Address` or `Signer` literals: both arrive as function arguments.

A named constant is a module-level Lean `def` of a literal, referenced by name
inside Move functions:

```lean
def E_TOO_SMALL : U64 := 7
```

## Expression forms

**Arithmetic, comparison, bit operations, casts.** `+ - * / %` on every
integer width are checked: overflow, underflow, and division/remainder by
zero abort the transaction with the VM's arithmetic failure code. Both
operands of a binary operation have one width. `<` and `<=` compare integers
numerically. On any other Move-representable type, `<` and `==` denote
Move's built-in structural comparison and equality (`std::cmp` lexicographic
order); no user-supplied ordering is consulted. `&&` and `||` are lowered
through branches.

`&&&`, `|||`, and `^^^` are the width-preserving bitwise operations; they
never abort. `<<<` and `>>>` shift by a `U8` amount, abort when the amount
reaches the width's bit count, and `<<<` truncates shifted-out bits.
`(x.cast : T)` is Move's `(x as T)`: it converts between integer widths and
aborts when the value does not fit the target — widening never aborts. In
specifications, integers compare as their unbounded value directly — `0 < v`,
`old(R[a]).value ≤ R[a].value`, no `.toNat`.  `x.toNat` (the mathematical
value) is written only where wrapping and unbounded arithmetic genuinely
differ, i.e. an overflow check on a sum (`aborts_if ¬x.toNat + y.toNat <
U64.size`, `aborts_if ¬amount.toNat < 64`).

**Borrows.** One scoped parser covers reference types and Move 2 places; a
type-directed elaborator classifies each operand, so `Balance[addr]` and
`balance.value` are place syntax, never runtime strings. The place forms and
their core expansions:

| Surface | Core expansion |
|---|---|
| `&x` / `&mut x` (local) | `borrowLocal x` / `borrowLocalMut x` |
| `&R[addr]` / `&mut R[addr]` | `borrowGlobal R addr` / `borrowGlobalMut R addr` |
| `&r.field` / `&mut r.field` | `borrowField(Mut) r (fieldOfProjection T.field)` |
| `&mut R[addr].a.b` | global borrow, then one checked field borrow per edge |
| `&v[i]` / `&mut v[i]` | local vector borrow, then `borrowElem(Mut)` (bounds-checked) |
| `&mut v` | whole-vector borrow for `insert`/`remove` |

Chained places are checked one owner/field edge at a time against the
elaborated owner type. Mutably borrowing through an immutable reference is an
error; borrowing `&v[i]` through a `&mut` vector reference freezes first.

**Dereference and assignment.** `*r` reads through either reference kind (in
`Action`, so it composes as `let x ← *r`). Assignment is type-directed on the
left-hand local:

| Left-hand side | Meaning |
|---|---|
| `r : &mut T` | `r := e` writes through the reference (`write r e`) |
| `r : &T` | error: cannot write through an immutable reference |
| `let mut x` local | Lean's ordinary local reassignment |

Within a reference assignment, a leading read of the same reference is
sequenced: `r := *r + e` and `r := *r - e` expand to an ordered read,
arithmetic, and write. Rebinding a mutable-reference local is deliberately not
expressible with `:=`.

**Vectors.** `Vector.empty`, `push`, `length`, `get`, `set`, `insert`,
`remove`, and element borrows lower to native Move vector operations. Receiver
notation (`v.length`, `v.push e`, `r.insert i e`, `r.remove i`) is available;
`insert`/`remove` mutate through a `&mut Move.Vector T`. Element access is
bounds-checked: element borrows abort with the VM execution-failure code,
`insert`/`remove` with the standard vector `indexOutOfBounds` code.

**Global storage.** `exists_ R addr`, `moveTo signer value`, and
`moveFrom R addr` are the storage primitives beyond global borrows; `freeze r`
explicitly converts `&mut T` to `&T`. `acquires` metadata is not written by
the author: Leaner seeds each function with the resources of its global
borrows and `moveFrom` uses and computes transitive summaries over the
selected call graph (cycles included).

**Aborts.** `abort e` terminates the transaction with code `e` and rolls back
all effects. It has no normal successor; statements after an unconditional
`abort` are dead.

**Calls.** A call names a Move function of the same or an imported module and
applies it to explicit arguments. Generic instantiation is inferred, or given
with Lean named type arguments: `hasGeneric (T := U64) addr`. At Move call
sites, a `&mut T` argument is implicitly frozen where `&T` is expected. Calls
to Lean-only `def` helpers from a `fun` are rejected — the boundary between
deployable and proof code is never blurred by inlining.

**Control flow.**

- `if c then e else e'` (expression) and `if c then s [else s']` (statement).
  A then-only statement `if` falls through to the following statements.
- `match e with | pat => e ...` performs exhaustive enum matching with
  variable and wildcard payload patterns, including nested patterns.
- `while c do s` and `loop s` are in-function loops compiled to CFG back
  edges. `loop@l s` names a loop; `break`/`continue` target the innermost
  loop, `break@l`/`continue@l` a named enclosing loop. Loop labels must be
  unique among active loops.
- `return e` exits the function early, including from inside a loop.
- `continue f a ...` (with `f` the enclosing function, in tail position) marks
  one direct self-call for checked tail-call-to-loop lowering: arguments are
  evaluated simultaneously, parameters updated in parallel, and control jumps
  to the entry block. It is an error on a non-self or non-tail call, and it is
  unavailable inside `loop`/`while` (use bare `continue`). Unmarked recursive
  calls — including other self-calls in the same function — keep ordinary
  call semantics.

**Effect sequencing.** Pure code is ordinary Lean expression code (a pure
`fun` may also use `do`; it is wrapped in `Id.run`). Effectful code has result
type `Action T` and uses `do` notation: `let x ← e` sequences an effect,
`pure e` returns a result. Storage, reference, vector-index, and abort
operations live in `Action`.

## Function declarations

```lean
fun helper (x : U64) : U64 := x + x                    -- private, pure

public fun get (m : &Map K V) (k : &K) : Action (&V) := ...   -- public

friend fun internalTransfer ... : Action Unit := ...   -- public(friend)

entry fun deposit (addr : Address) (amount : U64) : Action Unit := ...

partial fun countdown (n acc : U64) : U64 :=           -- recursive
  if n < 1 then acc else continue countdown (n - 1) (acc + 1)
```

`fun` occupies Lean command position inside a `move_module` and mirrors
Lean's `def` grammar. A plain `fun` is a **private** Move function. `public
fun` reuses Lean's `public` modifier for Move `public` visibility; `friend
fun` declares `public(friend)`; `entry fun` declares a public entry
function. The keywords expand to persistent internal metadata attributes
(`@[move_fun]`, `@[move_public]`, `@[move_friend]`, `@[move_entry]`,
`@[move_struct]`, `@[move_enum]`), which remain available as the low-level
compatibility spelling outside `move_module`; `@[move_native]` marks a
declaration supplied by a Move dependency.

**Attributes.** A `struct`, `enum`, or `fun` keyword (including the `entry`
and `friend` forms) may be preceded by an attribute list following the
`attributes` production: each instance is a head name applied to positional
arguments, which are name paths (`true` and `false` denote boolean
constants), `u64` constants, or parenthesized instantiated types.

```lean
@[resource_group (scope global)]
struct Registry where ...

@[randomness 7, lint.skip]
entry fun act (addr : Address) : Action Unit := ...
```

In this position `@[...]` always uses the Move attribute grammar, not Lean's
attribute grammar. The well-known internal names above (plus the alias
`entry`) desugar to their tag attributes and take no arguments; every other
instance is user-provided metadata, recorded on the declaration and carried
on the compiled module's struct and function metadata through the XIR
exchange. Argument names are kept as written; resolution is left to the
attribute's consumer.

Recursive functions are declared `partial`: Lean totality is not required,
and general direct and mutual recursion compile as ordinary calls (a
`continue`-marked direct tail call becomes a loop). Parameters are passed by
value or by reference type; results are a value, a reference, or `Action` of
either. A `fun` needs neither `@[noinline]` nor `@[move_fun]` — the macro
inserts the attribute, retains the source body for specification generation,
and preserves the call boundary in Lean's compiler IR.

Ordinary `def`s inside a module block remain Lean-only (semantic models,
constants, proof helpers). They are kept out of line so an accidental call
from deployable code is rejected instead of disappearing through inlining.

## Modules

```lean
import Move

open scoped Move Move.Spec

move_module Account where
  ...items...
```

`move_module M where` creates Lean namespace `M`, opens the Move API inside
it, and registers the enclosed attributed declarations as one Move module
named `M` (at the prototype's fixed address `0x0`). The borrow/deref/vector
syntax is scoped: open `scoped Move` before the module command. Items are
`struct` and `enum` declarations, `fun` declarations, specifications,
Lean-only declarations, and nested namespaces. The item keywords `struct`,
`enum`, `entry`, and `friend` are module-scoped: they stay ordinary
identifiers everywhere else. Compilation is deferred to end of input, so
item order does not affect discovery; an ordinary `lake build` validates the
module without writing artifacts.

**Visibility.** A plain `fun` is private and callable only within its
module. Cross-module calls require the callee to be `public` or an entry function and
both `.lean` modules to be compiler inputs; the caller uses an ordinary Lean
`import`, which also makes the callee's specs and theorems available to
proofs. `friend` functions export `public(friend)` visibility; since friend
module declarations do not exist yet, they are effectively module-internal.

**Compilation directives.** `#export_leaner "M"` compiles the attributed
declarations and marks the deployable module of a `.lean` compiler input;
`move_module` implies it. `move_module% "M"` elaborates the compiled module as
a Lean `MModule` value for interpreter tests and transformations, with an
explicit `structs [...] functions [...]` selection form; `#emit_leaner_xir m`
marks an existing `MModule` value as the deployable module. Compiling a
`.lean` source runs Lean elaboration including metaprograms — treat such
sources as trusted build inputs.

### Specifications

`spec f ... where <clauses>` attaches a declarative contract to `f`, and
`verify f` (automatic) or `verify f by <tactics>` (manual) proves it,
producing the Lean-only declarations `f.contract : Prop` and
`f.verified : f.contract`. Neither is serialized into the compiled module.

For a pure function a single `ensures` relates the implicit `result` to the
parameters. For an `Action` function the clauses are `requires`; `modifies`;
`ensures`; and `aborts_if P [with C]` (or the raw `aborts A`). A function
changes only the global memory its `modifies` clause lists, so a contract
never states a frame condition: with no clause the function changes no global
memory at all, and `modifies R[addr]` leaves every other address of `R` and
every other resource family untouched. A declared abort
condition excuses the postcondition where it holds, so `ensures` must be
established exactly where the declared aborts are ruled out. Omitting the
abort clause leaves abort behavior uninterpreted: any abort code is then
permitted, and nothing is excused, so `ensures` must hold for every
successful execution. The implicit binders are
`initial`, `final`, `result`, and `abortCode`. `old(place)` observes the
pre-state, a bare global place in `ensures` the post-state, and
`exists<R>(addr)` tests resource existence. For a `&mut` parameter the
contract speaks about values: the parameter name denotes the final referent in
`ensures` and `old(parameter)` its initial referent. Spec binders mirror the
function's signature; `{T : Type}` binders implicitly assume `Inhabited T`,
and `[C]` binders add instance assumptions such as `[Move.Compare.Total T]`
for structural-comparison laws. As in `fun` signatures, `{T}` may omit the
`: Type` ascription. The `attribute` production is also the designated
syntax for specification pragmas (`name arg ...` instances); no pragma
clause is defined yet.

**Invariants.** `spec T where invariant P` certifies every *value* of a
struct or enum type `T` at construction — `this` is the value, and a certified
value carries its proof, so no obligation recurs except where a value is
created. `spec global where invariant (all a: P)` instead certifies the
resource *state*: a regular invariant is assumed at reads and asserted at each
write; an `invariant update (all a: R)` relates a write's pre- and post-state
(`old(R[a])` against `R[a]`) and is asserted at each write only. A global
invariant may name several families and is registered under each, so a write
to any of them re-checks it — and only the invariants naming a written family
are checked there.

The relational semantics proved against is generated from the retained `fun`
body — see [`verification-design.md`](verification-design.md) for its
definition and current boundary.
