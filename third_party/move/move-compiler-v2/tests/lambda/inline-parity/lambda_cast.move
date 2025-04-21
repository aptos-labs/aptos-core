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

    fun test_1() : u64 {
        let gas_schedule_blob: vector<u8> = vector[
            115, 115, 95, 112, 97, 99, 107, 101, 100, 32, 0, 0, 0, 0, 0, 0, 0,
        ];
        vector_fold<u64, u8>(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }

    fun test_2() : u64 {
        let gas_schedule_blob: vector<u8> = vector[
            115, 115, 95, 112, 97, 99, 107, 101, 100, 32, 0, 0, 0, 0, 0, 0, 0,
        ];
        vector_fold(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }
}
