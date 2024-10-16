pragma circom 2.0.3;

// File copied from https://github.com/doubleblind-xyz/circom-rsa/blob/master/circuits/fp.circom

include "circomlib/circuits/bitify.circom";

include "./bigint.circom";
include "./bigint_func.circom";

// These functions operate over values in Z/Zp for some integer p (typically,
// but not necessarily prime). Values are stored as standard bignums with k
// chunks of n bits, but intermediate values often have "overflow" bits inside
// various chunks.
//
// These Fp functions will always correctly generate witnesses mod p, but they
// do not *check* that values are normalized to < p; they only check that
// values are correct mod p. This is to save the comparison circuit.
// They *will* always check for intended results mod p (soundness), but it may
// not have a unique intermediate signal.
//
// Conversely, some templates may not be satisfiable if the input witnesses are
// not < p. This does not break completeness, as honest provers will always
// generate witnesses which are canonical (between 0 and p).

// a * b = r mod p
// a * b - p * q - r for some q
template FpMul(n, k) {
    assert(n + n + log_ceil(k) + 2 <= 252);
    signal input a[k];
    signal input b[k];
    signal input p[k];

    signal output out[k];

    signal v_ab[2*k-1];
    for (var x = 0; x < 2*k-1; x++) {
        var v_a = poly_eval(k, a, x);
        var v_b = poly_eval(k, b, x);
        v_ab[x] <== v_a * v_b;
    }

    var ab[200] = poly_interp(2*k-1, v_ab);
    // ab_proper has length 2*k
    var ab_proper[200] = getProperRepresentation(n + n + log_ceil(k), n, 2*k-1, ab);

    var long_div_out[2][100] = long_div(n, k, k, ab_proper, p);

    // Since we're only computing a*b, we know that q < p will suffice, so we
    // know it fits into k chunks and can do size n range checks.
    signal q[k];
    component q_range_check[k];
    signal r[k];
    component r_range_check[k];
    for (var i = 0; i < k; i++) {
        q[i] <-- long_div_out[0][i];
        q_range_check[i] = Num2Bits(n);
        q_range_check[i].in <== q[i];

        r[i] <-- long_div_out[1][i];
        r_range_check[i] = Num2Bits(n);
        r_range_check[i].in <== r[i];
    }

    signal v_pq_r[2*k-1];
    for (var x = 0; x < 2*k-1; x++) {
        var v_p = poly_eval(k, p, x);
        var v_q = poly_eval(k, q, x);
        var v_r = poly_eval(k, r, x);
        v_pq_r[x] <== v_p * v_q + v_r;
    }

    signal v_t[2*k-1];
    for (var x = 0; x < 2*k-1; x++) {
        v_t[x] <== v_ab[x] - v_pq_r[x];
    }

    var t[200] = poly_interp(2*k-1, v_t);
    component tCheck = CheckCarryToZero(n, n + n + log_ceil(k) + 2, 2*k-1);
    for (var i = 0; i < 2*k-1; i++) {
        tCheck.in[i] <== t[i];
    }

    for (var i = 0; i < k; i++) {
        out[i] <== r[i];
    }
}
