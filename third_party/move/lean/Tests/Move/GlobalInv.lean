import Move
import Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Global invariants over the resource state, re-established at each change
(not at function end).  A *regular* invariant is assumed at reads and asserted
at writes; an *update* invariant relates the pre- and post-state of each write.
The primitives `moveTo`/`moveFrom`/`exists_` participate in the semantics. -/

move_module GlobalInv where
  struct Counter where
    value : U64
    deriving Key

  -- Regular: every stored Counter is positive.  Update: a Counter never
  -- decreases when its slot is written.  Comparisons read the unbounded value
  -- directly — no `.toNat`.
  spec global where
    invariant (all a: 0 < Counter[a].value);
    invariant update (all a: old(Counter[a]).value ≤ Counter[a].value)

  -- Incrementing preserves positivity and is monotone: verifies.
  entry fun increment (addr : Address) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + 1

  spec increment (addr : Address) where
    requires exists<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if ¬old(Counter[addr].value).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify increment

  -- Publishing must prove positivity of the new value; the update invariant is
  -- vacuous at a freshly published address.
  entry fun publish (account : Signer) (amount : U64) : Action Unit :=
    moveTo account ({ value := amount } : Counter)

  spec publish (account : Signer) (amount : U64) where
    requires 0 < amount.toNat;
    modifies Counter[account.address];
    ensures Counter[account.address].value = amount

  verify publish

  -- Existence test: a read leaves the state (and invariants) untouched, and
  -- the Bool result reflects existence.
  fun isPublished (addr : Address) : Action Bool :=
    exists_ Counter addr

  spec isPublished (addr : Address) where
    ensures result = true ↔ exists<Counter>(addr)

  verify isPublished

  -- Removing a resource keeps both invariants (only the frame is touched).
  fun remove (addr : Address) : Action U64 := do
    let counter ← moveFrom Counter addr
    pure counter.value

  spec remove (addr : Address) where
    requires exists<Counter>(addr);
    modifies Counter[addr];
    ensures result = old(Counter[addr].value)

  verify remove
