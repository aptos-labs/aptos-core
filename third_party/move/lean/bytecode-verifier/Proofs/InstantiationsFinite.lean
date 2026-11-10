-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage

/-!
# Local instantiation (L7) from the instantiation-loop check

UNTRUSTED (checked by the kernel). The model's instantiation-loop check
(`Model.verifyInstantiationLoops`) builds a graph on (function definition, type
parameter) nodes: a generic call adds an identity edge from the caller's type
parameter that a type argument is, and a type-constructor edge from every type
parameter a larger type argument mentions. It accepts when no type-constructor
edge lies on a cycle.

Then every node has a rank, the number of type-constructor edges from whose
target it is reachable (`rank`): identity edges never lower it and
type-constructor edges raise it (`rank_mono`, `tyRank_lt`), and it is at most the
number of edges. Each type argument of an instance reachable through the
module's own calls lies in the layer of its node's rank (`InstGood`), where layer 0
holds the start instance's type arguments and the closed translations of the
module's tokens, and layer `k + 1` adds the translations of the module's
tokens under argument lists drawn from layer `k` (`layer`). The layers are
finite lists, so the reachable instances are among finitely many
(`instantiationsFinite_of`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model MoveBytecode.Model.InstantiationLoops

/-! ## Lists -/

theorem filter_length_mono {α : Type} (p q : α → Bool) :
    ∀ (l : List α), (∀ x ∈ l, p x = true → q x = true) →
      (l.filter p).length ≤ (l.filter q).length
  | [], _ => Nat.le_refl _
  | x :: l, h => by
    have ih := filter_length_mono p q l (fun y hy => h y (List.mem_cons_of_mem _ hy))
    cases hp : p x <;> cases hq : q x <;> simp [hp, hq] <;>
      first | omega | (have := h x (List.mem_cons_self ..) hp; simp_all)

theorem filter_length_lt {α : Type} (p q : α → Bool) :
    ∀ (l : List α), (∀ x ∈ l, p x = true → q x = true) →
      (∃ x ∈ l, q x = true ∧ p x = false) → (l.filter p).length < (l.filter q).length
  | [], _, ⟨_, hx, _⟩ => absurd hx (List.not_mem_nil)
  | x :: l, h, ⟨y, hy, hqy, hpy⟩ => by
    have hmono := filter_length_mono p q l (fun z hz => h z (List.mem_cons_of_mem _ hz))
    rcases List.mem_cons.1 hy with rfl | hy
    · simp [hqy, hpy]
      omega
    · have ih := filter_length_lt p q l (fun z hz => h z (List.mem_cons_of_mem _ hz))
        ⟨y, hy, hqy, hpy⟩
      cases hp : p x <;> cases hq : q x <;> simp [hp, hq] <;>
        first | omega | (have := h x (List.mem_cons_self ..) hp; simp_all)

theorem contains_iff {l : List Node} {n : Node} : l.contains n = true ↔ n ∈ l :=
  List.contains_iff_mem

/-! ## Reachability -/

/-- Node `b` is reachable from node `a` along the edges. -/
inductive Reach (es : List Edge) : Node → Node → Prop where
  | refl (a : Node) : Reach es a a
  | step {a : Node} (e : Edge) : Reach es a e.src → e ∈ es → Reach es a e.dst

theorem Reach.trans {es : List Edge} {a b c : Node} (h1 : Reach es a b) (h2 : Reach es b c) :
    Reach es a c := by
  induction h2 with
  | refl => exact h1
  | step e _ he ih => exact Reach.step e ih he

theorem Reach.edge {es : List Edge} {e : Edge} (he : e ∈ es) : Reach es e.src e.dst :=
  Reach.step e (Reach.refl _) he

/-- A set of nodes closed under the edges holds every node reachable from it. -/
theorem Reach.closed {es : List Edge} {seen : List Node}
    (hc : ∀ e ∈ es, e.src ∈ seen → e.dst ∈ seen) {a b : Node} (h : Reach es a b)
    (ha : a ∈ seen) : b ∈ seen := by
  induction h with
  | refl => exact ha
  | step e _ he ih => exact hc e he ih

/-- The edge targets not yet seen, with repetitions. -/
def fresh (es : List Edge) (seen : List Node) : Nat :=
  ((es.map (·.dst)).filter fun n => !seen.contains n).length

/-- The search of `reaches` finds every reachable node: each round that does
not stop adds an edge target not seen before, so the fuel never runs out. -/
theorem go_complete (es : List Edge) (dst : Node) :
    ∀ (fuel : Nat) (seen : List Node), fresh es seen < fuel →
      (∃ s ∈ seen, Reach es s dst) → reaches.go es dst fuel seen = true
  | 0, _, hf, _ => absurd hf (Nat.not_lt_zero _)
  | fuel + 1, seen, hf, ⟨s, hs, hr⟩ => by
    unfold reaches.go
    by_cases hc : seen.contains dst = true
    · rw [if_pos hc]
    · simp only [hc, Bool.false_eq_true, ↓reduceIte]
      split
      · next hnil =>
        -- Nothing new: the seen nodes are closed, so `dst` would be seen.
        exfalso
        apply hc
        simp [List.isEmpty_iff] at hnil
        exact contains_iff.2 (Reach.closed (fun e he hsrc => hnil e he hsrc) hr hs)
      · next hne =>
        apply go_complete es dst fuel
        · -- A new target joins the seen nodes.
          simp [List.isEmpty_iff] at hne
          obtain ⟨e, hes, hsrc, hdst⟩ := hne
          have hx : e.dst ∈ ((es.filter fun e => seen.contains e.src && !seen.contains e.dst).map
              (·.dst)).eraseDups :=
            List.mem_eraseDups.2 (List.mem_map.2 ⟨e, List.mem_filter.2 ⟨hes, by simp [hsrc, hdst]⟩,
              rfl⟩)
          have hlt := filter_length_lt (fun n => !(seen ++ ((es.filter fun e =>
              seen.contains e.src && !seen.contains e.dst).map (·.dst)).eraseDups).contains n)
            (fun n => !seen.contains n) (es.map (·.dst))
            (fun n _ h => by simp_all)
            ⟨e.dst, List.mem_map.2 ⟨e, hes, rfl⟩, by simp [hdst], by
              simp only [Bool.not_eq_false', List.contains_append, Bool.or_eq_true]
              right
              exact contains_iff.2 hx⟩
          unfold fresh at hf ⊢
          omega
        · exact ⟨s, List.mem_append_left _ hs, hr⟩

theorem reaches_complete {es : List Edge} {a b : Node} (h : Reach es a b) :
    reaches es a b = true := by
  show reaches.go es b (es.length + 1) [a] = true
  refine go_complete es b _ [a] ?_ ⟨a, List.mem_singleton_self _, h⟩
  have := List.length_filter_le (fun n => ![a].contains n) (es.map (·.dst))
  simp only [List.length_map] at this
  unfold fresh
  omega

/-! ## Ranks -/

open Classical in
/-- The number of type-constructor edges from whose target node `n` is
reachable. -/
noncomputable def rank (es : List Edge) (n : Node) : Nat :=
  (es.filter fun e => decide (e.tyCon = true ∧ Reach es e.dst n)).length

theorem rank_le (es : List Edge) (n : Node) : rank es n ≤ es.length := by
  unfold rank
  exact List.length_filter_le _ _

theorem rank_mono {es : List Edge} {u v : Node} (h : Reach es u v) : rank es u ≤ rank es v := by
  unfold rank
  exact filter_length_mono _ _ es fun e _ hp => by
    simp only [decide_eq_true_eq] at hp ⊢
    exact ⟨hp.1, hp.2.trans h⟩

/-- Across a type-constructor edge outside every cycle, the rank grows. -/
theorem tyRank_lt {es : List Edge} {e : Edge} (he : e ∈ es) (ht : e.tyCon = true)
    (hacyc : ¬ Reach es e.dst e.src) : rank es e.src < rank es e.dst := by
  unfold rank
  refine filter_length_lt _ _ es (fun e' _ hp => ?_) ⟨e, he, ?_, ?_⟩
  · simp only [decide_eq_true_eq] at hp ⊢
    exact ⟨hp.1, hp.2.trans (Reach.edge he)⟩
  · simp only [decide_eq_true_eq]
    exact ⟨ht, Reach.refl _⟩
  · simp only [decide_eq_false_iff_not, not_and]
    exact fun _ => hacyc

/-! ## Tokens -/

theorem mem_typeParams {ty : SignatureToken} {q : Nat} :
    q ∈ typeParams ty ↔ SignatureToken.typeParameter q ∈ preorder ty := by
  unfold typeParams
  rw [List.mem_eraseDups, List.mem_filterMap]
  constructor
  · rintro ⟨a, ha, hf⟩
    cases a <;> simp at hf
    subst hf
    exact ha
  · intro h
    exact ⟨_, h, rfl⟩

mutual
/-- A token's translation reads the type arguments only at its type
parameters. -/
theorem ofTokenWith_agree {m : CompiledModule} {tys tys' : List RuntimeType} :
    ∀ (t : SignatureToken),
      (∀ i, SignatureToken.typeParameter i ∈ preorder t → tys[i]? = tys'[i]?) →
      RuntimeType.ofTokenWith m tys t = RuntimeType.ofTokenWith m tys' t
  | .vector e, h => by
    simp only [RuntimeType.ofTokenWith]
    rw [ofTokenWith_agree e (fun i hi => h i (by simp [preorder, hi]))]
  | .structInstantiation hh args, h => by
    simp only [RuntimeType.ofTokenWith]
    rw [ofTokenListWith_agree args (fun i hi => h i (by simp [preorder, hi]))]
  | .typeParameter i, h => by
    simp only [RuntimeType.ofTokenWith]
    exact h i (by simp [preorder])
  | .function ps rs a, h => by
    simp only [RuntimeType.ofTokenWith]
    rw [ofSlotTokensWith_agree ps (fun i hi => h i (by simp [preorder, hi])),
      ofSlotTokensWith_agree rs (fun i hi => h i (by simp [preorder, hi]))]
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ | .i8, _ | .i16, _
  | .i32, _ | .i64, _ | .i128, _ | .i256, _ | .address, _ | .signer, _ | .struct _, _
  | .reference _, _ | .mutableReference _, _ => by
    simp only [RuntimeType.ofTokenWith]

theorem ofSlotTokensWith_agree {m : CompiledModule} {tys tys' : List RuntimeType} :
    ∀ (ts : List SignatureToken),
      (∀ i, SignatureToken.typeParameter i ∈ preorderList ts → tys[i]? = tys'[i]?) →
      RuntimeType.ofSlotTokensWith m tys ts = RuntimeType.ofSlotTokensWith m tys' ts
  | [], _ => rfl
  | .reference t :: ts, h => by
    simp only [RuntimeType.ofSlotTokensWith]
    rw [ofTokenWith_agree t (fun i hi => h i (by simp [preorderList, preorder, hi])),
      ofSlotTokensWith_agree ts (fun i hi => h i (by simp [preorderList, hi]))]
  | .mutableReference t :: ts, h => by
    simp only [RuntimeType.ofSlotTokensWith]
    rw [ofTokenWith_agree t (fun i hi => h i (by simp [preorderList, preorder, hi])),
      ofSlotTokensWith_agree ts (fun i hi => h i (by simp [preorderList, hi]))]
  | .bool :: ts, h | .u8 :: ts, h | .u16 :: ts, h | .u32 :: ts, h | .u64 :: ts, h
  | .u128 :: ts, h | .u256 :: ts, h | .i8 :: ts, h | .i16 :: ts, h | .i32 :: ts, h
  | .i64 :: ts, h | .i128 :: ts, h | .i256 :: ts, h | .address :: ts, h | .signer :: ts, h
  | .vector _ :: ts, h | .struct _ :: ts, h | .structInstantiation _ _ :: ts, h
  | .typeParameter _ :: ts, h | .function _ _ _ :: ts, h => by
    simp only [RuntimeType.ofSlotTokensWith]
    rw [ofTokenWith_agree _ (fun i hi => h i (by simp [preorderList, hi])),
      ofSlotTokensWith_agree ts (fun i hi => h i (by simp [preorderList, hi]))]

theorem ofTokenListWith_agree {m : CompiledModule} {tys tys' : List RuntimeType} :
    ∀ (ts : List SignatureToken),
      (∀ i, SignatureToken.typeParameter i ∈ preorderList ts → tys[i]? = tys'[i]?) →
      RuntimeType.ofTokenListWith m tys ts = RuntimeType.ofTokenListWith m tys' ts
  | [], _ => rfl
  | t :: ts, h => by
    simp only [RuntimeType.ofTokenListWith]
    rw [ofTokenWith_agree t (fun i hi => h i (by simp [preorderList, hi])),
      ofTokenListWith_agree ts (fun i hi => h i (by simp [preorderList, hi]))]
end

theorem ofTokenListWith_getElem {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {ts : List SignatureToken} {rs : List RuntimeType},
      RuntimeType.ofTokenListWith m tys ts = some rs → rs.length = ts.length ∧
        ∀ (p : Nat) (t : RuntimeType), rs[p]? = some t → ∃ tok, ts[p]? = some tok ∧ RuntimeType.ofTokenWith m tys tok = some t
  | [], rs, h => by
    simp only [RuntimeType.ofTokenListWith, Option.some.injEq] at h
    subst h
    simp
  | t :: ts, rs, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨r, hr, rs', hrs', rfl⟩ := h
    obtain ⟨hlen, hget⟩ := ofTokenListWith_getElem hrs'
    refine ⟨by simp [hlen], fun p u hu => ?_⟩
    cases p with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hu
      subst hu
      exact ⟨t, rfl, hr⟩
    | succ p => simpa using hget p u (by simpa using hu)

/-! ## Layers -/

/-- The lists of at most `n` elements of `S`. -/
def listsUpTo (S : List RuntimeType) : Nat → List (List RuntimeType)
  | 0 => [[]]
  | n + 1 => [] :: S.flatMap fun x => (listsUpTo S n).map (x :: ·)

theorem mem_listsUpTo {S : List RuntimeType} : ∀ (n : Nat) (l : List RuntimeType),
    l.length ≤ n → (∀ x ∈ l, x ∈ S) → l ∈ listsUpTo S n
  | 0, [], _, _ => by simp [listsUpTo]
  | 0, _ :: _, h, _ => by simp at h
  | _ + 1, [], _, _ => by simp [listsUpTo]
  | n + 1, x :: l, h, hx => by
    simp only [listsUpTo, List.mem_cons, List.mem_flatMap, List.mem_map]
    exact Or.inr ⟨x, hx x (List.mem_cons_self ..), l, mem_listsUpTo n l (by simp at h; omega)
      (fun y hy => hx y (List.mem_cons_of_mem _ hy)), rfl⟩

/-- Every token of the module's signature pool. -/
def tokensOf (m : CompiledModule) : List SignatureToken := m.signatures.toList.flatten

theorem mem_tokensOf {m : CompiledModule} {s : Nat} {sig : Signature} {p : Nat}
    {ty : SignatureToken} (hs : m.signatures[s]? = some sig) (hp : sig[p]? = some ty) :
    ty ∈ tokensOf m :=
  List.mem_flatten.2 ⟨sig, Array.mem_toList_iff.2 (Array.mem_of_getElem? hs),
    List.mem_of_getElem? hp⟩

/-- One more layer: the translations of the module's tokens under argument
lists drawn from `S`. -/
def stepS (m : CompiledModule) (L : Nat) (S : List RuntimeType) : List RuntimeType :=
  S ++ (tokensOf m).flatMap fun tok =>
    (listsUpTo S L).filterMap fun args => RuntimeType.ofTokenWith m args tok

def layer (m : CompiledModule) (L : Nat) (S0 : List RuntimeType) : Nat → List RuntimeType
  | 0 => S0
  | k + 1 => stepS m L (layer m L S0 k)

theorem layer_mono {m : CompiledModule} {L : Nat} {S0 : List RuntimeType} {x : RuntimeType} :
    ∀ {i j : Nat}, i ≤ j → x ∈ layer m L S0 i → x ∈ layer m L S0 j
  | i, 0, hij, h => by
    rw [Nat.le_zero.1 hij] at h
    exact h
  | i, j + 1, hij, h => by
    rcases Nat.lt_or_eq_of_le hij with hlt | heq
    · exact List.mem_append_left _ (layer_mono (Nat.le_of_lt_succ hlt) h)
    · rw [← heq]
      exact h

theorem mem_layer_succ {m : CompiledModule} {L : Nat} {S0 : List RuntimeType} {k : Nat}
    {tok : SignatureToken} {args : List RuntimeType} {r : RuntimeType} (htok : tok ∈ tokensOf m)
    (hargs : args ∈ listsUpTo (layer m L S0 k) L) (hr : RuntimeType.ofTokenWith m args tok = some r) :
    r ∈ layer m L S0 (k + 1) :=
  List.mem_append_right _ (List.mem_flatMap.2 ⟨tok, htok, List.mem_filterMap.2 ⟨args, hargs, hr⟩⟩)

/-- The total length of the signature pool. -/
def sigSum (m : CompiledModule) : Nat := (m.signatures.toList.map List.length).sum

theorem length_le_sum : ∀ (l : List Signature) (x : Signature), x ∈ l →
    x.length ≤ (l.map List.length).sum
  | [], _, h => absurd h List.not_mem_nil
  | y :: l, x, h => by
    simp only [List.map_cons, List.sum_cons]
    rcases List.mem_cons.1 h with rfl | h
    · omega
    · have := length_le_sum l x h
      omega

theorem length_le_sigSum {m : CompiledModule} {s : Nat} {sig : Signature}
    (h : m.signatures[s]? = some sig) : sig.length ≤ sigSum m :=
  length_le_sum _ _ (Array.mem_toList_iff.2 (Array.mem_of_getElem? h))

/-! ## The module's calls and the graph -/

theorem mapM_ok_mem {α β ε : Type} {f : α → Except ε β} :
    ∀ {l : List α} {r : List β}, l.mapM f = .ok r → ∀ x ∈ l, ∃ y ∈ r, f x = .ok y
  | [], _, _, x, hx => absurd hx List.not_mem_nil
  | a :: as, r, h, x, hx => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨b, List.mem_cons_self .., hb⟩
    · obtain ⟨y, hy, hfy⟩ := mapM_ok_mem hbs x hx
      exact ⟨y, List.mem_cons_of_mem _ hy, hfy⟩

/-- The definition a handle names, when no two definitions share a handle. -/
theorem handleToDef_eq_own {m : CompiledModule}
    (huniq : ∀ (d1 d2 : Nat) (fd1 fd2 : FunctionDefinition), m.functionDefs[d1]? = some fd1 →
      m.functionDefs[d2]? = some fd2 → fd1.function = fd2.function → d1 = d2)
    {d : Nat} {fd : FunctionDefinition} (hd : m.functionDefs[d]? = some fd) :
    handleToDef m fd.function = some d := by
  unfold handleToDef
  have hmem : (fd, d) ∈ m.functionDefs.toList.zipIdx.filter (·.1.function == fd.function) :=
    List.mem_filter.2 ⟨List.mem_zipIdx_iff_getElem?.2 (by simpa [Array.getElem?_toList] using hd),
      by simp⟩
  cases hl : (m.functionDefs.toList.zipIdx.filter (·.1.function == fd.function)).getLast? with
  | none =>
    rw [List.getLast?_eq_none_iff] at hl
    rw [hl] at hmem
    exact absurd hmem List.not_mem_nil
  | some y =>
    obtain ⟨ys, hys⟩ := List.getLast?_eq_some_iff.1 hl
    have hy : y ∈ m.functionDefs.toList.zipIdx.filter (·.1.function == fd.function) := by
      rw [hys]
      exact List.mem_append_right _ (List.mem_singleton_self _)
    obtain ⟨hyz, hyf⟩ := List.mem_filter.1 hy
    have hget := List.mem_zipIdx_iff_getElem?.1 hyz
    rw [Array.getElem?_toList] at hget
    simp only [beq_iff_eq] at hyf
    simp [huniq y.2 d y.1 fd hget hd hyf]

/-- A definition of the module's own function that a handle names. -/
theorem ownFunctionAt?_some {m : CompiledModule} {h d : Nat} (hown : m.ownFunctionAt? h = some d) :
    d < m.functionDefs.size ∧ ∃ fd, m.functionDefs[d]? = some fd ∧ fd.function = h := by
  simp only [CompiledModule.ownFunctionAt?, bind, Option.bind_eq_some_iff] at hown
  obtain ⟨fh, -, hown⟩ := hown
  split at hown
  · refine ⟨List.mem_range.1 (List.mem_of_find?_eq_some hown), ?_⟩
    have hp := List.find?_some hown
    cases hfd : m.functionDefs[d]? with
    | none => simp [hfd] at hp
    | some fd =>
      simp only [hfd, beq_iff_eq] at hp
      exact ⟨fd, rfl, hp⟩
  · simp at hown

/-- The edges a generic call of the module's own function adds. -/
theorem callGeneric_edges {m : CompiledModule} {es : List Edge} (hes : edges m = .ok es)
    {d : Nat} {fd : FunctionDefinition} {code : CodeUnit} (hd : m.functionDefs[d]? = some fd)
    (hc : fd.code = some code) {i : Nat} {instr : Bytecode} (hi : instr ∈ code.code.toList)
    (hform : instr = .callGeneric i ∨ ∃ mask, instr = .packClosureGeneric i mask)
    {x : FunctionInstantiation} (hx : m.functionInstantiations[i]? = some x) {d' : Nat}
    (hdef : handleToDef m x.handle = some d') {sig : Signature}
    (hsig : m.signatures[x.typeParameters]? = some sig) {p' : Nat} {ty : SignatureToken}
    (hty : sig[p']? = some ty) :
    (∀ a, ty = .typeParameter a → ⟨(d, a), (d', p'), false⟩ ∈ es) ∧
      ((∀ a, ty ≠ .typeParameter a) → ∀ q ∈ typeParams ty, ⟨(d, q), (d', p'), true⟩ ∈ es) := by
  rcases hform with rfl | ⟨mask, rfl⟩
  all_goals
    unfold edges at hes
    simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at hes
    obtain ⟨ess, hess, rfl⟩ := hes
    obtain ⟨el, hel, hfe⟩ := mapM_ok_mem hess (fd, d)
      (List.mem_zipIdx_iff_getElem?.2 (by simpa [Array.getElem?_toList] using hd))
    simp only [functionEdges, hc, bind_ok, pure, Except.pure, Except.ok.injEq] at hfe
    obtain ⟨els, hels, hel'⟩ := hfe
    obtain ⟨y, hy, hfy⟩ := mapM_ok_mem hels _ hi
    simp only [bind_ok, lookup_ok, hx, Option.some.injEq, exists_eq_left', hdef] at hfy
    unfold callEdges at hfy
    simp only [bind_ok, lookup_ok, hsig, Option.some.injEq, exists_eq_left', pure, Except.pure,
      Except.ok.injEq] at hfy
    subst hfy
    have hz : (ty, p') ∈ sig.zipIdx := List.mem_zipIdx_iff_getElem?.2 hty
    have hin : ∀ e, e ∈ (match ty with
        | .typeParameter actual => [(⟨(d, actual), (d', p'), false⟩ : Edge)]
        | _ => (typeParams ty).map fun p => ⟨(d, p), (d', p'), true⟩) → e ∈ ess.flatten := by
      intro e he
      refine List.mem_flatten.2 ⟨el, hel, ?_⟩
      rw [← hel']
      exact List.mem_flatten.2 ⟨_, hy, List.mem_flatMap.2 ⟨(ty, p'), hz, he⟩⟩
    refine ⟨fun a ha => ?_, fun hnot q hq => ?_⟩
    · subst ha
      exact hin _ (by simp)
    · exact hin ⟨(d, q), (d', p'), true⟩ (by
        cases ty <;> first
          | exact absurd rfl (hnot _)
          | exact List.mem_map.2 ⟨q, hq, rfl⟩)

/-! ## Reachable instances -/

/-- A reachable instance: its function is a definition of the module, it has at
most `L` type arguments, and each lies in the layer of its node's rank. -/
def InstGood (m : CompiledModule) (es : List Edge) (L : Nat) (S0 : List RuntimeType)
    (inst : LocalInstance) : Prop :=
  inst.1 < m.functionDefs.size ∧ inst.2.length ≤ L ∧
    ∀ (p : Nat) (t : RuntimeType), inst.2[p]? = some t → t ∈ layer m L S0 (rank es (inst.1, p))

section Step

variable {m : CompiledModule} {es : List Edge} {L : Nat} {S0 : List RuntimeType}

/-- A type argument of a generic call of the module's own function lies in the
layer of its node's rank. -/
theorem good_arg (hacc : ∀ e ∈ es, e.tyCon = true → ¬ Reach es e.dst e.src)
    (hS0 : RuntimeType.bool ∈ S0)
    (hS0c : ∀ tok ∈ tokensOf m, ∀ r, RuntimeType.ofTokenWith m [] tok = some r → r ∈ S0)
    {d d' : Nat} {τ : List RuntimeType} (hτ : τ.length ≤ L)
    (hgood : ∀ (p : Nat) (t : RuntimeType), τ[p]? = some t → t ∈ layer m L S0 (rank es (d, p)))
    {s : Nat} {sig : Signature} (hsig : m.signatures[s]? = some sig) {p' : Nat}
    {ty : SignatureToken} (hty : sig[p']? = some ty) {t : RuntimeType}
    (htr : RuntimeType.ofTokenWith m τ ty = some t)
    (hid : ∀ a, ty = .typeParameter a → ⟨(d, a), (d', p'), false⟩ ∈ es)
    (hcon : (∀ a, ty ≠ .typeParameter a) → ∀ q ∈ typeParams ty, ⟨(d, q), (d', p'), true⟩ ∈ es) :
    t ∈ layer m L S0 (rank es (d', p')) := by
  have htok := mem_tokensOf hsig hty
  by_cases hta : ∃ a, ty = .typeParameter a
  · -- The argument is one of the caller's: an identity edge.
    obtain ⟨a, rfl⟩ := hta
    have he := hid a rfl
    simp only [RuntimeType.ofTokenWith] at htr
    exact layer_mono (rank_mono (Reach.edge he)) (hgood a t htr)
  · have hnot : ∀ a, ty ≠ .typeParameter a := fun a h => hta ⟨a, h⟩
    cases hps : typeParams ty with
    | nil =>
      -- A closed type: in the first layer.
      have hagree := ofTokenWith_agree (m := m) (tys := τ) (tys' := []) ty (fun i hi => by
        have := mem_typeParams.2 hi
        rw [hps] at this
        exact absurd this List.not_mem_nil)
      rw [hagree] at htr
      exact layer_mono (Nat.zero_le _) (hS0c ty htok t htr)
    | cons q0 qs =>
      -- A larger type: type-constructor edges from each type parameter it mentions.
      have hlt : ∀ q ∈ typeParams ty, rank es (d, q) < rank es (d', p') := fun q hq => by
        have he := hcon hnot q hq
        exact tyRank_lt he rfl (hacc _ he rfl)
      have hpos : 0 < rank es (d', p') :=
        Nat.lt_of_le_of_lt (Nat.zero_le _) (hlt q0 (by rw [hps]; exact List.mem_cons_self ..))
      obtain ⟨k, hk⟩ : ∃ k, rank es (d', p') = k + 1 := ⟨_, (Nat.succ_pred_eq_of_pos hpos).symm⟩
      rw [hk]
      let args : List RuntimeType :=
        τ.mapIdx fun i x => if (typeParams ty).contains i then x else RuntimeType.bool
      have hagree := ofTokenWith_agree (m := m) (tys := args) (tys' := τ) ty (fun i hi => by
        have hin : i ∈ typeParams ty := mem_typeParams.2 hi
        simp only [args, List.getElem?_mapIdx]
        cases τ[i]? <;> simp [hin])
      have hargs : args ∈ listsUpTo (layer m L S0 k) L := by
        refine mem_listsUpTo L args (by simp [args, hτ]) fun x hx => ?_
        obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
        simp only [args, List.getElem?_mapIdx, Option.map_eq_some_iff] at hi
        obtain ⟨y, hy, rfl⟩ := hi
        by_cases hin : (typeParams ty).contains i = true
        · simp only [hin, ↓reduceIte]
          have := hlt i (List.contains_iff_mem.1 hin)
          exact layer_mono (by omega) (hgood i y hy)
        · simp only [hin, Bool.false_eq_true, ↓reduceIte]
          exact layer_mono (Nat.zero_le _) hS0
      exact mem_layer_succ htok hargs (by rw [hagree]; exact htr)

/-- **One call keeps the invariant.** -/
theorem good_call (hes : edges m = .ok es)
    (hacc : ∀ e ∈ es, e.tyCon = true → ¬ Reach es e.dst e.src)
    (huniq : ∀ (d1 d2 : Nat) (fd1 fd2 : FunctionDefinition), m.functionDefs[d1]? = some fd1 →
      m.functionDefs[d2]? = some fd2 → fd1.function = fd2.function → d1 = d2)
    (hL : ∀ (s : Nat) (sig : Signature), m.signatures[s]? = some sig → sig.length ≤ L)
    (hS0 : RuntimeType.bool ∈ S0)
    (hS0c : ∀ tok ∈ tokensOf m, ∀ r, RuntimeType.ofTokenWith m [] tok = some r → r ∈ S0)
    {j k : LocalInstance} (hj : InstGood m es L S0 j) (hk : k ∈ m.ownCalls j.1 j.2) :
    InstGood m es L S0 k := by
  obtain ⟨d, τ⟩ := j
  obtain ⟨-, hτ, hgood⟩ := hj
  simp only [CompiledModule.ownCalls] at hk
  split at hk
  · simp at hk
  · next fd hd =>
    split at hk
    · simp at hk
    · next code hc =>
      obtain ⟨instr, hinstr, hk⟩ := List.mem_filterMap.1 hk
      cases instr <;> simp only [Bytecode.directCall?, reduceCtorEq] at hk
      case call h =>
        simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hk
        obtain ⟨d', hown, rfl⟩ := hk
        exact ⟨(ownFunctionAt?_some hown).1, by simp, fun p t ht => by simp at ht⟩
      case packClosure h mask =>
        simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hk
        obtain ⟨d', hown, rfl⟩ := hk
        exact ⟨(ownFunctionAt?_some hown).1, by simp, fun p t ht => by simp at ht⟩
      case packClosureGeneric i mask =>
        simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hk
        obtain ⟨x, hx, d', hown, τ', hτ', rfl⟩ := hk
        simp only [CompiledModule.typeArgsWith, bind, Option.bind_eq_some_iff] at hτ'
        obtain ⟨sig, hsig, hτ'⟩ := hτ'
        obtain ⟨hd'lt, fd', hfd', hfun⟩ := ownFunctionAt?_some hown
        have hdef : handleToDef m x.handle = some d' := by
          rw [← hfun]
          exact handleToDef_eq_own huniq hfd'
        obtain ⟨hlen, hget⟩ := ofTokenListWith_getElem hτ'
        refine ⟨hd'lt, by rw [hlen]; exact hL _ _ hsig, fun p' t ht => ?_⟩
        obtain ⟨ty, hty, htr⟩ := hget p' t ht
        obtain ⟨hid, hcon⟩ := callGeneric_edges hes hd hc hinstr (Or.inr ⟨mask, rfl⟩) hx hdef
          hsig hty
        exact good_arg hacc hS0 hS0c hτ hgood hsig hty htr hid hcon
      case callGeneric i =>
        simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hk
        obtain ⟨x, hx, d', hown, τ', hτ', rfl⟩ := hk
        simp only [CompiledModule.typeArgsWith, bind, Option.bind_eq_some_iff] at hτ'
        obtain ⟨sig, hsig, hτ'⟩ := hτ'
        obtain ⟨hd'lt, fd', hfd', hfun⟩ := ownFunctionAt?_some hown
        have hdef : handleToDef m x.handle = some d' := by
          rw [← hfun]
          exact handleToDef_eq_own huniq hfd'
        obtain ⟨hlen, hget⟩ := ofTokenListWith_getElem hτ'
        refine ⟨hd'lt, by rw [hlen]; exact hL _ _ hsig, fun p' t ht => ?_⟩
        obtain ⟨ty, hty, htr⟩ := hget p' t ht
        obtain ⟨hid, hcon⟩ := callGeneric_edges hes hd hc hinstr (Or.inl rfl) hx hdef hsig hty
        exact good_arg hacc hS0 hS0c hτ hgood hsig hty htr hid hcon

end Step

/-- **L7** for a module the model accepts: from every function of the module at
any closed type arguments, the instances reachable through the module's own
calls are finitely many. The instantiation-loop check, and that no two
function definitions share a handle (L2), suffice. -/
theorem instantiationsFinite_of {m : CompiledModule} (h : verifyInstantiationLoops m = .ok ())
    (huniq : ∀ (d1 d2 : Nat) (fd1 fd2 : FunctionDefinition), m.functionDefs[d1]? = some fd1 →
      m.functionDefs[d2]? = some fd2 → fd1.function = fd2.function → d1 = d2) :
    m.InstantiationsFinite := by
  unfold verifyInstantiationLoops at h
  simp only [bind_ok] at h
  obtain ⟨es, hes, h⟩ := h
  have hacc : ∀ e ∈ es, e.tyCon = true → ¬ Reach es e.dst e.src := by
    intro e he ht hr
    split at h
    · simp [reject] at h
    · next hany =>
      exact hany (List.any_eq_true.2 ⟨e, he, by simp [ht, reaches_complete hr]⟩)
  intro d fd fh tys hd hfh hlen
  clear hfh hlen
  let L := tys.length + sigSum m
  let S0 := RuntimeType.bool :: (tys ++ (tokensOf m).filterMap (RuntimeType.ofTokenWith m []))
  have hS0 : RuntimeType.bool ∈ S0 := List.mem_cons_self ..
  have hS0c : ∀ tok ∈ tokensOf m, ∀ r, RuntimeType.ofTokenWith m [] tok = some r → r ∈ S0 :=
    fun tok htok r hr => List.mem_cons_of_mem _ (List.mem_append_right _
      (List.mem_filterMap.2 ⟨tok, htok, hr⟩))
  have hL : ∀ (s : Nat) (sig : Signature), m.signatures[s]? = some sig → sig.length ≤ L :=
      fun s sig hs => by
    have := length_le_sigSum hs
    omega
  refine ⟨(List.range m.functionDefs.size).flatMap fun d =>
    (listsUpTo (layer m L S0 es.length) L).map fun τ => (d, τ), fun i hi => ?_⟩
  have hgood : InstGood m es L S0 i := by
    induction hi with
    | refl =>
      refine ⟨lt_of_getElem? hd, Nat.le_add_right _ _, fun p t ht => ?_⟩
      exact layer_mono (Nat.zero_le _)
        (List.mem_cons_of_mem _ (List.mem_append_left _ (List.mem_of_getElem? ht)))
    | call _ hk ih => exact good_call hes hacc huniq hL hS0 hS0c ih hk
  obtain ⟨d', τ⟩ := i
  obtain ⟨hlt, hlen, hel⟩ := hgood
  refine List.mem_flatMap.2 ⟨d', List.mem_range.2 hlt, List.mem_map.2 ⟨τ, ?_, rfl⟩⟩
  refine mem_listsUpTo L τ hlen fun x hx => ?_
  obtain ⟨p, hp⟩ := List.mem_iff_getElem?.1 hx
  exact layer_mono (rank_le es _) (hel p x hp)

end MoveBytecode.Proofs
