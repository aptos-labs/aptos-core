# Addresses — implemented design

## Status

Leaner supports literal addresses, package-level address aliases, address
values in expressions, and authored module identities:

```lean
import Move

open scoped Move

address_alias application = 0x42

module Registry at application where
  public fun own_address : Address := @application
  public fun literal_address : Address := @0xCAFE

module Other at 0x43 where
  ...
```

The `at` clause accepts either a numeric literal or an alias registered with
`address_alias`. It is optional for compatibility with existing Leaner source;
an omitted clause keeps the historical `0x0` module identity.

## Surface model

`Address` remains opaque to authored Move code. `Address.ofNat` is a
compiler-recognized introduction marker rather than a general conversion from
`Nat`: normalization accepts only a statically known literal and rejects a
computed address. Values are bounded by Move's 256-bit address domain.

The scoped `@0x...` syntax constructs a literal address. Because it is scoped,
a file using the notation must write `open scoped Move` before the command that
contains it. `module` opens the scope for the declarations it generates, but
Lean must already have parsed the module body, so opening it at file scope is
the reliable spelling.

An alias is declared with:

```lean
address_alias name = 0x42
```

The command creates an ordinary reducible constant `name : Move.Address` and
records persistent compiler metadata containing its source name, declaration
name, and resolved value. Consequently the same declaration supports both
roles:

- `@name` in value position uses Lean's existing explicit-name syntax and
  normalizes to an address load;
- `module M at name` resolves the persistent alias metadata and records the
  module's on-chain identity.

Only registered aliases are accepted by an `at` clause. An arbitrary
`def name : Address := ...` is intentionally not treated as package address
configuration. Alias metadata survives imports, so separately compiled Lean
modules retain the address needed for cross-module calls.

`Move.ConventionalAddresses` supplies the conventional Aptos names in the
`Move` namespace, including `std`, `aptos_framework`, `aptos_token`, and
`core_resources`. Projects can declare additional package aliases in their own
Lean files.

## Compiler representation

The underlying pipeline represents addresses as natural-number values:

| Layer | Representation |
| --- | --- |
| Source | opaque `Move.Address`; `Address.ofNat` marker |
| Alias metadata | `Move.AddressAlias` persistent environment entry |
| Module metadata | `Move.ModuleRef.address` plus optional source alias |
| LIR | `Instr.loadAddress`; module/external addresses encoded as hex strings |
| Semantic IR | `MoveModel.IR.Address = Nat`, `.address` values, module address |
| XIR | `{"address":"0x..."}` values and module/external address fields |
| compiler v2 | Move bytecode address constants and module identifiers |

Normalization recognizes both `Address.ofNat` and registered alias
declarations and emits `loadAddress`. The LIR boundary checks the 256-bit bound
again before constructing semantic IR. Imported module metadata supplies the
address of an external function reference; it is not replaced with the
caller's address or a fixed zero.

Address `==` uses the compiler-supported `Move.Compare.equal` marker through an
exact `BEq Address` instance. This prevents Lean from synthesizing the host
`DecidableEq` implementation, which is not part of the Move source language.
The separate question of which logical law verification may assume for generic
structural equality is documented in
[`structural-equality-design.md`](structural-equality-design.md).

## Validation and diagnostics

The implementation rejects:

- literal or alias values at or above `2^256`;
- unknown or unregistered aliases in a module identity;
- non-literal/non-alias terms after `at`;
- duplicate alias declarations;
- conflicting module registrations for one Lean namespace.

Lean-side coverage lives in `Move/Tests/Language/Addresses.lean` and its module fixtures.
It checks literal and aliased identities, imported aliases, cross-module
reference addresses, resource access at literal addresses, runtime address
equality, the bound, and unknown-alias diagnostics.

The compiler-v2 transactional test `tests/leaner/addresses.lean` publishes a
module at `0x42`, invokes it through that non-zero identity, returns both named
and literal addresses through the production VM, and checks true and false
address equality results.

## Deliberate non-goals

- `Move.toml` is not read by the Lean process. Package aliases are currently
  declared in Lean; generating an aliases file from a package manifest can be
  added when Leaner sources become first-class package targets.
- The `at` clause remains optional to avoid rewriting existing test modules.
- Address arithmetic, ordering, and an `Address.toNat` source operation are not
  exposed.
- Signer literals and Move `use` aliases are separate concerns.
