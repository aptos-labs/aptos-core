module 0x12391283::M {
    use std::vector;
    fun test(gas_schedule_blob: vector<u8>) : u64 {
        let gas_schedule_blob: vector<vector<u8>> = vector[
            vector[115], vector[115], vector[95],
        ];
        vector::fold(gas_schedule_blob, (0 as u64), |sum, addend| sum + (addend as u64))
    }
}
