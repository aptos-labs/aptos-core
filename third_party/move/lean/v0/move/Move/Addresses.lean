-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.Command
import Move.Basic

/-!
# Named Move addresses

Address aliases are ordinary Lean constants of type `Move.Address`, paired
with persistent compiler metadata containing their resolved 256-bit value.
The metadata survives imports and is shared by inline `@alias` expressions
and authored module identities.
-/

namespace Move

open Lean Elab Command Term

structure AddressAlias where
  sourceName : Name
  declaration : Name
  value : Nat
  deriving Inhabited, BEq, Repr

private def addAddressAlias (aliases : List AddressAlias)
    (alias : AddressAlias) : List AddressAlias :=
  alias :: aliases.filter (·.sourceName != alias.sourceName)

private initialize addressAliases :
    SimplePersistentEnvExtension AddressAlias (List AddressAlias) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := addAddressAlias
    addImportedFn := fun entries =>
      mkStateFromImportedEntries addAddressAlias [] entries
  }

def addressAliasBySource? (env : Environment) (sourceName : Name) :
    Option AddressAlias :=
  (addressAliases.getState env).find? (·.sourceName == sourceName)

def addressAliasByDeclaration? (env : Environment) (declaration : Name) :
    Option AddressAlias :=
  (addressAliases.getState env).find? (·.declaration == declaration)

def addressAlias? (env : Environment) (name : Name) : Option AddressAlias :=
  addressAliasBySource? env name <|> addressAliasByDeclaration? env name

def encodeAddress (value : Nat) : String :=
  "0x" ++ String.ofList (Nat.toDigits 16 value)

def addressLimit : Nat := 2 ^ 256

/-- Declare and register a package-level named address.

For example, `address_alias std = 0x1` creates the ordinary reducible
constant `std : Address` and makes both `@std` and `module M at std` legal.
-/
syntax (name := addressAliasCommand)
  "address_alias " ident " = " num : command

@[command_elab addressAliasCommand]
def elabAddressAliasCommand : CommandElab := fun stx => do
  let name : TSyntax `ident := ⟨stx[1]⟩
  let valueSyntax : TSyntax `num := ⟨stx[3]⟩
  let some value := valueSyntax.raw.isNatLit?
    | throwErrorAt valueSyntax "expected a numerical address value"
  unless value < addressLimit do
    throwErrorAt valueSyntax "address alias value does not fit in 256 bits"
  if let some previous := addressAliasBySource? (← getEnv) name.getId then
    throwErrorAt name
      "address alias `{name.getId}` is already registered by `{previous.declaration}`"
  let declaration ← `(def $name : Move.Address := Move.Address.ofNat $valueSyntax)
  elabCommand declaration
  let declarationName ← resolveGlobalConstNoOverload name.raw
  modifyEnv fun env => addressAliases.addEntry env {
    sourceName := name.getId
    declaration := declarationName
    value
  }

/- `@alias` needs no competing parser: Lean's existing explicit-application
syntax elaborates a zero-argument `Address` constant to the constant itself.
Keeping aliases as reducible definitions also lets normalization expose the
same `Address.ofNat` backend marker used by literal addresses. -/

end Move
