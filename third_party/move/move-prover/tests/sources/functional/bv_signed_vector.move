// exclude_for: cvc5
// Regression: bv twin generation for vector instantiations recurses into the
// element type, so a `vector<i64>` — not itself signed — used to reach the
// signed arm of `boogie_type` ("signed integer cannot be turned into bit
// vector"). Signed-CONTAINING types are now excluded from bv twins.
module 0x42::bv_signed_vector {

    public fun sum2(v: &vector<i64>): i64 {
        v[0] + v[1]
    }

    spec sum2 {
        aborts_if len(v) < 2;
        aborts_if v[0] + v[1] > MAX_I64;
        aborts_if v[0] + v[1] < MIN_I64;
        ensures result == v[0] + v[1];
    }
}
