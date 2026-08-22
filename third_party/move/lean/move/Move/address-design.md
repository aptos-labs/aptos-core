# Addresses — design plan

## Motivation

Leaner has the `Address` *type* but no way to **name** or **write** an address:

- `Move/Move/Basic.lean:25` — `Address` is an opaque structure whose payload is
  documented as a placeholder ("exists only so source declarations pass through
  Lean's compiler"); nothing constructs one.  Addresses arrive only as function
  arguments, so no source program can mention `@0x1`.
- `Move/Move/Compiler/Export.lean:949` — every `module` registers the fixed
  address `"0x0"`.  Two Leaner modules cannot be published at different
  addresses, and a cross-module call always records `0x0` as the callee's
  address (`Move/Move/Compiler/Normalize.lean:1784`).

This is the last piece of Move's *value* domain that Leaner cannot express, and
it blocks framework-shaped programs, which are pervasively address-keyed
(`@aptos_framework`, `@std`, resource ownership checks like
`addr == @owner`).

The target surface, in Move's own terms:

```lean
module Registry at aptos_framework where     -- named address (an alias)
module Registry at 0xCAFE where              -- literal address

  fun owner : Action Address := pure @0xCAFE  -- literal in expression position
  fun is_std (a : Address) : Bool := a == @std
```

## What already exists (nothing below the surface has to be invented)

The whole pipeline under the source language already carries addresses:

| layer | status |
|---|---|
| `MoveModel.IR.Address` | `abbrev Address := Nat` (`IR/Value.lean:29`) |
| IR value | `Value.address (a : Address)` (`IR/Value.lean:426`), `IsValid … .address` (`IR/ValueTyping.lean:143`) |
| IR instruction | `Instr.load dst (.address a)` — the generic constant load |
| IR module | `Module.address : Address` (`IR/Module.lean:22`) |
| XIR JSON | `encodeAddress` / `{"address": "0x…"}` for values, `module.address` (`Frontend/XIR/Json.lean:28,71,336`) |
| exchange schema | `Value::Address` documented and tested (`move-model/exchange/src/lib.rs:701,1024`) |
| Rust backend | `Constant::Address → StacklessConstant::Address` (`move-compiler-v2/src/xir.rs:2015`); module and external-ref addresses parsed at `xir.rs:88,185,314` |
| LIR | `Module.address`, `ExternalFunRef.address`, `parseAddress` (`Compiler/LIR.lean:155,185`) |
| Attributes | `ModuleRef.address`, persisted in `.olean`s (`Attributes.lean:173`) |

So the work is confined to the Lean surface, the LCNF normalizer, and one LIR
instruction.  **No Rust change, no IR change, no exchange-format change.**

## The design

Three additions, each the smallest thing that is faithful to Move.

### 1. An address literal: `@<numeral>`

`Address` gains a real constructor function:

```lean
/-- The address denoted by a numeral.  Only *literal* applications compile:
Move addresses cannot be computed from integers. -/
@[noinline] def Address.ofNat (n : Nat) : Address := ⟨n⟩
```

`@[noinline]` is the established literal pattern (`UInt.ofNat`,
`Basic.lean:546`): it keeps the application visible in LCNF instead of letting
the inliner erase the one-field wrapper into a bare `Nat`.

The surface notation is a **scoped term elaborator on Lean's own `@`**:

```lean
scoped elab_rules : term
  | `(@$n:num) => …  -- Address.ofNat, after the 2^256 bound check
```

`@` in term position is `Lean.Parser.Term.explicit`, and a numeral is never a
valid operand for it, so claiming `@<num>` takes nothing away from Lean.
Everything else falls through with `throwUnsupportedSyntax` to Lean's explicit
elaborator.  Verified experimentally on the pinned toolchain (4.32.2):
`@0x1` elaborates to `Address.ofNat 1`, `@List.map` still elaborates as
Lean's explicit marker, and the elaborator is `scoped` (active under
`open scoped Move`, which every `module` emits).

### 2. Address aliases are ordinary Lean `def`s — `@name` needs no new machinery

Move's book: *"you can think of `@` as an operator that takes an address from
being a namespace item to being an expression item"*, and *"named addresses
cannot be declared in Move"* — they come from `Move.toml`.  In Leaner the
analogue of that package file is a Lean file of constants:

```lean
namespace Move.Addresses
def std : Address := @0x1
def aptos_framework : Address := @0x1
def aptos_token : Address := @0x3
def core_resources : Address := @0xA550C18
end Move.Addresses
```

Then `@std` requires **zero implementation**: for a constant of non-function
type, Lean's own `@c` already elaborates to `c`.  The two readings of `@std`
coincide, so there is no ambiguity to resolve and no elaborator to write —
`@` is exactly the namespace-item-to-expression operator Move describes.

An alias is therefore the same thing as a Leaner integer constant (a `def` of a
literal, `leaner-move.md`), and a package declares its own the same way.
Shipped file: `Move/Move/Addresses.lean`, namespace `Move.Addresses`, imported
by `Move.lean` but **not** opened automatically (`std`, `vm` are plausible user
identifiers); a file that wants them writes `open Move.Addresses`.  Its values
mirror the framework `Move.toml` `[addresses]` sections, with a comment naming
them as the source of truth.

### 3. `module <name> at <alias | literal> where`

```
moveModuleCommand := "module " ident (" at " (num <|> ident))? " where" items
```

No `@` here, matching Move (`module 0x42::example`, `module std::other_module`);
`@` in this position is rejected with a message saying so.  Omitting `at` keeps
today's `0x0`, so no existing module changes.

The expander is a `Macro` and cannot read the environment, so it passes the
clause through to the existing internal command, whose elaborator resolves it:

```
#register_module_identity (num <|> ident) str
```

For an identifier the elaborator elaborates it with expected type
`Move.Address`, `whnf`s it, and reads the payload off the structure
constructor (obtained via `getStructureCtor`, so no private name is hard-coded)
with `Meta.evalNat`.  That accepts a literal alias, an alias defined in terms of
another alias, and rejects anything not a compile-time address with a
diagnostic.

## Layer-by-layer changes

**`Move/Move/Basic.lean`**
- Rename the `Address` payload `dummy → value`; it is now the address, so
  update the doc comment (the *model* meaning is `MoveModel.IR.Address = Nat`).
- Add `@[noinline] def Address.ofNat`.
- Optional: a `Repr`/`ToString` that prints `@0x…` rather than `{ value := 1 }`.

**`Move/Move/Syntax.lean`**
- The scoped `@<num>` elaborator, with the `n < 2^256` check reported at the
  numeral.

**`Move/Move/Compiler/LIR.lean`**
- `Instr.loadAddress (dst : String) (value : Nat)`, lowered to
  `.load (localId dst) (.address value)` with the 256-bit check.
- `Module.address`, `ExternalFunRef.address`: `String → Nat`.  Delete
  `parseAddress` and `hexDigit?` — they have no other caller, and with the
  surface parsing once and `Frontend/XIR/Json.encodeAddress` formatting once,
  no address is ever re-parsed mid-pipeline.

**`Move/Move/Attributes.lean`**
- `ModuleRef.address : Nat := 0`.

**`Move/Move/Compiler/Normalize.lean`**
- Next to the `UInt.ofNat` / `MoveInt.ofInt` literal case (line 1102): recognize
  `Move.Address.ofNat` applied to a tracked `natLiterals` entry → `.loadAddress`;
  otherwise *"address literal is not statically known"* — which is what rejects
  a computed address, Move's "addresses cannot be created from integers".
- `addressConstant?`, mirroring `uintConstant?` (line 450), so a `def` alias used
  inside a Move body lowers to its literal.  (`abbrev` aliases are inlined by
  LCNF and land in the `Address.ofNat` case instead; both work.)
- The type translation already maps `Move.Address → .address` (lines 105, 191).

**`Move/Move/Compiler/Export.lean`**
- `moveModuleCommand`: the optional `at` clause; `expandMoveModuleCommand`
  index shift (`stx[3] → stx[4]` for the items) and pass-through of the clause
  instead of the hard-coded `"0x0"` (line 949).
- `registerMoveModuleIdentity`: new grammar and the resolving elaborator above.
- Guard: reject registering an identity whose (address, name) is already claimed
  by a *different* Lean namespace — with real addresses in play, two `Vault`
  modules at `0x1` is now an expressible mistake.

**Docs**
- `leaner-move.md`: the `Address` row, the literals section ("There are no
  source-level `Address` or `Signer` literals" is retired for addresses), the
  constants section, and the `module` grammar.
- `project-plan.md`: drop address literals / module addresses from *Move
  language coverage — not in Leaner yet* and from priority item 4; leave signer
  literals and `use`-style renaming listed.

## Faithfulness

- **Opacity.** No arithmetic, no ordering, no `Address ↔ Nat` conversion is
  exposed at the surface; the only introduction form is a literal.  A
  non-literal `Address.ofNat` is rejected at the compilation boundary, the same
  safety net that already covers non-literal integer constants.
- **Bound.** `n < 2^256` is checked at the surface and again when LIR lowers the
  load.  The model itself is unbounded (`Address = Nat`), as it already is for
  module addresses — the bound is a source-language rule.
- **Logic.** `Address.ofNat` is a real function over a real payload, so
  `@0x1 ≠ @0x2` is `decide`-provable — which is exactly what a spec about a
  privileged address needs.  This is sound: the model's addresses *are* naturals.
- **Runtime equality.** Making literals distinguishable in the logic adds no
  axiom of its own: `@0x1 ≠ @0x2` is `decide`-provable because `ofNat` is a real
  injection.  Relating that to the source operator `addr == @0x1` is a separate
  matter, and the source model is wrong about it today — see
  [`structural-equality-design.md`](structural-equality-design.md).
- **`ModelDomain Address` stays the identity instance**, so
  `Move/Tests/ModelDomain.lean` is untouched.
- **Named-address substitution.** Move erases named addresses at the bytecode
  level; here they erase at LCNF, and module identity is the Lean namespace, so
  Move's "access members through the named address, not its value" rule has no
  Leaner counterpart to break.

## Tests

- `Move/Move/Tests/Addresses.lean` (added to the `Move.Tests` root):
  a `module … at 0xCAFE` with an address-literal result, an alias-valued
  constant, a resource borrowed at a literal address, and a `spec`/`verify`
  whose contract mentions `@0x1` (`requires addr = @0x1`, and a proof step that
  needs `@0x1 ≠ @0x2`).  Compiler side: `#test run "owner" [] [] =
  Tests.okRet [] [.address 0xCAFE]` and `#guard compiled.address = 0xCAFE`.
- A second module at a different address plus a cross-module call, asserting the
  caller's `externalFuns` records the callee's address — the regression that
  today's fixed `0x0` would hide.
- `Move/Tests/LowLevel/Rejections.lean`: computed `Address.ofNat`, a literal
  ≥ 2^256, `at` naming an unknown identifier, `at` naming a non-`Address`
  constant, `at @0x1`, and a duplicate module identity.
- `move-compiler-v2/transactional-tests/tests/leaner/addresses.lean`: publish a
  Leaner module at `0x42`, run `0x42::…`, and return an address literal — the
  end-to-end proof through the real toolchain and VM, which the Lean-side tests
  cannot give.
- Regenerate `Move/Tests/Account.xir.json` only if that module gains an address
  (it should not — leave it at `0x0` so the golden file also pins the default).

## Rollout (each step builds and tests green on its own)

0. [`structural-equality-design.md`](structural-equality-design.md) — `addr ==
   @admin` is the canonical framework idiom, and source `==` has no law today
   for any type but the unsigned integers.  Independent of the address work,
   but it must land before the address tests can say anything about `==`.
1. `Address.ofNat` + `@<num>` + normalizer + `Instr.loadAddress` — literals work
   in bodies, specs and proofs; module addresses still `0x0`.
2. `ModuleRef`/LIR address `String → Nat` — pure refactor, no surface change.
3. The `at` clause, the resolving identity elaborator, the duplicate-identity
   guard.
4. `Move/Move/Addresses.lean` with the conventional framework aliases.
5. Docs, transactional test.

## Non-goals / open questions

- **`Move.toml` integration.** A Leaner file is elaborated by a separate `lean`
  process (`move-compiler-v2/src/leaner.rs`), which passes no named-address map,
  so package aliases are declared in Lean and *duplicate* `Move.toml`.  The
  principled fix is to generate an `Addresses.lean` from the package manifest;
  worth doing once Leaner sources are consumed as real package targets.
- **Should `at` become mandatory?** Move requires an address on every module.
  Keeping it optional avoids churning ~20 test modules now; making it required
  is a one-line change once the framework files carry one.
- **`Address.toNat` / address ordering in specs.** Deliberately not exposed:
  the model would support it (`Address = Nat`), but nothing needs it yet, and
  the opaque surface is the Move-faithful default.
- **Signer literals** remain impossible, as in Move.
- **`use`-style module aliasing** (`use std::vector`) is a separate item: Leaner
  imports are Lean imports.
