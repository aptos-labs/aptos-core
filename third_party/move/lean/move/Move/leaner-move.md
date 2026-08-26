# The Leaner Move Language

Leaner Move is a Move dialect embedded in Lean 4. A Leaner module is ordinary
Lean source: every construct below is a Lean syntax fragment, elaborated by
Lean and lowered by the Leaner compiler to Move IR and bytecode. Host-language
constructs outside this definition (closures, `Nat`, recursive data, `IO`,
dependent runtime values, ...) are rejected at the compilation boundary with
source-positioned diagnostics rather than reinterpreted.

Because a Leaner module *is* Lean, the same file holds three kinds of text,
and it is worth keeping them apart while reading:

1. **The deployable program** — `struct`, `enum`, `fun`, and their bodies.
   This is what compiles to Move bytecode. Sections
   [Modules](#modules) ... [Functions](#functions) define it.
2. **The Lean layer** — `def`, `theorem`, `namespace`, imports, and the
   mathematical models a specification talks about. None of it reaches Move.
   [Lean on top of Move](#lean-on-top-of-move) covers it.
3. **The specification layer** — `spec` and `verify`. These are commands that
   generate ordinary Lean definitions and theorems from the retained source of
   a `fun`. [Specifications](#specifications) and [Verification](#verification)
   cover them.

Example-driven introductions are in [`README.md`](README.md); the semantics
behind the specification layer is in
[`verification-design.md`](verification-design.md) and
[`invariant-design.md`](invariant-design.md).

## A first module

This is [`Tests/Verification/Account.lean`](Tests/Verification/Account.lean) without its
test section — a resource, two entry functions, their contracts, and their
proofs.

```lean
import Move

open Move
open scoped Move Move.Spec

module Account where

  struct BalanceValue has Copy, Drop, Store where
    value : U64

  struct Balance has Key where
    balance : BalanceValue

  def E_INSUFFICIENT_BALANCE : U64 := 1

  entry fun deposit (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    value := *value + amount

  spec deposit (addr : Address) (amount : U64) where
    requires existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures
      Balance[addr].balance.value =
        old(Balance[addr].balance.value) + amount;
    aborts_if
      ¬old(Balance[addr].balance.value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  entry fun withdraw (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    let old ← *value
    if old < amount then
      abort E_INSUFFICIENT_BALANCE
    value := *value - amount

  spec withdraw (addr : Address) (amount : U64) where
    requires existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures
      Balance[addr].balance.value =
        old(Balance[addr].balance.value) - amount;
    aborts_if
      old(Balance[addr].balance.value).toNat < amount.toNat
      with E_INSUFFICIENT_BALANCE

  verify deposit

  verify withdraw
```

Reading it construct by construct:

- `module Account where` opens Lean namespace `Account`, opens the Move
  API inside it, and registers the enclosed declarations as one Move module.
- `struct Balance ... has Key` is a *resource*: a structure with Move's
  `key` ability, so it can live in global storage under an address. Abilities
  are exactly what is derived — nothing is implicit.
- `def E_INSUFFICIENT_BALANCE : U64 := 1` is a named constant. `def` is Lean's
  keyword; a literal constant is the one kind of Lean-only definition a `fun`
  may name.
- `&mut Balance[addr].balance.value` chains a global borrow with two checked
  field borrows; `value := *value + amount` reads through the reference and
  writes back, in that order.
- `abort E_INSUFFICIENT_BALANCE` aborts the transaction and rolls back its
  global effects.
- `spec deposit ... where` states the contract: what the caller owes
  (`requires`), what global memory may change (`modifies`), what holds
  afterwards (`ensures`), and which aborts are admitted with which codes
  (`aborts_if ... with`).
- `verify deposit` proves it and leaves the Lean theorem
  `deposit.verified : deposit.contract`.

Nothing in the `spec`/`verify` pair is compiled into the Move module, and
nothing in the module body is assumed by the proof: the contract is proved
against a relational semantics generated from the authored `fun` body.

## Grammar

The EBNF uses `[x]` for option, `{x}` for repetition, `|` for alternatives,
and quoted terminals. `ident`, `nat`, `string`, and `doc-comment` are Lean's
lexical classes; layout (newlines and indentation) follows Lean's `do` and
command conventions. Where a production says "Lean expression grammar
applies", the listed forms are the Move-recognized subset within Lean's
precedence rules.

Names follow Move, not Lean: types and enum variants are `PascalCase`,
functions and fields are `snake_case`, and constants are `UPPER_SNAKE_CASE`.
Variant casing is enforced — a lower-case `enum` variant is rejected with the
PascalCase spelling it should have had.

```ebnf
address-alias   = "address_alias" ident "=" nat ;
address-spec    = nat | ident ;
module          = "module" ident [ "at" address-spec ] "where" { item } ;

item            = struct-decl | enum-decl | fun-decl
                | "mutual" { fun-decl } "end"
                | spec-decl | data-invariant | global-invariant | verify-decl
                | lean-command                  (* def, theorem, namespace, ... *)
                | compile-directive ;

(* ---- attributes ---- *)

attributes      = "@[" attribute { "," attribute } "]" ;
attribute       = ident { attr-arg } ;
attr-arg        = ident | nat
                | "(" ident { attr-arg } ")" ;   (* instantiated type *)

(* ---- data types ---- *)

struct-decl     = [ doc-comment ] [ attributes ]
                  "struct" ident { type-param } [ abilities ] "where"
                  { ident ":" type } [ lean-deriving ] ;

enum-decl       = [ doc-comment ] [ attributes ]
                  "enum" ident { type-param } [ abilities ] "where"
                  { "|" variant ( { "(" ident { ident } ":" type ")" }
                                | ":" type { "→" type } ) }
                  [ lean-deriving ] ;

type-param      = "(" ident { ident } [ ":" ability { "," ability } ] ")" ;
abilities       = "has" ability { "," ability } ;
ability         = "Copy" | "Drop" | "Store" | "Key" ;
variant         = ident ;   (* PascalCase, as in Move *)
lean-deriving   = "deriving" lean-class { "," lean-class } ;

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

type            = int-type | "Bool" | "Address" | "Signer" | "Unit"
                | "Vector" type-atom
                | ident { type-atom }           (* struct/enum instantiation *)
                | "&" type | "&mut " type
                | "(" type ")" ;
type-atom       = ident | "(" type ")" ;
int-type        = "U8" | "U16" | "U32" | "U64" | "U128" | "U256"
                | "I8" | "I16" | "I32" | "I64" | "I128" | "I256" ;

(* ---- statements ---- *)

do-block        = "do" do-seq ;
do-seq          = do-elem { do-elem } ;         (* newline/indentation separated *)
do-elem         = "let" [ "mut" ] ident [ ":" type ] ":=" expr
                | "let" [ "mut" ] ident "←" expr
                | ident ":=" expr               (* write / local reassignment *)
                | ident "←" expr                (* effectful reassignment *)
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
                | "-" expr                      (* signed negation *)
                | "(" expr ".cast" ":" int-type ")"  (* checked width cast *)
                | borrow | "*" expr | "freeze" expr
                | "abort" expr
                | "pure" expr                   (* Action result *)
                | "vector![" [ expr { "," expr } ] "]"
                | struct-value | enum-value
                | call | receiver-call | field-access
                | "continue" ident { expr }     (* marked tail self-call *)
                | "if" [ ident ":" ] expr "then" expr "else" expr
                | "match" expr "with" { "|" pattern "=>" expr } ;

arith-op        = "+" | "-" | "*" | "/" | "%" ;
bit-op          = "&&&" | "|||" | "^^^" | "<<<" | ">>>" ;
cmp-op          = "<" | "≤" | "<=" | "==" ;
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
pattern         = "." ident { pattern-arg } | ident | "_" ;
pattern-arg     = ident | "_" | "(" pattern ")" ;
literal         = nat | "@" nat | "true" | "false" ;

(* ---- specifications ---- *)

spec-decl       = "spec" ident { spec-binder } "where" spec-clauses ;
spec-binder     = "{" ident [ ":" "Type" ] "}"
                | "[" term "]"                  (* instance assumption *)
                | "(" ident ":" param-type ")" ;
spec-clauses    = "ensures" term                (* only a postcondition *)
                | [ "requires" term ";" ] [ modifies-clause ]
                  "ensures" term
                  [ ";" aborts-clause ] ;       (* omitted: uninterpreted *)
modifies-clause = "modifies" modifies-target { "," modifies-target } ";" ;
modifies-target = family [ "[" expr "]" ] ;     (* family, or one address *)
family          = ident | "(" ident { type-atom } ")" ;   (* `(Vault T)` *)
aborts-clause   = "aborts_if" term "with" term
                  { ";" "aborts_if" term "with" term }
                | "aborts_if" term ;            (* any code *)

data-invariant  = "spec" ident { spec-binder } "where"
                  "invariant" term { ";" "invariant" term } ;
                  (* struct or enum type; `this` is the value, `.field` is
                     `this.field`; patterns of an enum with an invariant
                     bind its proof with a trailing `_` *)

global-invariant = "spec" "global" "where"
                   "invariant" invariant-pred
                   { ";" "invariant" invariant-pred } ;
invariant-pred  = "(" "all" ident ":" term ")"            (* regular *)
                | "update" "(" "all" ident ":" term ")" ; (* update *)
                  (* `term` ranges over the address `ident`, using `R[a]` and
                     `existsAt<R>(a)`; the `update` form may also use
                     `old(R[a])`. *)

verify-decl     = "verify" ident [ "by" tactic-seq ] ;
                                                (* tactic-seq: Lean tactics *)

(* ---- specification terms (in addition to Lean's term grammar) ---- *)

spec-term       = "result" | "initial" | "final" | "abortCode" | "this"
                | "old(" term ")"
                | "existsAt<" type ">(" term ")"
                | family "[" term "]" { "." ident } ;  (* global place *)

(* ---- compilation directives ---- *)

compile-directive = "#export_leaner" string [ selection ]
                | "def" ident ":" "MoveModel.IR.Module" ":="
                  "lowerToIR" "``" ident
                | "def" ident ":" "MoveModel.IR.Module" ":="
                  "module%" string selection
                | "def" ident ":" "MoveModel.IR.Module" ":="
                  "module_from_context%" string
                | "#emit_leaner_xir" ident ;
selection       = "structs" "[" [ ident { "," ident } ] "]"
                  "functions" "[" [ ident { "," ident } ] "]" ;
```

## Modules

```lean
import Move

open Move
open scoped Move Move.Spec

address_alias application = 0x42

module Account at application where
  ...items...
```

`module M at address where` creates Lean namespace `M`, opens the Move API
inside it, and registers the enclosed attributed declarations as one Move
module with that on-chain identity. The address is a numeral or a name declared
with `address_alias`; omitting `at` preserves the compatibility default `0x0`.
Alias registrations persist through imports, so cross-module calls retain the
callee's address. The borrow, deref, address-literal, and vector syntax is
scoped: open `scoped Move` before the module command, and `scoped Move.Spec` as
well if the module carries specifications.

Items are `struct` and `enum` declarations, `fun` declarations,
specifications, and *any other Lean command* — `def`, `theorem`, `namespace`,
`mutual`, `open`, `#test`. The item keywords `struct`, `enum`, `entry`, and
`friend` are module-scoped: they stay ordinary identifiers everywhere else.
Compilation is deferred to end of input, so item order does not affect
discovery; an ordinary `lake build` validates the module without writing
artifacts.

**Visibility.** A plain `fun` is private and callable only within its module.
`public fun` reuses Lean's `public` modifier for Move `public` visibility;
`friend fun` declares `public(friend)`; `entry fun` declares a public entry
function.

```lean
fun helper (x : U64) : U64 := x + x                    -- private
public fun get (m : &Map K V) (k : &K) : Action (&V) := ...
friend fun internal_transfer ... : Action Unit := ...   -- public(friend)
entry fun deposit (addr : Address) (amount : U64) : Action Unit := ...
```

Since friend-module declarations do not exist yet, `friend` functions are
effectively module-internal.

**Imports and cross-module calls.** Depending on another Lean-authored module
is an ordinary Lean `import`; the callee must be `public` or an entry
function, and both `.lean` modules must be compiler inputs. The import also
makes the callee's specs and theorems available to proofs:

```lean
-- Tests/Compiler/Fixtures/Modules/Math.lean
module Math where
  public fun identity {T} (value : T) : T := value

  spec identity {T} [Inhabited T] (value : T) where
    ensures result = value

  verify identity
```

```lean
-- Tests/Compiler/MultipleModules.lean
import Move.Tests.Compiler.Fixtures.Modules.Math

module Client where
  fun imported_identity (value : U64) : U64 :=
    Math.identity (Math.identity value)

  spec imported_identity (value : U64) where
    ensures result = value

/-- The imported contract and its kernel-checked proof are ordinary Lean
declarations. -/
theorem importedMathContract : Math.identity.contract := Math.identity.verified
```

**Compilation directives.** `#export_leaner "M"` compiles the attributed
declarations and marks the deployable module of a `.lean` compiler input;
`module` implies it. `lowerToIR ``Namespace` elaborates the registered Move
namespace as a semantic Lean `MoveModel.IR.Module` value for interpreter tests
and transformations; its registered identity supplies the output address and
name. `module% "M"
structs [...] functions [...]` remains the explicit-selection form, while
`module_from_context% "M"` preserves the older current-namespace behavior.
`#emit_leaner_xir m` materializes an existing `MoveModel.IR.Module` at the
deployable XIR boundary. Compiling a
`.lean` source runs Lean elaboration including metaprograms — treat such
sources as trusted build inputs.

## Types

| Type | Meaning |
|---|---|
| `U8` ... `U256` | unsigned integers with aborting (non-wrapping) arithmetic |
| `I8` ... `I256` | signed integers with aborting arithmetic, two's-complement range |
| `Bool` | Move boolean |
| `Address` | account address |
| `Signer` | transaction signer capability |
| `Vector T` | homogeneous growable vector, length certified within the `u64` domain |
| `&T`, `&mut T` | immutable resp. mutable reference (parameter/result positions) |
| `Action T` | a computation sequenced against the transaction (see [Effects](#effects-what-needs-action)) |

### Integers

All six Move widths (`U8`, `U16`, `U32`, `U64`, `U128`, `U256`) are supported.
They are one generic type: `UInt W` is the subtype of signed unbounded
integers within the width's range (`{ x : Int // 0 ≤ x ∧ x < 2 ^ bits }`),
indexed by a type-level width name `W8` ... `W256` — the compiler's
intermediate representation erases value indices from types, so the width is
carried as a type. A value's range bound is certified at construction and
available in every proof (`x.toNat < 2^n`, with no hypothesis tracking).
These are dedicated types — deliberately not Lean's wrapping `UInt64`.

The signed widths `I8` ... `I256` are the parallel generic type `SInt W`, the
subtype of unbounded integers within the two's-complement range
(`{ x : Int // -2^(bits-1) ≤ x ∧ x < 2^(bits-1) }`). Their value is exposed to
specifications directly as `SInt.toInt : Int` — there is no `.toNat`, since
the carrier already *is* the value one reasons about. Arithmetic aborts on
overflow (result outside the range); division truncates toward zero and also
overflows at `minInt / -1`; shifts are arithmetic (sign-extending);
comparisons and equality use the mathematical value, so they are correct for
both signs. Signed literals are written `(5 : I32)` / `(-5 : I32)`; `-x` is
negation.

Each operator is also available under its primitive name (`Move.UInt.add`,
`Move.SInt.div`, ...), which is the spelling `Tests/Language/Signed.lean` uses:

```lean
fun add_values (left : I64) (right : I64) : Action I64 :=
  pure (Move.SInt.add left right)       -- aborts outside [-2^63, 2^63)

fun narrow (value : U64) : U8 :=
  (value.cast : U8)                     -- aborts when the value does not fit
```

`Address` and `Signer` are opaque. Address values can be introduced only by a
literal (`@0x1`) or a registered alias (`address_alias framework = 0x1`, then
`@framework`); they cannot be computed from integers. `Signer` values enter
only as entry-function arguments. A signer is passed by reference, as in Move:
`Ref.address : Ref Signer → Address` is `signer::address_of`, uninterpreted
except that a signer determines one address — which is where `moveTo` publishes.

### Structures

`struct Name ... has ... where ...` declares a Move struct. Fields must be
Move-representable; recursive structures are rejected. Abilities are exactly
the declared `Copy`, `Drop`, `Store`, `Key` markers — `struct` carries no
implicit abilities. A *resource* is a structure declaring `has Key`.

```lean
struct Plain where                        -- no abilities
  value : U64

struct Entry (K V) has Copy, Drop, Store where
  key : K
  value : V

struct Counter has Key where              -- a resource
  value : U64

struct Vault (T : Store) has Key where    -- `T` bounded independently
  value : T

struct Phantom (T) has Copy, Drop where   -- no field uses `T`
```

A parameter may carry its own ability bounds, `(T : Store, Copy)` — Move's
`<T: store + copy>`. A parameter *without* a declared bound is inferred from
non-phantom field usage instead: `has Copy, Store` bounds each used parameter
by those abilities, `has Key` bounds used parameters by `Store`, and phantom
parameters receive no bounds. Every Move structure implicitly derives Lean's
`Inhabited`; this host detail is erased during lowering.

### Enums

`enum Name ... where` declares a native Move enum with constructor payloads in
either named-binder or arrow form. Recursive, indexed, and empty enums are
rejected; non-recursive generic enums are supported. Borrowing a field
directly out of an enum variant is not supported.

```lean
enum Op has Copy, Drop, Store where
  | Idle
  | Transfer (amount : U64)
  | Split (left right : U64)

enum Positional has Copy, Drop, Store where
  | Pair : U64 → U64 → Positional

fun total (op : Op) : U64 :=
  match op with
  | .Idle => 0
  | .Transfer amount => amount
  | .Split left right => left + right
```

Patterns may nest and use wildcards:

```lean
fun nested_total (envelope : Envelope) : U64 :=
  match envelope with
  | .One (.Number value) => value
  | .Two (.Number left) (.Number right) => left + right
  | _ => 0
```

### References

`&T` and `&mut T` elaborate to the opaque `Move.Ref T` and `Move.MutRef T`.
References appear in parameter and result positions only: they cannot be
stored in structures or vectors, and entry functions cannot take or return
them where Move forbids it. Passing a `&mut T` where `&T` is expected inserts
an implicit freeze (lowered to `freeze_ref`).

### Generics

Function type parameters are declared `{T}` and struct or enum parameters
`(T)`; the `: Type` ascription may be written but is implied. Each type
parameter of a `fun` implicitly requires `Inhabited` (erased during lowering).
Generic functions, structs, enums, and resources are compiled as true
generics — executable compilation does not monomorphize. Function type
parameters currently receive the conservative `copy + drop + store` bound.

```lean
struct Vault (T) has Key where
  value : T

fun publish_generic {T} (signer : &Signer) (value : T) : Action Unit :=
  moveTo signer ({ value } : Vault T)

fun has_vault (address : Address) : Action Bool :=
  hasGeneric (T := U64) address           -- explicit instantiation
```

### Move-representability

Only the types above, annotated non-recursive structs/enums over them, and
references to such values cross the compilation boundary. Lean `Nat`, `Int`,
`UInt*`, recursive inductives, functions, proofs, `IO`, and dependent runtime
values are rejected.

## Values and constants

| Form | Meaning |
|---|---|
| `0`, `1`, `42`, ... | integer literal; the width comes from the expected type (`(1 : U8)`, `(5 : I32)`) |
| `-5` | signed negation, on `I8` ... `I256` |
| `true`, `false` | `Bool` literals |
| `@0x1` | 256-bit account-address literal |
| `b"Move"`, `x"DEAD"` | ASCII or hexadecimal `Vector U8` literal |
| `vector![a, b, c]` | vector value; expands to `Vector.push` chains from `Vector.empty` |
| `{ f := e, g := e' }` | struct value; `{ key, value }` abbreviates `key := key, value := value` |
| `.ctor e ...` / `T.ctor e ...` | enum value |
| `()` | the `Unit` value |

Numeric literal instances are compiler-recognized primitives; their Lean
definitions are not a competing wrapping semantics. Signers arrive as
function arguments; literal addresses are written `@0x…`. Where the expected
type is not evident, ascribe it: `({ value := 1 } : Box U64)`,
`(vector![10, 30] : Vector U64)`.

Package-level named addresses use `address_alias name = 0x…`. The declaration
can be used as `@name` in a function body and as `module M at name where` in a
module identity. `Move.ConventionalAddresses`, imported by `Move`, registers
the conventional Aptos framework names in the `Move` namespace.

A named integer constant is a module-level Lean `def` of a compile-time
expression, referenced by name inside Move functions. Arithmetic, bit
operations, shifts, negation, and casts are evaluated with checked Move
constant semantics:

```lean
def E_TOO_SMALL : U64 := 1 + 2 * 3
```

## Expressions and statements

### Arithmetic, comparison, bit operations, casts

`+ - * / %` on every integer width are checked: overflow, underflow, and
division or remainder by zero abort the transaction with the VM's arithmetic
failure code. Both operands of a binary operation have one width.

`<`, `<=`, `>`, `>=`, `==`, and `!=` compare integers numerically, in
conditions and in value position (`left < right` as a `Bool`). On any other
Move-representable type, `<` and `==` denote Move's built-in structural
comparison and equality (`std::cmp` lexicographic order); no user-supplied
ordering is consulted. `!` negates a `Bool`.
`&&` and `||` are lowered through branches.

`&&&`, `|||`, and `^^^` are the width-preserving bitwise operations; they
never abort. `<<<` and `>>>` shift by a `U8` amount, abort when the amount
reaches the width's bit count, and `<<<` truncates shifted-out bits.
`(x.cast : T)` is Move's `(x as T)`: it converts between integer widths and
aborts when the value does not fit the target — widening never aborts.

```lean
fun masked (value mask : U64) : U64 := value &&& mask
fun shifted (value : U64) (amount : U8) : U64 := value <<< amount
fun halved (value : U16) : U16 := value >>> (1 : U8)
fun widen (value : U8) : U256 := (value.cast : U256)
```

### Borrows and places

One scoped parser covers reference types and Move 2 places; a type-directed
elaborator classifies each operand, so `Balance[addr]` and `balance.value` are
place syntax, never runtime strings. The place forms and their core
expansions:

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

```lean
fun read_counter (addr : Address) : Action U64 := do
  let value ← &Counter[addr].value
  (*value)
```

### Dereference and assignment

`*r` reads through either reference kind, so it composes as `let x ← *r`.
Assignment is type-directed on the left-hand local:

| Left-hand side | Meaning |
|---|---|
| `r : &mut T` | `r := e` writes through the reference (`write r e`) |
| `r : &T` | error: cannot write through an immutable reference |
| `let mut x` local | Lean's ordinary local reassignment |

Within a reference assignment, a leading read of the same reference is
sequenced: `r := *r + e` and `r := *r - e` expand to an ordered read,
arithmetic, and write. Rebinding a mutable-reference local is deliberately not
expressible with `:=`.

`freeze r` explicitly converts `&mut T` to `&T`; at Move call sites the
conversion is inserted implicitly where `&T` is expected.

### Vectors

`Move.Vector.empty`, `singleton`, `push`, `popBack`, `length`, `isEmpty`,
`get`, `set`, `insert`, `remove`, and element borrows lower to native Move
vector operations. Receiver notation
(`v.length`, `v.push e`, `r.insert i e`, `r.remove i`) is available;
`insert`/`remove`/`popBack` mutate through a `&mut Vector T`. Element access is
bounds-checked: element borrows abort with the VM execution-failure code,
`insert`/`remove` with the standard vector `indexOutOfBounds` code.

```lean
fun replace : Action U64 := do
  let values : Vector U64 := vector![10, 20, 30]
  let middle ← &mut values[1]
  middle := 42
  (*middle)

fun insert_middle : Action U64 := do
  let values : Vector U64 := vector![10, 30]
  let valuesRef ← &mut values
  Move.Vector.insert valuesRef 1 20
  let updated ← *valuesRef
  let middle ← &updated[1]
  (*middle)
```

The logical vector certifies Move's `u64` length domain by construction, so
`length` is exact in specifications and cursor arithmetic over indices
provably cannot overflow.

### Global storage

`existsAt R addr`, `moveTo signer value`, and `moveFrom R addr` are the storage
primitives beyond global borrows.

```lean
entry fun publish (account : &Signer) (amount : U64) : Action Unit :=
  moveTo account ({ value := amount } : Counter)

fun is_published (addr : Address) : Action Bool :=
  existsAt Counter addr

fun remove (addr : Address) : Action U64 := do
  let counter ← moveFrom Counter addr
  pure counter.value
```

A generic resource family is named at its instantiation: `existsAt (Vault T)
addr`, `moveTo signer ({ value } : Vault T)`, `&mut (Vault U64)[addr].value`.

`acquires` metadata is not written by the author: Leaner seeds each function
with the resources of its global borrows and `moveFrom` uses and computes
transitive summaries over the selected call graph (cycles included).

### Aborts

`abort e` terminates the transaction with code `e` and rolls back all effects.
It has no normal successor; statements after an unconditional `abort` are
dead. Note that an explicit `abort` is not the only way a function aborts:
checked arithmetic, casts, shifts, vector indices, and a missing resource
abort too, with the VM's own codes.

### Calls

A call names a Move function of the same or an imported module and applies it
to explicit arguments. Generic instantiation is inferred, or given with Lean
named type arguments: `hasGeneric (T := U64) addr`. At Move call sites, a
`&mut T` argument is implicitly frozen where `&T` is expected. Calls to
Lean-only `def` helpers from a `fun` are rejected — the boundary between
deployable and proof code is never blurred by inlining.

### Control flow

- `if c then e else e'` (expression) and `if c then s [else s']` (statement).
  A then-only statement `if` falls through to the following statements.
  `if h : c then ... else ...` is Lean's dependent `if`, which puts the branch
  condition in scope; it is used where creating a value owes a
  [data invariant](#data-invariants).
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
  calls — including other self-calls in the same function — keep ordinary call
  semantics.

```lean
fun count_down (n : U64) : U64 := do
  let mut n := n
  while 0 < n do
    n := n - 1
  n

fun labeled_exit (n : U64) : U64 := do
  let mut n := n
  loop@outer
    loop
      if n < 1 then break@outer
      n := n - 1
      break
  n

partial fun countdown (remaining accumulator : U64) : U64 :=
  if remaining < 1 then accumulator
  else continue countdown (remaining - 1) (accumulator + 1)
```

### Effects: what needs `Action`

`Action T` is the type of a Move computation whose steps are sequenced
explicitly. It is a compile-time device — an opaque world token threaded
through Lean's `do` notation — and the compiler erases it while lowering the
recognized primitives to Move stackless operations. It is *not* a model of
on-chain state: the verification semantics represents global storage as typed
resource stores and references as pure values.

An operation needs `Action` when its position in the sequence is observable:

| Category | Operations |
|---|---|
| Global storage | `existsAt`, `moveTo`, `moveFrom`, `&R[a]`, `&mut R[a]` |
| Reference operations | `&x`, `&mut x`, `&r.f`, `&v[i]`, `*r`, `r := e` through a `&mut`, `freeze` |
| Vector mutation through a reference | `Move.Vector.insert`, `Move.Vector.remove` |
| Explicit abort | `abort e` |

Everything else is ordinary pure Lean code and is written without `do`:
arithmetic, comparisons, casts and shifts, field access, struct and enum
construction, `match`, `vector![...]`, `Vector.empty`/`push`/`length`/
`get`/`set`, `Ref.length`, `if`/`while`/`loop`/`return`, and calls to pure
functions.

Two consequences are worth stating explicitly, because they are easy to guess
wrong:

- **Aborting does not require `Action`.** A pure `fun` aborts whenever its
  arithmetic overflows, its divisor is zero, or a cast does not fit. That is
  why a pure function may — and usually should — carry an `aborts_if` clause.

  ```lean
  fun small_sum (left right : U8) : U8 :=       -- pure, yet it can abort
    left + right

  spec small_sum (left : U8) (right : U8) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U8.size
      with Semantics.Checked.arithmeticAbortCode
  ```

- **References are in `Action` for ordering, not because they are state.** A
  borrow fixes where a loan opens and dies, which the semantics needs; but
  logically `&T` is the observed value itself and `&mut T` is a
  current/prophesied-final pair, and neither appears in the state. See
  [`verification-design.md`](verification-design.md).

Pure code may also use `do` (Lean wraps it in `Id.run`), which is how
`while`/`loop` appear in a pure function; the effect type is still absent.

## Functions

```lean
fun helper (x : U64) : U64 := x + x                    -- private, pure

public fun get (m : &Map K V) (k : &K) : Action (&V) := ...   -- public

friend fun internal_transfer ... : Action Unit := ...   -- public(friend)

entry fun deposit (addr : Address) (amount : U64) : Action Unit := ...

partial fun countdown (n acc : U64) : U64 :=           -- recursive
  if n < 1 then acc else continue countdown (n - 1) (acc + 1)

mutual                                                 -- mutual recursion
  partial fun even_flag (value : U64) : U64 :=
    if value < 1 then 1 else odd_flag (value - 1)

  partial fun odd_flag (value : U64) : U64 :=
    if value < 1 then 0 else even_flag (value - 1)
end
```

`fun` occupies Lean command position inside a `module` and mirrors Lean's
`def` grammar. The visibility keywords expand to persistent internal metadata
attributes (`@[move_fun]`, `@[move_public]`, `@[move_friend]`,
`@[move_entry]`, `@[move_struct]`, `@[move_enum]`), which remain available as
the low-level compatibility spelling outside `module`; `@[move_native]`
marks a declaration supplied by a Move dependency.

Recursive functions are declared `partial`: Lean totality is not required, and
general direct and mutual recursion compile as ordinary calls (a
`continue`-marked direct tail call becomes a loop). Parameters are passed by
value or by reference type; results are a value, a reference, or `Action` of
either. A `fun` needs neither `@[noinline]` nor `@[move_fun]` — the macro
inserts the attribute, retains the source body for specification generation,
and preserves the call boundary in Lean's compiler IR.

### Attributes

A `struct`, `enum`, or `fun` keyword (including the `entry` and `friend`
forms) may be preceded by an attribute list following the `attributes`
production: each instance is a head name applied to positional arguments,
which are name paths (`true` and `false` denote boolean constants), `u64`
constants, or parenthesized instantiated types.

```lean
@[resource_group (scope global)]
struct Registry has Key where
  value : U64

@[randomness 7, lint.skip]
entry fun act (addr : Address) : Action Unit := ...
```

In this position `@[...]` always uses the Move attribute grammar, not Lean's
attribute grammar. The well-known internal names above (plus the alias
`entry`) desugar to their tag attributes and take no arguments; every other
instance is user-provided metadata, recorded on the declaration and carried on
the compiled module's struct and function metadata through the XIR exchange.
Argument names are kept as written; resolution is left to the attribute's
consumer.

## Lean on top of Move

A `module` block accepts any Lean command, so the whole of Lean is
available *beside* the deployable program. This is the point of the embedding:
a specification talks about a mathematical model, and the model is written in
the same file, in the same language, as the contract it constrains.

### What is Lean-only

`def`, `abbrev`, `theorem`, `instance`, `namespace`, `section`, `open`, and
the diagnostic commands are ordinary Lean and are never selected for Move
lowering. Only declarations carrying the Move tags (`struct`, `enum`, `fun`,
and their `entry`/`friend` forms) become part of the module.

The boundary is enforced in one direction only:

- A `fun` **may not** call a Lean-only `def`. The call is rejected
  ("unsupported call ... while compiling Move function") rather than inlined,
  so deployable code cannot silently acquire host-language behaviour. It
  *may* reference a `def` of a literal — that is what a named constant is.
- Lean code **may** freely mention a `fun`: it elaborates to an ordinary Lean
  definition, so proofs unfold it and theorems quantify over it.

The two things the Lean layer is normally used for are the mathematical model
a contract talks about (see [The `Model` namespace](#the-model-namespace)) and
the lemmas its proof needs.

### File layout

The checked-in modules follow one layout, which keeps a long file navigable:

```lean
module M where
  /-! ## Types -/          -- structs, enums, data invariants
  /-! ## Model -/          -- namespace Model: specification vocabulary
  /-! ## Functions -/      -- fun declarations with their `spec` beside them
  /-! ## Proofs -/         -- Model lemmas, then `verify` commands
  /-! ## Tests -/          -- `lowerToIR`, `#test`, `#guard`
```

A `spec` is written directly under the `fun` it constrains, so the contract is
read together with the code; `verify` commands are collected in the proof
section, ordered by dependency, because a proof may cite the theorem another
`verify` produced.

### Names a module generates

| Written | Generated Lean declarations |
|---|---|
| `fun f` | `f` (an ordinary definition, retaining its source for specification) |
| `spec f` with only `ensures`, on a non-`Action` function | `f.contract : Prop` |
| `spec f` with `requires`/`modifies`/`aborts_if`, or on an `Action` function | `f.sourceSpec` (and `f.bodySpec` when recursive), `f.contract : Prop` |
| `verify f` | `f.verified : f.contract` |
| `struct T` with `spec T where invariant` | `T.Raw`, `T.Invariant`, and the `invariant` proof field of `T` |
| `spec module where invariant` | `GlobalInvariant_<R>[_i]`, `..._at`, and one reestablishment lemma per family and write shape (`GlobalUpdate_...` for `update` clauses) |

None of these is serialized into the compiled module.

## Specifications

`spec f ... where <clauses>` attaches a declarative contract to `f`. The
clause set determines which of two contract shapes is generated.

### Pure value contracts

A `spec` whose only clause is `ensures`, on a function that does not return
`Action`, is a value predicate: `result` is bound to the function applied to
its arguments, and `f.contract` is `∀ args, ensures`.

```lean
enum Choice where
  | Fallback
  | Chosen (value : U64)

fun choose (fallback : U64) (choice : Choice) : U64 :=
  match choice with
  | .Fallback => fallback
  | .Chosen value => value

spec choose (fallback : U64) (choice : Choice) where
  ensures
    result = match choice with
      | .Fallback => fallback
      | .Chosen value => value

verify choose
```

A pure value contract relates the function's result to its arguments and says
nothing about aborts: on the value level `left + right` is the wrapping host
value, which agrees with the checked semantics exactly on the non-aborting
executions. It is the right form for a body that cannot abort — no arithmetic
that can overflow, no vector index, no cast. To constrain abort behaviour, add
an `aborts_if` clause, which switches the contract to the relational form
below; this is available for pure functions too.

### Relational contracts

Every other clause set — an `Action` function, or any function with
`requires`, `modifies`, or `aborts_if` — generates a *relational* contract:
`f.sourceSpec`, the semantics derived from the retained `fun` body, and
`f.contract`, the statement that `f.sourceSpec` satisfies the clauses. Clauses
are written in this order, separated by `;`:

```lean
spec f (params) where
  requires  <precondition>;
  modifies  <resource places>;
  ensures   <postcondition>;
  aborts_if <condition> with <code>;
  aborts_if <condition> with <code>       -- repeatable
```

`requires`, `modifies`, and the abort clauses may each be omitted; `ensures`
is required. Note that a *pure* function takes this form too whenever it can
abort:

```lean
fun narrow (value : U64) : U8 :=
  (value.cast : U8)

spec narrow (value : U64) where
  ensures result.toNat = value.toNat;
  aborts_if ¬value.toNat < U8.size
    with Semantics.Checked.arithmeticAbortCode
```

### Specification vocabulary

| Form | Meaning |
|---|---|
| `result` | the returned value |
| `old(place)` | the place observed in the pre-state |
| `R[addr]`, `R[addr].f.g` | a global place in the post-state (in `ensures`) |
| `(R T)[addr]`, `(R T)[addr].f` | the same for a generic family at an instantiation |
| `existsAt<R>(addr)`, `existsAt<R T>(addr)` | whether resource `R` (at `T`) is published at `addr` |
| `x.toNat` | the mathematical value of an unsigned integer |
| `x.toInt` | the mathematical value of a signed integer |
| `v.toList` | the logical contents of a `Vector` |
| `U8.size` ... `U256.size` | the exclusive upper bound of a width |
| `I8.halfSize` ... | half the value count of a signed width; the range is `[-halfSize, halfSize)` |
| `initial`, `final`, `abortCode` | the implicit state and code binders |
| `this` | the constrained value, inside a data invariant |

Integers compare as their unbounded value directly — `0 < v`,
`old(R[a]).value ≤ R[a].value`, with no `.toNat`. Write `.toNat` only where
wrapping and unbounded arithmetic genuinely differ, which in practice means an
overflow condition: `aborts_if ¬x.toNat + y.toNat < U64.size`. Where the
expected type of `result` is not determined by the clause, ascribe it:
`ensures (result : U64).toNat ≤ 100`.

The width-directed names are *literals and bounds only* — `U8.ofNat` …
`U256.ofNat`, `U8.size` … `U256.size`, `I8.ofInt` …, `I8.halfSize` …  The named
integer *operations* are width-generic and live on the view: `UInt.less`,
`UInt.lessEq`, `UInt.equal`, `UInt.add` … and their `SInt` counterparts.  There
is deliberately no `U64.lessEq`: one operation family is the whole point of the
unified integer model, and a per-width alias would split its discrimination-tree
key (see `unified-int-design.md`).

Abort codes are `Nat` or `U64`; the named VM codes are
`Semantics.Checked.arithmeticAbortCode` (overflow, underflow, division by
zero, a failing cast or shift), `Semantics.Vector.indexOutOfBounds`
(`vector::insert`/`remove`), and `Semantics.Resource.executionFailure` (an
out-of-range element borrow).

### The `Model` namespace

Beyond that built-in vocabulary a contract says whatever Lean can say, and for
anything non-trivial it should say it in terms of a *model* rather than by
restating the implementation. The convention is a nested `Model` namespace
holding the mathematical vocabulary the specifications use, and the lemmas the
proofs need. It is ordinary Lean and never compiled.

```lean
module OrderedMap where

  struct Entry (K V) has Copy, Drop, Store where
    key : K
    value : V

  namespace Model

  /-- Strict key order: the data invariant of the map itself. -/
  def SortedEntries : List (Entry K V) → Prop
    | [] => True
    | entry :: rest =>
        (∀ next ∈ rest, entry.key < next.key) ∧ SortedEntries rest

  end Model

  struct Map (K V) has Copy, Drop, Store where
    entries : Vector (Entry K V)

  spec Map {K} {V} where
    invariant Model.SortedEntries .entries.toList

  namespace Model

  def Contains (map : Map K V) (key : K) : Prop :=
    ∃ entry ∈ map.entries.toList, entry.key = key

  /-- The abstract insertion the contract of `add` promises. -/
  def insert (entries : List (Entry K V)) (key : K) (value : V) :
      List (Entry K V) :=
    match entries with
    | [] => [{ key, value }]
    | entry :: rest =>
        if entry.key < key then entry :: insert rest key value
        else { key, value } :: entry :: rest

  end Model
```

Every contract in the module then reads as a statement about that model —
`invariant Model.SortedEntries .entries.toList` above, and below
`ensures (result = true) ↔ Model.Contains map key` for `contains`,
`ensures map.entries.toList = Model.add (old(map)) key value` for `add`.

Three conventions make this work in practice:

1. **Model definitions are placed where they are first needed.** A data
   invariant may only mention declarations that precede its type, so a
   predicate used by `spec Map` (here `SortedEntries`) goes in a `Model` block
   *above* `struct Map`, and the rest of the model follows the type. Splitting
   `namespace Model` into several blocks is normal and cheap.
2. **Model predicates speak about components, not about the certified type.**
   `Model.SortedEntries .entries.toList` rather than a `Model.Sorted map`:
   a definition over `Map` would be circular with `Map`'s own invariant.
   `Move.Vector.toList` is the specification accessor that exposes a vector's
   contents; it is never compiled.
3. **Supporting lemmas live in the same namespace, grouped by the operation
   they serve.** In `Tests/Verification/OrderedMap.lean` the proof library is
   `Model.Search`, `Model.Insertion`, and `Model.Removal`, each documented with
   which verified function cites it. These are proved, not assumed: they are
   ordinary theorems, and the source proofs reference them by name.

`Tests/Verification/Quicksort.lean` uses the same convention with `Model.slice`,
`Model.Partitioned`, and `Model.Sorts` as the contract vocabulary of a generic
in-place sort.

### Binders

Spec binders mirror the function's signature:

| Binder | Meaning |
|---|---|
| `(x : T)` | a value parameter |
| `(r : &mut T)` | a mutable-reference parameter; the contract speaks about `T` |
| `{T}` or `{T : Type}` | a type parameter; implicitly assumes `Inhabited T` |
| `[C]` | an instance assumption, e.g. `[Move.Compare.Total K]` |

An immutable-reference parameter `(m : &Map K V)` is written in the spec as
the value it observes, `(map : Map K V)`. Beyond the mirrored parameters, a
spec may add the proof-side assumptions its contract needs:

```lean
public fun contains {K V} (map : &Map K V) (key : &K) : Action Bool := ...

spec contains {K} {V} [Move.Compare.Total K]
    (map : Map K V) (key : K) where
  ensures (result = true) ↔ Model.Contains map key;
  aborts_if False
```

The state type, the typed resource stores, and their frame laws are inferred
and universally quantified; a contract never declares a `World` record or a
resource descriptor. Global state therefore stays compositional: adding a
resource in another module never forces a shared state type.

### Framing with `modifies`

A function changes only the global memory its `modifies` clause lists, so a
contract never states a frame condition. The frame is owed by every successful
execution unconditionally — unlike the postcondition, it is not excused where
a declared abort may happen.

| Clause | Generated frame |
|---|---|
| *(omitted)* | no global memory changes at all (`final = initial`) |
| `modifies R[addr];` | every other address of `R`, and every other family the function uses, is unchanged |
| `modifies R;` | family `R` is unconstrained; the others are still framed |
| `modifies R[a], S[a];` | both narrowed families, the rest framed |
| `modifies (R T)[addr];` | a generic family at an instantiation, like `R[addr]`; an instantiation neither the body nor the clauses name is left unconstrained |

```lean
spec shift (addr : Address) (amount : U64) where
  requires existsAt<Debit>(addr) ∧ existsAt<Credit>(addr) ∧
    amount.toNat ≤ old(Debit[addr].value).toNat ∧
    old(Credit[addr].value).toNat + amount.toNat < U64.size;
  modifies Debit[addr], Credit[addr];
  ensures
    Debit[addr].value = old(Debit[addr].value) - amount ∧
    Credit[addr].value = old(Credit[addr].value) + amount;
  aborts_if False
```

### Abort behaviour

A contract records two independent things about aborts: which abort outcomes
it *permits* (what an aborting execution is checked against, and what a caller
inherits), and where a declared abort *excuses* the postcondition.

- `aborts_if P with C` — the function may abort where `P` holds, with code
  `C`. Repeated clauses are disjoined. `ensures` must then be established
  exactly where every declared condition is ruled out; that reading is part of
  the semantics of contract satisfaction, not text written into the clauses.
- `aborts_if P` — constrains the condition but permits any code.
- `aborts_if False` — the function never aborts. This is the right clause for
  a function whose arithmetic is provably in range.
- *Omitted* — abort behaviour is **uninterpreted**: any code is permitted, and
  nothing is excused, so `ensures` must hold for every successful execution.

```lean
fun increment_unspecified (value : U64) : Action U64 := do
  pure (value + 1)

-- No abort clause: the postcondition still has to hold whenever the call
-- succeeds, and the precondition is what makes that provable.
spec increment_unspecified (value : U64) where
  requires value.toNat + 1 < U64.size;
  ensures result = value + 1
```

Because `aborts_if` is an over-approximation — the function *may* abort where
the condition holds — a contract cannot express that a function *must* abort.
`ensures False; aborts_if True with C` says that any abort carries code `C`,
not that the function never succeeds.

### Mutable-reference parameters

For a `&mut` parameter the contract speaks about values, never about reference
identities: the parameter name denotes the final referent in `ensures`, and
`old(parameter)` its initial referent. The prophecy-backed loan implementing
this stays internal to the proof.

```lean
public fun add {K V} (map : &mut Map K V) (key : K) (value : V) :
    Action Unit := ...

spec add {K} {V} [Move.Compare.Total K]
    (map : &mut Map K V) (key : K) (value : V) where
  ensures map.entries.toList = Model.add (old(map)) key value;
  aborts_if Model.Contains map key with 1;
  aborts_if U64.size ≤ map.entries.toList.length + 1
    with Move.Semantics.Vector.indexOutOfBounds
```

A contract supports two simultaneous mutable-reference parameters, each with
an independent prophecy. A callee with `&mut` parameters is called with the
caller's live mutable references, and every final referent is written back to
the place the caller borrowed:

```lean
entry fun bump_counter (addr : Address) : Action Unit := do
  let value ← &mut Counter[addr].value
  bump value
```

### Data invariants

`spec T where invariant P` constrains every *value* of struct or enum type
`T`. A value of such a type carries its proof, so reading, passing, storing,
and returning one owes nothing, and operations need no well-formedness
precondition. `this` denotes the value and `.field` abbreviates `this.field`;
clauses may be repeated and are conjoined.

```lean
struct Percent where
  value : U64

spec Percent where
  invariant .value.toNat ≤ 100

struct Range where
  low : U64
  high : U64

spec Range where
  invariant .low.toNat ≤ .high.toNat;
  invariant .high.toNat ≤ 1000
```

The invariant is available from the value itself, with no precondition, and it
is strong enough to rule out the abort of an operation that depends on it:

```lean
fun span (range : Range) : U64 :=
  range.high - range.low

spec span (range : Range) where
  ensures (result : U64).toNat ≤ range.high.toNat ∧ result.toNat ≤ 1000;
  aborts_if False
```

An enum invariant matches on `this`. Each constructor then carries the proof
of its own variant, so *patterns of a certified enum bind it with a trailing
`_`*:

```lean
enum Payment where
  | None
  | Direct (amount : U64)
  | Split (left right : U64)

spec Payment where
  invariant match this with
    | .None => True
    | .Direct amount => 0 < amount.toNat
    | .Split left right => 0 < left.toNat ∧ 0 < right.toNat

fun first_part (payment : Payment) : U64 :=
  match payment with
  | .None _ => 0
  | .Direct amount _ => amount
  | .Split left _ _ => left
```

The obligation lands only where a value is created, in one of three shapes:

- **A literal** discharges it during elaboration, so the source carries no
  proof text and a violation is reported at the literal: `{ value := 50 }`,
  `.Direct 5`.
- **A conditional creation** uses a dependent `if`, whose branch hypothesis
  discharges it: `if h : 0 < amount then .Direct amount else .None`. A plain
  `if` would not — there would be nothing to prove the invariant from.
- **A mutation** is unconstrained while the borrow is live; the obligation
  lands where the value is rebuilt, when the loan dies. This is the same for a
  local value and for a resource behind `&mut R[addr].field`.

The proof field is erased before Move sees the type.

### Global invariants

`spec module where invariant ...` constrains the *state* rather than a value:
a condition over the resources in global memory, quantified over addresses and
possibly relating several families. It is the Move Prover's module-level
`invariant`.

```lean
struct Counter has Key where
  value : U64

spec module where
  -- Regular: a state predicate, assumed at reads, asserted at each write.
  invariant ∀ a, 0 < Counter[a].value;
  -- Update: a pre/post relation, asserted at each write, never assumed.
  invariant update ∀ a, old(Counter[a]).value ≤ Counter[a].value
```

- A **regular** invariant is assumed on entry (it holds because every prior
  write re-established it) and asserted immediately after each write to a
  family it names — not at the end of the function.
- An **update** invariant relates the pre- and post-state of a write:
  `old(R[a])` is the pre-state, bare `R[a]` the post-state. It is asserted at
  each write only.
- A value-accessed family `R[a].field` carries an implicit `existsAt<R>(a)`
  guard, so absent addresses are unconstrained; families named only through
  `existsAt<R>(a)` add no guard.
- An invariant is registered under **every** family it names, so a write to
  any of them re-checks it — and a write re-checks only the invariants naming
  the written family. Cross-resource invariants are therefore expressible:

```lean
spec module where
  invariant ∀ a, Debit[a].value.toNat ≤ Credit[a].value.toNat
```

The global-storage primitives participate: `moveTo` and `moveFrom` re-certify
the state immediately afterwards, exactly as a `&mut` write does, while
`existsAt` is a read and re-certifies nothing. Publishing must therefore prove
the invariant of the new value, and the update invariant is vacuous at a
freshly published address:

```lean
entry fun publish (account : &Signer) (amount : U64) : Action Unit :=
  moveTo account ({ value := amount } : Counter)

spec publish (account : &Signer) (amount : U64) where
  requires 0 < amount.toNat;
  modifies Counter[account.address];
  ensures Counter[account.address].value = amount

verify publish
```

`modifies` and global invariants are complementary: the frame bounds *what*
changes, the invariant certifies that *what changed still satisfies* the
condition.

## Verification

`verify f` proves `f.contract` and produces `f.verified : f.contract`. Both
are Lean-only; neither is serialized into the compiled module.

### Automatic proofs

`verify f` alone runs the standard proof. For a relational contract it opens
the contract into one weakest-precondition goal, executes the body
symbolically by the `wp` rules — linear in the body, no existentials, each
prophecy eliminated as soon as its reconciliation equation appears — and
finishes the arithmetic and data obligations with the shared simp and `grind`
inventories. For a pure value contract it reduces the function directly.

This discharges most contracts whose remaining obligations are arithmetic:

```lean
verify deposit
verify withdraw
verify shift
verify set_level
verify credit
```

### Manual proofs

`verify f by <tactics>` supplies the proof explicitly. It is needed when a
contract's obligation is a domain fact — a model-level `Contains`, a
sortedness lemma, a loop invariant — rather than arithmetic.

```lean
verify reading by
  simp only [reading.contract, reading]
  exact fun percent => percent.invariant
```

`contract_intro` is the entry point for a relational contract. It unfolds
`f.contract` and `f.sourceSpec`, switches to weakest-precondition reasoning,
and introduces the binders under fixed names:

| Name | Meaning |
|---|---|
| `args` | the argument tuple; destructure it with `obtain ⟨a, b⟩ := args` |
| `initial` | the initial state |
| `permitted` | the precondition, including any assumed global invariant |
| `recursive`, `recursiveVerified` | for a recursive function: the recursive occurrence and its contract, from fixed-point induction |

The tactic vocabulary, all of it thin syntax over the public proof lemmas:

| Tactic | Purpose |
|---|---|
| `contract_intro` | open the contract into one `wp` goal (above) |
| `move_step [names]` | one symbolic step, chosen by the goal's leading construct |
| `move_cases h : c` | split a source condition and normalize `h` in both branches |
| `move_hyp h` | normalize a branch hypothesis into facts about integer values |
| `checked_cases h` | split a checked operation into success and abort, discharging the abort branch |
| `abort_clause` | close an abort obligation against the declared clauses, or refute the branch |
| `abort_norm` | normalize the `¬mayAbort` guard into one hypothesis per declared abort |
| `spec_norm` | normalize value-level source semantics |
| `wp_norm` | rewrite with the weakest-precondition rules |
| `uint_bounds`, `u64_omega` | make the certified range facts available; finish numeric goals |
| `data_invariants` | make the data invariant of every certified-typed local available |
| `wp_call verified using permitted` | step through a bound call using its established contract |
| `spec_defined` | discharge a well-definedness side goal structurally |
| `move_invariant` | discharge a data-invariant creation obligation |

The shared simp inventories are `move_norm` (value-level normalization),
`wp_norm` (weakest-precondition rules), `move_data` (references, stores,
vectors, monad shells), `move_spec` (the raw relational semantics), and
`move_invariant_norm`. User proofs and project-specific summaries can extend
them with the corresponding attributes.

Every checked operation has the same two-branch weakest precondition — a
success condition and an abort with a fixed code — so one tactic splits them
all. The observed abort code selects the matching clause, arithmetic proves
its condition, and a branch no clause admits is refuted from the context. A
condition needing a semantic argument is left as the only remaining goal, so
the tactic never hides a real obligation.

### Loops and recursion

Recursive source semantics is the least finite-unfolding relation, so `verify`
proofs are partial correctness. `Move.Verify.satisfies_fix` (with its `wp`
form `satisfies_fix_of_wp`, packaged as `contract_intro`) is the induction
rule; structured `while`/`loop` bodies get the same treatment, since they are
fixed points too.

When the function's own contract *is* the loop invariant, `contract_intro`
provides it as `recursiveVerified` and the proof is one case split per loop
exit:

```lean
fun count_down (n : U64) : U64 := do
  let mut n := n
  while 0 < n do
    n := n - 1
  n

spec count_down (n : U64) where
  ensures result = 0;
  aborts_if False

verify count_down by
  contract_intro
  move_cases hloop : Move.Verify.Source.logicalLT 0 args
  · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
      Move.Semantics.Spec.pure_bind]
    exact Move.Verify.wp_of_satisfies recursiveVerified trivial
  · subst args
    simp [Move.Verify.wp, Move.Semantics.Spec.pure]
```

When it is not — a function with two sequential loops, or a loop carrying a
stronger invariant than the postcondition — the loop's invariant is stated as
a separate `Move.Verify.Satisfies` lemma about its fixed point, proved with
`satisfies_fix_of_wp`, and cited through `wp_of_satisfies`.
`Tests/Language/Loops.lean` shows both shapes (`upToThreeLoop`,
`countToZeroLoop`, `drainLoop`); `Tests/Verification/OrderedMap.lean` proves a
recursive binary search against a `Model.Search.Window` invariant; and
`Tests/Verification/Quicksort.lean` verifies a generic in-place sort.

### Calls

A call to a Move callee is verified from the callee's relational semantics,
`f.sourceSpec`, which is generated from its retained `fun` body — by its own
`spec`, or on demand for a callee (pure or effectful) that has none. In an
automatic proof the callee's `sourceSpec` is unfolded into the caller; a
callee with a `&mut` parameter is called with the caller's live mutable
reference (`bump value` after `let value ← &mut Counter[addr].value`) and its
final referent is written back. Calls with two mutable parameters carry two
independent prophecies and write both final referents back in declaration
order. A verified recursive callee stays opaque in automatic caller proofs:
`verified_call` applies its contract and leaves only the local precondition,
abort exclusion, and weakening obligations. The explicit `wp_call` form is
also available when those obligations need a hand-written proof:

```lean
verify contains by
  contract_intro
  obtain ⟨map, key⟩ := args
  dsimp only
  wp_call (lowerBound.verified _) using permitted
  · rintro index middle ⟨⟨rfl, -⟩, rfl⟩
    ...
  · exact fun code h => h.elim
```

### What automatic generation rejects

Source-semantics generation covers the accepted fragment and rejects the rest
at the `spec` command, with a source-positioned message, rather than assigning
an approximate semantics. A rejected function can still be compiled and
executed; it simply has no `spec`/`verify` yet. The current boundary:

| Rejected at `spec` | Workaround |
|---|---|
| a call to a callee that is not a `fun` (no retained source) | declare the callee with `fun` |
| receiver-style `values.get i` / `r.insert i e` / `r.remove i` | write `Move.Vector.get` / `Move.Vector.insert` / `Move.Vector.remove` |
| a core primitive in a form the surface cannot express (`borrowField` with a computed descriptor, `assert`) | use the surface syntax (`&r.f`, `abort c`) |
| an overlapping sibling mutable borrow of the same field path | borrow disjoint fields or close the first loan |
| a Lean-only `match` motive or `generalizing` clause | use Move's ordinary match form |
| a clause naming a resource family the function does not touch | name only families the function uses |

Mutually recursive functions are represented by one heterogeneous
`Spec.fixFamily`. Their generated member projections have ordinary
`sourceSpec`s, and `contract_intro` opens the family-wide induction step.

## Compiling and testing

Move compiler v2 launches Lean with a private output path, reads the versioned
XIR, and runs its normal checking, optimization, file-format, and
bytecode-verification pipeline. Direct source lists and Move package source
discovery both recognize `.lean` beside `.move`. Lean is launched from the
Lake package root (`third_party/move/lean`); set `LEANER_ROOT` to point
elsewhere for a different checkout layout.

For tests and transformations the compiled module is available as a Lean
value, and the modeled IR interpreter runs its functions:

```lean
def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Account

private def balanceId := compiled.resourceId "Balance"
private def memory (addr value : Nat) : MoveModel.IR.IMem :=
  [(balanceId, addr, .struct [.struct [.u64 value]])]
private def run := Tests.run compiled

#test run "deposit" (memory 7 10) [.address 7, .u64 5]
  = Tests.okRet (memory 7 15) []
#test run "deposit" (memory 7 18446744073709551615) [.address 7, .u64 1]
  = Tests.abortedIn (memory 7 18446744073709551615) 0
```

`#test` and `#guard` are elaboration-time checks, so a failure fails the
build. `#guard` over `compiled.funMeta` / `compiled.structMeta` is the
convention for asserting compiled metadata — visibility, abilities,
attributes, `acquires`.

Verification and compilation are independent paths: a `verify` proof neither
consumes nor produces XIR, and XIR carries no contract. A compiler-correctness
theorem connecting `f.verified` to the emitted bytecode remains future work,
and the prototype does not conflate the two claims.

## Further reading

- [`README.md`](README.md) — example-based tour of the language.
- [`overview.md`](overview.md) — motivation and the two compilation paths.
- [`verification-design.md`](verification-design.md) — the relational
  semantics, prophecy references, contracts, and the proof interface.
- [`invariant-design.md`](invariant-design.md) — data and global invariants.
- [`design-plan.md`](design-plan.md), [`loop-design.md`](loop-design.md) —
  compiler architecture and loop lowering.
- [`performance-analysis.md`](performance-analysis.md) — cost of the
  verification encoding.
