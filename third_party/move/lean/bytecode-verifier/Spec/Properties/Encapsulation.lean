-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Ownership

/-!
# Encapsulation and unforgeability D3

TRUSTED. D3 (plan §7) over annotated executions (`Trace`).

- **D3a encapsulation.** "Struct types can only be created ("packed"),
  destroyed ("unpacked") inside the module that defines the struct" (book,
  `structs-and-enums.md`, "Privileged Struct Operations"), and "the fields of
  a struct are only accessible inside the module that defines the struct";
  the global storage operations at `T` "must be declared in the current
  module" (book, `global-storage.md`). Every `Pack`, `Unpack`, `PackVariant`,
  `UnpackVariant`, field and variant field borrow, `TestVariant`, `MoveTo`,
  `MoveFrom`, `BorrowGlobal` and `Exists` at struct type `T` executes code of
  `T`'s defining module (plan §7, D3a). Construction and destruction are
  stated on values: every struct or enum value of type `T` created during an
  execution was created by code of `T`'s module, by a native with that
  module's privileges, by the transaction as an accepted argument (E1), or
  was in global storage when the transaction started; every such value taken
  apart was taken apart by code of `T`'s module or such a native. The other
  operations are stated on the `privileged` events the ghost layer records
  for them. Struct visibility (Move 2.4) does not widen this at the bytecode
  level: other modules "construct and destruct" a public struct through calls
  to its module's struct-API functions (book, "Struct and Enum Visibility",
  "Performance Consideration"), and "global storage operations remain
  module-only".
- **D3b unforgeability of signers.** "`signer` values are special because
  they cannot be created via literals or instructions — only by the Move VM"
  (book, `primitive-types.md`, "Signer"). No instruction creates a signer;
  signers come from authentication (E2) and from natives the policy allows.
- **D3c unforgeability of closures** (Phase 4, stage B). A closure over a
  function lets whoever holds it run the function, private or not:
  "`CallClosure` invokes whatever function the closure targets, including a
  private one; a module handing out a closure over its private function is
  intended" (plan §5.2). What keeps a module's private functions its own is
  who may create a closure over them: closure creation "must respect
  visibility" (plan §5.2; K2). Every closure value was created by `PackClosure`
  in code of a module its target is visible from, or was in global storage
  when the transaction started. No transaction argument is a closure (E1),
  and no native creates one (`Creator.MayCreateClosure`).

All three are also stated on the values present: every struct or enum value
in a state was created, with its type, by a creator allowed to, every signer
in a state is the very signer an allowed creator created, and every closure
in a state is the very closure an allowed creator created. This makes D3
independent of whether the ghost records a creation at the right step.

D3 does not restrict copying, dropping or comparing values of `T` in other
modules when `T`'s abilities allow it: copies keep their identities, and
discarding is not consumption. Nor does it restrict natives that read values
of `T` through references, as `bcs::to_bytes` and `cmp::compare` do: a
read-shape native writes nothing, creates no value of `T` and takes none apart
(plan §16, question 2; `Spec.Properties.Natives`, "Inspection"). D3 is
integrity, not confidentiality (language-owner ruling, 2026-11-04): it is about
who may create, take apart or change a value of a module's types, not who may
read it. A native's writes through a mutable reference to a value of `T` move
or consume nodes, and D3a judges those as for any native. The table natives
(language-owner rulings, 2026-11-04) are natives of `0x1::table`, with its
privileges over `Table` and `Box`: `add_box` and `remove_box` take a table apart
and make the one they write back, and an entry holding a value of another
module's type moves in and out with its identity, created and taken apart by
that module only.

**Generics** (plan Phase 4, stage A). A struct type is an instance, its
name with type arguments (`StructId`), and the privileges over it are its
defining module's, whatever the type arguments: "struct types can only be
created ("packed"), destroyed ("unpacked") inside the module that defines the
struct", and a generic struct is defined once for all its instances. So D3a
holds "under every instantiation" (plan §7, D3a): every event at
`Coin<A>` runs code of `Coin`'s module, wherever `A` is defined, and a module
defining `A` gains no privilege over `Coin<A>`. The ghost layer records each
generic form's event at its instance (`Spec.Semantics.Trace`).

Status: plan Phase 4, stage B.
-/

namespace MoveBytecode

namespace Encapsulation

/-- **D3a** for one event: a struct or enum value is created only by a
creator that may create its type, and taken apart only by a consumer that may
take its type apart; a field or variant field borrow, `TestVariant` or global
storage operation at a struct type runs code of the type's module. -/
def structsRespected (π : NativePolicy) (A : List StructId) : Event → Bool
  | .created _ v c =>
    match v.type with
    | .struct s => c.MayCreateStruct π A s
    | _ => true
  | .consumed _ v c =>
    match v.type with
    | .struct s => c.MayTakeApart π s
    | _ => true
  | .privileged m s _ => s.module == m
  | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered .. | .frameEnded .. => true

