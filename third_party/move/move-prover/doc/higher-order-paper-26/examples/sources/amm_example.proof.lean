-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The denominator contains the input, so division cannot exceed the output
-- reserve; a positive input reserve makes that bound strict.
verify constant_product by
  all_goals
    have bound : reserve_out.val * amount_in.val / (reserve_in.val + amount_in.val)
        ≤ reserve_out.val := by
      apply Int.ediv_le_of_le_mul (by omega)
      exact Int.mul_le_mul_of_nonneg_left (by omega) (by omega)
    first
      | omega
      | (intro positive_in positive_out positive_amount
         apply Int.ediv_lt_of_lt_mul (by omega)
         exact Int.mul_lt_mul_of_pos_left (by omega) positive_out)

-- Bounds for fees and rounded constant-product swaps.
theorem effective_bound (a scale : Int) (ha : 0 ≤ a) (hs : 0 ≤ scale) (hs' : scale ≤ 10000) :
    0 ≤ a * scale / 10000 ∧ a * scale / 10000 ≤ a := by
  constructor
  · exact Int.ediv_nonneg (Int.mul_nonneg ha hs) (by decide)
  · apply Int.ediv_le_of_le_mul (by decide)
    exact Int.mul_le_mul_of_nonneg_left hs' ha

theorem market_bounds (r o a e : Int) (hr : 0 ≤ r) (ho : 0 ≤ o)
    (ho' : o ≤ 18446744073709551615) (ha : a ≤ 18446744073709551615)
    (he : 0 ≤ e) (hea : e ≤ a) :
    (0 ≤ o * e ∧ o * e ≤ 340282366920938463463374607431768211455) ∧
    (0 ≤ o * e / (r + e) ∧ o * e / (r + e) ≤ o) ∧
    r * o ≤ (r + a) * (o - o * e / (r + e)) := by
  have product_nonneg := Int.mul_nonneg ho he
  have product_bound := Int.mul_le_mul ho' (show e ≤ 18446744073709551615 by omega) he (by omega)
  refine ⟨⟨product_nonneg, by omega⟩, ?_⟩
  by_cases zero : e = 0
  · subst e
    simp only [Int.mul_zero, Int.zero_ediv, Int.sub_zero]
    exact ⟨⟨by omega, ho⟩, Int.mul_le_mul_of_nonneg_right (by omega) ho⟩
  · have hden : 0 < r + e := by omega
    have bound : o * e / (r + e) ≤ o :=
      Int.ediv_le_of_le_mul hden (Int.mul_le_mul_of_nonneg_left (by omega) ho)
    refine ⟨⟨Int.ediv_nonneg product_nonneg (by omega), bound⟩, ?_⟩
    have divided := Int.ediv_mul_le (o * e) (Int.ne_of_gt hden)
    have extension := Int.mul_nonneg (Int.sub_nonneg_of_le hea) (Int.sub_nonneg_of_le bound)
    grind

verify constant_product_with_fee by
  all_goals
    have bound := effective_bound amount_in.val 9500 (by omega) (by omega) (by omega)
    have facts := market_bounds reserve_in.val reserve_out.val amount_in.val
      (amount_in.val * 9500 / 10000) (by omega) (by omega) (by omega) (by omega) bound.1 bound.2
    first
      | omega
      | (simpa only [Int.sub_zero] using
          Int.mul_le_mul_of_nonneg_right
            (show reserve_in.val ≤ reserve_in.val + amount_in.val by omega)
            (show 0 ≤ reserve_out.val by omega))
      | (let fee : LeanerIR.SpecInt (.bits 64) false := by assumption
         have feeBounds : 0 ≤ fee.val ∧ fee.val ≤ 10000 := by
           dsimp only [fee]
           omega
         have boundFee := effective_bound amount_in.val (10000 - fee.val)
           (by omega) (by omega) (by omega)
         have factsFee := market_bounds reserve_in.val reserve_out.val amount_in.val
           (amount_in.val * (10000 - fee.val) / 10000)
           (by omega) (by omega) (by omega) (by omega) boundFee.1 boundFee.2
         try dsimp only [fee] at boundFee factsFee
         omega)

verify constant_product_with_fee_non_compliant by
  all_goals
    let fee : LeanerIR.SpecInt (.bits 64) false := by assumption
    have feeBounds : 0 ≤ fee.val ∧ fee.val ≤ 10000 := by
      dsimp only [fee]
      omega
    have bound := effective_bound amount_in.val (10000 - fee.val)
      (by omega) (by omega) (by omega)
    have facts := market_bounds reserve_in.val reserve_out.val amount_in.val
      (amount_in.val * (10000 - fee.val) / 10000)
      (by omega) (by omega) (by omega) (by omega) bound.1 bound.2
    dsimp only [fee] at bound facts
    try simp only [reduceCtorEq, false_or]
    omega

-- Rounded constant-product pricing is monotone in its input.
theorem ratio_monotone (r o a b : Int) (hr : 0 ≤ r) (ho : 0 ≤ o)
    (_ha : 0 ≤ a) (hab : a ≤ b) (hden : 0 < r + a) :
    o * a / (r + a) ≤ o * b / (r + b) := by
  have bound : o * a / (r + a) ≤ o :=
    Int.ediv_le_of_le_mul hden (Int.mul_le_mul_of_nonneg_left (by omega) ho)
  have divided := Int.ediv_mul_le (o * a) (Int.ne_of_gt hden)
  have extension := Int.mul_le_mul_of_nonneg_right bound (show 0 ≤ b - a by omega)
  apply Int.le_ediv_of_mul_le (by omega)
  grind

verify create_constant_product_pool by
  all_goals first
    | (have nonneg := Int.mul_nonneg
          (show 0 ≤ quantified_19 by omega) (show 0 ≤ quantified_20 by omega)
       simp only [Int.tdiv_eq_ediv_of_nonneg nonneg] at *
       have bound := market_bounds quantified_18 quantified_19 quantified_20 quantified_20
         (by omega) (by omega) (by omega) (by omega) (by omega) (by omega)
       grind)
    | (have nonneg1 := Int.mul_nonneg
          (show 0 ≤ quantified_14 by omega) (show 0 ≤ quantified_15 by omega)
       have nonneg2 := Int.mul_nonneg
          (show 0 ≤ quantified_14 by omega) (show 0 ≤ quantified_16 by omega)
       simp only [Int.tdiv_eq_ediv_of_nonneg nonneg1, Int.tdiv_eq_ediv_of_nonneg nonneg2] at *
       by_cases zero : quantified_13 + quantified_15 = 0
       · grind
       · have monotone := ratio_monotone quantified_13 quantified_14 quantified_15 quantified_16
           (by omega) (by omega) (by omega) (by omega) (by omega)
         grind)
    | (leaner_denote_memory_invariants
       all_goals try dsimp only [storedInvariant]
       all_goals simp_all [lir_denote_norm, LeanerIR.Proofs.Denote.NTy.encode_function,
         LeanerIR.Proofs.Obligation_iff, LeanerIR.Proofs.Denote.IntegerValueFits_unsigned_succ]
       all_goals grind)

-- Specialize the quantified pricing laws, then reduce its singleton result.
verify swap by
  all_goals try clear leanerContinuation
  all_goals simp_all [lir_denote_norm, LeanerIR.Proofs.Denote.NTy.encode_function]
  all_goals grind [LeanerIR.packResults_single]

-- Selecting a bounded fee preserves monotonicity of the effective input.
open LeanerIR LeanerIR.Proofs.Denote in
theorem selected_fee_bounds (slot : Option (SpecInt (.bits 64) false × Unit)) :
    let raw := ((slot.map fun value => RuntimeValue.integer value.1.val).getD RuntimeValue.unit).asInt
    let fee := if slot.isSome = true ∧ raw ≤ 10000 then raw else 500
    0 ≤ fee ∧ fee ≤ 10000 := by
  cases slot with
  | none => decide
  | some value =>
    have bounds := value.1.fits
    simp only [IntegerValueFits_unsigned_succ] at bounds
    dsimp [RuntimeValue.asInt]
    split <;> omega

theorem fee_monotone (r o a b fee : Int) {q1 q2 : Int}
    (hr : 0 ≤ r) (ho : 0 ≤ o) (ha : 0 ≤ a) (hab : a ≤ b)
    (hf : 0 ≤ fee) (hf' : fee ≤ 10000) (hq2 : 0 ≤ q2)
    (zero1 : (a * (10000 - fee)).tdiv 10000 = 0 → q1 = 0)
    (value1 : 0 < (a * (10000 - fee)).tdiv 10000 →
      q1 = (o * ((a * (10000 - fee)).tdiv 10000)).tdiv
        (r + (a * (10000 - fee)).tdiv 10000))
    (value2 : 0 < (b * (10000 - fee)).tdiv 10000 →
      q2 = (o * ((b * (10000 - fee)).tdiv 10000)).tdiv
        (r + (b * (10000 - fee)).tdiv 10000)) : q1 ≤ q2 := by
  have scaled1 := Int.mul_nonneg ha (show 0 ≤ 10000 - fee by omega)
  have scaled2 := Int.mul_nonneg (show 0 ≤ b by omega) (show 0 ≤ 10000 - fee by omega)
  have eff1 : 0 ≤ a * (10000 - fee) / 10000 := Int.ediv_nonneg scaled1 (by decide)
  have eff2 : 0 ≤ b * (10000 - fee) / 10000 := Int.ediv_nonneg scaled2 (by decide)
  simp only [Int.tdiv_eq_ediv_of_nonneg scaled1, Int.tdiv_eq_ediv_of_nonneg scaled2,
    Int.tdiv_eq_ediv_of_nonneg (Int.mul_nonneg ho eff1),
    Int.tdiv_eq_ediv_of_nonneg (Int.mul_nonneg ho eff2)] at zero1 value1 value2
  have increasing := Int.ediv_le_ediv (show (0 : Int) < 10000 by decide)
    (Int.mul_le_mul_of_nonneg_right hab (show 0 ≤ 10000 - fee by omega))
  by_cases zero : a * (10000 - fee) / 10000 = 0
  · have := zero1 zero
    omega
  · rw [value1 (by omega), value2 (by omega)]
    exact ratio_monotone r o _ _ hr ho eff1 increasing (by omega)

verify create_compliant_fee_pool by
  all_goals first
    | (leaner_denote_memory_invariants
       all_goals try dsimp only [storedInvariant]
       all_goals simp_all [lir_denote_norm, LeanerIR.Proofs.Denote.NTy.encode_function,
         LeanerIR.Proofs.Obligation_iff, LeanerIR.Proofs.Denote.IntegerValueFits_unsigned_succ])
    | (try clear leanerContinuation
       run_tac
         let goal ← Lean.Elab.Tactic.getMainGoal
         goal.withContext do
          let hyps ← goal.getNondepPropHyps
          let mut fee? : Option Lean.Expr := none
          for hyp in hyps do
            let ty ← Lean.instantiateMVars (← hyp.getType)
            if let some fee := ty.find? (fun e => e.isAppOfArity ``ite 5 &&
                e.getArg! 0 == Lean.mkConst ``Int && e.appArg! == Lean.toExpr (500 : Int)) then
              fee? := some fee
              break
          let some fee := fee? | throwError "no selected fee"
          let (_, _, next) ← goal.generalizeHyp #[{ expr := fee, xName? := some `fee, hName? := some `feeDef }] hyps
          Lean.Elab.Tactic.replaceMainGoal [next]
       have feeBounds : 0 ≤ fee ∧ fee ≤ 10000 := by
         rw [← feeDef]
         exact selected_fee_bounds _
       apply fee_monotone quantified_13 quantified_14 quantified_15 quantified_16 fee
       all_goals first | assumption | omega)
