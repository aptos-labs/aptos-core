module 0x12391283::M {
    use std::vector;

    fun vector_for_each<Element>(v: vector<Element>, f: |Element|) {
        vector::reverse(&mut v); // We need to reverse the vector to consume it efficiently
        while (!vector::is_empty(&v)) {
            let e = vector::pop_back(&mut v);
            f(e);
        };
    }

    fun vector_fold<Accumulator, Element>(
        v: vector<Element>,
        init: Accumulator,
        f: |Accumulator,Element|Accumulator
    ): Accumulator {
        let accu = init;
        vector_for_each(v, |elem| accu = f(accu, elem));
        accu
    }

    fun test(gas_schedule_blob: vector<u8>) : u64 {
        let gas_schedule_blob: vector<vector<u8>> = vector[
            vector[115], vector[115], vector[95],
        ];
        vector_fold(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }
}