/-- **D3b** for one event: a signer is created only by a creator that may
create a signer for its address. -/
def signersRespected (π : NativePolicy) (auth : List Address) : Event → Bool
  | .created _ (.signer a) c => c.MayCreateSigner π auth a
  | .created _ _ _ => true
  | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
  | .frameEnded .. | .privileged .. => true

/-- **D3a** on the values present: every struct or enum value in `occ` was
created, as a value of its type, by a creator that may create that type. -/
def structsAuthentic (π : NativePolicy) (A : List StructId) (occ : List (Nat × Value))
    (evs : List Event) : Bool :=
  occ.all fun (i, v) =>
    match v.type with
    | .struct s =>
      evs.any fun
        | .created j w c => j == i && w.type == v.type && c.MayCreateStruct π A s
        | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
        | .frameEnded .. | .privileged .. => false
    | _ => true

/-- **D3b** on the values present: every signer in `occ` is the signer a
creator that may create it created. -/
def signersAuthentic (π : NativePolicy) (auth : List Address) (occ : List (Nat × Value))
    (evs : List Event) : Bool :=
  occ.all fun (i, v) =>
    match v with
    | .signer a =>
      evs.any fun
        | .created j w c => j == i && w == .signer a && c.MayCreateSigner π auth a
        | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
        | .frameEnded .. | .privileged .. => false
    | _ => true

/-- Whether `c` may create a closure over function instance `f` (D3c): code
of a module `f` is visible from, or global storage, for a closure stored
before the transaction. -/
def closureCreatorOk (P : Program) (c : Creator) (f : FunctionRef) : Bool :=
  match c with
  | .instruction m _ => P.visibleFrom m f
  | .storage => true
  | .transaction => false
  | .native _ _ _ => false

/-- **D3c** for one event: a closure is created only by a creator that may
create a closure over its target. -/
def closuresRespected (P : Program) : Event → Bool
  | .created _ (.closure f ..) c => closureCreatorOk P c f
  | .created _ _ _ => true
  | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
  | .frameEnded .. | .privileged .. => true

/-- **D3c** on the values present: every closure in `occ` is the closure a
creator that may create it created. -/
def closuresAuthentic (P : Program) (occ : List (Nat × Value)) (evs : List Event) : Bool :=
  occ.all fun (i, v) =>
    match v with
    | .closure f .. =>
      evs.any fun
        | .created j w c => j == i && w == v && closureCreatorOk P c f
        | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
        | .frameEnded .. | .privileged .. => false
    | _ => true

end Encapsulation

/-- **D3a.** Along every annotated execution from `s₀`, and in its last step,
every event respects the privileges of struct and enum types, and every
struct or enum value present was created by a creator allowed to. -/
def StructsEncapsulated (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (A : List StructId) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs →
    (∀ e ∈ evs, Encapsulation.structsRespected π A e = true) ∧
    Encapsulation.structsAuthentic π A (h.occurrences s) evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      (∀ e ∈ evs', Encapsulation.structsRespected π A e = true) ∧
      Encapsulation.structsAuthentic π A (post.occurrences o) (evs ++ evs') = true

/-- **D3b.** Along every annotated execution from `s₀`, and in its last step,
no signer is created except by authentication or an allowed native, and every
signer present is one they created. -/
def SignersUnforgeable (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (auth : List Address) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs →
    (∀ e ∈ evs, Encapsulation.signersRespected π auth e = true) ∧
    Encapsulation.signersAuthentic π auth (h.occurrences s) evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      (∀ e ∈ evs', Encapsulation.signersRespected π auth e = true) ∧
      Encapsulation.signersAuthentic π auth (post.occurrences o) (evs ++ evs') = true

/-- **D3c.** Along every annotated execution from `s₀`, and in its last step,
no closure is created except by code its target is visible from, and every
closure present is one an allowed creator created. -/
def ClosuresUnforgeable (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs →
    (∀ e ∈ evs, Encapsulation.closuresRespected P e = true) ∧
    Encapsulation.closuresAuthentic P (h.occurrences s) evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      (∀ e ∈ evs', Encapsulation.closuresRespected P e = true) ∧
      Encapsulation.closuresAuthentic P (post.occurrences o) (evs ++ evs') = true

/-- **D3**, encapsulation and unforgeability, for executions from `s₀` of a
transaction authenticated for `auth` whose layer accepts struct types `A`,
with natives `N`, their flows `ν`, and native policy `π`. Accounting must be
total (`ShadowAccounts`) for D3 to cover every step. -/
def Encapsulated (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (auth : List Address) (A : List StructId) (s₀ : State) : Prop :=
  ShadowAccounts P N ν s₀ ∧ StructsEncapsulated P N ν π A s₀ ∧
    SignersUnforgeable P N ν π auth s₀ ∧ ClosuresUnforgeable P N ν s₀

end MoveBytecode
