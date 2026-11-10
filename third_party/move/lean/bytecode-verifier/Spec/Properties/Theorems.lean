-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Certificates
import Spec.Properties.Constants
import Spec.Properties.Declarations
import Spec.Properties.Instructions
import Spec.Properties.Instantiation
import Spec.Properties.Limits
import Spec.Properties.NonVacuity
import Spec.Properties.Ownership
import Spec.Properties.Encapsulation
import Spec.Properties.References
import Spec.Properties.FunctionValues

/-!
# The theorems' statements

TRUSTED. What the proof layer proves, for the model's `VerifierInterface`
(plan §8). Each statement fixes its premises: the production configuration,
the bytecode versions in scope, and, where execution is involved, the
supported subset and the native contracts. N2 and N3 are facts about the
semantics, the subset and the contracts alone, with no verifier premise.

D1, D4 and D5 are stated for every admissible initial state; D2 and D3 for the
initial states of transactions (E1, E2), which are admissible
(`EntryInitial.admissible`). D2, D3 and D4 also assume the natives' ownership
contracts (`NativesRespectOwnership`), which N3
(`NativeContractsSatisfiable`, `Spec.Properties.NonVacuity`) shows
satisfiable: they make native steps accountable. D4 does not depend on where
a value came from, only on reference parameters being distinct roots and the
held values containing no reference, which every admissible initial state
guarantees (`heldArguments`).

Status: plan Phase 4, stage B. `Supported` includes generic functions,
structs and enums and the generic instruction forms, and every statement
covers them: a frame runs a function instance with closed type arguments, and
the statements quantify over instances (`FunctionRef.typeArgs`). It includes
function types and the closure instructions too, with closures over natives
and natives naming function types left out; D3 gains D3c (closures are
created only where their target is visible), K2 the storability of
`persistent` handles, and D5 is stated (`FunctionValueSafety`).

Natives with references (language-owner rulings, 2026-11-03): `Supported`
admits the modeled natives (`Spec.Semantics.ModeledNatives`), and the native
premises describe their shapes. `NativesRespect` (D1, C1–C2, D5) asks every
result to fit its shape (`Program.nativeResultFits`); `NativesRespectOwnership`
(D2, D3, D4) asks every flow over the results and the written-back values to
respect ownership (`NativeCallRespects`). D4 needs no further premise: a
native reaches locations only through its reference arguments, which the ghost
layer records as reads, writes and uses, and the references it returns derive
from them. For a native that takes and returns values only, both premises are
what they were. The natives of `std::vector` marked `#[bytecode_instruction]`
are shape natives like the others (ruling of 2026-11-03): the premises
assume no more of them than their shapes.

Table natives (language-owner rulings, 2026-11-04): a table holds its entries
(`Value.table`), and the six table natives with references are modeled
natives where the program's `0x1::table` keeps the table rules
(`CompiledModule.followsTableRules`), part of their identification, so no
theorem gains a premise. `drop_unchecked_box` consumes a table with its
entries; the native policy `π` that D2 and D3 are stated under says which
values it may destroy (`NativePolicy.destroys`).
-/

namespace MoveBytecode

/-- **L1–L11** from local verification. L5 and L6 are stated in general, for
generic code and function types too; L7 for calls (closure creation joins
with function values); L11 against the production configuration. -/
def LocalProperties (V : VerifierInterface) : Prop :=
  ∀ m, V.verifyModule VerifierConfig.production m = true → 6 ≤ m.version → m.version ≤ 10 →
    m.IndexIntegrity ∧ m.Uniqueness ∧
    m.CodeStructure VerifierConfig.production.maxValueStackSize ∧
    m.ConstantsWellFormed ∧ m.InstructionConsistency ∧
    m.TypeFormation ∧ m.StructDefinitionsWellFormed ∧ m.FriendsWellFormed ∧
    m.InstantiationsFinite ∧ m.WithinLimits VerifierConfig.production

/-- **N2**: every function the soundness theorems cover, and whose parameter
types are inhabited, has an admissible initial state. -/
def EntriesAdmissible : Prop := ∀ P : Program, P.EntriesAdmissible

