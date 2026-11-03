-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostNative

/-!
# The ghost layer annotates every call of a modeled native

UNTRUSTED (checked by the kernel). A call of a modeled native
(`Program.nativeShape?`) is annotated (`Shadow.nativeStep`) as a call of a
native on values is (`Proofs.GhostNative`), and besides reads the locations
its reference arguments designate, writes back through its mutable ones and
borrows the places it returns:

- its events are accesses through its reference arguments, uses of them,
  borrows of its places, derived from a reference argument, consumptions and
  creations (`nativeEvents_cases`): it enters and ends no frame;
- the annotated call decomposes into the results pushed and the values
  written back (`nativeStep_decomp`).

Under the native contract its annotation exists and establishes `StepGhost`
(`Proofs.ModeledAnnotated`).
-/

namespace MoveBytecode.Proofs

/-! ## The parts of a native call's annotation -/

theorem outTreesOf_eq (φ : Nat → Path → Origin) (seen : List IdTree) (c : Creator)
    (outs : List Value) (fresh : Nat) :
    outTreesOf φ seen c outs fresh =
      outs.zipIdx.foldl (resultStep (resultIdOf φ seen) c) ([], fresh, []) := by
  unfold outTreesOf
  congr 1

/-- The trees of a native's outputs fit them; the native creates them with
identities from the fresh counter on. -/
theorem outTreesOf_spec (φ : Nat → Path → Origin) (seen : List IdTree) (c : Creator)
    (outs : List Value) (fresh : Nat) :
    IdTree.fitsAll (outTreesOf φ seen c outs fresh).1 outs = true ∧
      fresh ≤ (outTreesOf φ seen c outs fresh).2.1 ∧
      (∀ e ∈ (outTreesOf φ seen c outs fresh).2.2, ∃ i w, e = .created i w c) ∧
      (∀ o ∈ createdOf (outTreesOf φ seen c outs fresh).2.2,
        fresh ≤ o.1 ∧ o.1 < (outTreesOf φ seen c outs fresh).2.1) ∧
      ((createdOf (outTreesOf φ seen c outs fresh).2.2).map (·.1)).Nodup := by
  obtain ⟨ts, n, evs, hfold, hfit, -, -, hcr, hnd, hle, hev⟩ := results_fold
    (resultIdOf φ seen) c outs.zipIdx [] fresh []
  have heq : outTreesOf φ seen c outs fresh = (ts, n, evs) := by
    rw [outTreesOf_eq, hfold]; simp
  rw [heq]
  refine ⟨by simpa [List.zipIdx_map_fst] using hfit, hle, fun e he => ?_, hcr, hnd⟩
  obtain ⟨_, _, _, _, _, i, rfl⟩ := hev e he
  exact ⟨_, _, rfl⟩

/-- **Where the nodes of a native's outputs come from**: each takes the
identity of a node the native saw, or a new one. -/
theorem outTrees_occ (φ : Nat → Path → Origin) (seen : List IdTree) (c : Creator)
    (outs : List Value) (fresh : Nat) :
    ∀ o ∈ IdTree.occurrencesAll (outTreesOf φ seen c outs fresh).1 outs,
      (fresh ≤ o.1 ∧ o.1 < (outTreesOf φ seen c outs fresh).2.1) ∨
        ∃ k q, seenIdAt seen k q = some o.1 := by
  obtain ⟨ts, n, evs, hfold, -, hocc, -, hcr, -, -, -⟩ := results_fold
    (resultIdOf φ seen) c outs.zipIdx [] fresh []
  have heq : outTreesOf φ seen c outs fresh = (ts, n, evs) := by
    rw [outTreesOf_eq, hfold]; simp
  rw [heq]
  rw [List.zipIdx_map_fst] at hocc
  intro o ho
  rcases List.mem_append.1 (hocc.subset ho) with ho | ho
  · right
    obtain ⟨⟨v, j⟩, -, ho⟩ := List.mem_flatMap.1 ho
    obtain ⟨y, -, hyo⟩ := List.mem_filterMap.1 ho
    cases hi : resultIdOf φ seen j y.1 with
    | none => rw [hi] at hyo; cases hyo
    | some i =>
      rw [hi, Option.map_some, Option.some.injEq] at hyo
      subst hyo
      simp only [resultIdOf] at hi
      split at hi
      · next k q _ => exact ⟨k, q, hi⟩
      · cases hi
  · exact Or.inl (hcr o ho)

theorem consumedOfNative_mem {P : Program} {f : FunctionRef} {m : ModuleId}
    {ptys : List RuntimeType} {seen : List IdTree} {views : List Value}
    {origins : List (Nat × Path)} {e : Event}
    (he : e ∈ consumedOfNative P f m ptys seen views origins) :
    ∃ i w b, e = .consumed i w (.native f m b) := by
  simp only [consumedOfNative, List.mem_flatMap] at he
  obtain ⟨x, -, hx⟩ := he
  split at hx
  · simp at hx
  · obtain ⟨y, -, hy⟩ := List.mem_filterMap.1 hx
    split at hy
    · cases hy
    · simp only [Option.some.injEq] at hy
      exact ⟨_, _, _, hy.symm⟩

/-- **The events of a native call**: reads and writes through its reference
arguments, uses of them, borrows derived from them, consumptions and
creations. -/
theorem nativeEventsOf_cases {refArgs written : List (Nat × Reference)}
    {newRefs : List (Nat × Reference × Nat)} {consumed created : List Event} {e : Event}
    (he : e ∈ nativeEventsOf refArgs written newRefs consumed created) :
    (∃ x ∈ refArgs, e = .accessed .read x.2.location (some x.1)) ∨
      (∃ x ∈ written, e = .accessed .write x.2.location (some x.1)) ∨
      (∃ x ∈ refArgs, e = .used x.1 x.2) ∨
      (∃ x ∈ newRefs, e = .borrowed x.1 x.2.1 (some x.2.2)) ∨ e ∈ consumed ∨ e ∈ created := by
  simp only [nativeEventsOf, List.mem_append, List.mem_map] at he
  rcases he with ((((⟨x, hx, rfl⟩ | ⟨x, hx, rfl⟩) | ⟨x, hx, rfl⟩) | ⟨x, hx, rfl⟩) | he) | he
  · exact Or.inl ⟨x, hx, rfl⟩
  · exact Or.inr (Or.inl ⟨x, hx, rfl⟩)
  · exact Or.inr (Or.inr (Or.inl ⟨x, hx, rfl⟩))
  · exact Or.inr (Or.inr (Or.inr (Or.inl ⟨x, hx, rfl⟩)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl he))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr he))))

/-- **The events of an annotated native call**, by kind. -/
theorem nativeStep_events {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {e : Event} (he : e ∈ evs) :
    ∃ g hs, h.frames = g :: hs ∧ args.length ≤ g.stack.length ∧
      ((∃ x ∈ refArgsOf (g.stack.take args.length).reverse args,
          e = .accessed .read x.2.location (some x.1)) ∨
        (∃ x ∈ refArgsOf (g.stack.take args.length).reverse args,
          x.2.mutable = true ∧ P.nativeShape? f = some .update ∧
          e = .accessed .write x.2.location (some x.1)) ∨
        (∃ x ∈ refArgsOf (g.stack.take args.length).reverse args, e = .used x.1 x.2) ∨
        (∃ d r k, e = .borrowed d r (some k) ∧ h.fresh ≤ d ∧
          ∃ t ∈ (g.stack.take args.length).reverse, t.id = k) ∨
        (∃ i w c, e = .consumed i w c) ∨ (∃ i w c, e = .created i w c)) := by
  obtain ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, hh, -, -, -, hlen,
    -, -, -, -, -, -, -, hnew, -, -, -, -, hevs⟩ := nativeStep_spec hn
  refine ⟨g, hs, hh, hlen, ?_⟩
  rw [hevs] at he
  rcases nativeEventsOf_cases he with h1 | h1 | h1 | ⟨x, hx, rfl⟩ | h1 | h1
  · exact Or.inl h1
  · obtain ⟨x, hx, rfl⟩ := h1
    simp only [writtenOf, List.mem_filter, Bool.and_eq_true, beq_iff_eq] at hx
    exact Or.inr (Or.inl ⟨x, hx.1, hx.2.1, hx.2.2, rfl⟩)
  · exact Or.inr (Or.inr (Or.inl h1))
  · refine Or.inr (Or.inr (Or.inr (Or.inl ⟨x.1, x.2.1, x.2.2, rfl, ?_⟩)))
    unfold newRefsOf at hnew
    obtain ⟨y, hy, hyx⟩ := mapM_some_mem hnew x hx
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hyx
    obtain ⟨t, ht, rfl⟩ := hyx
    have hfr := (outTreesOf_spec (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
      (vals ++ writes) h.fresh).2.1
    exact ⟨Nat.le_trans hfr (Nat.le_add_right _ _), t, List.mem_of_getElem? ht, rfl⟩
  · obtain ⟨i, w, b, rfl⟩ := consumedOfNative_mem h1
    exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl ⟨i, w, _, rfl⟩))))
  · obtain ⟨i, w, rfl⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 e h1
    exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr ⟨i, w, _, rfl⟩))))

