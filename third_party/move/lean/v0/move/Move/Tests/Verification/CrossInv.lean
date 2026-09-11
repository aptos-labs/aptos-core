-- Test category: specification and verification.
import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! A CROSS-RESOURCE global invariant relating two families. -/

module CrossInv where
  struct Debit has Key where
    value : U64
  struct Credit has Key where
    value : U64

  -- Debit never exceeds Credit at any address (relates two resources).
  spec module where
    invariant ∀ a, Debit[a].value.toNat ≤ Credit[a].value.toNat

  -- Shift shrinks Debit and grows Credit: maintains Debit ≤ Credit at each write.
  entry fun shift (addr : Address) (amount : U64) : Action Unit := do
    let debit ← &mut Debit[addr].value
    debit := *debit - amount
    let credit ← &mut Credit[addr].value
    credit := *credit + amount

  spec shift (addr : Address) (amount : U64) where
    requires existsAt<Debit>(addr) ∧ existsAt<Credit>(addr) ∧
      amount.toNat ≤ old(Debit[addr].value).toNat ∧
      old(Credit[addr].value).toNat + amount.toNat < U64.size;
    modifies Debit[addr], Credit[addr];
    ensures
      Debit[addr].value = old(Debit[addr].value) - amount ∧
      Credit[addr].value = old(Credit[addr].value) + amount;
    aborts_if False

  verify shift
