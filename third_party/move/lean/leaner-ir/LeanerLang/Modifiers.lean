-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR
import LeanerLang.Ast

/-!
# Function modifiers of validated declarations

Visibility and the `entry`, `native`, `opaque`, `view`, and `deprecated`
modifiers, as the printer and the Move bytecode backend read them.
-/

namespace LeanerLang

open LeanerIR.Validation

private def visibilityOf (value : String) : Except String Visibility :=
  match value with
  | "private" | "visibility.private" => pure .private_
  | "public" | "visibility.public" => pure .public_
  | "package" | "visibility.package" => pure .package
  | "friend" | "visibility.friend" => pure .friend
  | _ => throw s!"unknown function visibility `{value}`"

private def mergeVisibility (current : Option Visibility)
    (next : Visibility) : Except String (Option Visibility) :=
  match current with
  | none => pure (some next)
  | some previous =>
      if previous == next then pure current
      else throw "function carries conflicting visibility metadata"

/-- The modifiers of a function of `profile` with these profile properties and
attributes, read from either spelling: the Move frontend's profile properties
or LeanerLang's attributes. -/
def metadataModifiers (profile : LeanerIR.Profile) (properties : Array LeanerIR.ProfileValue)
    (attributes : Array LeanerIR.Attribute) : Except String FunctionModifiers := do
  let mut result : FunctionModifiers := {}
  let mut visibility : Option Visibility := none
  for value in properties do
    unless value.profile == profile do
      throw s!"function profile metadata `{value.tag}` belongs to a different profile"
    if value.tag.startsWith "visibility." then
      visibility ← mergeVisibility visibility (← visibilityOf value.tag)
    else match value.tag with
      | "function.regular" => pure ()
      | "function.entry" => result := { result with isEntry := true }
      | "function.native" => result := { result with isNative := true }
      -- Move classifies both tags as frontend-only provenance. Canonical
      -- LeanerLang retains the ordinary function and prefix-call semantics.
      | "function.inlineRetained" | "function.receiver" => pure ()
      | tag => throw s!"function profile property `{tag}` has no canonical LeanerLang spelling"
  for attr in attributes do
    match attr with
    | .assign "visibility" (.qualifiedName value) _ =>
        visibility ← mergeVisibility visibility (← visibilityOf value)
    | .call "entry" arguments _ =>
        unless arguments.isEmpty do throw "the `entry` modifier attribute has arguments"
        result := { result with isEntry := true }
    | .call "native" arguments _ =>
        unless arguments.isEmpty do throw "the `native` modifier attribute has arguments"
        result := { result with isNative := true }
    | .call "opaque" arguments _ =>
        unless arguments.isEmpty do throw "the `opaque` modifier attribute has arguments"
        result := { result with isOpaque := true }
    | .call "deprecated" arguments _ =>
        unless arguments.isEmpty do throw "the `deprecated` modifier attribute has arguments"
        result := { result with isDeprecated := true }
    | .call "view" arguments _ =>
        unless arguments.isEmpty do throw "the `view` modifier attribute has arguments"
        result := { result with isView := true }
    | .call "bytecode_instruction" arguments _ =>
        unless arguments.isEmpty do
          throw "the `bytecode_instruction` provenance attribute has arguments"
    | _ => pure () -- Ordinary metadata is printed above the declaration.
  pure { result with visibility := visibility.getD .private_ }

/-- The modifiers a function declaration carries. A bodyless function without
`native` is opaque. -/
def declaredModifiers
    (declaration : LeanerIR.FunctionDecl FunctionBody) : Except String FunctionModifiers := do
  let result ← metadataModifiers declaration.profile declaration.profileData declaration.attributes
  if result.isNative && result.isOpaque then
    throw "a function cannot be both native and opaque"
  match declaration.body with
  | .structured _ =>
      if result.isNative || result.isOpaque then
        throw "a native or opaque function unexpectedly has an executable body"
      pure result
  | .absent =>
      pure <| if result.isNative then result else { result with isOpaque := true }

/-- Whether a closure with `store` may name a Move function: a public or entry
function, or one marked `@[persistent]`. -/
def moveStorableTarget (properties : Array LeanerIR.ProfileValue)
    (attributes : Array LeanerIR.Attribute) : Bool :=
  match metadataModifiers .move properties attributes with
  | .ok modifiers =>
      modifiers.visibility == .public_ || modifiers.isEntry || attributes.any fun
        | .call "persistent" _ _ => true
        | _ => false
  | .error _ => false

end LeanerLang