/-- **A native call accesses before it borrows**: its accesses come first
among its events, its borrows after its uses. -/
theorem nativeStep_order {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {j k d : Nat} {r : Reference} {par : Option Nat} {a : Access} {l : Location}
    {via : Option Nat} (hj : evs[j]? = some (.borrowed d r par))
    (hk : evs[k]? = some (.accessed a l via)) : k < j := by
  obtain ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, c1, c2, c3, c4, c5,
    c6, c7, c8, c9, c10, c11, c12, c13, c14, c15, c16, c17, hevs⟩ := nativeStep_spec hn
  generalize hX : (refArgsOf (g.stack.take args.length).reverse args).map
      (fun (x : Nat × Reference) => Event.accessed .read x.2.location (some x.1)) ++
    (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).map
      (fun (x : Nat × Reference) => Event.accessed .write x.2.location (some x.1)) = X at hevs
  have hsplit : evs = X ++ ((refArgsOf (g.stack.take args.length).reverse args).map
      (fun (x : Nat × Reference) => Event.used x.1 x.2) ++
    newRefs.map (fun (x : Nat × Reference × Nat) => Event.borrowed x.1 x.2.1 (some x.2.2)) ++
    consumedOfNative P f info.moduleId ptys seen views
      (passedOn (ν f views (vals ++ writes)) (vals ++ writes)) ++
    (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
      (vals ++ writes) h.fresh).2.2) := by
    rw [hevs, ← hX]
    simp [nativeEventsOf, List.append_assoc]
  have hXa : ∀ e ∈ X, ∃ a l via, e = .accessed a l via := by
    rw [← hX]
    intro e he
    simp only [List.mem_append, List.mem_map] at he
    rcases he with ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩
    · exact ⟨_, _, _, rfl⟩
    · exact ⟨_, _, _, rfl⟩
  rw [hsplit] at hj hk
  rcases Nat.lt_or_ge j X.length with hjx | hjx
  · rw [List.getElem?_append_left hjx] at hj
    obtain ⟨a', l', v', he⟩ := hXa _ (List.mem_of_getElem? hj)
    cases he
  rcases Nat.lt_or_ge k X.length with hkx | hkx
  · omega
  rw [List.getElem?_append_right hkx] at hk
  have hm := List.mem_of_getElem? hk
  simp only [List.mem_append, List.mem_map] at hm
  rcases hm with ((⟨x, -, he⟩ | ⟨x, -, he⟩) | he) | he
  · cases he
  · cases he
  · obtain ⟨_, _, _, he'⟩ := consumedOfNative_mem he; cases he'
  · obtain ⟨_, _, he'⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 _ he; cases he'

/-- A native call enters, returns from and ends nothing. -/
theorem nativeStep_quiet {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {e : Event} (he : e ∈ evs) :
    e.entered? = none ∧ e.returned? = none ∧ e.ends? = none ∧ (∀ x, e ≠ .vacated x) := by
  obtain ⟨g, hs, -, -, h1⟩ := nativeStep_events hn he
  rcases h1 with ⟨x, -, rfl⟩ | ⟨x, -, -, -, rfl⟩ | ⟨x, -, rfl⟩ | ⟨d, r, k, rfl, -⟩ |
    ⟨i, w, c, rfl⟩ | ⟨i, w, c, rfl⟩ <;>
    exact ⟨rfl, rfl, rfl, fun x h => by cases h⟩

/-- **The result of an annotated native call fits the native's shape**, as it
saw its arguments. -/
theorem nativeStep_fits {P : Program} {ν : NativeFlows} {s s₁ : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places
      (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs)) :
    ∃ tys, P.returnTypes f = some tys ∧
      P.nativeResultFits f tys views vals writes places = true := by
  obtain ⟨tys, views', -, -, -, -, htys, hviews, hfits, -⟩ := resume_spec (nativeStep_continues hn)
  obtain ⟨-, -, -, -, -, -, -, hfacts⟩ := step_callNative hstep
  rw [hfacts.view, Option.some.injEq] at hviews
  subst hviews
  exact ⟨tys, htys, hfits⟩

/-- A native that returns places is of the borrow shape. -/
theorem places_borrow {P : Program} {f : FunctionRef} {tys : List RuntimeType}
    {views vals writes : List Value} {places : List NativePlace}
    (hfits : P.nativeResultFits f tys views vals writes places = true) (hne : places ≠ []) :
    P.nativeShape? f = some .borrow := by
  cases hk : P.nativeShape? f with
  | none =>
    rw [nativeResultFits_none hk] at hfits
    simp only [Bool.and_eq_true, List.isEmpty_iff] at hfits
    exact absurd hfits.2 hne
  | some k =>
    cases k with
    | borrow => rfl
    | read => exact absurd (fits_values hk (by simp) hfits).2 hne
    | update => exact absurd (fits_values hk (by simp) hfits).2 hne

/-- **A reference an annotated native call borrows** is new, and derives from
the reference argument the place it designates is inside. -/
theorem nativeStep_borrowed {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {d : Nat} {r : Reference} {par : Option Nat} (hb : Event.borrowed d r par ∈ evs) :
    ∃ g hs, h.frames = g :: hs ∧ h.fresh ≤ d ∧ ∃ p ∈ places, ∃ t,
      (g.stack.take args.length).reverse[p.arg]? = some t ∧ par = some t.id := by
  obtain ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, hh, -, -, -, -,
    -, -, -, -, -, -, -, hnew, -, -, -, -, hevs⟩ := nativeStep_spec hn
  refine ⟨g, hs, hh, ?_⟩
  rw [hevs] at hb
  rcases nativeEventsOf_cases hb with ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, hx, he⟩ | he | he
  · cases he
  · cases he
  · cases he
  · simp only [Event.borrowed.injEq] at he
    obtain ⟨rfl, rfl, rfl⟩ := he
    unfold newRefsOf at hnew
    obtain ⟨y, hy, hyx⟩ := mapM_some_mem hnew x hx
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hyx
    obtain ⟨t, ht, rfl⟩ := hyx
    have hfr := (outTreesOf_spec (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
      (vals ++ writes) h.fresh).2.1
    have hyp : y.1.2 ∈ places := by
      have := List.mem_map_of_mem (f := Prod.fst) hy
      rw [List.zipIdx_map_fst] at this
      exact (List.of_mem_zip this).2
    exact ⟨Nat.le_trans hfr (Nat.le_add_right _ _), y.1.2, hyp, t, ht, rfl⟩
  · obtain ⟨i, w, b, he⟩ := consumedOfNative_mem he
    cases he
  · obtain ⟨i, w, he⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 _ he
    cases he

/-- **A reference an annotated native call uses** is one of its arguments,
on the caller's segment with its identity. -/
theorem nativeStep_used {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {i : Nat} {r : Reference} (hu : Event.used i r ∈ evs) :
    ∃ g hs, h.frames = g :: hs ∧ args.length ≤ g.stack.length ∧
      (i, r) ∈ refArgsOf (g.stack.take args.length).reverse args := by
  obtain ⟨g, hs, hh, hlen, h1⟩ := nativeStep_events hn hu
  refine ⟨g, hs, hh, hlen, ?_⟩
  rcases h1 with ⟨x, -, he⟩ | ⟨x, -, -, -, he⟩ | ⟨x, hx, he⟩ | ⟨_, _, _, he, -⟩ | ⟨_, _, _, he⟩ |
    ⟨_, _, _, he⟩
  all_goals first
    | cases he; done
    | (simp only [Event.used.injEq] at he
       obtain ⟨rfl, rfl⟩ := he
       exact hx)

theorem mem_refArgsOf {argIds : List IdTree} {args : List Value} {i : Nat} {r : Reference}
    (h : (i, r) ∈ refArgsOf argIds args) : ∃ t, t.id = i ∧ (t, Value.ref r) ∈ argIds.zip args := by
  simp only [refArgsOf, List.mem_filterMap] at h
  obtain ⟨⟨t, a⟩, hm, ha⟩ := h
  cases a with
  | ref r' =>
    simp only [Option.some.injEq, Prod.mk.injEq] at ha
    obtain ⟨rfl, rfl⟩ := ha
    exact ⟨t, rfl, hm⟩
  | _ => cases ha

theorem nativeView_getElem {s : State} :
    ∀ {args views : List Value}, s.nativeView args = some views → ∀ {i : Nat} {r : Reference},
      args[i]? = some (.ref r) → views[i]? = s.deref? r.location r.since
  | [], _, _, _, _, hi => by simp at hi
  | a :: args, views, h, i, r, hi => by
    unfold State.nativeView at h
    rw [List.mapM_cons] at h
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hi ⊢
      subst hi
      exact hb.symm
    | succ i =>
      simp only [List.getElem?_cons_succ] at hi ⊢
      exact nativeView_getElem hbs hi

theorem refArgsOf_first {argIds : List IdTree} {args : List Value} {r0 : Reference}
    (hlen : argIds.length = args.length) (h0 : args[0]? = some (.ref r0))
    (hrest : ∀ i v, args[i + 1]? = some v → ∀ r, v ≠ .ref r) :
    ∃ t0, argIds[0]? = some t0 ∧ refArgsOf argIds args = [(t0.id, r0)] := by
  cases args with
  | nil => simp at h0
  | cons a as =>
  cases argIds with
  | nil => simp at hlen
  | cons t0 ts =>
  simp only [List.getElem?_cons_zero, Option.some.injEq] at h0
  subst h0
  refine ⟨t0, rfl, ?_⟩
  simp only [refArgsOf, List.zip_cons_cons, List.filterMap_cons, List.cons.injEq, true_and]
  rw [List.filterMap_eq_nil_iff]
  intro x hx
  obtain ⟨t, a⟩ := x
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hx
  rw [List.getElem?_zip_eq_some] at hk
  cases a with
  | ref r => exact absurd rfl (hrest k (.ref r) (by simpa using hk.2) r)
  | _ => rfl

/-- **A call of a borrow-shape native**, annotated: its first argument is its
only reference argument; it writes nothing back, returns nothing but one new
reference, to a place inside what that argument designates, which exists
there and is of the result's target type, and the caller resumes with it. -/
theorem nativeStep_borrowShape {P : Program} {ν : NativeFlows} {s s₁ : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places
      (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs))
    (hshape : P.nativeShape? f = some .borrow) :
    ∃ fr rest n g hs r0 t0 m tgt X p, s.frames = fr :: rest ∧ h.frames = g :: hs ∧
      n ≤ fr.stack.length ∧ n ≤ g.stack.length ∧ args = (fr.stack.take n).reverse ∧
      s₁ = { s with frames := { fr with stack := fr.stack.drop n } :: rest } ∧
      args[0]? = some (.ref r0) ∧ (g.stack.take n).reverse[0]? = some t0 ∧
      refArgsOf (g.stack.take n).reverse args = [(t0.id, r0)] ∧
      P.returnTypes f = some [.ref m tgt] ∧ writes = [] ∧ vals = [] ∧ places = [p] ∧
      p.arg = 0 ∧ (m = true → r0.mutable = true) ∧ r0.target = X ∧ rigid X = true ∧
      (X = .vector tgt ∨ (X = .signer ∧ tgt = .address) ∨
        ∃ κ ν, X = .struct (tableStructId κ ν) ∧ tgt = tableEntryType ν) ∧
      RuntimeType.placeType? P X p.path = some tgt ∧
      (m = true → p.path.all PathStep.writable = true) ∧
      (∃ w, s.deref? r0.location r0.since = some w ∧ (w.get? p.path).isSome = true) ∧
      resumeNative P s₁ f args (.returned vals writes places) = .running
        { s with
          frames := { fr with
            stack := Value.ref ⟨m, tgt, r0.location.descend p.path, s.invalidations.length⟩ ::
              fr.stack.drop n
            pc := fr.pc + 1 } :: rest } := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  obtain ⟨tys, htys, hfits⟩ := nativeStep_fits hstep hn
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, hmam, hrig, hXt, hplace, hmw, view0,
    hv0, hget⟩ := borrow_result hshape hpt htys (typeArgsOk_values hfacts.typeArgs) hfits
  obtain ⟨r0, hra, hrm, hrX⟩ := conformsAll_ref_at hconfa hpX
  have hrX' : r0.target = X := assignable_rigid hrX hrig
  have hrest : ∀ i v, args[i + 1]? = some v → ∀ r, v ≠ .ref r := by
    intro i v hv
    obtain ⟨t, ht, hc⟩ := conformsAll_get hconfa hv
    refine not_ref_of_value hc ?_
    rcases borrow_sigs hshape hpt htys with ⟨τ, rfl, -⟩ | ⟨τ, rfl, -⟩ | ⟨rfl, -⟩ |
      ⟨κ, ν, hκ, rfl, -⟩ | ⟨κ, ν, hκ, rfl, -⟩ <;>
      (cases i <;> simp at ht) <;> subst ht <;>
      first | rfl | exact typeArgsOk_values hfacts.typeArgs _ (List.mem_of_getElem? hκ)
  obtain ⟨g, hs, -, -, -, -, -, -, -, -, -, -, hh, -, -, -, hlen, -⟩ := nativeStep_spec hn
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hal : ((g.stack.take n).reverse).length = args.length := by
    rw [hargl]; simp; omega
  obtain ⟨t0, ht0, hra'⟩ := refArgsOf_first hal hra hrest
  have hw0 : s.deref? r0.location r0.since = some view0 := by
    rw [← hv0, nativeView_getElem hfacts.view hra, hs₁]
    exact (deref_eq (s := s) (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest })
      (by simp [hsf]) rfl rfl rfl _ _).symm
  refine ⟨fr, rest, n, g, hs, r0, t0, m, tgt, X, p, hsf, ?_, hnl, ?_, hargs, hs₁, hra, ht0, hra',
    htys, rfl, rfl, rfl, harg, fun hm => by rw [hrm]; exact hmam hm, hrX', hrig, hXt, hplace, hmw,
    ⟨view0, hw0, hget⟩, ?_⟩
  · exact hh
  · omega
  · obtain ⟨tys', views', sW, results, frW, restW, htys', -, -, hwb, hres, hsW, ho⟩ :=
      resume_spec (nativeStep_continues hn)
    rw [htys, Option.some.injEq] at htys'
    subst htys'
    rw [writtenReferences_other (by rw [hshape]; simp)] at hwb
    simp only [writeBack, Option.some.injEq] at hwb
    subst hwb
    have hpr : (placeReferences s₁ [.ref m tgt] args [p]).bind (nativeResultsIn [.ref m tgt] []) =
        some [Step.borrow s₁ m tgt (r0.location.descend p.path)] := by
      simp [placeReferences, nativeResultsIn, harg, hra, RuntimeType.isValue]
    rw [hpr, Option.some.injEq] at hres
    subst hres
    rw [ho]
    subst hs₁
    simp only [List.cons.injEq] at hsW
    obtain ⟨rfl, rfl⟩ := hsW
    simp [Step.borrow]

/-- **The events of a call of a borrow-shape native**: a read through its
reference argument, a use of it, the borrow of the new reference derived from
it, consumptions and creations. -/
theorem nativeStep_borrowEvents {P : Program} {ν : NativeFlows} {s s₁ : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places
      (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs))
    (hshape : P.nativeShape? f = some .borrow) :
    ∃ (fr : Frame) (rest : List Frame) (g : ShadowFrame) (hs : List ShadowFrame) (r0 : Reference)
      (t0 : IdTree) (m : Bool) (tgt : RuntimeType) (p : NativePlace) (d : Nat),
      s.frames = fr :: rest ∧ h.frames = g :: hs ∧ t0 ∈ g.stack ∧ Value.ref r0 ∈ fr.stack ∧ (m = true → r0.mutable = true) ∧ h.fresh ≤ d ∧
      p.path ≠ [] ∧
      Event.used t0.id r0 ∈ evs ∧
      Event.borrowed d ⟨m, tgt, r0.location.descend p.path, s.invalidations.length⟩ (some t0.id) ∈
        evs ∧
      (∀ e ∈ evs, e = .accessed .read r0.location (some t0.id) ∨ e = .used t0.id r0 ∨
        e = .borrowed d ⟨m, tgt, r0.location.descend p.path, s.invalidations.length⟩
          (some t0.id) ∨
        (∃ i w c, e = .consumed i w c) ∨ (∃ i w c, e = .created i w c)) ∧
      (∃ w, s.deref? r0.location r0.since = some w ∧ (w.get? p.path).isSome = true) ∧
      ∃ n, resumeNative P s₁ f args (.returned vals writes places) = .running
        { s with
          frames := { fr with
            stack := Value.ref ⟨m, tgt, r0.location.descend p.path, s.invalidations.length⟩ ::
              fr.stack.drop n
            pc := fr.pc + 1 } :: rest } := by
  obtain ⟨fr, rest, n, g, hs, r0, t0, m, tgt, X, p, hsf, hh, hnl, hng, hargs, -, hra, ht0, hrefs,
    htys, rfl, rfl, rfl, harg, hm, -, -, hXt, hplace, -, hderef, ho⟩ := nativeStep_borrowShape hstep
      hn hshape
  have hp0 : p.path ≠ [] := by
    intro hnil
    rw [hnil, placeType?_nil, Option.some.injEq] at hplace
    subst hplace
    exact borrow_ne hXt rfl
  rw [ho] at hn
  obtain ⟨g', hs', info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, hh', -, htys', -,
    -, -, -, -, -, hs'', hf', -, hnew, -, -, -, -, hevs⟩ := nativeStep_spec hn
  rw [hh, List.cons.injEq] at hh'
  obtain ⟨rfl, rfl⟩ := hh'
  rw [htys, Option.some.injEq] at htys'
  subst htys'
  simp only [Outcome.running.injEq] at hs''
  subst hs''
  simp only [List.cons.injEq] at hf'
  obtain ⟨rfl, rfl⟩ := hf'
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  rw [hargl] at hnew hevs
  simp [refResultsOf, newRefsOf, RuntimeType.isValue, harg, ht0] at hnew
  subst hnew
  rw [hrefs] at hevs
  have hw : writtenOf P f [(t0.id, r0)] = [] := by simp [writtenOf, hshape]
  rw [hw] at hevs
  have hfr := (outTreesOf_spec (ν f views ([] ++ [])) seen (.native f info.moduleId views)
    ([] ++ []) h.fresh).2.1
  refine ⟨fr, rest, g, hs, r0, t0, m, tgt, p, _, hsf, hh, ?_, ?_, hm, Nat.le_trans hfr
    (Nat.le_refl _), hp0, by simp [hevs, nativeEventsOf], by simp [hevs, nativeEventsOf],
    fun e he => ?_, hderef, n, ho⟩
  · have := List.mem_of_getElem? ht0
    exact List.mem_of_mem_take (List.mem_reverse.1 this)
  · have := List.mem_of_getElem? hra
    rw [hargs] at this
    exact List.mem_of_mem_take (List.mem_reverse.1 this)
  · rw [hevs] at he
    rcases nativeEventsOf_cases he with ⟨x, hx, rfl⟩ | ⟨x, hx, -⟩ | ⟨x, hx, rfl⟩ | ⟨x, hx, rfl⟩ |
      he | he
    · simp only [List.mem_singleton] at hx; subst hx; exact Or.inl rfl
    · simp at hx
    · simp only [List.mem_singleton] at hx; subst hx; exact Or.inr (Or.inl rfl)
    · simp only [List.mem_singleton] at hx; subst hx; exact Or.inr (Or.inr (Or.inl rfl))
    · obtain ⟨i, w, b, rfl⟩ := consumedOfNative_mem he
      exact Or.inr (Or.inr (Or.inr (Or.inl ⟨i, w, _, rfl⟩)))
    · obtain ⟨i, w, rfl⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 _ he
      exact Or.inr (Or.inr (Or.inr (Or.inr ⟨i, w, _, rfl⟩)))

/-! ## Writing back under the caller's segment -/

/-- Replacing a root's value changes at most the current frame's locals, and
commutes with replacing its operand stack and position. -/
theorem setRoot_top {s : State} {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest)
    (ρ : Root) (v : Value) :
    ∃ L rest', (s.setRoot ρ v).frames = { fr with locals := L } :: rest' ∧
      ∀ st pc, ({ s with frames := { fr with stack := st, pc := pc } :: rest } : State).setRoot ρ v =
        { s.setRoot ρ v with frames := { fr with stack := st, pc := pc, locals := L } :: rest' } := by
  cases ρ with
  | local_ f i =>
    by_cases hf : fr.id = f
    · refine ⟨fr.locals.setIfInBounds i (some v), rest.map fun g =>
        if g.id == f then { g with locals := g.locals.setIfInBounds i (some v) } else g, ?_, ?_⟩
      · simp [State.setRoot, hsf, hf]
      · intro st pc; simp [State.setRoot, hf]
    · refine ⟨fr.locals, rest.map fun g =>
        if g.id == f then { g with locals := g.locals.setIfInBounds i (some v) } else g, ?_, ?_⟩
      · simp [State.setRoot, hsf, hf]
      · intro st pc; simp [State.setRoot, hf]
  | global a t => exact ⟨fr.locals, rest, by simp [State.setRoot, hsf], fun st pc => rfl⟩
  | held k => exact ⟨fr.locals, rest, by simp [State.setRoot, hsf], fun st pc => rfl⟩

/-- **Writing back changes at most the current frame's locals**, and
commutes with replacing its operand stack and position. -/
theorem writeBack_top : ∀ (refs : List Reference) (ws : List Value) {s s' : State} {fr : Frame}
    {rest : List Frame}, s.frames = fr :: rest → writeBack s refs ws = some s' →
    ∃ L rest', s'.frames = { fr with locals := L } :: rest' ∧
      ∀ st pc, writeBack { s with frames := { fr with stack := st, pc := pc } :: rest } refs ws =
        some { s' with frames := { fr with stack := st, pc := pc, locals := L } :: rest' }
  | [], [], s, s', fr, rest, hsf, h => by
    simp only [writeBack, Option.some.injEq] at h
    subst h
    exact ⟨fr.locals, rest, by simp [hsf], fun st pc => by simp [writeBack]⟩
  | r :: refs, w :: ws, s, s', fr, rest, hsf, h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨v, hv, v'', hset, rfl⟩ := hw
    obtain ⟨L₁, rest₁, hf₁, htop⟩ := setRoot_top hsf r.location.root v''
    obtain ⟨L, rest', hf', hrest⟩ := writeBack_top refs ws (s := (s.setRoot r.location.root v'').invalidate
      (.mutated r.location)) hf₁ h
    refine ⟨L, rest', by rw [hf'], fun st pc => ?_⟩
    have hroot : ({ s with frames := { fr with stack := st, pc := pc } :: rest } : State).root?
        r.location.root = some v := by
      rw [root_eq (s := s) (s' := { s with frames := { fr with stack := st, pc := pc } :: rest })
        (by simp [hsf]) rfl rfl]
      exact hv
    simp only [writeBack, State.write?, hroot, hset, bind, Option.bind_some, pure]
    rw [htop st pc]
    exact hrest st pc
  | [], _ :: _, _, _, _, _, _, h => by simp [writeBack] at h
  | _ :: _, [], _, _, _, _, _, h => by simp [writeBack] at h

/-! ## The shadow written back -/

theorem get_of_set : ∀ {p : List PathStep} {v w v' : Value}, v.set? p w = some v' →
    ∃ u, v.get? p = some u
  | [], v, _, _, _ => ⟨v, rfl⟩
  | st :: p, v, w, v', h => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨c, hc, c', hc', -⟩ := h
    obtain ⟨u, hu⟩ := get_of_set hc'
    exact ⟨u, by simp [Value.get?, hc, hu]⟩

theorem read_of_write {s s' : State} {l : Location} {w : Value} (h : s.write? l w = some s') :
    ∃ u, s.read? l = some u := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨v, hv, v'', hset, -⟩ := h
  obtain ⟨u, hu⟩ := get_of_set hset
  exact ⟨u, by simp [State.read?, hv, hu]⟩

/-- The shadow's trees at locations depend on the state through its frames'
identities, its resources' keys and the node indices of the location in the
value at its root only. -/
theorem tree_state' {h : Shadow} {s s' : State} (hids : s'.frames.map (·.id) = s.frames.map (·.id))
    (hst : s'.storage.map (fun r => (r.address, r.type)) =
      s.storage.map (fun r => (r.address, r.type))) (l : Location) (t : IdTree)
    (hidx : (s'.root? l.root).map l.indicesIn = (s.root? l.root).map l.indicesIn) :
    h.tree? s' l = h.tree? s l ∧ h.withTree? s' l t = h.withTree? s l t := by
  have := rootTree_congr (h := h) hids hst
  simp only [Shadow.tree?, Shadow.withTree?, this.1, this.2]
  cases h1 : s'.root? l.root <;> cases h2 : s.root? l.root <;> rw [h1, h2] at hidx <;>
    simp_all

@[simp] theorem root?_invalidate (s : State) (e : Invalidation) (ρ : Root) :
    (s.invalidate e).root? ρ = s.root? ρ := by
  cases ρ <;> rfl

/-- **The locations written back agree**: they overlap one another nowhere,
and each has in `s` the node indices it has in `s₀`, in the value at its
root. -/
def WritesAgree (s s₀ : State) (W : List (Nat × Reference)) : Prop :=
  (W.map (·.2)).Pairwise (fun a b => a.location.overlaps b.location = false) ∧
    ∀ x ∈ W, (s.root? x.2.location.root).map x.2.location.indicesIn =
      (s₀.root? x.2.location.root).map x.2.location.indicesIn

theorem WritesAgree.of_roots {s s₀ : State} {W : List (Nat × Reference)}
    (hW : (W.map (·.2)).Pairwise (fun a b => a.location.overlaps b.location = false))
    (hroot : ∀ ρ, s.root? ρ = s₀.root? ρ) : WritesAgree s s₀ W :=
  ⟨hW, fun x _ => by rw [hroot]⟩

theorem WritesAgree.tail {s s₀ s₂ : State} {x : Nat × Reference} {W : List (Nat × Reference)}
    {w : Value} (h : WritesAgree s s₀ (x :: W)) (hw : s.write? x.2.location w = some s₂)
    (e : Invalidation) : WritesAgree (s₂.invalidate e) s₀ W := by
  obtain ⟨hp, hi⟩ := h
  simp only [List.map_cons, List.pairwise_cons] at hp
  refine ⟨hp.2, fun y hy => ?_⟩
  simp only [root?_invalidate]
  rw [indicesIn_write_disjoint hw (hp.1 y.2 (List.mem_map_of_mem hy))]
  exact hi y (List.mem_cons_of_mem _ hy)

theorem WritesAgree.head {s s₀ : State} {x : Nat × Reference} {W : List (Nat × Reference)}
    (h : WritesAgree s s₀ (x :: W)) :
    (s.root? x.2.location.root).map x.2.location.indicesIn =
      (s₀.root? x.2.location.root).map x.2.location.indicesIn :=
  h.2 x List.mem_cons_self

theorem setRoot_storageKeys (s : State) (ρ : Root) (v : Value) :
    (s.setRoot ρ v).storage.map (fun r => (r.address, r.type)) =
      s.storage.map (fun r => (r.address, r.type)) := by
  cases ρ with
  | local_ f i => rfl
  | held k => rfl
  | global a t =>
    simp only [State.setRoot, GlobalStorage.update, List.map_map]
    refine List.map_congr_left fun r _ => ?_
    simp only [Function.comp]
    split <;> rfl

theorem setRoot_frameIds (s : State) (ρ : Root) (v : Value) :
    (s.setRoot ρ v).frames.map (·.id) = s.frames.map (·.id) := by
  cases ρ with
  | local_ f i =>
    simp only [State.setRoot, List.map_map]
    refine List.map_congr_left fun g _ => ?_
    simp only [Function.comp]
    split <;> rfl
  | held k => rfl
  | global a t => rfl

theorem shadow_frames_length {h : Shadow} {s : State} (hf : h.fits s = true) :
    h.frames.length = s.frames.length := by
  rw [shadow_fits] at hf
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq] at hf
  exact hf.1.1

/-- **Writing back, in the state and its shadow**: the shadow written back,
each location's tree replaced by a written value's, fits the state written
back; every occurrence in a frame after was there before, or is in a written
value. -/
theorem writes_ghost {s₀ : State} :
    ∀ (W : List (Nat × Reference)) (ts : List IdTree) (ws : List Value) {h h' : Shadow}
      {s s' : State},
    h.fits s = true → (s.frames.map (·.id)).Nodup →
    (s.storage.map fun r => (r.address, r.type)).Nodup →
    s.frames.map (·.id) = s₀.frames.map (·.id) →
    s.storage.map (fun r => (r.address, r.type)) = s₀.storage.map (fun r => (r.address, r.type)) →
    WritesAgree s s₀ W →
    writeBack s (W.map (·.2)) ws = some s' →
    (W.zip ts).foldlM (fun h x => h.withTree? s₀ x.1.2.location x.2) h = some h' →
    IdTree.fitsAll ts ws = true →
    h'.fits s' = true ∧ h'.fresh = h.fresh ∧ s'.frames.map (·.id) = s.frames.map (·.id) ∧
      s'.storage.map (fun r => (r.address, r.type)) =
        s.storage.map (fun r => (r.address, r.type)) ∧
      (∀ (k : Nat) (g' : ShadowFrame) (fr' : Frame) (g : ShadowFrame) (fr : Frame),
        h'.frames[k]? = some g' → s'.frames[k]? = some fr' →
        h.frames[k]? = some g → s.frames[k]? = some fr → ∀ x ∈ g'.occurrences fr',
          (x.1, x.2.key) ∈ keys (g.occurrences fr) ∨
            ∃ y ∈ ts.zip ws, (x.1, x.2.key) ∈ keys (y.1.occurrences y.2)) ∧
      (∀ x ∈ framesOcc h'.frames s'.frames, (x.1, x.2.key) ∈ keys (framesOcc h.frames s.frames) ∨
          ∃ y ∈ ts.zip ws, (x.1, x.2.key) ∈ keys (y.1.occurrences y.2)) ∧
      (∀ x ∈ keys (h'.occurrences s'), x ∈ keys (h.occurrences s) ∨
          ∃ y ∈ ts.zip ws, x ∈ keys (y.1.occurrences y.2))
  | [], [], [], h, h', s, s', hfit, _, _, _, _, _, hwb, hfold, _ => by
    simp only [List.map_nil, writeBack, Option.some.injEq] at hwb
    simp only [List.zip_nil_left, List.foldlM_nil, pure, Option.some.injEq] at hfold
    subst hwb hfold
    exact ⟨hfit, rfl, rfl, rfl, fun k g' fr' g fr h1 h2 h3 h4 x hx => by
      rw [h1, Option.some.injEq] at h3; rw [h2, Option.some.injEq] at h4
      subst h3 h4; exact Or.inl (mem_keys hx), fun x hx => Or.inl (mem_keys hx),
      fun x hx => Or.inl hx⟩
  | x :: W, t :: ts, w :: ws, h, h', s, s', hfit, hids, hres, hids₀, hst₀, hag, hwb, hfold,
      hfa => by
    simp only [List.map_cons, writeBack, bind, Option.bind_eq_some_iff] at hwb
    obtain ⟨s₂, hw, hwb⟩ := hwb
    simp only [List.zip_cons_cons, List.foldlM_cons, bind, Option.bind_eq_some_iff] at hfold
    obtain ⟨h₂, hwith, hfold⟩ := hfold
    simp only [IdTree.fitsAll, Bool.and_eq_true] at hfa
    obtain ⟨u, hu⟩ := read_of_write hw
    have hag₂ := hag.tail hw (.mutated x.2.location)
    have hwith' : h.withTree? s x.2.location t = some h₂ := by
      rw [(tree_state' (h := h) hids₀ hst₀ x.2.location t hag.head).2]; exact hwith
    obtain ⟨tw, v, v'', h₃, hv, -, -, hset, hwr, hwith₃, hfit₃, hfresh₃, -, hfrm₃, R, hR, hR'⟩ :=
      tree_write hfit hids hres hu hfa.1 (write?_writable hw)
    rw [hwith', Option.some.injEq] at hwith₃
    subst hwith₃
    rw [hwr, Option.some.injEq] at hw
    subst hw
    have hpf := tree_write_frames hfit hids hu hfa.1 hwith' hwr
    have hids₂ := setRoot_frameIds s x.2.location.root v''
    have hst₂ := setRoot_storageKeys s x.2.location.root v''
    obtain ⟨hfit', hfresh', hids', hst', hpf', hfrm', hall'⟩ := writes_ghost W ts ws
      (s := (s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location))
      hfit₃ (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).frames.map (·.id) =
          s.frames.map (·.id) from hids₂]; exact hids)
      (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).storage.map _ =
          s.storage.map _ from hst₂]; exact hres)
      (hids₂.trans hids₀) (hst₂.trans hst₀) hag₂ hwb hfold hfa.2
    refine ⟨hfit', hfresh'.trans hfresh₃, hids'.trans hids₂, hst'.trans hst₂,
      fun k g' fr' g fr h1 h2 h3 h4 y hy => ?_, fun y hy => ?_, fun y hy => ?_⟩
    · have hk : k < s.frames.length := (List.getElem?_eq_some_iff.1 h4).1
      have hl₃ : (s.setRoot x.2.location.root v'').frames.length = s.frames.length := by
        rw [← List.length_map (f := (·.id)), hids₂, List.length_map]
      obtain ⟨g₂, hg₂⟩ : ∃ g₂, h₂.frames[k]? = some g₂ :=
        ⟨_, List.getElem?_eq_getElem (by
          rw [shadow_frames_length hfit₃]
          show k < (s.setRoot x.2.location.root v'').frames.length
          rw [hl₃]; exact hk)⟩
      obtain ⟨fr₂, hfr₂⟩ : ∃ fr₂, ((s.setRoot x.2.location.root v'').invalidate
          (.mutated x.2.location)).frames[k]? = some fr₂ :=
        ⟨_, List.getElem?_eq_getElem (by
          show k < (s.setRoot x.2.location.root v'').frames.length
          rw [hl₃]; exact hk)⟩
      rcases hpf' k g' fr' g₂ fr₂ h1 h2 hg₂ hfr₂ y hy with hy | ⟨z, hz, hy⟩
      · obtain ⟨o, ho, ho1, ho2⟩ := mem_keys_iff.1 hy
        rcases hpf k g₂ fr₂ g fr hg₂ hfr₂ h3 h4 o ho with ho' | ho'
        · rw [ho1, ho2] at ho'; exact Or.inl ho'
        · rw [ho1, ho2] at ho'; exact Or.inr ⟨(t, w), List.mem_cons_self, ho'⟩
      · exact Or.inr ⟨z, List.mem_cons_of_mem _ hz, hy⟩
    · rcases hfrm' y hy with hy | ⟨z, hz, hy⟩
      · obtain ⟨o, ho, ho1, ho2⟩ := mem_keys_iff.1 hy
        rcases hfrm₃ o ho with ho' | ho'
        · rw [ho1, ho2] at ho'; exact Or.inl ho'
        · rw [ho1, ho2] at ho'; exact Or.inr ⟨(t, w), List.mem_cons_self, ho'⟩
      · exact Or.inr ⟨z, List.mem_cons_of_mem _ hz, hy⟩
    · rcases hall' y hy with hy | ⟨z, hz, hy⟩
      · have hy' : y ∈ keys (h₂.occurrences (s.setRoot x.2.location.root v'')) := hy
        rcases List.mem_append.1 (hR'.subset hy') with hy | hy
        · exact Or.inr ⟨(t, w), List.mem_cons_self, hy⟩
        · exact Or.inl (hR.symm.subset (List.mem_append_right _ hy))
      · exact Or.inr ⟨z, List.mem_cons_of_mem _ hz, hy⟩
  | [], _, _ :: _, _, _, _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | [], _ :: _, [], _, _, _, _, _, _, _, _, _, _, _, _, hfa => by simp [IdTree.fitsAll] at hfa
  | _ :: _, _, [], _, _, _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | _ :: _, [], _ :: _, _, _, _, _, _, _, _, _, _, _, _, _, hfa => by simp [IdTree.fitsAll] at hfa

theorem mapM_getElem?_rev {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → ∀ {i : Nat} {b : β}, l'[i]? = some b →
      ∃ a, l[i]? = some a ∧ f a = some b
  | [], l', h, i, b, hb => by simp at h; subst h; simp at hb
  | x :: l, l', h, i, b, hb => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b', hb', l'', hl'', rfl⟩ := h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hb
      subst hb
      exact ⟨x, rfl, hb'⟩
    | succ i =>
      simp only [List.getElem?_cons_succ] at hb ⊢
      exact mapM_getElem?_rev hl'' hb

theorem nodup_of_offset : ∀ {l : List Nat} {c : Nat}, (∀ i x, l[i]? = some x → x = c + i) → l.Nodup
  | [], _, _ => List.nodup_nil
  | a :: l, c, h => by
    have ha := h 0 a rfl
    have hl : ∀ i x, l[i]? = some x → x = (c + 1) + i := fun i x hx => by
      have := h (i + 1) x (by simpa using hx); omega
    refine List.nodup_cons.2 ⟨fun hm => ?_, nodup_of_offset hl⟩
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hm
    have := hl i a hi
    omega

/-! ## The slots written back -/

/-- **Per frame, replacing a root**: a frame keeps its operand stack, function
and position, and each local is as it was, or is the root replaced. -/
theorem root_write_locals {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup) {root : Root} {v : Value}
    (hr : s.root? root = some v) {t t' : IdTree} {v' : Value} {h' : Shadow}
    (ht : h.rootTree? s root = some t) (hw : h.withRootTree? s root t' = some h') :
    ∀ (k : Nat) (g' : ShadowFrame) (fr' : Frame) (g : ShadowFrame) (fr : Frame),
      h'.frames[k]? = some g' → (s.setRoot root v').frames[k]? = some fr' →
      h.frames[k]? = some g → s.frames[k]? = some fr →
      g'.stack = g.stack ∧ fr'.stack = fr.stack ∧ fr'.function = fr.function ∧ fr'.pc = fr.pc ∧
      ∀ j : Nat, (g'.locals[j]? = g.locals[j]? ∧ fr'.locals[j]? = fr.locals[j]?) ∨
        (g.locals[j]? = some (some t) ∧ fr.locals[j]? = some (some v) ∧
          g'.locals[j]? = some (some t') ∧ fr'.locals[j]? = some (some v')) := by
  rw [shadow_fits, Bool.and_eq_true] at hfit
  obtain ⟨hfr, -⟩ := hfit
  intro k g' fr' g fr h1 h2 h3 h4
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hr
    obtain ⟨fr₀, hfr', slot, hslot, rfl⟩ := hr
    have hfr'' : s.frames.find? (·.id == f) = some fr₀ := hfr'
    have hpf : (fr₀.id == f) = true := List.find?_some (p := fun (x : Frame) => x.id == f) hfr''
    obtain ⟨k₀, hk, hfk⟩ := frame_get hfr'
    obtain ⟨g₀, hgk, hg⟩ := framesFit_get hfr hfk
    obtain ⟨t₀, R, hgt, -, -, -, -⟩ := slot_rw hg hslot t' v'
    have htt : t₀ = t := by
      simp only [Shadow.rootTree?, hk, hgk, hgt, bind, Option.bind_some] at ht
      exact Option.some.inj ht
    subst htt
    have hh' : h'.frames =
        h.frames.set k₀ { g₀ with locals := g₀.locals.setIfInBounds i (some t') } := by
      simp only [Shadow.withRootTree?, hk, hgk, hgt, bind, Option.bind_some, pure,
        Option.some.injEq] at hw
      rw [← hw]
    have hmap : (s.setRoot (.local_ f i) v').frames =
        s.frames.set k₀ { fr₀ with locals := fr₀.locals.setIfInBounds i (some v') } := by
      simp only [State.setRoot]
      refine map_ite_set (p := fun fr' => fr'.id == f) hfk hpf fun j y hy hpy => ?_
      exact key_unique hids hy hfk (by simp at hpf hpy; rw [hpy, hpf])
    rw [hh', List.getElem?_set] at h1
    rw [hmap, List.getElem?_set] at h2
    by_cases hkk : k₀ = k
    · subst hkk
      rw [if_pos rfl, if_pos (List.getElem?_eq_some_iff.1 hgk).1, Option.some.injEq] at h1
      rw [if_pos rfl, if_pos (List.getElem?_eq_some_iff.1 hfk).1, Option.some.injEq] at h2
      rw [hgk, Option.some.injEq] at h3
      rw [hfk, Option.some.injEq] at h4
      subst h1 h2 h3 h4
      have hgl : i < g₀.locals.size := (Array.getElem?_eq_some_iff.1 hgt).1
      have hfl : i < fr₀.locals.size := (Array.getElem?_eq_some_iff.1 hslot).1
      refine ⟨rfl, rfl, rfl, rfl, fun j => ?_⟩
      by_cases hij : i = j
      · subst hij
        exact Or.inr ⟨hgt, hslot, by simp [hgl], by simp [hfl]⟩
      · exact Or.inl ⟨Array.getElem?_setIfInBounds_ne hij,
          Array.getElem?_setIfInBounds_ne hij⟩
    · rw [if_neg hkk, h3, Option.some.injEq] at h1
      rw [if_neg hkk, h4, Option.some.injEq] at h2
      subst h1 h2
      exact ⟨rfl, rfl, rfl, rfl, fun j => Or.inl ⟨rfl, rfl⟩⟩
  | global a ty =>
    have hh' : h'.frames = h.frames := by
      simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hw
      obtain ⟨_, -, _, -, rfl⟩ := hw
      rfl
    rw [hh', h3, Option.some.injEq] at h1
    simp only [State.setRoot] at h2
    rw [h4, Option.some.injEq] at h2
    subst h1 h2
    exact ⟨rfl, rfl, rfl, rfl, fun j => Or.inl ⟨rfl, rfl⟩⟩
  | held j =>
    have hh' : h'.frames = h.frames := by
      simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hw
      obtain ⟨_, -, rfl⟩ := hw
      rfl
    rw [hh', h3, Option.some.injEq] at h1
    simp only [State.setRoot] at h2
    rw [h4, Option.some.injEq] at h2
    subst h1 h2
    exact ⟨rfl, rfl, rfl, rfl, fun j => Or.inl ⟨rfl, rfl⟩⟩

theorem idTree_set_cons_id {t x t'' : IdTree} {i : Nat} {p : Path}
    (h : t.set? (i :: p) x = some t'') : t''.id = t.id := by
  obtain ⟨id, cs⟩ := t
  simp only [IdTree.set?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨_, -, _, -, rfl⟩ := h
  rfl

theorem withChild_nonref {v c v' : Value} {st : PathStep} (h : v.withChild? st c = some v') :
    v.isRef = false ∧ v'.isRef = false := by
  cases v <;> cases st <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  all_goals (split at h <;> simp only [Option.some.injEq, reduceCtorEq] at h <;> subst h <;>
    exact ⟨rfl, rfl⟩)

theorem set_cons_nonref {v w v'' : Value} {st : PathStep} {p : List PathStep}
    (h : v.set? (st :: p) w = some v'') : v.isRef = false ∧ v''.isRef = false := by
  simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨_, -, _, -, h⟩ := h
  exact withChild_nonref h

/-- What a local slot is after writing back: as before, or a value that is no
reference, whose tree has the identity of the tree before, of a value that
was no reference, or of a tree written back. -/
def SlotAfter (ts : List IdTree) (g gB : ShadowFrame) (fr frB : Frame) (j : Nat) : Prop :=
  (gB.locals[j]? = g.locals[j]? ∧ frB.locals[j]? = fr.locals[j]?) ∨
  ∃ tB vB, gB.locals[j]? = some (some tB) ∧ frB.locals[j]? = some (some vB) ∧
    vB.isRef = false ∧
    ((∃ tA vA, g.locals[j]? = some (some tA) ∧ fr.locals[j]? = some (some vA) ∧
      vA.isRef = false ∧ tB.id = tA.id) ∨ ∃ y ∈ ts, tB.id = y.id)

/-- **The slots after writing back**: every frame keeps its operand stack,
function and position, and each local is as before, or holds a value that is
no reference, under an identity it had before, or of a tree written back. -/
theorem writes_slots {s₀ : State} :
    ∀ (W : List (Nat × Reference)) (ts : List IdTree) (ws : List Value) {h h' : Shadow}
      {s s' : State},
    h.fits s = true → (s.frames.map (·.id)).Nodup →
    (s.storage.map fun r => (r.address, r.type)).Nodup →
    s.frames.map (·.id) = s₀.frames.map (·.id) →
    s.storage.map (fun r => (r.address, r.type)) = s₀.storage.map (fun r => (r.address, r.type)) →
    WritesAgree s s₀ W →
    writeBack s (W.map (·.2)) ws = some s' →
    (W.zip ts).foldlM (fun h x => h.withTree? s₀ x.1.2.location x.2) h = some h' →
    IdTree.fitsAll ts ws = true → (∀ w ∈ ws, w.isRef = false) →
    ∀ (k : Nat) (gB : ShadowFrame) (frB : Frame) (g : ShadowFrame) (fr : Frame),
      h'.frames[k]? = some gB → s'.frames[k]? = some frB → h.frames[k]? = some g →
      s.frames[k]? = some fr →
      gB.stack = g.stack ∧ frB.stack = fr.stack ∧ frB.function = fr.function ∧ frB.pc = fr.pc ∧
      ∀ j : Nat, SlotAfter ts g gB fr frB j
  | [], [], [], h, h', s, s', _, _, _, _, _, _, hwb, hfold, _, _ => by
    simp only [List.map_nil, writeBack, Option.some.injEq] at hwb
    simp only [List.zip_nil_left, List.foldlM_nil, pure, Option.some.injEq] at hfold
    subst hwb hfold
    intro k gB frB g fr h1 h2 h3 h4
    rw [h1, Option.some.injEq] at h3
    rw [h2, Option.some.injEq] at h4
    subst h3 h4
    exact ⟨rfl, rfl, rfl, rfl, fun j => Or.inl ⟨rfl, rfl⟩⟩
  | x :: W, t :: ts, w :: ws, h, h', s, s', hfit, hids, hres, hids₀, hst₀, hag, hwb, hfold, hfa,
      hwn => by
    simp only [List.map_cons, writeBack, bind, Option.bind_eq_some_iff] at hwb
    obtain ⟨s₂, hw, hwb⟩ := hwb
    simp only [List.zip_cons_cons, List.foldlM_cons, bind, Option.bind_eq_some_iff] at hfold
    obtain ⟨h₂, hwith, hfold⟩ := hfold
    simp only [IdTree.fitsAll, Bool.and_eq_true] at hfa
    obtain ⟨u, hu⟩ := read_of_write hw
    have hag₂ := hag.tail hw (.mutated x.2.location)
    have hwith' : h.withTree? s x.2.location t = some h₂ := by
      rw [(tree_state' (h := h) hids₀ hst₀ x.2.location t hag.head).2]; exact hwith
    obtain ⟨tw, v, v'', h₃, hv, -, -, hset, hwr, hwith₃, hfit₃, -, -, -, -⟩ :=
      tree_write hfit hids hres hu hfa.1 (write?_writable hw)
    rw [hwith', Option.some.injEq] at hwith₃
    subst hwith₃
    rw [hwr, Option.some.injEq] at hw
    subst hw
    -- The root replaced.
    have hw2 := hwith'
    simp only [Shadow.withTree?, bind, Option.bind_eq_some_iff] at hw2
    obtain ⟨rt, hrt, rv, hrv, rt', hrt', hwr'⟩ := hw2
    have hids₂ := setRoot_frameIds s x.2.location.root v''
    have hst₂ := setRoot_storageKeys s x.2.location.root v''
    have IH := writes_slots W ts ws
      (s := (s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location))
      hfit₃ (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).frames.map (·.id) =
          s.frames.map (·.id) from hids₂]; exact hids)
      (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).storage.map _ =
          s.storage.map _ from hst₂]; exact hres)
      (hids₂.trans hids₀) (hst₂.trans hst₀) hag₂ hwb hfold hfa.2
      (fun w' hw' => hwn w' (List.mem_cons_of_mem _ hw'))
    have hstep := root_write_locals hfit hids hv hrt hwr' (v' := v'')
    -- The root's new tree and value.
    have hnew : (rt'.id = rt.id ∧ v.isRef = false) ∨ rt' = t := by
      cases hp : x.2.location.path with
      | nil =>
        right
        simp only [Location.indicesIn, hp, Value.indicesOf, IdTree.set?, Option.some.injEq] at hrt'
        exact hrt'.symm
      | cons st p =>
        left
        simp only [Location.indicesIn, hp, Value.indicesOf] at hrt'
        rw [hp] at hset
        exact ⟨idTree_set_cons_id hrt', (set_cons_nonref hset).1⟩
    have hv'' : v''.isRef = false := by
      cases hp : x.2.location.path with
      | nil =>
        rw [hp] at hset
        simp only [Value.set?, Option.some.injEq] at hset
        subst hset
        exact hwn w List.mem_cons_self
      | cons st p =>
        rw [hp] at hset
        exact (set_cons_nonref hset).2
    intro k gB frB g fr h1 h2 h3 h4
    have hk : k < s.frames.length := (List.getElem?_eq_some_iff.1 h4).1
    have hl₃ : (s.setRoot x.2.location.root v'').frames.length = s.frames.length := by
      rw [← List.length_map (f := (·.id)), hids₂, List.length_map]
    obtain ⟨g₂, hg₂⟩ : ∃ g₂, h₂.frames[k]? = some g₂ :=
      ⟨_, List.getElem?_eq_getElem (by
        rw [shadow_frames_length hfit₃]
        show k < (s.setRoot x.2.location.root v'').frames.length
        rw [hl₃]; exact hk)⟩
    obtain ⟨fr₂, hfr₂⟩ : ∃ fr₂, (s.setRoot x.2.location.root v'').frames[k]? = some fr₂ :=
      ⟨_, List.getElem?_eq_getElem (by rw [hl₃]; exact hk)⟩
    obtain ⟨e1, e2, e3, e4, hsl⟩ := IH k gB frB g₂ fr₂ h1 h2 hg₂ hfr₂
    obtain ⟨f1, f2, f3, f4, hsl₂⟩ := hstep k g₂ fr₂ g fr hg₂ hfr₂ h3 h4
    refine ⟨e1.trans f1, e2.trans f2, e3.trans f3, e4.trans f4, fun j => ?_⟩
    rcases hsl j with ⟨a1, a2⟩ | ⟨tB, vB, hgB, hfB, hvB, hor⟩
    · rcases hsl₂ j with ⟨b1, b2⟩ | ⟨b1, b2, b3, b4⟩
      · exact Or.inl ⟨a1.trans b1, a2.trans b2⟩
      · right
        refine ⟨rt', v'', by rw [a1, b3], by rw [a2, b4], hv'', ?_⟩
        rcases hnew with ⟨hid, hvn⟩ | rfl
        · exact Or.inl ⟨rt, v, b1, b2, hvn, hid⟩
        · exact Or.inr ⟨rt', List.mem_cons_self, rfl⟩
    · right
      refine ⟨tB, vB, hgB, hfB, hvB, ?_⟩
      rcases hor with ⟨tA, vA, hgA, hfA, hvA, hid⟩ | ⟨y, hy, hid⟩
      · rcases hsl₂ j with ⟨b1, b2⟩ | ⟨b1, b2, b3, b4⟩
        · exact Or.inl ⟨tA, vA, by rw [← b1]; exact hgA, by rw [← b2]; exact hfA, hvA, hid⟩
        · rw [hgA, Option.some.injEq, Option.some.injEq] at b3
          subst b3
          rcases hnew with ⟨hid', hvn⟩ | rfl
          · exact Or.inl ⟨rt, v, b1, b2, hvn, hid.trans hid'⟩
          · exact Or.inr ⟨tA, List.mem_cons_self, hid⟩
      · exact Or.inr ⟨y, List.mem_cons_of_mem _ hy, hid⟩
  | [], _, _ :: _, _, _, _, _, _, _, _, _, _, _, hwb, _, _, _ => by simp [writeBack] at hwb
  | [], _ :: _, [], _, _, _, _, _, _, _, _, _, _, _, _, hfa, _ => by simp [IdTree.fitsAll] at hfa
  | _ :: _, _, [], _, _, _, _, _, _, _, _, _, _, hwb, _, _, _ => by simp [writeBack] at hwb
  | _ :: _, [], _ :: _, _, _, _, _, _, _, _, _, _, _, _, _, hfa, _ => by simp [IdTree.fitsAll] at hfa

/-! ## A native call, decomposed -/

theorem writtenOf_map {P : Program} {f : FunctionRef} :
    ∀ {argIds : List IdTree} {args : List Value}, argIds.length = args.length →
      (writtenOf P f (refArgsOf argIds args)).map (·.2) = P.writtenReferences f args
  | [], [], _ => by simp [writtenOf, refArgsOf, Program.writtenReferences]
  | t :: ts, a :: as, hl => by
    have ih := writtenOf_map (P := P) (f := f) (argIds := ts) (args := as) (by simpa using hl)
    unfold writtenOf refArgsOf at ih ⊢
    unfold Program.writtenReferences at ih ⊢
    simp only [List.zip_cons_cons, List.filterMap_cons]
    cases a with
    | ref r =>
      simp only [List.filter_cons]
      by_cases hu : P.nativeShape? f == some .update
      · cases hm : r.mutable <;> simp_all
      · simp_all
    | _ => exact ih

theorem noRef_of_values {P : Program} {vs : List Value} {ts : List RuntimeType} (h : conformsAll P vs ts = true)
    (hts : ∀ t ∈ ts, t.isValue = true) : ∀ v ∈ vs, ∀ u ∈ subvalues v, u.isRef = false := by
  intro v hv
  obtain ⟨t, ht, hc⟩ := conformsAll_mem h v hv
  obtain ⟨hs, hty⟩ := conforms_shaped hc
  exact shaped_noRef v hs (by rw [assignable_isValue hty]; exact hts t ht)

/-- The targets of a modeled native's reference parameters are value types. -/
theorem param_targets {P : Program} {f : FunctionRef} {k : NativeShape} {ptys : List RuntimeType}
    (hshape : P.nativeShape? f = some k) (hpt : P.paramTypes f = some ptys)
    (hτ : ∀ x ∈ f.typeArgs, x.isValue = true) :
    ∀ t ∈ ptys, ∀ m u, t = .ref m u → u.isValue = true := by
  obtain ⟨info, e, -, -, -, -, -, hpe, -⟩ := nativeShape_entry hshape
  rw [hpt] at hpe
  intro t ht m u htu
  subst htu
  obtain ⟨q, -, hq⟩ := mapM_some_mem hpe.symm _ ht
  cases q with
  | reference q' =>
    simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
    obtain ⟨u', h1, h2⟩ := hq
    simp only [RuntimeType.ref.injEq] at h2
    rw [← h2.2]
    exact slotType_values hτ h1
  | mutableReference q' =>
    simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
    obtain ⟨u', h1, h2⟩ := hq
    simp only [RuntimeType.ref.injEq] at h2
    rw [← h2.2]
    exact slotType_values hτ h1
  | _ =>
    have := slotType_values hτ (p := _) hq
    simp [RuntimeType.isValue] at this

/-- The places a modeled native writes back are of value types. -/
theorem mutableTargets_values {P : Program} {f : FunctionRef} {k : NativeShape} {ptys : List RuntimeType}
    (hshape : P.nativeShape? f = some k) (hpt : P.paramTypes f = some ptys)
    (hτ : ∀ x ∈ f.typeArgs, x.isValue = true) :
    ∀ t ∈ RuntimeType.mutableTargets ptys, t.isValue = true := by
  obtain ⟨info, e, -, -, -, -, -, hpe, -⟩ := nativeShape_entry hshape
  rw [hpt] at hpe
  intro t ht
  simp only [RuntimeType.mutableTargets, List.mem_filterMap] at ht
  obtain ⟨u, hu, hut⟩ := ht
  obtain ⟨q, -, hq⟩ := mapM_some_mem hpe.symm u hu
  cases u with
  | ref m t' =>
    cases m with
    | false => cases hut
    | true =>
      simp only [Option.some.injEq] at hut
      subst hut
      cases q with
      | mutableReference q' =>
        simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
        obtain ⟨t'', h1, h2⟩ := hq
        simp only [RuntimeType.ref.injEq] at h2
        rw [← h2.2]
        exact slotType_values hτ h1
      | reference q' =>
        simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
        obtain ⟨t'', -, h2⟩ := hq
        simp at h2
      | _ =>
        have := slotType_values hτ (p := _) hq
        simp [RuntimeType.isValue] at this
  | _ => cases hut

/-- The identities of the references a native call makes for its places. -/
theorem newRefs_ids {refResults : List Reference} {places : List NativePlace}
    {argIds : List IdTree} {F : Nat} {newRefs : List (Nat × Reference × Nat)}
    (h : newRefsOf refResults places argIds F = some newRefs) :
    ∀ i x, newRefs[i]? = some x → x.1 = F + i := by
  intro i x hx
  unfold newRefsOf at h
  obtain ⟨z, hz, hzy⟩ := mapM_getElem?_rev h hx
  rw [List.getElem?_zipIdx] at hz
  obtain ⟨w, -, rfl⟩ := Option.map_eq_some_iff.1 hz
  simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hzy
  obtain ⟨t, -, rfl⟩ := hzy
  simp

theorem newRefs_lt {refResults : List Reference} {places : List NativePlace}
    {argIds : List IdTree} {F : Nat} {newRefs : List (Nat × Reference × Nat)}
    (h : newRefsOf refResults places argIds F = some newRefs) :
    ∀ x ∈ newRefs, x.1 < F + newRefs.length := by
  intro x hx
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
  have hl := (List.getElem?_eq_some_iff.1 hi).1
  rw [newRefs_ids h i x hi]
  omega

theorem withTree_fresh {h h' : Shadow} {s : State} {l : Location} {t : IdTree}
    (hw : h.withTree? s l t = some h') : h'.fresh = h.fresh := by
  simp only [Shadow.withTree?, bind, Option.bind_eq_some_iff] at hw
  obtain ⟨_, -, _, -, _, -, hw⟩ := hw
  unfold Shadow.withRootTree? at hw
  split at hw <;>
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw <;>
    first
    | (obtain ⟨_, -, _, -, _, -, rfl⟩ := hw; rfl)
    | (obtain ⟨_, -, _, -, rfl⟩ := hw; rfl)
    | (obtain ⟨_, -, rfl⟩ := hw; rfl)

theorem foldl_withTree_fresh {s : State} :
    ∀ (L : List ((Nat × Reference) × IdTree)) {h h' : Shadow},
      L.foldlM (fun h x => h.withTree? s x.1.2.location x.2) h = some h' → h'.fresh = h.fresh
  | [], h, h', hf => by simp only [List.foldlM_nil, pure, Option.some.injEq] at hf; rw [hf]
  | x :: L, h, h', hf => by
    simp only [List.foldlM_cons, bind, Option.bind_eq_some_iff] at hf
    obtain ⟨h₂, hw, hf⟩ := hf
    rw [foldl_withTree_fresh L hf, withTree_fresh hw]

/-- **The locations a native writes back agree** between the state it was
called from and any state with the same roots: they are its mutable reference
arguments, which overlap nowhere (`Step.exclusiveArguments`). -/
theorem writesAgree_native {P : Program} {s s₁ s₂ : State} {f : FunctionRef}
    {args views : List Value} {argIds : List IdTree}
    (hstep : step P s = .callNative s₁ f args views) (hl : argIds.length = args.length)
    (hroot : ∀ ρ, s₂.root? ρ = s.root? ρ) :
    WritesAgree s₂ s (writtenOf P f (refArgsOf argIds args)) := by
  obtain ⟨-, -, -, -, -, -, -, hfacts⟩ := step_callNative hstep
  refine WritesAgree.of_roots ?_ hroot
  rw [writtenOf_map hl]
  by_cases hu : P.nativeShape? f = some .update
  · rw [writtenReferences_update hu]; exact mutRefs_disjoint hfacts.exclusive
  · rw [writtenReferences_other hu]; exact List.Pairwise.nil

/-- **An annotated call of a modeled native that keeps running, decomposed**:
the caller's segment loses the arguments and gains the results; then the
values are written back, in the state and in its shadow. The results hold no
reference but, for the borrow shape, the new one; the values written back
hold none. -/
theorem nativeStep_decomp {P : Program} {ν : NativeFlows} {s s₁ s' : State}
    {h h' : Shadow} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} {evs : List Event} {k : NativeShape}
    (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    ∃ (fr : Frame) (rest : List Frame) (n : Nat) (g : ShadowFrame) (hs : List ShadowFrame)
      (F : Nat) (results : List Value) (resultTrees ts : List IdTree),
      s.frames = fr :: rest ∧ h.frames = g :: hs ∧ n ≤ fr.stack.length ∧ n ≤ g.stack.length ∧
      args = (fr.stack.take n).reverse ∧
      writeBack
        { s with
          frames := { fr with
            stack := results.reverse ++ fr.stack.drop n
            pc := fr.pc + 1 } :: rest }
        ((writtenOf P f (refArgsOf (g.stack.take n).reverse args)).map (·.2)) writes = some s' ∧
      ((writtenOf P f (refArgsOf (g.stack.take n).reverse args)).zip ts).foldlM
        (fun h x => h.withTree? s x.1.2.location x.2)
        { h with
          frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
          fresh := F } = some h' ∧
      IdTree.fitsAll resultTrees results = true ∧ IdTree.fitsAll ts writes = true ∧
      (∀ w ∈ writes, ∀ u ∈ subvalues w, u.isRef = false) ∧
      ((∀ v ∈ results, ∀ u ∈ subvalues v, u.isRef = false) ∨
        ∃ d b t, results = [.ref b] ∧ resultTrees = [.node d []] ∧
          Event.borrowed d b (some t) ∈ evs) ∧
      h.fresh ≤ F ∧ (∀ d r par, Event.borrowed d r par ∈ evs → d < F) ∧ h'.fresh = F ∧
      (∀ x ∈ writtenOf P f (refArgsOf (g.stack.take n).reverse args),
        Event.accessed .write x.2.location (some x.1) ∈ evs) ∧
      ∃ F₀, h.fresh ≤ F₀ ∧ F₀ ≤ F ∧ (∀ d r par, Event.borrowed d r par ∈ evs → F₀ ≤ d) ∧
        (∀ o ∈ IdTree.occurrencesAll ts writes, (h.fresh ≤ o.1 ∧ o.1 < F₀) ∨
          ∃ seen k q, seenTrees h s args (g.stack.take n).reverse = some seen ∧
            seenIdAt seen k q = some o.1) ∧
        ((∀ o ∈ IdTree.occurrencesAll resultTrees results, (h.fresh ≤ o.1 ∧ o.1 < F₀) ∨
          ∃ seen k q, seenTrees h s args (g.stack.take n).reverse = some seen ∧
            seenIdAt seen k q = some o.1) ∨
          ∃ d b t, results = [.ref b] ∧ resultTrees = [.node d []] ∧
            Event.borrowed d b (some t) ∈ evs) := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  obtain ⟨tys, views', sW, results, frW, restW, htys, hviews, hfits, hwb, hres, hsW, ho'⟩ :=
    resume_spec hcont
  rw [hfacts.view, Option.some.injEq] at hviews
  subst hviews
  rw [ho, Outcome.running.injEq] at ho'
  obtain ⟨g, hs, info, tys', ptys, seen, s'', fr'', tl, newRefs, resultTrees, h₂, hh, -, htys',
    hpt, hlen, hseen, -, -, -, hs'', hf'', hrr, hnew, hrt, hfold, hpost, -, hevs⟩ := nativeStep_spec hn
  rw [htys, Option.some.injEq] at htys'
  subst htys'
  simp only [Outcome.running.injEq] at hs''
  subst hs''
  simp only [ShadowPost.running.injEq] at hpost
  subst hpost
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  rw [hargl] at hnew hfold hevs hlen hseen
  obtain ⟨ptys', hpt', hconfa⟩ := hfacts.params
  rw [hpt, Option.some.injEq] at hpt'
  subst hpt'
  have hal : ((g.stack.take n).reverse).length = args.length := by
    rw [hargl]; simp; omega
  have hwm := writtenOf_map (P := P) (f := f) hal
  obtain ⟨L, restW', hsWf, htop⟩ := writeBack_top _ _ (s := s₁) (by rw [hs₁]) hwb
  rw [hsW, List.cons.injEq] at hsWf
  obtain ⟨rfl, rfl⟩ := hsWf
  have hwb' : writeBack
      { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }
      ((writtenOf P f (refArgsOf (g.stack.take n).reverse args)).map (·.2)) writes = some s' := by
    rw [hwm, ho']
    subst hs₁
    exact htop _ _
  -- The trees of the outputs.
  obtain ⟨hfitO, hF0, -, -, -⟩ := outTreesOf_spec (ν f views (vals ++ writes)) seen
    (.native f info.moduleId views) (vals ++ writes) h.fresh
  obtain ⟨hfv, hfw⟩ := fitsAll_take_drop vals.length hfitO
  have hsplitO := stack_split hfitO vals.length
  rw [List.take_left] at hfv
  rw [List.drop_left] at hfw
  rw [List.take_left, List.drop_left] at hsplitO
  have horigO := outTrees_occ (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
    (vals ++ writes) h.fresh
  have horig : ∀ o ∈ IdTree.occurrencesAll
      (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
        (vals ++ writes) h.fresh).1 (vals ++ writes),
      (h.fresh ≤ o.1 ∧ o.1 < (outTreesOf (ν f views (vals ++ writes)) seen
        (.native f info.moduleId views) (vals ++ writes) h.fresh).2.1) ∨
        ∃ seen k q, seenTrees h s args (g.stack.take n).reverse = some seen ∧
          seenIdAt seen k q = some o.1 := by
    intro o ho
    rcases horigO o ho with ⟨h1, h2⟩ | ⟨k', q, hq⟩
    · exact Or.inl ⟨h1, h2⟩
    · exact Or.inr ⟨seen, k', q, hseen, hq⟩
  -- The new references come after the outputs' new nodes.
  have hge : ∀ d r par, Event.borrowed d r par ∈ evs →
      (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
        (vals ++ writes) h.fresh).2.1 ≤ d := by
    intro d r par hb
    rw [hevs] at hb
    rcases nativeEventsOf_cases hb with ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, hx, he⟩ | he |
      he
    · cases he
    · cases he
    · cases he
    · simp only [Event.borrowed.injEq] at he
      rw [he.1]
      obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
      rw [newRefs_ids hnew i x hi]
      omega
    · obtain ⟨_, _, _, he'⟩ := consumedOfNative_mem he; cases he'
    · obtain ⟨_, _, he'⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 _ he; cases he'
  -- The values written back hold no reference.
  have hwr : ∀ w ∈ writes, ∀ u ∈ subvalues w, u.isRef = false := by
    by_cases hku : k = .update
    · subst hku
      have hfit' := hfits
      simp only [Program.nativeResultFits, hshape, hpt, Bool.and_eq_true] at hfit'
      exact noRef_of_values hfit'.1.2
        (mutableTargets_values hshape hpt (typeArgsOk_values hfacts.typeArgs))
    · rw [fits_writes_nil hshape hku hfits]; simp
  have hlt : ∀ d r par, Event.borrowed d r par ∈ evs → d <
      (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
        (vals ++ writes) h.fresh).2.1 + newRefs.length := by
    intro d r par hb
    rw [hevs] at hb
    rcases nativeEventsOf_cases hb with ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, -, he⟩ | ⟨x, hx, he⟩ | he |
      he
    · cases he
    · cases he
    · cases he
    · simp only [Event.borrowed.injEq] at he
      rw [he.1]
      exact newRefs_lt hnew x hx
    · obtain ⟨_, _, _, he'⟩ := consumedOfNative_mem he; cases he'
    · obtain ⟨_, _, he'⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 _ he; cases he'
  have hres2 : IdTree.fitsAll resultTrees results = true ∧
      (((∀ v ∈ results, ∀ u ∈ subvalues v, u.isRef = false) ∧
          ∀ o ∈ IdTree.occurrencesAll resultTrees results,
            (h.fresh ≤ o.1 ∧ o.1 < (outTreesOf (ν f views (vals ++ writes)) seen
              (.native f info.moduleId views) (vals ++ writes) h.fresh).2.1) ∨
            ∃ seen k q, seenTrees h s args (g.stack.take n).reverse = some seen ∧
              seenIdAt seen k q = some o.1) ∨
        ∃ d b t, results = [.ref b] ∧ resultTrees = [.node d []] ∧
          Event.borrowed d b (some t) ∈ evs) := by
    by_cases hkb : k = .borrow
    · subst hkb
      obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, -, -, -, -, -, -⟩ :=
        borrow_result hshape hpt htys (typeArgsOk_values hfacts.typeArgs) hfits
      obtain ⟨r0, hra, -, -⟩ := conformsAll_ref_at hconfa hpX
      have hpr : (placeReferences sW [.ref m tgt] args [p]).bind (nativeResultsIn [.ref m tgt] []) =
          some [Step.borrow sW m tgt (r0.location.descend p.path)] := by
        simp [placeReferences, nativeResultsIn, harg, hra, RuntimeType.isValue]
      rw [hpr, Option.some.injEq] at hres
      subst hres
      rw [ho'] at hf''
      simp only [List.cons.injEq] at hf''
      obtain ⟨rfl, rfl⟩ := hf''
      have hn0 : 0 < n := by
        have := (List.getElem?_eq_some_iff.1 hra).1; omega
      obtain ⟨t0, ht0⟩ : ∃ t0, (g.stack.take n).reverse[0]? = some t0 :=
        ⟨_, List.getElem?_eq_getElem (by simp; omega)⟩
      simp [refResultsOf, newRefsOf, RuntimeType.isValue, harg, ht0, Step.borrow] at hnew
      subst hnew
      simp [nativeResultsIn, RuntimeType.isValue] at hrt
      subst hrt
      refine ⟨by simp [IdTree.fitsAll, IdTree.fits, Value.children, Step.borrow], Or.inr ⟨_, _, t0.id, rfl, rfl,
        ?_⟩⟩
      rw [hevs]
      simp [nativeEventsOf]
    · obtain ⟨hcv, rfl⟩ := fits_values hshape hkb hfits
      have hvt := returns_values hshape hkb htys (typeArgsOk_values hfacts.typeArgs)
      simp only [placeReferences_nil, Option.bind_some,
        nativeResultsIn_values hvt (conformsAll_length hcv), Option.some.injEq] at hres
      subst hres
      have hnr : newRefs = [] := by simp [newRefsOf] at hnew; exact hnew
      subst hnr
      have hlt' : ((outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).1.take vals.length).length = tys.length := by
        rw [List.length_take, fitsAll_length hfitO, ← conformsAll_length hcv]; simp
      rw [List.map_nil, nativeResultsIn_values hvt hlt', Option.some.injEq] at hrt
      subst hrt
      exact ⟨hfv, Or.inl ⟨noRef_of_values hcv hvt, fun o ho => horig o
        (by rw [hsplitO]; exact List.mem_append_left _ ho)⟩⟩
  refine ⟨fr, rest, n, g, hs, _, results, resultTrees, _, hsf, hh, hnl, by omega, hargs, hwb',
    hfold, hres2.1, hfw, hwr, ?_, Nat.le_trans hF0 (Nat.le_add_right _ _), hlt,
    foldl_withTree_fresh _ hfold, fun x hx => ?_, _, hF0, Nat.le_add_right _ _, hge, fun o ho => horig o
      (by rw [hsplitO]; exact List.mem_append_right _ ho), ?_⟩
  · rcases hres2.2 with ⟨h1, -⟩ | h1
    · exact Or.inl h1
    · exact Or.inr h1
  · rw [hevs]
    simp only [nativeEventsOf, List.mem_append, List.mem_map]
    exact Or.inl (Or.inl (Or.inl (Or.inl (Or.inr ⟨x, hx, rfl⟩))))
  · rcases hres2.2 with ⟨-, h1⟩ | h1
    · exact Or.inl h1
    · exact Or.inr h1

/-- **A resumed call of a modeled native, in the state alone**: the caller's
segment loses the arguments and gains the results, then the values are written
back. The values written back are well formed and hold no reference; the
results too, or the result is the one new reference, to a place strictly
inside a location. -/
theorem resume_decomp {P : Program} {s s₁ s' : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {k : NativeShape}
    (hshape : P.nativeShape? f = some k) (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s') :
    ∃ (fr : Frame) (rest : List Frame) (n : Nat) (results : List Value),
      s.frames = fr :: rest ∧
      writeBack
        { s with
          frames := { fr with
            stack := results.reverse ++ fr.stack.drop n
            pc := fr.pc + 1 } :: rest }
        (P.writtenReferences f args) writes = some s' ∧
      (∀ w ∈ writes, w.wellFormed P = true ∧ ∀ u ∈ subvalues w, u.isRef = false) ∧
      ((∀ v ∈ results, v.wellFormed P = true ∧ ∀ u ∈ subvalues v, u.isRef = false) ∨
        ∃ b : Reference, results = [.ref b] ∧ b.location.path ≠ [] ∧ b.target.isValue = true) := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  obtain ⟨tys, views', sW, results, frW, restW, htys, hviews, hfits, hwb, hres, hsW, ho'⟩ :=
    resume_spec hcont
  rw [hfacts.view, Option.some.injEq] at hviews
  subst hviews
  rw [ho, Outcome.running.injEq] at ho'
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  obtain ⟨L, restW', hsWf, htop⟩ := writeBack_top _ _ (s := s₁) (by rw [hs₁]) hwb
  rw [hsW, List.cons.injEq] at hsWf
  obtain ⟨rfl, rfl⟩ := hsWf
  have hwb' : writeBack
      { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }
      (P.writtenReferences f args) writes = some s' := by
    rw [ho']
    subst hs₁
    exact htop _ _
  have hwr : ∀ w ∈ writes, w.wellFormed P = true ∧ ∀ u ∈ subvalues w, u.isRef = false := by
    by_cases hku : k = .update
    · subst hku
      have hfit' := hfits
      simp only [Program.nativeResultFits, hshape, hpt, Bool.and_eq_true] at hfit'
      intro w hw
      obtain ⟨t, -, hc⟩ := conformsAll_mem hfit'.1.2 w hw
      exact ⟨conforms_wellFormed hc, noRef_of_values hfit'.1.2
        (mutableTargets_values hshape hpt (typeArgsOk_values hfacts.typeArgs)) w hw⟩
    · rw [fits_writes_nil hshape hku hfits]; simp
  refine ⟨fr, rest, n, results, hsf, hwb', hwr, ?_⟩
  by_cases hkb : k = .borrow
  · subst hkb
    obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, -, hrig, hXt, hplace, -, -⟩ :=
      borrow_result hshape hpt htys (typeArgsOk_values hfacts.typeArgs) hfits
    obtain ⟨r0, hra, -, hrX⟩ := conformsAll_ref_at hconfa hpX
    have hpr : (placeReferences sW [.ref m tgt] args [p]).bind (nativeResultsIn [.ref m tgt] []) =
        some [Step.borrow sW m tgt (r0.location.descend p.path)] := by
      simp [placeReferences, nativeResultsIn, harg, hra, RuntimeType.isValue]
    rw [hpr, Option.some.injEq] at hres
    subst hres
    have hp0 : p.path ≠ [] := by
      intro hnil
      rw [hnil, placeType?_nil, Option.some.injEq] at hplace
      subst hplace
      exact borrow_ne hXt rfl
    have htv : tgt.isValue = true := by
      have hτ := typeArgsOk_values hfacts.typeArgs
      have hpv := param_targets hshape hpt hτ (.ref am X) (List.mem_of_getElem? hpX) am X rfl
      exact borrow_value hXt hpv
    refine Or.inr ⟨_, rfl, ?_, htv⟩
    simp only [Location.descend]
    intro hnil
    exact hp0 (List.append_eq_nil_iff.1 hnil).2
  · obtain ⟨hcv, rfl⟩ := fits_values hshape hkb hfits
    have hvt := returns_values hshape hkb htys (typeArgsOk_values hfacts.typeArgs)
    simp only [placeReferences_nil, Option.bind_some,
      nativeResultsIn_values hvt (conformsAll_length hcv), Option.some.injEq] at hres
    subst hres
    refine Or.inl fun v hv => ?_
    obtain ⟨t, -, hc⟩ := conformsAll_mem hcv v hv
    exact ⟨conforms_wellFormed hc, noRef_of_values hcv hvt v hv⟩

/-- **A call of a modeled native keeps the state invariant**, but for the
held values' `drop`. -/
theorem inv_native {σ : Bool} {P : Program} {s s₁ s' : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {k : NativeShape}
    (ht : StateTyped P s) (hinv : StateInv σ P s) (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s') :
    StateInv false P s' := by
  obtain ⟨fr, rest, n, hsf, -, hargs, rfl, hfacts⟩ := step_callNative hstep
  have hcont : (resumeNative P { s with frames := { fr with stack := fr.stack.drop n } :: rest } f
      args (.returned vals writes places)).continues = true := by rw [ho]; rfl
  obtain ⟨tys, views', sW, results, fr', rest', -, -, hfits, hwb, -, hsW, ho'⟩ := resume_spec hcont
  rw [ho, Outcome.running.injEq] at ho'
  subst ho'
  obtain ⟨ht₁, hinv₁⟩ := typed_popped ht hinv hsf n
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
  have hargRef : ∀ r, Value.ref r ∈ args →
      RefTyped P { s with frames := { fr with stack := fr.stack.drop n } :: rest } r ∧
      r.target.isValue = true ∧
      (({ s with frames := { fr with stack := fr.stack.drop n } :: rest } : State).read?
        r.location).isSome = true := by
    intro r hr
    have hrs : Value.ref r ∈ fr.stack := by
      rw [hargs] at hr; exact List.mem_of_mem_take (List.mem_reverse.1 hr)
    have hrt := ht.refs fr hfr r (mem_frameVals_stack hrs)
    refine ⟨refTyped_congr hrt (rootType_congr (by simp [hsf]) rfl) (Nat.le_refl _)
      (held_same rfl), ?_, ?_⟩
    · simpa [shaped] using ht.valsShaped fr hfr _ (mem_frameVals_stack hrs)
    · obtain ⟨v, hv⟩ := nativeView_deref hfacts.view r hr
      rw [deref?_read hv]; rfl
  have hinvW : StateInv false P sW := by
    by_cases hku : k = .update
    · subst hku
      rw [writtenReferences_update hshape] at hwb
      have hfit' := hfits
      simp only [Program.nativeResultFits, hshape, hpt, Bool.and_eq_true] at hfit'
      exact (typed_writeBack ht₁ hinv₁.weaken (fun r hr => hargRef r (mem_mutRefsOf hr).1)
        (mutRefs_disjoint hfacts.exclusive) (by rw [mutRefs_targets hconfa]; exact hfit'.1.2)
        hwb).2
    · rw [writtenReferences_other (by rw [hshape]; simpa using hku)] at hwb
      cases writes with
      | nil =>
        simp only [writeBack, Option.some.injEq] at hwb
        subst hwb
        exact hinv₁.weaken
      | cons => simp [writeBack] at hwb
  exact frames_inv hinvW hsW rfl rfl rfl rfl

/-- **A reference an annotated native call borrows** is strictly inside the
location of the reference argument it derives from. -/
theorem nativeStep_borrowed_path {P : Program} {ν : NativeFlows} {s s₁ : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places
      (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs))
    {d : Nat} {r : Reference} {par : Option Nat} (hb : Event.borrowed d r par ∈ evs) :
    r.location.path ≠ [] := by
  obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
  obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
  have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
  obtain ⟨-, -, -, -, r0, t0, m, tgt, p', d', -, -, -, -, -, -, hp0, -, -, hall, -⟩ :=
    nativeStep_borrowEvents hstep hn hkb
  rcases hall _ hb with he | he | he | ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he
  simp only [Location.descend]
  intro hnil
  exact hp0 (List.append_eq_nil_iff.1 hnil).2

end MoveBytecode.Proofs
