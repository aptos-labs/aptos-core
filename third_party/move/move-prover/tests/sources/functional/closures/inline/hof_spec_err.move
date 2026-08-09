// Function spec blocks on inline functions with function-typed parameters are
// not supported: the spec would need to refer to the behavior of the lambda
// arguments.
module 0x42::hof_spec_err {
    use std::vector;

    inline fun map(v: &vector<u64>, f: |u64| u64): vector<u64> {
        let result = vector::empty();
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            vector::push_back(&mut result, f(*vector::borrow(v, i)));
            i = i + 1;
        };
        result
    }
    spec map { // error: specs on inline functions with function-typed parameters are not supported
        pragma opaque;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == result_of<f>(v[i]);
    }

    fun doubles(v: &vector<u64>): vector<u64> {
        map(v, |e| e * 2)
    }
}
