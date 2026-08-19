// use-aptos-stdlib
// Nested `map_ref` uses the inner call's result and abort summary.
module 0x42::nested_map_ref {
    fun nested(v: &vector<vector<u64>>): vector<vector<u64>> {
        v.map_ref(|inner| inner.map_ref(|x| *x + 1))
    }
    spec nested {
        requires forall i in 0..len(v):
            forall j in 0..len(v[i]): v[i][j] < MAX_U64;
        aborts_if false;
    }

    fun captured_mutation(v: &vector<vector<u64>>): u64 {
        let count = 0;
        let _ = v.map_ref(|inner| inner.map_ref(|x| {
            count = count + 1;
            *x
        } spec {
            aborts_if false;
            ensures result == x;
        }));
        count
    }

    spec captured_mutation {
        requires len(v) == 1 && len(v[0]) == 1;
        ensures result == 0; // error: the nested lambda increments count
    }
}
