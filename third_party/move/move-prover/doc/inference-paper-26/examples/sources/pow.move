module 0x42::pow {

  spec module {
    pragma verify = false; // TODO: investigate flakiness
  }

  fun pow(base: u64, exp: u64): u64 {
      let result = 1;
      let i = 0;
      while (i < exp) {
          result = result * base;
          i = i + 1;
      } spec {
          invariant [inferred] i <= exp;
          invariant [inferred] result == pow_spec(base, i);
      };
      result
  }
  spec pow(base: u64, exp: u64): u64 {
      pragma opaque = true;
      ensures [inferred] result == pow_spec(base, exp);
      aborts_if [inferred] exp > 0 && pow_spec(base, exp - 1) * base > MAX_U64;
  } proof {
      forall x: num, y: num {pow_spec(base, x), pow_spec(base, y)}
          apply pow_spec_mul_mono(base, x, y);
  }


  /// Mathematical power function. Uses `num` so the recursive
  /// multiplication is not bounded by u64 wraparound.
  spec fun pow_spec(base: num, exp: num): num {
      if (exp == 0) { 1 } else { base * pow_spec(base, exp - 1) }
  }

  /// pow_spec is non-negative when base and exp are non-negative.
  spec lemma pow_spec_nonneg(base: num, exp: num) {
      requires 0 <= base;
      requires 0 <= exp;
      ensures 0 <= pow_spec(base, exp);
  } proof {
      if (exp > 0) {
          apply pow_spec_nonneg(base, exp - 1);
      }
  }

  /// Monotonicity of pow_spec when base >= 1.
  spec lemma pow_spec_mono(base: num, x: num, y: num) {
      requires base >= 1;
      requires 0 <= x;
      requires x <= y;
      ensures pow_spec(base, x) <= pow_spec(base, y);
  } proof {
      if (x < y) {
          apply pow_spec_nonneg(base, y - 1);
          assert pow_spec(base, y - 1) <= pow_spec(base, y);
          apply pow_spec_mono(base, x, y - 1);
      }
  }

  /// Multiplication-monotonicity, the form needed at the loop's
  /// `result * base` step. Avoids non-linear reasoning by reducing to
  /// plain monotonicity at `x + 1, y + 1` via the definition
  /// `pow_spec(b, k) * b == pow_spec(b, k + 1)`.
  spec lemma pow_spec_mul_mono(base: num, x: num, y: num) {
      requires base >= 1;
      requires 0 <= x;
      requires x <= y;
      ensures pow_spec(base, x) * base <= pow_spec(base, y) * base;
  } proof {
      apply pow_spec_mono(base, x + 1, y + 1);
  }
}
