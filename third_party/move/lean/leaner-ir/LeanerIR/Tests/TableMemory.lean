-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Resources
import LeanerIR.Proofs.Denote.TableStorage
import LeanerIR.Proofs.Denote.TableReads

namespace LeanerIR.Tests.TableMemory

open Proofs.Denote

variable {unit : Validation.ValidatedUnit}

-- Native storage accepts any finite number of typed entries. In particular,
-- its carrier must not require a Move-vector bound merely to store entries.
-- Distinct keys are a separate map invariant, not a carrier-size restriction.
example (count : Nat) :
    ∃ contents : TableMemory.Contents unit .bool .bool,
      (TableMemory.entries .bool .bool contents).length = count := by
  exact ⟨Array.replicate count (true, false, ()), by simp [TableMemory.entries_length]⟩

-- Ordinary Move vectors still carry their unsigned-64 size bound.
example (value : (⟨.vector .bool, .nil, .value⟩ : ResourceType).carrier unit) :
    value.values.size < 2 ^ 64 := value.bounded

-- Generic instantiation preserves the native collection domain and also
-- instantiates the owner, which may mention phantom key/value arguments.
example (owner key value : NTy) (types : NRow) :
    (TableMemory.resource owner key value).subst types =
      TableMemory.resource (owner.subst types) (key.subst types) (value.subst types) := rfl

example (types : NRow) :
    TableMemory.allocationResource.subst types = TableMemory.allocationResource := rfl

-- An ordinary value resource cannot alias the native collection resource,
-- even if its type, arguments, and storage key happen to match.
example (owner key value : NTy) :
    TableMemory.resource owner key value ≠
      (⟨.tuple (.cons key (.cons value .nil)), .cons owner .nil, .value⟩ : ResourceType) := by
  intro equal
  cases congrArg ResourceType.kind equal

-- Two snapshots of the same allocation may observe different contents.
example (memory : Memory unit) (owner : NTy) (handle : String) :
    let before := TableMemory.write memory owner .bool .bool handle #[(true, false, ())]
    let after := TableMemory.write before owner .bool .bool handle #[(true, true, ())]
    Maps.Table.Snapshot.SameIdentity
        (TableMemory.snapshot before owner .bool .bool handle)
        (TableMemory.snapshot after owner .bool .bool handle) ∧
      (TableMemory.snapshot before owner .bool .bool handle).entries ≠
        (TableMemory.snapshot after owner .bool .bool handle).entries := by
  dsimp only
  constructor
  · rfl
  · simp [TableMemory.snapshot_write_entries, TableMemory.entries]

end LeanerIR.Tests.TableMemory
