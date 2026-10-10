-- Test category: specification and verification.
import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Global invariants over the resource state, re-established at each change
(not at function end).  A *regular* invariant is assumed at reads and asserted
at writes; an *update* invariant relates the pre- and post-state of each write.
The primitives `moveTo`/`moveFrom`/`existsAt` participate in the semantics. -/

module GlobalInv where
  struct Counter has Key where
    value : U64

  -- A second family, so one module carries invariants over more than one
  -- resource and a single function can touch both.
  struct Ledger has Key where
    total : U64

  -- Regular: every stored Counter is positive.  Update: a Counter never
  -- decreases when its slot is written.  Comparisons read the unbounded value
  -- directly — no `.toNat`.  Each clause is registered under every family it
  -- names, so a write to that family re-checks it and no other.
  spec module where
    invariant ∀ a, 0 < Counter[a].value;
    invariant update ∀ a, old(Counter[a]).value ≤ Counter[a].value;
    invariant update ∀ a, old(Ledger[a]).total ≤ Ledger[a].total

  -- Incrementing preserves positivity and is monotone: verifies.
  entry fun increment (addr : Address) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + 1

  spec increment (addr : Address) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if ¬old(Counter[addr].value).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify increment

  -- Publishing must prove positivity of the new value; the update invariant is
  -- vacuous at a freshly published address.
  entry fun publish (account : &Signer) (amount : U64) : Action Unit :=
    moveTo account ({ value := amount } : Counter)

  spec publish (account : &Signer) (amount : U64) where
    requires 0 < amount.toNat;
    modifies Counter[account.address];
    ensures Counter[account.address].value = amount

  verify publish

  -- Existence test: a read leaves the state (and invariants) untouched, and
  -- the Bool result reflects existence.
  fun is_published (addr : Address) : Action Bool :=
    existsAt Counter addr

  spec is_published (addr : Address) where
    ensures result = true ↔ existsAt<Counter>(addr)

  verify is_published

  -- Removing a resource keeps both invariants (only the frame is touched).
  fun remove (addr : Address) : Action U64 := do
    let counter ← moveFrom Counter addr
    let Counter { value } := counter
    pure value

  spec remove (addr : Address) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures result = old(Counter[addr].value)

  verify remove

  -- Two resource families written by one function: each write re-establishes
  -- the invariants naming the family it wrote, and leaves the other's alone.
  entry fun record (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + amount
    let total ← &mut Ledger[addr].total
    total := *total + amount

  spec record (addr : Address) (amount : U64) where
    requires existsAt<Counter>(addr) ∧ existsAt<Ledger>(addr);
    modifies Counter[addr], Ledger[addr];
    ensures
      Counter[addr].value = old(Counter[addr].value) + amount ∧
      Ledger[addr].total = old(Ledger[addr].total) + amount;
    aborts_if ¬old(Counter[addr].value).toNat + amount.toNat < U64.size ∨
      ¬old(Ledger[addr].total).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify record
