module 0x12391283::M {
    use std::vector;
    fun test_1() : u64 {
        let gas_schedule_blob: vector<u8> = vector[
            115, 115, 95, 112, 97, 99, 107, 101, 100, 32, 0, 0, 0, 0, 0, 0, 0,
        ];
        vector::fold<u64, u8>(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }

    fun test_2() : u64 {
        let gas_schedule_blob: vector<u8> = vector[
            115, 115, 95, 112, 97, 99, 107, 101, 100, 32, 0, 0, 0, 0, 0, 0, 0,
        ];
        vector::fold(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }
}
