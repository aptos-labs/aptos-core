spec velor_std::crypto_algebra {

    spec handles_from_elements<S>(elements: &vector<Element<S>>): vector<u64> {
        aborts_if false;
        ensures forall i in 0..len(elements): result[i] == elements[i].handle;
    }

    spec add_internal<S>(handle_1: u64, handle_2: u64): u64 {
        pragma opaque;
    }

    spec deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64) {
        pragma opaque;
    }

    spec div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64) {
        pragma opaque;
    }

    spec double_internal<G>(element_handle: u64): u64 {
        pragma opaque;
    }

    spec downcast_internal<L,S>(handle: u64): (bool, u64) {
        pragma opaque;
    }

    spec from_u64_internal<S>(value: u64): u64 {
        pragma opaque;
    }

    spec eq_internal<S>(handle_1: u64, handle_2: u64): bool {
        pragma opaque;
    }

    spec hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64 {
        pragma opaque;
    }

    spec inv_internal<F>(handle: u64): (bool, u64) {
        pragma opaque;
    }

    spec mul_internal<F>(handle_1: u64, handle_2: u64): u64 {
        pragma opaque;
    }

    spec multi_pairing_internal<G1,G2,Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64 {
        pragma opaque;
    }

    spec multi_scalar_mul_internal<G, S>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64 {
        pragma opaque;
    }

    spec neg_internal<F>(handle: u64): u64 {
        pragma opaque;
    }

    spec one_internal<S>(): u64 {
        pragma opaque;
    }

    spec order_internal<G>(): vector<u8> {
        pragma opaque;
    }

    spec pairing_internal<G1,G2,Gt>(g1_handle: u64, g2_handle: u64): u64 {
        pragma opaque;
    }

    spec scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64 {
        pragma opaque;
    }

    spec serialize_internal<S, F>(handle: u64): vector<u8> {
        pragma opaque;
    }

    spec sqr_internal<G>(handle: u64): u64 {
        pragma opaque;
    }

    spec sub_internal<G>(handle_1: u64, handle_2: u64): u64 {
        pragma opaque;
    }

    spec upcast_internal<S,L>(handle: u64): u64 {
        pragma opaque;
    }

    spec zero_internal<S>(): u64 {
        pragma opaque;
    }

}
