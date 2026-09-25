-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Move address aliases

A Move named address is an alias for an address: `address_alias std = 0x1`
lets a Move path begin with `std` where it would begin with `0x1`. The alias
is only a spelling. A module's identity is its address and name, so
`std::vector` and `0x1::vector` name one module. Aliases persist across
imports; `LeanerLang.Addresses` declares the conventional Aptos ones.
-/

namespace LeanerLang

open Lean Elab Command

/-- Move addresses are 256 bits. -/
def moveAddressBound : Nat := 2 ^ 256

/-- The canonical spelling of an address: lowercase hexadecimal without
leading zeros. -/
def canonicalAddress (value : Nat) : String :=
  "0x" ++ String.ofList (Nat.toDigits 16 value)

/-- The value of a hexadecimal address spelling, `0x` followed by digits. -/
def addressValue? (spelling : String) : Option Nat := do
  guard (spelling.startsWith "0x" || spelling.startsWith "0X")
  let digits := (spelling.drop 2).toString.toList
  guard (!digits.isEmpty)
  digits.foldlM (init := 0) fun value digit => do
    let d ← if digit.isDigit then some (digit.toNat - '0'.toNat)
      else if 'a' ≤ digit && digit ≤ 'f' then some (digit.toNat - 'a'.toNat + 10)
      else if 'A' ≤ digit && digit ≤ 'F' then some (digit.toNat - 'A'.toNat + 10)
      else none
    some (value * 16 + d)

/-- A Move named address and the canonical address it stands for. -/
structure AddressAlias where
  name : String
  address : String
  deriving Inhabited, BEq, Repr

initialize addressAliasExtension :
    SimplePersistentEnvExtension AddressAlias (Std.HashMap String String) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun aliases alias => aliases.insert alias.name alias.address
    addImportedFn := fun imported => imported.foldl (init := {}) fun aliases entries =>
      entries.foldl (init := aliases) fun aliases alias => aliases.insert alias.name alias.address }

/-- The address aliases declared in, or imported into, an environment. -/
def addressAliases (environment : Environment) : Std.HashMap String String :=
  addressAliasExtension.getState environment

syntax (name := addressAliasCommand) "address_alias " ident " = " num : command

/-- Declare a Move named address. Declaring an alias again at the same
address is idempotent; at another address it is rejected. -/
@[command_elab addressAliasCommand]
def elabAddressAlias : CommandElab := fun stx => do
  let name := stx[1].getId.toString (escape := false)
  let some value := stx[3].isNatLit?
    | throwErrorAt stx[3] "an address alias stands for a numeric address"
  unless value < moveAddressBound do
    throwErrorAt stx[3] "a Move address is at most 256 bits"
  let address := canonicalAddress value
  match (addressAliases (← getEnv))[name]? with
  | some existing =>
      unless existing == address do
        throwErrorAt stx[1] s!"address alias `{name}` already stands for {existing}"
  | none => modifyEnv (addressAliasExtension.addEntry · { name, address })

end LeanerLang