/-- **K1–K2** from local verification and link checks. K1 compares a
function handle's types with the definition's as runtime types, at every
instance, which exist only on the supported subset, so the statement is
restricted to it like the others; struct types compare by resolved name. A
function handle has the definition's number of type parameters and at least
its constraints. K1 for struct handles requires each handle to be a
conservative view of its definition: no more abilities, the same number of
type parameters, phantom only where the definition's is, and at least the
definition's constraints (`StructHandle.ConservativeFor`; corrected from
equality by triage, 2026-10-02). -/
def LinkedProperties (V : VerifierInterface) : Prop :=
  ∀ P, WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    P.HandleResolution ∧ P.CallVisibility ∧ P.StructHandleResolution ∧ P.PersistenceResolution

/-- **C1–C2**: certificates that are statically consistent and agree with
every execution, whatever the natives do within their contracts. The type
certificate is expressed over each function's type parameters
(`TypeCertificate.Parametric`). -/
def CertificatesHold (V : VerifierInterface) : Prop :=
  ∀ P, WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    ∃ (tc : TypeCertificate) (ac : AvailabilityCertificate),
      tc.Consistent P ∧ tc.Parametric P ∧ ac.Consistent P ∧
      ∀ N, NativesRespect P N → StaticTypes P N tc ∧ StaticAvailability P N ac

/-- **D1**, the main theorem: verified programs never go wrong. -/
def Soundness (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **N2 for transactions**: every entry function that returns nothing and a
transaction can supply arguments for has a transaction initial state. -/
def TransactionEntriesAdmissible : Prop := ∀ P : Program, P.TransactionEntriesAdmissible

/-- **D2**, ownership accounting: in every execution of a verified program
started by a transaction (E1, E2), with natives that meet their contracts
and flows `ν` that explain them, the ghost layer accounts for every step,
identities of types without `copy` are unique (D2a), occurrences of types
without `drop` disappear only through legitimate transfer or consumption
(D2b), and a returning execution conserves them (D2c). -/
def OwnershipAccounting (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (auth : List Address)
    (A : List StructId) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → NativesRespectOwnership P N ν π → EntryInitial P auth A s₀ →
    OwnershipAccounted P N ν π s₀

/-- **D3**, encapsulation and unforgeability: in every execution of a
verified program started by a transaction authenticated for `auth` (E1, E2),
with natives that meet their contracts under policy `π`, struct and enum
values are created and taken apart only by their defining module, natives
with its privileges, or the transaction for accepted argument types (D3a),
signers come only from authentication and the natives `π` allows (D3b), and
closures only from code their target is visible from, or from global storage
(D3c). -/
def Encapsulation (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (auth : List Address)
    (A : List StructId) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → NativesRespectOwnership P N ν π → EntryInitial P auth A s₀ →
    Encapsulated P N ν π auth A s₀

/-- **D4**, reference safety: in every execution of a verified program from
an admissible initial state, with natives that meet their contracts and flows
`ν` that explain them, the ghost layer accounts for every step, no reference
is used dangling (D4a), no access reaches over a `&mut` reference still to be
used, directly or from strictly above it (D4b′), no access while a frame is
active conflicts with a live `&mut` reference parameter of the frame (D4b″),
no write overlaps a live `&` reference (D4c), no reference escapes into a
value, into storage, or out of its frame other than through a reference
parameter (D4d), and no local or frame dies while a live reference points
into it (D4e). -/
def ReferenceSafety (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → NativesRespectOwnership P N ν π → AdmissibleInitial P s₀ →
    ReferencesSafe P N ν s₀

/-- **D5**, function values: in every execution of a verified program from an
admissible initial state, with natives that meet their contracts, captured
values hold no reference (D5a), every closure's abilities are justified by its
target and its captured values (D5b), `CallClosure` enters its target with
exactly its parameters, conforming (D5c), and only closures over storable
functions are in global storage (D5d). -/
def FunctionValueSafety (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → AdmissibleInitial P s₀ → FunctionValuesSafe P N s₀

/-- **D4a from D4b′, D4c and D4e**: a reference becomes invalid only through an
invalidation recorded since its creation, or the end of its frame, and each
comes with an access or a frame end that D4b′, D4c or D4e excludes before the
reference's last use (`Spec.Properties.References`). A fact about the
semantics and the ghost layer alone, with no verifier premise. -/
def NoDanglingFromContainment : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    P.Supported → AdmissibleInitial P s₀ → MutableContained P N ν s₀ →
    ImmutableStable P N ν s₀ → DeadSlotsUnborrowed P N ν s₀ → NoDanglingUse P N ν s₀

end MoveBytecode
