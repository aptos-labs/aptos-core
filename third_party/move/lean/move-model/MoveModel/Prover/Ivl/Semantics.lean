-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Syntax

/-!
# IVL Operational Semantics

A big-step relational semantics for the block-structured IVL, in the demonic
verifier reading:

* `havoc R` yields any successor allowed by `R` (nondeterminism);
* `assume p` has **no** successor when `p` is false (the run is silently
  discarded — this is why `assume` is not an error);
* `assert p` reaches the distinguished error outcome `fail` when `p` is
  false;
* a `goto` may transfer to any target whose guard holds; if no guard holds,
  the run is discarded like a false `assume`.

An `Outcome` is either a normal state `ok s` or the single error token
`fail`.  There is deliberately no separate divergence token: the relation
simply does not relate a nonterminating execution to any outcome, so
soundness speaks only about terminating runs (partial correctness), which is
the intended reading of a verification condition.

Execution ignores loop annotations — a cyclic program simply iterates
through its back edges.  The annotations matter only for `wpB`; the
soundness theorem connects the two.
-/

namespace MoveModel.Prover.Ivl

/-- Result of executing IVL code: a normal final state, or the
assertion-failure token. -/
inductive Outcome (σ : Type) where
  | ok (s : σ)
  | fail

namespace Outcome

/-- An outcome *satisfies* a postcondition `Q` when normal states satisfy
`Q` and the failure outcome is disallowed.  WP validity means both "no
assertion failure" and "postcondition on normal exit". -/
def sat {σ : Type} (Q : σ → Prop) : Outcome σ → Prop
  | .ok s => Q s
  | .fail => False

@[simp] theorem sat_ok {σ : Type} (Q : σ → Prop) (s : σ) :
    (Outcome.ok s).sat Q = Q s := rfl

@[simp] theorem sat_fail {σ : Type} (Q : σ → Prop) :
    (Outcome.fail (σ := σ)).sat Q = False := rfl

end Outcome

/-- Big-step execution of a straight-line command list. -/
inductive CmdsExec {σ : Type} : List (BCmd σ) → σ → Outcome σ → Prop where
  | nil {s : σ} : CmdsExec [] s (.ok s)
  | assign {f : σ → σ} {cs : List (BCmd σ)} {s : σ} {o : Outcome σ}
      (h : CmdsExec cs (f s) o) :
      CmdsExec (.assign f :: cs) s o
  | havoc {R : σ → σ → Prop} {cs : List (BCmd σ)} {s s' : σ} {o : Outcome σ}
      (hR : R s s') (h : CmdsExec cs s' o) :
      CmdsExec (.havoc R :: cs) s o
  | assume {p : σ → Prop} {cs : List (BCmd σ)} {s : σ} {o : Outcome σ}
      (hp : p s) (h : CmdsExec cs s o) :
      CmdsExec (.assume p :: cs) s o
  | assertOk {p : σ → Prop} {cs : List (BCmd σ)} {s : σ} {o : Outcome σ}
      (hp : p s) (h : CmdsExec cs s o) :
      CmdsExec (.assert p :: cs) s o
  | assertFail {p : σ → Prop} {cs : List (BCmd σ)} {s : σ}
      (hp : ¬ p s) :
      CmdsExec (.assert p :: cs) s .fail

/-- Big-step execution of a program from the start of block `l`: run the
block's commands; on normal completion follow any enabled `goto` edge, or
end the frame at `ret`. -/
inductive BExec {σ : Type} (G : BProgram σ) : Label → σ → Outcome σ → Prop where
  | fail {l : Label} {cs : List (BCmd σ)} {t : BTerm σ} {s : σ}
      (hblk : G.blocks l = some ⟨cs, t⟩)
      (hcmds : CmdsExec cs s .fail) :
      BExec G l s .fail
  | ret {l : Label} {cs : List (BCmd σ)} {s s' : σ}
      (hblk : G.blocks l = some ⟨cs, .ret⟩)
      (hcmds : CmdsExec cs s (.ok s')) :
      BExec G l s (.ok s')
  | goto {l : Label} {cs : List (BCmd σ)}
      {targets : List ((σ → Prop) × Label)} {s s' : σ}
      {gt : (σ → Prop) × Label} {o : Outcome σ}
      (hblk : G.blocks l = some ⟨cs, .goto targets⟩)
      (hcmds : CmdsExec cs s (.ok s'))
      (hmem : gt ∈ targets)
      (hg : gt.1 s')
      (hnext : BExec G gt.2 s' o) :
      BExec G l s o

/-- Compose command executions: a normal run of `cs₁` followed by a run of
`cs₂`. -/
theorem CmdsExec.append_ok {σ : Type} {cs₁ cs₂ : List (BCmd σ)} {s s' : σ}
    {o : Outcome σ}
    (h₁ : CmdsExec cs₁ s (.ok s')) (h₂ : CmdsExec cs₂ s' o) :
    CmdsExec (cs₁ ++ cs₂) s o := by
  generalize ho : Outcome.ok s' = o₁ at h₁
  induction h₁ with
  | nil => cases ho; exact h₂
  | assign h ih => exact .assign (ih ho)
  | havoc hR h ih => exact .havoc hR (ih ho)
  | assume hp h ih => exact .assume hp (ih ho)
  | assertOk hp h ih => exact .assertOk hp (ih ho)
  | assertFail hp => cases ho

/-- A failing run of a prefix fails the whole list. -/
theorem CmdsExec.append_fail {σ : Type} {cs₁ cs₂ : List (BCmd σ)} {s : σ}
    (h₁ : CmdsExec cs₁ s .fail) :
    CmdsExec (cs₁ ++ cs₂) s .fail := by
  generalize ho : Outcome.fail (σ := σ) = o₁ at h₁
  induction h₁ with
  | nil => cases ho
  | assign h ih => exact .assign (ih ho)
  | havoc hR h ih => exact .havoc hR (ih ho)
  | assume hp h ih => exact .assume hp (ih ho)
  | assertOk hp h ih => exact .assertOk hp (ih ho)
  | assertFail hp => exact .assertFail hp

end MoveModel.Prover.Ivl
